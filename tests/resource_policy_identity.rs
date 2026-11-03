//! Runtime evidence that resource identity and policy survive admission.
//!
//! Each test drives the real specimen seam in `peregrine_web::spike`. The
//! enforcer fixture suspends between the admission decision and the responder,
//! so observations are compared across an actual await rather than a
//! synchronous stand-in. Repeated admissions read the same registration
//! snapshot, and two instances of one concrete type keep distinct identities.

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
use googletest::prelude::{eq, verify_that};
use http::HeaderMap;
use http_body_util::Empty;
use peregrine_web::spike::{
    AllocationId,
    Decision,
    Method,
    Policy,
    RegisteredResource,
    RequestView,
    ResourceRunner,
    adapt_body,
};
use rstest::rstest;
use spike_admission::{AdmissionEvent, AdmissionPhase};
use spike_components::{CatalogueResource, DependencyState, OwnedResource};
use spike_fixtures::{
    as_enforcer,
    catalogue_dependency,
    enforcer_with,
    invoke_through_rendezvous,
    protected_catalogue,
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
async fn repeated_admissions_observe_the_frozen_policy(
    protected_catalogue: Arc<CatalogueResource>,
) -> Result<(), Box<dyn Error>> {
    let (enforcer, mut events) = enforcer_with(Decision::Permit);
    let runner = ResourceRunner::with_enforcer(as_enforcer(&enforcer));
    let registered = runner.register(protected_catalogue.clone(), Method::Get)?;
    let registration = registered.policy().clone();
    let owned = protected_catalogue.state().policy().clone();

    for admission in 0..2 {
        let mut body = adapt_body(Empty::<Bytes>::new());
        let outcome = invoke_through_rendezvous(&enforcer, &runner, &registered, &mut body).await;
        let (before, after) = await_admission_pair(&mut events).await?;

        assert_only_the_phase_changed(&before, &after);
        pretty_assertions::assert_eq!(
            before.policy,
            registration,
            "admission {admission} must observe the policy captured at registration"
        );
        pretty_assertions::assert_eq!(
            before.policy,
            owned,
            "admission {admission} must observe the policy the resource owns"
        );
        verified(verify_that!(outcome.responded().is_some(), eq(true)))?;
    }

    verified(verify_that!(
        protected_catalogue.state().responder_calls(),
        eq(2)
    ))?;
    Ok(())
}

#[rstest]
#[tokio::test]
async fn same_type_instances_keep_distinct_identity(
    catalogue_dependency: Arc<DependencyState>,
) -> Result<(), Box<dyn Error>> {
    let selected = CatalogueResource::new("selected", Policy::Public, catalogue_dependency);
    let other_dependency = DependencyState::new("other", "other-record");
    let unselected = CatalogueResource::new("unselected", Policy::Public, other_dependency);
    let runner = ResourceRunner::new();
    let selected_registration = runner.register(selected.clone(), Method::Get)?;
    let _unselected_registration = runner.register(unselected.clone(), Method::Get)?;
    let headers = HeaderMap::new();
    let view = RequestView::new(Method::Get, "/catalogue", &headers, None);
    let mut body = adapt_body(Empty::<Bytes>::new());

    let outcome = runner.invoke(&selected_registration, view, &mut body).await;

    verified(verify_that!(outcome.responded().is_some(), eq(true)))?;
    let draft = outcome
        .responded()
        .ok_or("the selected instance should respond")?;
    let expected = Bytes::from_static(b"catalogue-record /catalogue 0");
    verified(verify_that!(draft.body(), eq(&expected)))?;
    verified(verify_that!(selected.state().responder_calls(), eq(1)))?;
    verified(verify_that!(unselected.state().responder_calls(), eq(0)))?;
    verified(verify_that!(unselected.dependency().calls(), eq(0)))?;
    pretty_assertions::assert_ne!(
        AllocationId::of(selected.state()),
        AllocationId::of(unselected.state()),
        "two instances of one concrete type must not share an allocation identity"
    );
    pretty_assertions::assert_eq!(
        AllocationId::of(selected_registration.resource()),
        AllocationId::of(&selected),
        "registration must retain the selected instance, not a sibling of the same type"
    );
    Ok(())
}
