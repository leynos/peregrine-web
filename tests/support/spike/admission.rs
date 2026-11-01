//! A policy enforcer that suspends inside admission, plus its observations.
//!
//! The fixture records one observation before it suspends and one after it
//! resumes, so a test compares identity, method, policy, and path across a real
//! await boundary rather than a synchronous stand-in. The rendezvous driver is
//! what lets the invocation and the observation run to completion together.
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

use std::sync::Arc;

use async_trait::async_trait;
use peregrine_web::spike::{
    AdmissionRequest,
    AllocationId,
    Decision,
    ErasedBody,
    InvocationOutcome,
    Method,
    Policy,
    PolicyEnforcer,
    RegisteredResource,
    RequestView,
    ResourceObservation,
    ResourceRunner,
};
use tokio::sync::{
    Notify,
    mpsc::{self, UnboundedReceiver, UnboundedSender},
};

/// The point in admission at which an observation was taken.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionPhase {
    /// Recorded before the enforcer suspends.
    BeforeAwait,
    /// Recorded after the enforcer resumes.
    AfterAwait,
}

/// One observation recorded by the fixture enforcer.
#[derive(Debug, Eq, PartialEq)]
pub struct AdmissionEvent {
    /// When the observation was taken.
    pub phase: AdmissionPhase,
    /// Identity of the resource retained by registration.
    pub handle: AllocationId,
    /// Identity observations declared by that resource.
    pub observation: ResourceObservation,
    /// The frozen method presented to admission.
    pub method: Method,
    /// The policy captured at registration.
    pub policy: Policy,
    /// The request path presented to admission.
    pub path: String,
    /// The presented principal name, when one was supplied.
    pub identity: Option<String>,
}

/// The fixture policy hook with a controllable suspension point.
#[derive(Debug)]
pub struct FixtureEnforcer {
    /// The decision returned after resuming.
    decision: Decision,
    /// Signalled once admission has recorded its pre-await observation.
    entered: Notify,
    /// Released by the driver to let admission resume.
    release: Notify,
    /// Observations sent to the test.
    events: UnboundedSender<AdmissionEvent>,
}

impl FixtureEnforcer {
    /// Creates an enforcer and the receiver of its observations.
    pub fn new(decision: Decision) -> (Arc<Self>, UnboundedReceiver<AdmissionEvent>) {
        let (events, receiver) = mpsc::unbounded_channel();
        let enforcer = Arc::new(Self {
            decision,
            entered: Notify::new(),
            release: Notify::new(),
            events,
        });
        (enforcer, receiver)
    }

    /// Waits until admission has reached its suspension point.
    pub async fn wait_until_entered(&self) { self.entered.notified().await; }

    /// Lets the suspended admission continue.
    pub fn release(&self) { self.release.notify_one(); }

    /// Records one observation for the test.
    fn record(&self, phase: AdmissionPhase, request: AdmissionRequest<'_>) {
        let event = AdmissionEvent {
            phase,
            handle: request.resource_handle(),
            observation: request.resource_observation(),
            method: request.method(),
            policy: request.policy().clone(),
            path: request.view().path().to_owned(),
            identity: request
                .view()
                .identity()
                .map(|identity| identity.principal().to_owned()),
        };
        let _sent = self.events.send(event);
    }
}

#[async_trait]
impl PolicyEnforcer for FixtureEnforcer {
    async fn admit(&self, request: AdmissionRequest<'_>) -> Decision {
        self.record(AdmissionPhase::BeforeAwait, request);
        self.entered.notify_one();
        self.release.notified().await;
        self.record(AdmissionPhase::AfterAwait, request);
        self.decision
    }
}

/// Everything one invocation needs except the enforcer that admits it.
pub struct InvocationSubject<'a> {
    /// The runner that owns registration and admission.
    pub runner: &'a ResourceRunner,
    /// The registration being invoked.
    pub registered: &'a RegisteredResource,
    /// The request view presented to admission.
    pub view: RequestView<'a>,
    /// The request body handed to the responder.
    pub body: &'a mut ErasedBody,
}

/// Invokes a protected registration across the enforcer's real suspension.
///
/// The caller must know the request actually reaches the enforcer. An outcome
/// decided before admission, such as a missing identity, never suspends, so the
/// driver would wait for an entry that cannot happen.
pub async fn invoke_with_rendezvous(
    enforcer: &FixtureEnforcer,
    subject: InvocationSubject<'_>,
) -> InvocationOutcome {
    let invocation = subject
        .runner
        .invoke(subject.registered, subject.view, subject.body);
    let driver = async {
        enforcer.wait_until_entered().await;
        enforcer.release();
    };
    let (outcome, ()) = tokio::join!(invocation, driver);
    outcome
}
