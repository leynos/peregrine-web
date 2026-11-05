//! Runtime evidence for resource identity and policy admission.
//!
//! Each test drives the real specimen seam in `peregrine_web::spike`. The
//! enforcer fixture suspends between the admission decision and the responder,
//! so identity observations are compared across an actual await rather than a
//! synchronous stand-in. What a dropped or spawned invocation releases is
//! observed in `resource_policy_cancellation`.

#[path = "support/spike/admission.rs"]
mod spike_admission;
#[path = "support/spike/components.rs"]
mod spike_components;
#[path = "support/spike/fixtures.rs"]
mod spike_fixtures;
#[path = "support/spike/frames.rs"]
mod spike_frames;

use std::{error::Error, sync::Arc};

use bytes::Bytes;
use googletest::prelude::{eq, matches_pattern, verify_that};
use http::HeaderMap;
use http_body_util::Empty;
use peregrine_web::spike::{
    Decision,
    DenialReason,
    Identity,
    InvocationOutcome,
    Method,
    Policy,
    PolicyError,
    RegistrationError,
    RequestView,
    ResourceRunner,
    ScopeName,
    ScopeSet,
    adapt_body,
};
use rstest::rstest;
use spike_admission::AdmissionEvent;
use spike_components::{BillingResource, CatalogueResource, DependencyState, OwnedResource};
use spike_fixtures::{
    as_enforcer,
    billing_dependency,
    catalogue_dependency,
    enforcer_with,
    invoke_through_rendezvous,
    protected_catalogue,
    public_catalogue,
    scoped_catalogue,
    verified,
};
use tokio::sync::mpsc::{UnboundedReceiver, error::TryRecvError};

/// Checks that a public resource registers and answers a direct invocation
/// without an enforcer, consulting its injected dependency exactly once.
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

/// Checks that registering a protected resource without an enforcer fails with
/// the typed missing-enforcer error naming the frozen method and policy.
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

/// Checks that an anonymous protected request is denied for a missing identity
/// before admission is consulted, leaving the responder untouched.
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

/// Checks that an enforcer denial withholds the request from the responder and
/// its dependency, even though the identified request reached the hook.
#[rstest]
#[tokio::test]
async fn enforcer_denial_withholds_the_request(
    protected_catalogue: Arc<CatalogueResource>,
) -> Result<(), Box<dyn Error>> {
    let (enforcer, _events) = enforcer_with(Decision::Deny(DenialReason::EnforcerDenied));
    let runner = ResourceRunner::with_enforcer(as_enforcer(&enforcer));
    let registered = runner.register(protected_catalogue.clone(), Method::Get)?;
    let mut body = adapt_body(Empty::<Bytes>::new());

    let (outcome, entry) =
        invoke_through_rendezvous(&enforcer, &runner, &registered, &mut body).await;
    assert!(
        entry.entered(),
        "a protected request with an identity must reach the enforcer hook"
    );

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

/// Checks that invoking one registration leaves another registered resource
/// untouched: the sibling's responder and dependency stay unvisited while its
/// frozen method remains intact.
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

/// Checks that an empty scope set is rejected as `PolicyError::EmptyScopeSet`
/// rather than accepted as a policy requiring no scopes.
#[rstest]
fn empty_scope_set_is_rejected() -> Result<(), Box<dyn Error>> {
    let error = ScopeSet::new(Vec::<ScopeName>::new())
        .expect_err("an empty requirement set is not a policy");

    verified(verify_that!(error, eq(PolicyError::EmptyScopeSet)))?;
    Ok(())
}

/// Checks that a scoped policy still delegates to the enforcer, reports that it
/// requires enforcement, and reaches the responder without the specimen
/// computing any scopes.
#[rstest]
#[tokio::test]
async fn scoped_policy_delegates_without_computing_scopes(
    scoped_catalogue: Result<Arc<CatalogueResource>, PolicyError>,
) -> Result<(), Box<dyn Error>> {
    let scoped = scoped_catalogue?;
    let (enforcer, _events) = enforcer_with(Decision::Permit);
    let runner = ResourceRunner::with_enforcer(as_enforcer(&enforcer));
    let registered = runner.register(scoped.clone(), Method::Get)?;
    let mut body = adapt_body(Empty::<Bytes>::new());

    let (outcome, entry) =
        invoke_through_rendezvous(&enforcer, &runner, &registered, &mut body).await;
    assert!(
        entry.entered(),
        "a scoped request with an identity must reach the enforcer hook"
    );

    verified(verify_that!(outcome.responded().is_some(), eq(true)))?;
    verified(verify_that!(
        registered.policy().requires_enforcement(),
        eq(true)
    ))?;
    verified(verify_that!(scoped.state().responder_calls(), eq(1)))?;
    Ok(())
}

/// Asserts that admission recorded nothing, because the request never reached it.
fn assert_no_admission(events: &mut UnboundedReceiver<AdmissionEvent>, why: &str) {
    assert!(
        matches!(events.try_recv(), Err(TryRecvError::Empty)),
        "{why}: the enforcer must not be consulted"
    );
}

/// A registration whose method has no responder refuses before admission.
///
/// Freezing `POST` is legal, but this specimen only implements `get`, so the
/// refusal must name the frozen method. It must also stop short of admission,
/// which keeps an unsupported verb from being treated as a policy question.
#[rstest]
#[tokio::test]
async fn a_registered_non_get_method_is_not_dispatched(
    protected_catalogue: Arc<CatalogueResource>,
) -> Result<(), Box<dyn Error>> {
    let (enforcer, mut events) = enforcer_with(Decision::Permit);
    let runner = ResourceRunner::with_enforcer(as_enforcer(&enforcer));
    let registered = runner.register(protected_catalogue.clone(), Method::Post)?;
    let headers = HeaderMap::new();
    let identity = Identity::new("user-42");
    let view = RequestView::new(Method::Post, "/catalogue", &headers, Some(&identity));
    let mut body = adapt_body(Empty::<Bytes>::new());

    let outcome = runner.invoke(&registered, view, &mut body).await;

    match outcome {
        InvocationOutcome::MethodNotAllowed(method) => pretty_assertions::assert_eq!(
            method,
            Method::Post,
            "the refusal must report the method frozen at registration"
        ),
        other => {
            return Err(format!("expected an unsupported-method outcome, got {other:?}").into());
        }
    }
    assert_no_admission(
        &mut events,
        "a method without a responder is not an admission question",
    );
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

/// A request method that differs from the registration is rejected unnamed.
///
/// The mismatch is reported before the unsupported-method check, so a caller
/// that registers `GET` and requests `POST` learns both methods rather than
/// only that `POST` has no responder.
#[rstest]
#[tokio::test]
async fn a_requested_method_that_differs_from_the_registration_is_rejected(
    protected_catalogue: Arc<CatalogueResource>,
) -> Result<(), Box<dyn Error>> {
    let (enforcer, mut events) = enforcer_with(Decision::Permit);
    let runner = ResourceRunner::with_enforcer(as_enforcer(&enforcer));
    let registered = runner.register(protected_catalogue.clone(), Method::Get)?;
    let headers = HeaderMap::new();
    let identity = Identity::new("user-42");
    let view = RequestView::new(Method::Post, "/catalogue", &headers, Some(&identity));
    let mut body = adapt_body(Empty::<Bytes>::new());

    let outcome = runner.invoke(&registered, view, &mut body).await;

    match outcome {
        InvocationOutcome::MethodMismatch {
            registered: frozen,
            requested,
        } => {
            pretty_assertions::assert_eq!(
                frozen,
                Method::Get,
                "the mismatch must report the method frozen at registration"
            );
            pretty_assertions::assert_eq!(
                requested,
                Method::Post,
                "the mismatch must report the method carried by the request"
            );
        }
        other => return Err(format!("expected a method-mismatch outcome, got {other:?}").into()),
    }
    assert_no_admission(
        &mut events,
        "a mismatched method is rejected before admission",
    );
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

/// Pins the wire methods the specimen recognises, and the ones it refuses.
///
/// `Method::from_http` is the only bridge from Hyper's method type, so a
/// missing or mis-mapped arm would silently reclassify a supported request.
/// The extension method proves an unknown verb is reported rather than mapped
/// onto a neighbour.
#[rstest]
#[case::get("GET", Some(Method::Get))]
#[case::post("POST", Some(Method::Post))]
#[case::put("PUT", Some(Method::Put))]
#[case::delete("DELETE", Some(Method::Delete))]
#[case::patch("PATCH", Some(Method::Patch))]
#[case::head("HEAD", Some(Method::Head))]
#[case::options("OPTIONS", Some(Method::Options))]
#[case::extension_method("PURGE", None)]
fn http_methods_map_to_the_specimen_vocabulary(
    #[case] wire: &str,
    #[case] expected: Option<Method>,
) -> Result<(), Box<dyn Error>> {
    let parsed = http::Method::from_bytes(wire.as_bytes())
        .map_err(|error| format!("{wire} must parse as an HTTP method: {error}"))?;
    let converted = Method::from_http(&parsed);

    pretty_assertions::assert_eq!(
        converted,
        expected,
        "{wire} must map to the specimen's own method value"
    );
    if let Some(method) = converted {
        pretty_assertions::assert_eq!(
            method.as_str(),
            wire,
            "a recognised method must render back to its wire form"
        );
    }
    Ok(())
}
