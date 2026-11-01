//! Behavioural scenarios for resource identity across async admission.
//!
//! Every step drives `peregrine_web::spike` directly. The world records only
//! what the seam returned, so a scenario cannot pass against a parallel model
//! of admission. The scenarios cover the public no-enforcer path, a denial from
//! the fixture enforcer, registration without enforcement, the permitted
//! protected path, and denial before the hook when no identity is presented.

#[path = "support/spike/admission.rs"]
mod spike_admission;
#[path = "support/spike/components.rs"]
mod spike_components;
#[path = "support/spike/frames.rs"]
mod spike_frames;

use std::sync::Arc;

use bytes::Bytes;
use http::HeaderMap;
use http_body_util::Empty;
use peregrine_web::spike::{
    Decision,
    DenialReason,
    ErasedBody,
    Identity,
    InvocationOutcome,
    Method,
    Policy,
    PolicyEnforcer,
    RegisteredResource,
    RegistrationError,
    RequestView,
    ResourceRunner,
    adapt_body,
};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};
use spike_admission::{FixtureEnforcer, InvocationSubject, invoke_with_rendezvous};
use spike_components::{CatalogueResource, DependencyState, OwnedResource};

/// State accumulated by one scenario while it drives the spike seam.
#[derive(Default)]
struct SpikeWorld {
    /// The catalogue-shaped resource under test.
    catalogue: Option<Arc<CatalogueResource>>,
    /// The enforcer the scenario installed, when one is required.
    enforcer: Option<Arc<FixtureEnforcer>>,
    /// The runner that owns registration and invocation.
    runner: Option<ResourceRunner>,
    /// The outcome of the invocation performed by the `When` step.
    outcome: Option<InvocationOutcome>,
    /// The result of a registration the `When` step validated.
    registration: Option<Result<RegisteredResource, RegistrationError>>,
}

impl SpikeWorld {
    /// Installs a fresh catalogue resource and the runner admitting it.
    ///
    /// A `decision` of `Some` installs the fixture enforcer; `None` leaves the
    /// runner without one, which only a public policy permits.
    fn install(&mut self, policy: Policy, decision: Option<Decision>) {
        let dependency = DependencyState::new("catalogue", "catalogue-record");
        self.catalogue = Some(CatalogueResource::new("catalogue", policy, dependency));
        if let Some(chosen) = decision {
            let (enforcer, _events) = FixtureEnforcer::new(chosen);
            // Bind the concrete clone first; `Arc::clone` would otherwise
            // infer `Arc<dyn PolicyEnforcer>` and refuse the coercion.
            let retained: Arc<FixtureEnforcer> = Arc::clone(&enforcer);
            let erased: Arc<dyn PolicyEnforcer> = retained;
            self.runner = Some(ResourceRunner::with_enforcer(erased));
            self.enforcer = Some(enforcer);
        } else {
            self.runner = Some(ResourceRunner::new());
            self.enforcer = None;
        }
    }

    /// Borrows the catalogue resource, failing the scenario when it is absent.
    fn catalogue(&self) -> &Arc<CatalogueResource> {
        let Some(catalogue) = self.catalogue.as_ref() else {
            panic!("the scenario did not install a catalogue resource");
        };
        catalogue
    }

    /// Borrows the installed runner, failing the scenario when it is absent.
    fn runner(&self) -> &ResourceRunner {
        let Some(runner) = self.runner.as_ref() else {
            panic!("the scenario did not install a runner");
        };
        runner
    }

    /// Borrows the outcome of the `When` step, failing when it has not run.
    fn outcome(&self) -> &InvocationOutcome {
        let Some(outcome) = self.outcome.as_ref() else {
            panic!("the scenario did not invoke the selected resource");
        };
        outcome
    }

    /// Registers the catalogue resource for `GET`, failing on registration.
    fn register_selected(&self) -> RegisteredResource {
        let resource = Arc::clone(self.catalogue());
        match self.runner().register(resource, Method::Get) {
            Ok(registered) => registered,
            Err(error) => panic!("the catalogue resource must register: {error}"),
        }
    }
}

/// Provides a fresh world to each scenario.
///
/// `rstest-bdd` resolves the scenario function's parameter by fixture name, so
/// this fixture is what gives every scenario its own isolated `SpikeWorld`.
#[fixture]
fn world() -> SpikeWorld {
    // Each scenario gets its own world, so recorded state cannot leak.
    SpikeWorld::default()
}

/// Invokes a registration, using the enforcer rendezvous when one is installed.
///
/// The rendezvous suspends admission between its two observations, so a
/// scenario that reaches the responder has crossed a real await boundary.
async fn invoke_selected(
    world: &SpikeWorld,
    registered: &RegisteredResource,
    view: RequestView<'_>,
    body: &mut ErasedBody,
) -> InvocationOutcome {
    let runner = world.runner();
    // Only a request that presents an identity reaches the enforcer. Admission
    // denies an anonymous protected request before the hook, so driving the
    // rendezvous would wait for a suspension that never happens.
    let reaches_hook = view.identity().is_some();
    match world.enforcer.as_ref() {
        Some(enforcer) if reaches_hook => {
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
        _ => runner.invoke(registered, view, body).await,
    }
}

/// Creates a public resource with an injected dependency and no enforcer.
#[given("a public resource with an injected dependency and no enforcer")]
fn public_resource(world: &mut SpikeWorld) { world.install(Policy::Public, None); }

/// Creates a protected resource with an enforcer that denies admission.
#[given("a protected resource and an enforcer returning denial")]
fn protected_resource_denied(world: &mut SpikeWorld) {
    world.install(
        Policy::Authenticated,
        Some(Decision::Deny(DenialReason::EnforcerDenied)),
    );
}

/// Creates a protected resource with an enforcer that permits admission.
#[given("a protected resource and an enforcer returning permission")]
fn protected_resource_permitted(world: &mut SpikeWorld) {
    world.install(Policy::Authenticated, Some(Decision::Permit));
}

/// Creates a protected resource with no enforcer available to the runner.
#[given("a protected resource without an enforcer")]
fn protected_resource_without_enforcer(world: &mut SpikeWorld) {
    world.install(Policy::Authenticated, None);
}

/// Invokes the selected resource without presenting an identity.
#[when("the selected resource crosses an awaited hook")]
async fn selected_resource_crosses_hook(world: &mut SpikeWorld) {
    let registered = world.register_selected();
    let headers = HeaderMap::new();
    let view = RequestView::new(Method::Get, "/catalogue", &headers, None);
    let mut body = adapt_body(Empty::<Bytes>::new());
    let outcome = invoke_selected(world, &registered, view, &mut body).await;
    world.outcome = Some(outcome);
}

/// Invokes the selected resource with a present identity.
#[when("its frozen policy and method cross an awaited hook")]
async fn frozen_policy_crosses_hook(world: &mut SpikeWorld) {
    let registered = world.register_selected();
    let headers = HeaderMap::new();
    let identity = Identity::new("user-42");
    let view = RequestView::new(Method::Get, "/catalogue", &headers, Some(&identity));
    let mut body = adapt_body(Empty::<Bytes>::new());
    let outcome = invoke_selected(world, &registered, view, &mut body).await;
    world.outcome = Some(outcome);
}

/// Invokes the selected resource without presenting an identity.
#[when("an anonymous request crosses the admission boundary")]
async fn anonymous_request_crosses_boundary(world: &mut SpikeWorld) {
    selected_resource_crosses_hook(world).await;
}

/// Validates registration for the protected resource and records the result.
#[when("the specimen validates registration")]
fn validates_registration(world: &mut SpikeWorld) {
    let runner = world.runner();
    let resource = Arc::clone(world.catalogue());
    world.registration = Some(runner.register(resource, Method::Get));
}

/// Checks that the responder ran exactly once.
#[then("that resource's responder runs once")]
fn responder_runs_once(world: &SpikeWorld) {
    assert_eq!(
        world.catalogue().state().responder_calls(),
        1,
        "the selected resource's responder must run exactly once"
    );
}

/// Checks that the injected dependency produced the observed response body.
#[then("its dependency supplies the response")]
fn dependency_supplies_response(world: &SpikeWorld) {
    let Some(draft) = world.outcome().responded() else {
        panic!("a permitted invocation must respond");
    };
    let expected = Bytes::from_static(b"catalogue-record /catalogue 0");
    assert_eq!(
        draft.body(),
        &expected,
        "the injected dependency must supply the response body"
    );
}

/// Checks that the denied invocation left the resource untouched.
#[then("the selected resource is unchanged")]
fn resource_unchanged(world: &SpikeWorld) {
    assert_eq!(
        world.catalogue().state().responder_calls(),
        0,
        "a denial must not reach the responder"
    );
    assert_eq!(
        world.catalogue().dependency().calls(),
        0,
        "a denial must not consult the dependency"
    );
}

/// Checks that no responder ran during the scenario.
#[then("no responder runs")]
fn no_responder_runs(world: &SpikeWorld) {
    assert_eq!(
        world.catalogue().state().responder_calls(),
        0,
        "an invocation that was not permitted must not reach the responder"
    );
}

/// Checks that registration failed with the typed missing-enforcer error.
#[then("registration fails with a typed missing-enforcer error")]
fn registration_fails_typed(world: &SpikeWorld) {
    let Some(registration) = world.registration.as_ref() else {
        panic!("the scenario did not validate registration");
    };
    match registration {
        Ok(_) => panic!("a protected resource must not register without an enforcer"),
        Err(error) => assert!(
            matches!(error, RegistrationError::MissingEnforcer { .. }),
            "expected a typed missing-enforcer error, got {error:?}"
        ),
    }
}

/// Checks that the invocation was denied for a missing identity.
#[then("the request is denied for a missing identity")]
fn denied_for_missing_identity(world: &SpikeWorld) {
    assert_eq!(
        world.outcome().denied(),
        Some(DenialReason::MissingIdentity),
        "an anonymous request must be denied for a missing identity"
    );
}

#[scenario(
    path = "tests/features/resource_policy_spike.feature",
    name = "Public resource permits direct invocation"
)]
#[tokio::test(flavor = "current_thread")]
async fn public_resource_permits_direct_invocation(world: SpikeWorld) { let _ = world; }

#[scenario(
    path = "tests/features/resource_policy_spike.feature",
    name = "Protected policy denies before invocation"
)]
#[tokio::test(flavor = "current_thread")]
async fn protected_policy_denies_before_invocation(world: SpikeWorld) { let _ = world; }

#[scenario(
    path = "tests/features/resource_policy_spike.feature",
    name = "Protected registration requires enforcement"
)]
fn protected_registration_requires_enforcement(world: SpikeWorld) { let _ = world; }

#[scenario(
    path = "tests/features/resource_policy_spike.feature",
    name = "Present identity permits a protected invocation"
)]
#[tokio::test(flavor = "current_thread")]
async fn present_identity_permits_protected_invocation(world: SpikeWorld) { let _ = world; }

#[scenario(
    path = "tests/features/resource_policy_spike.feature",
    name = "Absent identity is denied before the hook"
)]
#[tokio::test(flavor = "current_thread")]
async fn absent_identity_is_denied_before_the_hook(world: SpikeWorld) { let _ = world; }
