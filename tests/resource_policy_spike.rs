//! Runtime evidence for resource identity, policy admission, and cancellation.
//!
//! Each test drives the real specimen seam in `peregrine_web::spike`. The
//! enforcer fixture suspends between the admission decision and the responder,
//! so identity observations are compared across an actual await rather than a
//! synchronous stand-in. Cancellation tests poll a pending invocation and then
//! drop it, which is the only way to observe what a dropped future released.

#[path = "support/spike/admission.rs"]
mod spike_admission;
#[path = "support/spike/components.rs"]
mod spike_components;
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
use googletest::prelude::{eq, matches_pattern, verify_that};
use http::HeaderMap;
use http_body_util::Empty;
use peregrine_web::spike::{
    AllocationId,
    Decision,
    DenialReason,
    ErasedBody,
    Identity,
    InvocationOutcome,
    Method,
    Policy,
    PolicyEnforcer,
    PolicyError,
    RegistrationError,
    RequestView,
    ResourceRunner,
    ScopeName,
    ScopeSet,
    adapt_body,
};
use rstest::{fixture, rstest};
use spike_admission::{
    AdmissionEvent,
    AdmissionPhase,
    FixtureEnforcer,
    InvocationSubject,
    invoke_with_rendezvous,
};
use spike_components::{BillingResource, CatalogueResource, DependencyState, OwnedResource};
use spike_frames::collect_frames;
use spike_synthetic::{SyntheticBody, drop_counter, poll_once};
use tokio::sync::mpsc::UnboundedReceiver;

/// Converts a Googletest assertion failure into the error the harness prints.
///
/// `verify_that!` reports failures as `googletest::Result`, whose failure type
/// is not `std::error::Error`. Tests here also propagate domain errors, so the
/// assertion failure travels back to the harness as its rendered message.
fn verified(outcome: googletest::Result<()>) -> Result<(), Box<dyn Error>> {
    outcome.map_err(|failure| failure.to_string().into())
}

/// The dependency the catalogue fixture injects.
#[fixture]
fn catalogue_dependency() -> Arc<DependencyState> {
    DependencyState::new("catalogue", "catalogue-record")
}

/// The dependency the billing fixture injects.
#[fixture]
fn billing_dependency() -> Arc<DependencyState> {
    DependencyState::new("billing", "billing-record")
}

/// A catalogue resource whose policy is public.
#[fixture]
fn public_catalogue(catalogue_dependency: Arc<DependencyState>) -> Arc<CatalogueResource> {
    CatalogueResource::new("catalogue", Policy::Public, catalogue_dependency)
}

/// A catalogue resource whose policy requires an authenticated identity.
#[fixture]
fn protected_catalogue(catalogue_dependency: Arc<DependencyState>) -> Arc<CatalogueResource> {
    CatalogueResource::new("catalogue", Policy::Authenticated, catalogue_dependency)
}

/// A catalogue resource whose policy requires one scope.
#[fixture]
fn scoped_catalogue(
    catalogue_dependency: Arc<DependencyState>,
) -> Result<Arc<CatalogueResource>, PolicyError> {
    let scope = ScopeName::new("catalogue:read")?;
    let scopes = ScopeSet::new([scope])?;
    Ok(CatalogueResource::new(
        "catalogue",
        Policy::Scopes(scopes),
        catalogue_dependency,
    ))
}

/// Builds a suspended-at-admission enforcer and the receiver of its events.
fn enforcer_with(decision: Decision) -> (Arc<FixtureEnforcer>, UnboundedReceiver<AdmissionEvent>) {
    FixtureEnforcer::new(decision)
}

/// Wraps an enforcer in the trait object the runner accepts.
///
/// The concrete clone is bound first so the unsizing coercion happens on the
/// value rather than being pushed back into `Arc::clone`'s type parameter.
fn as_enforcer(enforcer: &Arc<FixtureEnforcer>) -> Arc<dyn PolicyEnforcer> {
    let concrete: Arc<FixtureEnforcer> = Arc::clone(enforcer);
    concrete
}

#[rstest]
#[tokio::test]
async fn public_resource_permits_direct_invocation(
    public_catalogue: Arc<CatalogueResource>,
) -> Result<(), Box<dyn Error>> {
    let runner = ResourceRunner::new();
    let registered = runner.register(public_catalogue.clone(), Method::Get)?;
    let headers = HeaderMap::new();
    let view = RequestView::new(Method::Get, "/catalogue", &headers, None);
    let mut body = adapt_body(Empty::<Bytes>::new());

    let outcome = runner.invoke(&registered, view, &mut body).await;

    verified(verify_that!(outcome.responded().is_some(), eq(true)))?;
    verified(verify_that!(
        public_catalogue.state().responder_calls(),
        eq(1)
    ))?;
    verified(verify_that!(public_catalogue.dependency().calls(), eq(1)))?;
    let draft = outcome
        .responded()
        .ok_or("a public invocation should respond")?;
    let expected = Bytes::from_static(b"catalogue-record /catalogue 0");
    verified(verify_that!(draft.body(), eq(&expected)))?;
    Ok(())
}

#[rstest]
fn protected_registration_requires_an_enforcer(
    protected_catalogue: Arc<CatalogueResource>,
) -> Result<(), Box<dyn Error>> {
    let runner = ResourceRunner::new();

    let Err(error) = runner.register(protected_catalogue, Method::Get) else {
        return Err("a protected resource must not register without an enforcer".into());
    };

    verified(verify_that!(
        error,
        matches_pattern!(RegistrationError::MissingEnforcer {
            method: eq(&Method::Get),
            policy: eq(&Policy::Authenticated),
        })
    ))?;
    Ok(())
}

#[rstest]
#[tokio::test]
async fn missing_identity_is_denied_before_the_hook_runs(
    protected_catalogue: Arc<CatalogueResource>,
) -> Result<(), Box<dyn Error>> {
    let (enforcer, mut events) = enforcer_with(Decision::Permit);
    let runner = ResourceRunner::with_enforcer(as_enforcer(&enforcer));
    let registered = runner.register(protected_catalogue.clone(), Method::Get)?;
    let headers = HeaderMap::new();
    let view = RequestView::new(Method::Get, "/catalogue", &headers, None);
    let mut body = adapt_body(Empty::<Bytes>::new());

    let outcome = runner.invoke(&registered, view, &mut body).await;

    verified(verify_that!(
        outcome.denied(),
        eq(Some(DenialReason::MissingIdentity))
    ))?;
    verified(verify_that!(events.try_recv().is_err(), eq(true)))?;
    verified(verify_that!(
        protected_catalogue.state().responder_calls(),
        eq(0)
    ))?;
    Ok(())
}

#[rstest]
#[tokio::test]
async fn enforcer_denial_withholds_the_request(
    protected_catalogue: Arc<CatalogueResource>,
) -> Result<(), Box<dyn Error>> {
    let (enforcer, _events) = enforcer_with(Decision::Deny(DenialReason::EnforcerDenied));
    let runner = ResourceRunner::with_enforcer(as_enforcer(&enforcer));
    let registered = runner.register(protected_catalogue.clone(), Method::Get)?;
    let headers = HeaderMap::new();
    let identity = Identity::new("user-42");
    let view = RequestView::new(Method::Get, "/catalogue", &headers, Some(&identity));
    let mut body = adapt_body(Empty::<Bytes>::new());

    let outcome = invoke_with_rendezvous(
        &enforcer,
        InvocationSubject {
            runner: &runner,
            registered: &registered,
            view,
            body: &mut body,
        },
    )
    .await;

    verified(verify_that!(
        outcome.denied(),
        eq(Some(DenialReason::EnforcerDenied))
    ))?;
    verified(verify_that!(
        protected_catalogue.state().responder_calls(),
        eq(0)
    ))?;
    verified(verify_that!(
        protected_catalogue.dependency().calls(),
        eq(0)
    ))?;
    Ok(())
}

#[rstest]
#[tokio::test]
async fn identity_and_policy_survive_the_awaited_hook(
    protected_catalogue: Arc<CatalogueResource>,
) -> Result<(), Box<dyn Error>> {
    let (enforcer, mut events) = enforcer_with(Decision::Permit);
    let runner = ResourceRunner::with_enforcer(as_enforcer(&enforcer));
    let registered = runner.register(protected_catalogue.clone(), Method::Get)?;
    let headers = HeaderMap::new();
    let identity = Identity::new("user-42");
    let view = RequestView::new(Method::Get, "/catalogue", &headers, Some(&identity));
    let mut body = adapt_body(Empty::<Bytes>::new());

    let outcome = invoke_with_rendezvous(
        &enforcer,
        InvocationSubject {
            runner: &runner,
            registered: &registered,
            view,
            body: &mut body,
        },
    )
    .await;

    let before = events.recv().await.ok_or("a pre-await observation")?;
    let after = events.recv().await.ok_or("a post-await observation")?;
    pretty_assertions::assert_eq!(
        (before.phase, after.phase),
        (AdmissionPhase::BeforeAwait, AdmissionPhase::AfterAwait),
        "the enforcer must observe one event before and one after its suspension"
    );
    pretty_assertions::assert_eq!(
        (
            &before.handle,
            &before.observation,
            &before.method,
            &before.policy,
            &before.path,
            &before.identity,
        ),
        (
            &after.handle,
            &after.observation,
            &after.method,
            &after.policy,
            &after.path,
            &after.identity,
        ),
        "registration must freeze every observation except the phase across the await"
    );
    let retained = AllocationId::of(registered.resource());
    pretty_assertions::assert_eq!(
        before.handle,
        retained,
        "admission must observe the handle of the retained resource"
    );
    pretty_assertions::assert_eq!(
        before.method,
        Method::Get,
        "admission must observe the frozen method"
    );
    pretty_assertions::assert_eq!(
        before.policy,
        Policy::Authenticated,
        "admission must observe the policy captured at registration"
    );
    pretty_assertions::assert_eq!(
        before.path,
        "/catalogue",
        "admission must observe the request path"
    );
    pretty_assertions::assert_eq!(
        before.identity.as_deref(),
        Some("user-42"),
        "admission must observe the presented principal"
    );
    pretty_assertions::assert_eq!(
        before.observation.resource,
        AllocationId::of(protected_catalogue.state()),
        "the declared resource identity must name the fixture's own state"
    );
    pretty_assertions::assert_eq!(
        before.observation.dependency,
        AllocationId::of(protected_catalogue.dependency()),
        "the declared dependency identity must name the fixture's own store"
    );
    verified(verify_that!(outcome.responded().is_some(), eq(true)))?;
    verified(verify_that!(
        protected_catalogue.state().responder_calls(),
        eq(1)
    ))?;
    Ok(())
}

#[rstest]
#[tokio::test]
async fn an_unselected_resource_is_not_invoked(
    catalogue_dependency: Arc<DependencyState>,
    billing_dependency: Arc<DependencyState>,
) -> Result<(), Box<dyn Error>> {
    let catalogue = CatalogueResource::new("catalogue", Policy::Public, catalogue_dependency);
    let billing = BillingResource::new("billing", Policy::Public, billing_dependency);
    let runner = ResourceRunner::new();
    let selected = runner.register(catalogue.clone(), Method::Get)?;
    let unselected = runner.register(billing.clone(), Method::Get)?;
    let headers = HeaderMap::new();
    let view = RequestView::new(Method::Get, "/catalogue", &headers, None);
    let mut body = adapt_body(Empty::<Bytes>::new());

    let outcome = runner.invoke(&selected, view, &mut body).await;

    verified(verify_that!(outcome.responded().is_some(), eq(true)))?;
    verified(verify_that!(catalogue.state().responder_calls(), eq(1)))?;
    verified(verify_that!(billing.state().responder_calls(), eq(0)))?;
    verified(verify_that!(billing.dependency().calls(), eq(0)))?;
    verified(verify_that!(unselected.method(), eq(Method::Get)))?;
    Ok(())
}

#[rstest]
fn empty_scope_set_is_rejected() -> Result<(), Box<dyn Error>> {
    let error = ScopeSet::new(Vec::<ScopeName>::new())
        .expect_err("an empty requirement set is not a policy");

    verified(verify_that!(error, eq(PolicyError::EmptyScopeSet)))?;
    Ok(())
}

#[rstest]
#[tokio::test]
async fn scoped_policy_delegates_without_computing_scopes(
    scoped_catalogue: Result<Arc<CatalogueResource>, PolicyError>,
) -> Result<(), Box<dyn Error>> {
    let scoped = scoped_catalogue?;
    let (enforcer, _events) = enforcer_with(Decision::Permit);
    let runner = ResourceRunner::with_enforcer(as_enforcer(&enforcer));
    let registered = runner.register(scoped.clone(), Method::Get)?;
    let headers = HeaderMap::new();
    let identity = Identity::new("user-42");
    let view = RequestView::new(Method::Get, "/catalogue", &headers, Some(&identity));
    let mut body = adapt_body(Empty::<Bytes>::new());

    let outcome = invoke_with_rendezvous(
        &enforcer,
        InvocationSubject {
            runner: &runner,
            registered: &registered,
            view,
            body: &mut body,
        },
    )
    .await;

    verified(verify_that!(outcome.responded().is_some(), eq(true)))?;
    verified(verify_that!(
        registered.policy().requires_enforcement(),
        eq(true)
    ))?;
    verified(verify_that!(scoped.state().responder_calls(), eq(1)))?;
    Ok(())
}

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
    let (enforcer, _events) = enforcer_with(Decision::Permit);
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
async fn a_dropped_future_returns_its_borrowed_arguments(
    protected_catalogue: Arc<CatalogueResource>,
) -> Result<(), Box<dyn Error>> {
    let (enforcer, _events) = enforcer_with(Decision::Permit);
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
