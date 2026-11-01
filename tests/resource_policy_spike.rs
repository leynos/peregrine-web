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
    AllocationId,
    Decision,
    DenialReason,
    ErasedBody,
    Identity,
    InvocationOutcome,
    Method,
    Policy,
    PolicyError,
    RegisteredResource,
    RegistrationError,
    RequestView,
    ResourceRunner,
    ScopeName,
    ScopeSet,
    adapt_body,
};
use rstest::rstest;
use spike_admission::{
    AdmissionEvent,
    AdmissionPhase,
    FixtureEnforcer,
    InvocationSubject,
    invoke_with_rendezvous,
};
use spike_components::{BillingResource, CatalogueResource, DependencyState, OwnedResource};
use spike_fixtures::{
    as_enforcer,
    billing_dependency,
    catalogue_dependency,
    enforcer_with,
    protected_catalogue,
    public_catalogue,
    scoped_catalogue,
    verified,
};
use tokio::sync::mpsc::UnboundedReceiver;

/// Receives the enforcer's pre-suspension and post-suspension observations.
async fn await_admission_pair(
    events: &mut UnboundedReceiver<AdmissionEvent>,
) -> Result<(AdmissionEvent, AdmissionEvent), Box<dyn Error>> {
    let before = events.recv().await.ok_or("a pre-await observation")?;
    let after = events.recv().await.ok_or("a post-await observation")?;
    Ok((before, after))
}

/// Asserts that only the phase changed across the enforcer's suspension.
fn assert_only_the_phase_changed(before: &AdmissionEvent, after: &AdmissionEvent) {
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
}

/// Invokes a registration across the enforcer's real suspension.
///
/// Every caller presents the same authenticated request, so the only variation
/// a caller supplies is the registration under test.
async fn invoke_through_rendezvous(
    enforcer: &Arc<FixtureEnforcer>,
    runner: &ResourceRunner,
    registered: &RegisteredResource,
    body: &mut ErasedBody,
) -> InvocationOutcome {
    let headers = HeaderMap::new();
    let identity = Identity::new("user-42");
    let view = RequestView::new(Method::Get, "/catalogue", &headers, Some(&identity));
    invoke_with_rendezvous(
        enforcer,
        InvocationSubject {
            runner,
            registered,
            view,
            body,
        },
    )
    .await
}

/// Asserts that admission observed the registration that retained the resource.
fn assert_registration_was_retained(
    before: &AdmissionEvent,
    registered: &RegisteredResource,
    resource: &CatalogueResource,
) {
    pretty_assertions::assert_eq!(
        before.handle,
        AllocationId::of(registered.resource()),
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
        AllocationId::of(resource.state()),
        "the declared resource identity must name the fixture's own state"
    );
    pretty_assertions::assert_eq!(
        before.observation.dependency,
        AllocationId::of(resource.dependency()),
        "the declared dependency identity must name the fixture's own store"
    );
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
    let mut body = adapt_body(Empty::<Bytes>::new());

    let outcome = invoke_through_rendezvous(&enforcer, &runner, &registered, &mut body).await;

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
    let mut body = adapt_body(Empty::<Bytes>::new());

    let outcome = invoke_through_rendezvous(&enforcer, &runner, &registered, &mut body).await;

    let (before, after) = await_admission_pair(&mut events).await?;
    assert_only_the_phase_changed(&before, &after);
    assert_registration_was_retained(&before, &registered, &protected_catalogue);
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
    let mut body = adapt_body(Empty::<Bytes>::new());

    let outcome = invoke_through_rendezvous(&enforcer, &runner, &registered, &mut body).await;

    verified(verify_that!(outcome.responded().is_some(), eq(true)))?;
    verified(verify_that!(
        registered.policy().requires_enforcement(),
        eq(true)
    ))?;
    verified(verify_that!(scoped.state().responder_calls(), eq(1)))?;
    Ok(())
}
