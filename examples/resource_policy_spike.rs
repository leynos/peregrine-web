//! Runnable demonstration of the resource-policy and ownership spike.
//!
//! The example builds one public and one protected resource, each with its own
//! injected dependency, and drives both through the same registration and
//! invocation seam used by the tests. Every observation is checked
//! programmatically: the example returns a diagnostic error instead of printing
//! ad hoc output, so `cargo test` can load and exercise it.
//!
//! Run it with `cargo run --example resource_policy_spike`.
//!
//! This is an experiment, not a framework API. See
//! `docs/resource-policy-ownership-spike.md`.

use std::{error::Error, sync::Arc};

use async_trait::async_trait;
use bytes::Bytes;
use http::HeaderMap;
use http_body_util::{BodyExt, Empty};
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
    RegisteredResource,
    RegistrationError,
    RequestView,
    Resource,
    ResourceError,
    ResourceObservation,
    ResourceRunner,
    ResponseDraft,
    adapt_body,
};

/// A dependency owned by exactly one resource.
struct CatalogueStore {
    /// The payload this store supplies.
    payload: &'static str,
}

impl CatalogueStore {
    /// Creates a store with its own payload.
    fn new(payload: &'static str) -> Arc<Self> { Arc::new(Self { payload }) }

    /// Builds the response for one request path and body length.
    fn response_for(&self, path: &str, received: usize) -> Bytes {
        Bytes::from(format!("{} {path} {received}", self.payload))
    }
}

/// A resource that owns its policy and its injected store.
struct Catalogue {
    /// The policy this resource declares for every method.
    policy: Policy,
    /// The dependency consulted by the responder.
    store: Arc<CatalogueStore>,
    /// A per-instance allocation that names this resource in observations.
    marker: Arc<()>,
}

impl Catalogue {
    /// Creates a catalogue resource over an injected store.
    fn new(policy: Policy, store: Arc<CatalogueStore>) -> Arc<Self> {
        Arc::new(Self {
            policy,
            store,
            marker: Arc::new(()),
        })
    }
}

#[async_trait]
impl Resource for Catalogue {
    fn policy(&self, _method: Method) -> &Policy { &self.policy }

    fn observation(&self) -> ResourceObservation {
        ResourceObservation {
            resource: AllocationId::of(&self.marker),
            dependency: AllocationId::of(&self.store),
            label: self.store.payload,
        }
    }

    async fn get(
        &self,
        view: RequestView<'_>,
        body: &mut ErasedBody,
    ) -> Result<ResponseDraft, ResourceError> {
        let received = drain(body).await?;
        Ok(ResponseDraft::ok(
            self.store.response_for(view.path(), received),
        ))
    }
}

/// Drains an erased body and returns its length.
async fn drain(body: &mut ErasedBody) -> Result<usize, ResourceError> {
    let collected = body.collect().await.map_err(|source| ResourceError::Body {
        source: Box::new(source),
    })?;
    Ok(collected.to_bytes().len())
}

/// Permits exactly one principal and denies everybody else.
struct SinglePrincipalEnforcer {
    /// The only principal this enforcer admits.
    permitted: String,
}

#[async_trait]
impl PolicyEnforcer for SinglePrincipalEnforcer {
    async fn admit(&self, request: peregrine_web::spike::AdmissionRequest<'_>) -> Decision {
        match request.view().identity() {
            Some(identity) if identity.principal() == self.permitted => Decision::Permit,
            Some(_) | None => Decision::Deny(DenialReason::EnforcerDenied),
        }
    }
}

/// Builds the empty request body each invocation carries.
fn empty_body() -> ErasedBody { adapt_body(Empty::<Bytes>::new()) }

/// Fails the example when a required outcome did not occur.
fn require(condition: bool, message: &str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(Box::<dyn Error>::from(message.to_owned()))
    }
}

/// Returns the response body of a permitted invocation.
fn response_body(outcome: &InvocationOutcome) -> Result<Bytes, Box<dyn Error>> {
    outcome.responded().map_or_else(
        || {
            Err(Box::<dyn Error>::from(format!(
                "expected a response, observed {outcome:?}"
            )))
        },
        |draft| Ok(draft.body().clone()),
    )
}

/// Demonstrates the public no-enforcer path.
async fn demonstrate_public_path() -> Result<(), Box<dyn Error>> {
    let store = CatalogueStore::new("public-catalogue");
    let resource = Catalogue::new(Policy::Public, Arc::clone(&store));
    let runner = ResourceRunner::new();
    let registered = runner.register(resource, Method::Get)?;

    let headers = HeaderMap::new();
    let view = RequestView::new(Method::Get, "/catalogue", &headers, None);
    let outcome = runner.invoke(&registered, view, &mut empty_body()).await;

    let body = response_body(&outcome)?;
    require(
        body == Bytes::from_static(b"public-catalogue /catalogue 0"),
        "the public resource must return the store's payload",
    )
}

/// Demonstrates the protected path: denial without identity, then permission.
async fn demonstrate_protected_path() -> Result<(), Box<dyn Error>> {
    let store = CatalogueStore::new("protected-catalogue");
    let resource = Catalogue::new(Policy::Authenticated, store);
    let enforcer: Arc<dyn PolicyEnforcer> = Arc::new(SinglePrincipalEnforcer {
        permitted: "user-42".to_owned(),
    });
    let runner = ResourceRunner::with_enforcer(enforcer);
    let registered: RegisteredResource = runner.register(resource, Method::Get)?;

    let headers = HeaderMap::new();
    let anonymous = RequestView::new(Method::Get, "/catalogue", &headers, None);
    let denied = runner
        .invoke(&registered, anonymous, &mut empty_body())
        .await;
    require(
        denied.denied() == Some(DenialReason::MissingIdentity),
        "an anonymous request must be denied before the hook runs",
    )?;

    let identity = Identity::new("user-42");
    let known = RequestView::new(Method::Get, "/catalogue", &headers, Some(&identity));
    let permitted = runner.invoke(&registered, known, &mut empty_body()).await;
    require(
        response_body(&permitted)? == Bytes::from_static(b"protected-catalogue /catalogue 0"),
        "a permitted identity must reach the resource responder",
    )
}

/// Demonstrates that a protected resource cannot register without enforcement.
fn demonstrate_missing_enforcer() -> Result<(), Box<dyn Error>> {
    let store = CatalogueStore::new("unguarded");
    let resource = Catalogue::new(Policy::Authenticated, store);
    let runner = ResourceRunner::new();

    match runner.register(resource, Method::Get) {
        Ok(_) => Err(Box::<dyn Error>::from(
            "a protected resource must not register without an enforcer",
        )),
        Err(RegistrationError::MissingEnforcer { policy, .. }) => require(
            policy.requires_enforcement(),
            "the rejected registration must name the enforcing policy",
        ),
    }
}

/// Runs the public, protected, and missing-enforcer demonstrations in sequence,
/// returning an error that exits the example non-zero when one fails.
fn main() -> Result<(), Box<dyn Error>> {
    // Build the runtime explicitly so a construction failure propagates as an
    // error: `#[tokio::main]` panics from inside the generated `main`, which the
    // repository's lints treat as a production panic path.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(demonstrate())
}

/// Drives every demonstration and returns the first failed expectation.
///
/// Public so `tests/resource_policy_example.rs` can load this file as a module
/// and exercise the same demonstrations under `cargo test`.
///
/// # Errors
///
/// Returns the first failed registration, denied admission, or unexpected
/// response the demonstrations observe. A passing run means every documented
/// outcome still holds.
pub async fn demonstrate() -> Result<(), Box<dyn Error>> {
    demonstrate_public_path().await?;
    demonstrate_protected_path().await?;
    demonstrate_missing_enforcer()?;
    Ok(())
}
