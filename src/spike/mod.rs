//! Experimental resource-policy and async-ownership specimen.
//!
//! This module exists to compile and exercise the design questions recorded in
//! the roadmap 1.1.3 execution plan: whether a resource can own its policy and
//! dependencies, whether policy can be frozen before admission, whether an
//! erased body can stay `Send` but not `Sync` across a real await, and whether
//! one conditional invocation is expressible without route tables or shared
//! mutable state.
//!
//! **Nothing here is a stable framework API.** The specimen is not wired into a
//! production router or server, and it may change or disappear once the
//! evidence is recorded. See `docs/resource-policy-ownership-spike.md`.
//!
//! The specimen is deliberately the inbound-adapter side of the seam. Its
//! transport-facing edge is [`RequestView`], [`ResponseDraft`], and
//! [`ErasedBody`]; its domain-facing contracts are [`Resource`], [`Policy`], and
//! [`PolicyEnforcer`]. Translating between the two at a dedicated adapter
//! boundary, and giving that boundary its own namespace, is the phase-2
//! decision recorded in ADR 007 and tracked by issue #48. This specimen exists
//! to compile the seam as it stands, not to settle that boundary.

mod body;
mod invoke;
mod method;
mod policy;
mod policy_enforcer;
mod request;
mod resource;

pub use self::{
    body::{BodyError, ErasedBody, adapt_body},
    invoke::{InvocationOutcome, RegisteredResource, RegistrationError, ResourceRunner},
    method::Method,
    policy::{Policy, PolicyError, ScopeName, ScopeSet},
    policy_enforcer::{AdmissionRequest, Decision, DenialReason, PolicyEnforcer},
    request::{AllocationId, Identity, RequestView, ResourceObservation, ResponseDraft},
    resource::{Resource, ResourceError},
};
