//! What cancellation and spawning reveal about erased body ownership.
//!
//! Each test drives the real specimen seam in `peregrine_web::spike`. The
//! fixture enforcer suspends inside admission, so a polled invocation is
//! genuinely pending when it is dropped; the drop probe then shows whether the
//! invocation or its caller released the body. A spawned invocation proves the
//! same seam is `Send` when it owns everything it touches.

#[path = "support/spike/admission.rs"]
mod spike_admission;
#[path = "support/spike/components.rs"]
mod spike_components;
#[path = "support/spike/fixtures.rs"]
mod spike_fixtures;
#[path = "support/spike/frames.rs"]
mod spike_frames;
#[path = "support/spike/synthetic.rs"]
mod spike_synthetic;

use std::{
    error::Error,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use bytes::Bytes;
use googletest::prelude::{eq, verify_that};
use http::HeaderMap;
use http_body_util::Empty;
use peregrine_web::spike::{
    ErasedBody,
    Identity,
    InvocationOutcome,
    Method,
    RequestView,
    ResourceRunner,
    adapt_body,
};
use rstest::rstest;
use spike_components::{CatalogueResource, OwnedResource};
use spike_fixtures::{as_enforcer, enforcer_with, protected_catalogue, public_catalogue, verified};
use spike_frames::collect_frames;
use spike_synthetic::{SyntheticBody, drop_counter, poll_once};

/// Builds a body whose drop is observable through a shared counter.
fn body_with_probe() -> (ErasedBody, Arc<AtomicUsize>) {
    let (probe, drops) = drop_counter();
    let polls = Arc::new(AtomicUsize::new(0));
    (
        adapt_body(SyntheticBody::new(Vec::new(), Some(probe), polls)),
        drops,
    )
}

#[rstest]
#[tokio::test]
async fn dropping_a_pending_invocation_releases_the_owned_body_once(
    protected_catalogue: Arc<CatalogueResource>,
) -> Result<(), Box<dyn Error>> {
    let (enforcer, _events) = enforcer_with(peregrine_web::spike::Decision::Permit);
    let runner = ResourceRunner::with_enforcer(as_enforcer(&enforcer));
    let registered = runner.register(protected_catalogue.clone(), Method::Get)?;
    let headers = HeaderMap::new();
    let identity = Identity::new("user-42");
    let (owned_body, drops) = body_with_probe();

    let invocation = async move {
        let view = RequestView::new(Method::Get, "/catalogue", &headers, Some(&identity));
        let mut body = owned_body;
        runner.invoke(&registered, view, &mut body).await
    };
    let mut pending = Box::pin(invocation);
    assert!(
        poll_once(pending.as_mut()).is_pending(),
        "the fixture enforcer suspends, so the invocation must stay pending"
    );
    pretty_assertions::assert_eq!(
        drops.load(Ordering::SeqCst),
        0,
        "the owned body must outlive the pending invocation"
    );

    drop(pending);

    pretty_assertions::assert_eq!(
        drops.load(Ordering::SeqCst),
        1,
        "dropping the invocation must release the owned body exactly once"
    );
    pretty_assertions::assert_eq!(
        protected_catalogue.state().responder_calls(),
        0,
        "a cancelled invocation must not reach the responder"
    );
    Ok(())
}

#[rstest]
#[tokio::test]
async fn dropping_a_pending_invocation_releases_body_and_request_guard(
    protected_catalogue: Arc<CatalogueResource>,
) -> Result<(), Box<dyn Error>> {
    let (enforcer, _events) = enforcer_with(peregrine_web::spike::Decision::Permit);
    let runner = ResourceRunner::with_enforcer(as_enforcer(&enforcer));
    let registered = runner.register(protected_catalogue.clone(), Method::Get)?;
    let headers = HeaderMap::new();
    let identity = Identity::new("user-42");
    let (owned_body, body_drops) = body_with_probe();
    let (guard, guard_drops) = drop_counter();

    let invocation = async move {
        // The guard stands in for request-local state that is not the body, so
        // the two counters show that cancellation releases both owners.
        let _guard = guard;
        let view = RequestView::new(Method::Get, "/catalogue", &headers, Some(&identity));
        let mut body = owned_body;
        runner.invoke(&registered, view, &mut body).await
    };
    let mut pending = Box::pin(invocation);
    assert!(
        poll_once(pending.as_mut()).is_pending(),
        "the fixture enforcer suspends, so the invocation must stay pending"
    );
    pretty_assertions::assert_eq!(
        (
            body_drops.load(Ordering::SeqCst),
            guard_drops.load(Ordering::SeqCst)
        ),
        (0, 0),
        "a pending invocation must still own both the body and the request guard"
    );

    drop(pending);

    pretty_assertions::assert_eq!(
        (
            body_drops.load(Ordering::SeqCst),
            guard_drops.load(Ordering::SeqCst)
        ),
        (1, 1),
        "cancellation must release the body and the request guard exactly once each"
    );
    pretty_assertions::assert_eq!(
        protected_catalogue.state().responder_calls(),
        0,
        "a cancelled invocation must not reach the responder"
    );
    Ok(())
}

#[rstest]
#[tokio::test]
async fn a_dropped_future_returns_its_borrowed_arguments(
    protected_catalogue: Arc<CatalogueResource>,
) -> Result<(), Box<dyn Error>> {
    let (enforcer, _events) = enforcer_with(peregrine_web::spike::Decision::Permit);
    let runner = ResourceRunner::with_enforcer(as_enforcer(&enforcer));
    let registered = runner.register(protected_catalogue.clone(), Method::Get)?;
    let headers = HeaderMap::new();
    let identity = Identity::new("user-42");
    let view = RequestView::new(Method::Get, "/catalogue", &headers, Some(&identity));
    let (mut body, drops) = body_with_probe();

    {
        let mut invocation = Box::pin(runner.invoke(&registered, view, &mut body));
        assert!(
            poll_once(invocation.as_mut()).is_pending(),
            "the fixture enforcer suspends, so the invocation must stay pending"
        );
    }

    pretty_assertions::assert_eq!(
        drops.load(Ordering::SeqCst),
        0,
        "a caller-owned body must not be dropped by a cancelled invocation"
    );
    let frames = collect_frames(&mut body).await?;
    pretty_assertions::assert_eq!(
        frames,
        Vec::new(),
        "the caller must be able to consume the returned body"
    );
    drop(body);
    pretty_assertions::assert_eq!(
        drops.load(Ordering::SeqCst),
        1,
        "the caller releases its own body exactly once"
    );
    pretty_assertions::assert_eq!(
        protected_catalogue.state().responder_calls(),
        0,
        "a cancelled invocation must not reach the responder"
    );
    Ok(())
}

#[rstest]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_fully_owned_invocation_can_be_spawned(
    public_catalogue: Arc<CatalogueResource>,
) -> Result<(), Box<dyn Error>> {
    let runner = ResourceRunner::new();
    let registered = Arc::new(runner.register(public_catalogue.clone(), Method::Get)?);
    let headers = HeaderMap::new();

    let invocation = tokio::spawn(async move {
        let view = RequestView::new(Method::Get, "/catalogue", &headers, None);
        let mut body = adapt_body(Empty::<Bytes>::new());
        runner.invoke(&registered, view, &mut body).await
    });

    let outcome = invocation.await?;

    verified(verify_that!(
        matches!(outcome, InvocationOutcome::Responded(_)),
        eq(true)
    ))?;
    verified(verify_that!(
        public_catalogue.state().responder_calls(),
        eq(1)
    ))?;
    Ok(())
}
