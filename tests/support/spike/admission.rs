//! A policy enforcer that suspends inside admission, plus its observations.
//!
//! The fixture records one observation before it suspends and one after it
//! resumes, so a test compares identity, method, policy, and path across a real
//! await boundary rather than a synchronous stand-in. The rendezvous driver is
//! what lets the invocation and the observation run to completion together.
//!
//! # Lint scope
//!
//! Several integration binaries include this module through `#[path]`, and each
//! compiles its own copy, so an item that only a sibling binary uses looks
//! unused here. The allowance is confined to this fixture module rather than
//! the whole test crate; a module with a single consumer carries none.
//! Splitting the fixtures per consumer would duplicate them, and per-item
//! allowances would scatter the same suppression through ordinary fixture code.

#![allow(
    dead_code,
    reason = "each integration binary includes a different subset of these fixtures"
)]

use std::{sync::Arc, time::Duration};

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

/// How long the rendezvous driver waits for admission to reach the hook.
///
/// This is a failure bound, not a timing assertion. A permitted request reaches
/// the hook without a meaningful delay, so a driver that exhausts the bound is
/// reporting that the invocation took a path which never suspends.
pub const ENTRY_DEADLINE: Duration = Duration::from_secs(5);

/// Whether the driver saw the invocation reach the enforcer hook.
///
/// Tests assert on this rather than assuming entry, because an outcome decided
/// before admission never suspends and would otherwise be indistinguishable
/// from one that did.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RendezvousEntry {
    /// The invocation reached the hook, and the driver released it.
    Entered,
    /// The invocation never reached the hook within [`ENTRY_DEADLINE`].
    NeverEntered,
}

impl RendezvousEntry {
    /// Returns whether the invocation reached the enforcer hook.
    ///
    /// # Examples
    ///
    /// ```text
    /// assert!(RendezvousEntry::Entered.entered());
    /// assert!(!RendezvousEntry::NeverEntered.entered());
    /// ```
    #[must_use]
    pub const fn entered(self) -> bool { matches!(self, Self::Entered) }
}

/// Invokes a protected registration across the enforcer's real suspension.
///
/// The driver waits for admission to enter the hook and releases it only after
/// it has, so a request that never suspends cannot be released by accident. The
/// wait is bounded by [`ENTRY_DEADLINE`]; the returned [`RendezvousEntry`] tells
/// the caller which of the two happened, and a caller that expects a suspension
/// must assert that entry occurred.
pub async fn invoke_with_rendezvous(
    enforcer: &FixtureEnforcer,
    subject: InvocationSubject<'_>,
) -> (InvocationOutcome, RendezvousEntry) {
    let invocation = subject
        .runner
        .invoke(subject.registered, subject.view, subject.body);
    let (outcome, entry) = tokio::join!(invocation, drive_rendezvous(enforcer));
    (outcome, entry)
}

/// Waits for admission to enter, releasing it only once it has.
async fn drive_rendezvous(enforcer: &FixtureEnforcer) -> RendezvousEntry {
    let reached = tokio::time::timeout(ENTRY_DEADLINE, enforcer.wait_until_entered()).await;
    if reached.is_err() {
        return RendezvousEntry::NeverEntered;
    }
    enforcer.release();
    RendezvousEntry::Entered
}
