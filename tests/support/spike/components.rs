//! Application-owned resources used by the spike's runtime witnesses.
//!
//! Every fixture resource owns its typed policy, its per-instance state, and its
//! injected dependency. Nothing consults a registry, a service locator, or
//! shared mutable application state, so an observation made by the enforcer can
//! only have come from the registration that retained the resource.
//!
//! # Lint scope
//!
//! Each integration binary includes a different subset of the fixture modules,
//! so an item used only by a sibling binary looks unused here. The allowance is
//! confined to this fixture module rather than the whole test crate.

#![allow(
    dead_code,
    reason = "each integration binary includes a different subset of these fixtures"
)]

use std::{
    fmt,
    fmt::Formatter,
    sync::{
        Arc,
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use async_trait::async_trait;
use bytes::Bytes;
use peregrine_web::spike::{
    AllocationId,
    ErasedBody,
    Method,
    Policy,
    RequestView,
    Resource,
    ResourceError,
    ResourceObservation,
    ResponseDraft,
};

use crate::spike_frames::drain_body;

/// An injected dependency owned by a fixture resource.
pub struct DependencyState {
    /// A label owned by this allocation.
    label: &'static str,
    /// The response payload this dependency supplies.
    payload: &'static str,
    /// How many times a responder consulted this dependency.
    calls: AtomicUsize,
}

impl DependencyState {
    /// Creates a shared dependency with its own label and payload.
    pub fn new(label: &'static str, payload: &'static str) -> Arc<Self> {
        Arc::new(Self {
            label,
            payload,
            calls: AtomicUsize::new(0),
        })
    }

    /// Returns how many responders consulted this dependency.
    pub fn calls(&self) -> usize { self.calls.load(Ordering::SeqCst) }

    /// Records one consultation by a responder.
    fn record_call(&self) { self.calls.fetch_add(1, Ordering::SeqCst); }

    /// Builds the response this dependency supplies for one request path.
    fn response_for(&self, path: &str, received: usize) -> Bytes {
        Bytes::from(format!("{} {path} {received}", self.payload))
    }
}

/// Renders the dependency without hiding the allocation it stands for.
///
/// The derived implementation would be ignored by unused-field analysis, so the
/// manual one both names the allocation in diagnostics and reads every field.
impl fmt::Debug for DependencyState {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DependencyState")
            .field("label", &self.label)
            .field("payload", &self.payload)
            .field("calls", &self.calls())
            .finish()
    }
}

/// Per-resource state that owns the typed policy.
#[derive(Debug)]
pub struct ResourceState {
    /// A label owned by this resource instance.
    label: &'static str,
    /// The immutable policy this resource owns.
    policy: Policy,
    /// How many times the responder ran.
    responder_calls: AtomicUsize,
    /// Trailer pairs the responder received, in arrival order.
    received_trailers: Mutex<Vec<(String, Vec<u8>)>>,
}

impl ResourceState {
    /// Creates shared resource state with its own label and policy.
    pub fn new(label: &'static str, policy: Policy) -> Arc<Self> {
        Arc::new(Self {
            label,
            policy,
            responder_calls: AtomicUsize::new(0),
            received_trailers: Mutex::new(Vec::new()),
        })
    }

    /// Returns the label owned by this resource instance.
    pub const fn label(&self) -> &'static str { self.label }

    /// Returns the policy owned by this resource.
    pub const fn policy(&self) -> &Policy { &self.policy }

    /// Returns how many times the responder ran.
    pub fn responder_calls(&self) -> usize { self.responder_calls.load(Ordering::SeqCst) }

    /// Records one responder invocation.
    fn record_responder_call(&self) { self.responder_calls.fetch_add(1, Ordering::SeqCst); }

    /// Returns the trailer pairs the responder has received.
    pub fn trailers(&self) -> Vec<(String, Vec<u8>)> {
        match self.received_trailers.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// Records the trailer pairs one request body carried.
    fn record_trailers(&self, trailers: Vec<(String, Vec<u8>)>) {
        match self.received_trailers.lock() {
            Ok(mut guard) => guard.extend(trailers),
            Err(poisoned) => poisoned.into_inner().extend(trailers),
        }
    }
}

/// Shared accessors implemented by every fixture resource type.
pub trait OwnedResource: Send + Sync {
    /// Returns the resource's own retained state.
    fn state(&self) -> &Arc<ResourceState>;

    /// Returns the resource's injected dependency.
    fn dependency(&self) -> &Arc<DependencyState>;

    /// Builds identity observations from this resource's own allocations.
    fn observe(&self) -> ResourceObservation {
        ResourceObservation {
            resource: AllocationId::of(self.state()),
            dependency: AllocationId::of(self.dependency()),
            label: self.state().label(),
        }
    }
}

/// A catalogue-shaped fixture resource.
#[derive(Debug)]
pub struct CatalogueResource {
    /// The resource's own retained state.
    state: Arc<ResourceState>,
    /// The resource's injected dependency.
    dependency: Arc<DependencyState>,
}

impl CatalogueResource {
    /// Creates a catalogue resource with its own state and dependency.
    pub fn new(label: &'static str, policy: Policy, dependency: Arc<DependencyState>) -> Arc<Self> {
        Arc::new(Self {
            state: ResourceState::new(label, policy),
            dependency,
        })
    }
}

impl OwnedResource for CatalogueResource {
    fn state(&self) -> &Arc<ResourceState> { &self.state }

    fn dependency(&self) -> &Arc<DependencyState> { &self.dependency }
}

/// A billing-shaped fixture resource with a different concrete type.
#[derive(Debug)]
pub struct BillingResource {
    /// The resource's own retained state.
    state: Arc<ResourceState>,
    /// The resource's injected dependency.
    dependency: Arc<DependencyState>,
}

impl BillingResource {
    /// Creates a billing resource with its own state and dependency.
    pub fn new(label: &'static str, policy: Policy, dependency: Arc<DependencyState>) -> Arc<Self> {
        Arc::new(Self {
            state: ResourceState::new(label, policy),
            dependency,
        })
    }
}

impl OwnedResource for BillingResource {
    fn state(&self) -> &Arc<ResourceState> { &self.state }

    fn dependency(&self) -> &Arc<DependencyState> { &self.dependency }
}

/// Implements [`Resource`] for one fixture resource type.
macro_rules! impl_fixture_resource {
    ($type:ty) => {
        #[async_trait]
        impl Resource for $type {
            fn policy(&self, _method: Method) -> &Policy { self.state().policy() }

            fn observation(&self) -> ResourceObservation { self.observe() }

            async fn get(
                &self,
                view: RequestView<'_>,
                body: &mut ErasedBody,
            ) -> Result<ResponseDraft, ResourceError> {
                respond(self, view, body).await
            }
        }
    };
}

impl_fixture_resource!(CatalogueResource);
impl_fixture_resource!(BillingResource);

/// Runs one fixture responder, consuming the request body and consulting the
/// injected dependency.
async fn respond<T: OwnedResource>(
    resource: &T,
    view: RequestView<'_>,
    body: &mut ErasedBody,
) -> Result<ResponseDraft, ResourceError> {
    resource.state().record_responder_call();
    let drained = drain_body(body).await?;
    resource.state().record_trailers(drained.trailers);
    resource.dependency().record_call();
    Ok(ResponseDraft::ok(
        resource
            .dependency()
            .response_for(view.path(), drained.data.len()),
    ))
}
