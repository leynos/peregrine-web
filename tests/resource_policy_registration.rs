//! Registration-time policy selection and the enforcer's reach.
//!
//! The shared fixtures declare one policy for every method, so they cannot show
//! that registration asks the resource for the frozen method's policy, nor that
//! a public registration never reaches an enforcer. These witnesses cover those
//! boundaries through the real `peregrine_web::spike` seam: per-method policy
//! selection, a denying enforcer that a public resource never consults, a
//! protected registration invoked through a runner without an enforcer, and the
//! full ordered scope list that registration and admission both observe.

#[path = "support/spike/admission.rs"]
mod spike_admission;
#[path = "support/spike/components.rs"]
mod spike_components;
#[path = "support/spike/fixtures.rs"]
mod spike_fixtures;
#[path = "support/spike/frames.rs"]
mod spike_frames;

use std::{
    error::Error,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use async_trait::async_trait;
use bytes::Bytes;
use googletest::prelude::{eq, matches_pattern, verify_that};
use http::HeaderMap;
use http_body_util::Empty;
use peregrine_web::spike::{
    AdmissionRequest,
    AllocationId,
    Decision,
    DenialReason,
    ErasedBody,
    Identity,
    Method,
    Policy,
    PolicyEnforcer,
    PolicyError,
    RegistrationError,
    RequestView,
    Resource,
    ResourceError,
    ResourceObservation,
    ResourceRunner,
    ResponseDraft,
    ScopeName,
    adapt_body,
};
use rstest::rstest;
use spike_components::{CatalogueResource, OwnedResource};
use spike_fixtures::{
    as_enforcer,
    enforcer_with,
    invoke_through_rendezvous,
    multi_scoped_catalogue,
    protected_catalogue,
    public_catalogue,
    verified,
};

/// A resource whose policy depends on the method it is registered for.
///
/// Registration freezes the method before asking for a policy, so this resource
/// answers `GET` publicly and every other method as authenticated. Reading the
/// captured policy back proves which method the question was asked about.
struct MethodScopedResource {
    /// Names this resource in its own observation.
    marker: Arc<()>,
    /// Stands in for the resource's injected dependency.
    dependency: Arc<()>,
}

impl MethodScopedResource {
    /// Creates the resource with its own identity allocations.
    fn new() -> Arc<Self> {
        Arc::new(Self {
            marker: Arc::new(()),
            dependency: Arc::new(()),
        })
    }
}

#[async_trait]
impl Resource for MethodScopedResource {
    fn policy(&self, method: Method) -> &Policy {
        if method == Method::Get {
            &Policy::Public
        } else {
            &Policy::Authenticated
        }
    }

    fn observation(&self) -> ResourceObservation {
        ResourceObservation {
            resource: AllocationId::of(&self.marker),
            dependency: AllocationId::of(&self.dependency),
            label: "method-scoped",
        }
    }

    async fn get(
        &self,
        _view: RequestView<'_>,
        _body: &mut ErasedBody,
    ) -> Result<ResponseDraft, ResourceError> {
        Ok(ResponseDraft::ok(Bytes::new()))
    }
}

/// Checks that registration captures the policy belonging to the frozen method.
#[rstest]
fn registration_captures_the_policy_for_the_frozen_method() -> Result<(), Box<dyn Error>> {
    let (enforcer, _events) = enforcer_with(Decision::Permit);
    let runner = ResourceRunner::with_enforcer(as_enforcer(&enforcer));

    let get = runner.register(MethodScopedResource::new(), Method::Get)?;
    let post = runner.register(MethodScopedResource::new(), Method::Post)?;

    pretty_assertions::assert_eq!(
        (get.method(), get.policy()),
        (Method::Get, &Policy::Public),
        "a GET registration must capture the public policy for GET"
    );
    pretty_assertions::assert_eq!(
        (post.method(), post.policy()),
        (Method::Post, &Policy::Authenticated),
        "a POST registration must capture the authenticated policy for POST"
    );
    Ok(())
}

/// Checks that an enforcer-less runner accepts only the public method.
#[rstest]
fn registration_without_an_enforcer_accepts_only_public_methods() -> Result<(), Box<dyn Error>> {
    let runner = ResourceRunner::new();

    let public = runner.register(MethodScopedResource::new(), Method::Get)?;

    pretty_assertions::assert_eq!(
        public.policy(),
        &Policy::Public,
        "a public method must register without an enforcer"
    );
    let Err(error) = runner.register(MethodScopedResource::new(), Method::Post) else {
        return Err("a protected method must not register without an enforcer".into());
    };
    verified(verify_that!(
        error,
        matches_pattern!(RegistrationError::MissingEnforcer {
            method: eq(&Method::Post),
            policy: eq(&Policy::Authenticated),
        })
    ))?;
    Ok(())
}

/// A denying enforcer that counts its consultations and never suspends.
///
/// The shared `spike_admission::FixtureEnforcer` suspends until its driver
/// releases it, which would turn a regression that consults the enforcer for a
/// public resource into a hang rather than a failed assertion. This double
/// returns immediately and counts calls, so the witness fails promptly with the
/// count it observed.
#[derive(Debug, Default)]
struct CountingDenier {
    /// How many admissions the enforcer was asked to decide.
    consultations: AtomicUsize,
}

impl CountingDenier {
    /// Returns the number of admissions asked for so far.
    fn consultations(&self) -> usize { self.consultations.load(Ordering::SeqCst) }
}

#[async_trait]
impl PolicyEnforcer for CountingDenier {
    async fn admit(&self, _request: AdmissionRequest<'_>) -> Decision {
        self.consultations.fetch_add(1, Ordering::SeqCst);
        Decision::Deny(DenialReason::EnforcerDenied)
    }
}

/// Checks that a public resource answers without consulting a denying enforcer.
///
/// The enforcer is configured to deny everything, so a response can only mean
/// admission never asked it. This is the public half of the reach boundary that
/// `enforcer_denial_withholds_the_request` covers for protected resources.
#[rstest]
#[tokio::test]
async fn a_public_resource_never_reaches_a_denying_enforcer(
    public_catalogue: Arc<CatalogueResource>,
) -> Result<(), Box<dyn Error>> {
    let enforcer = Arc::new(CountingDenier::default());
    let hook: Arc<dyn PolicyEnforcer> = Arc::clone(&enforcer) as Arc<dyn PolicyEnforcer>;
    let runner = ResourceRunner::with_enforcer(hook);
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
    pretty_assertions::assert_eq!(
        enforcer.consultations(),
        0,
        "a public registration must never reach the enforcer"
    );
    Ok(())
}

/// Checks that a protected registration invoked without an enforcer is denied.
///
/// Registration is allowed because the registering runner has an enforcer, but
/// the invoking runner has none. The request must be denied for that reason and
/// must never reach the responder or its dependency.
#[rstest]
#[tokio::test]
async fn a_protected_invocation_without_an_enforcer_is_denied(
    protected_catalogue: Arc<CatalogueResource>,
) -> Result<(), Box<dyn Error>> {
    let (enforcer, _events) = enforcer_with(Decision::Permit);
    let registering = ResourceRunner::with_enforcer(as_enforcer(&enforcer));
    let registered = registering.register(protected_catalogue.clone(), Method::Get)?;
    let invoking = ResourceRunner::new();
    let headers = HeaderMap::new();
    let identity = Identity::new("user-42");
    let view = RequestView::new(Method::Get, "/catalogue", &headers, Some(&identity));
    let mut body = adapt_body(Empty::<Bytes>::new());

    let outcome = invoking.invoke(&registered, view, &mut body).await;

    pretty_assertions::assert_eq!(
        outcome.denied(),
        Some(DenialReason::MissingEnforcer),
        "an identified request without a configured enforcer must be denied for that reason"
    );
    pretty_assertions::assert_eq!(
        protected_catalogue.state().responder_calls(),
        0,
        "a denied request must never reach the responder"
    );
    pretty_assertions::assert_eq!(
        protected_catalogue.dependency().calls(),
        0,
        "a denied request must never reach the resource dependency"
    );
    Ok(())
}

/// The scopes the multi-scope fixture declares, in declaration order.
///
/// Kept here rather than in the fixture so the witness states the expectation
/// independently of the code that builds it.
const DECLARED_SCOPES: [&str; 3] = ["catalogue:write", "catalogue:read", "catalogue:admin"];

/// Returns the scope names a policy carries, in order.
///
/// A policy that carries no scopes yields an empty list, which fails the
/// ordered comparison rather than silently matching a scoped expectation.
fn scope_names(policy: &Policy) -> Vec<&str> {
    match policy {
        Policy::Scopes(scopes) => scopes.as_slice().iter().map(ScopeName::as_str).collect(),
        _ => Vec::new(),
    }
}

/// Checks that every declared scope survives registration and reaches admission.
///
/// A registration that kept only the first scope, or that reordered the set,
/// would pass a single-scope witness but fail this one at both boundaries: the
/// policy captured by registration and the policy reported to the enforcer.
#[rstest]
#[tokio::test]
async fn every_declared_scope_reaches_registration_and_the_enforcer(
    multi_scoped_catalogue: Result<Arc<CatalogueResource>, PolicyError>,
) -> Result<(), Box<dyn Error>> {
    let scoped = multi_scoped_catalogue?;
    let (enforcer, mut events) = enforcer_with(Decision::Permit);
    let runner = ResourceRunner::with_enforcer(as_enforcer(&enforcer));
    let registered = runner.register(scoped.clone(), Method::Get)?;

    pretty_assertions::assert_eq!(
        scope_names(registered.policy()),
        DECLARED_SCOPES,
        "registration must retain every declared scope in declaration order"
    );

    let mut body = adapt_body(Empty::<Bytes>::new());
    let (outcome, entry) =
        invoke_through_rendezvous(&enforcer, &runner, &registered, &mut body).await;
    assert!(
        entry.entered(),
        "a scoped request with an identity must reach the enforcer hook"
    );

    let before = events.recv().await.ok_or("a pre-await observation")?;
    let after = events.recv().await.ok_or("a post-await observation")?;
    for event in [&before, &after] {
        pretty_assertions::assert_eq!(
            scope_names(&event.policy),
            DECLARED_SCOPES,
            "the enforcer must observe every declared scope in declaration order"
        );
    }
    verified(verify_that!(outcome.responded().is_some(), eq(true)))?;
    Ok(())
}
