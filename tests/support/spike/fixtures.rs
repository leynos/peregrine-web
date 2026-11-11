//! Resources, dependencies, and helpers shared by the spike's runtime suites.
//!
//! The settings are mundane on purpose: each fixture owns its typed policy and
//! its injected dependency, so a test can assert that an observation came from
//! the registration that retained the resource rather than from a global.
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

use std::{error::Error, sync::Arc};

use http::HeaderMap;
use peregrine_web::spike::{
    Decision,
    ErasedBody,
    Identity,
    InvocationOutcome,
    Method,
    Policy,
    PolicyEnforcer,
    PolicyError,
    RegisteredResource,
    RequestView,
    ResourceRunner,
    ScopeName,
    ScopeSet,
};
use rstest::fixture;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::{
    spike_admission::{
        AdmissionEvent,
        FixtureEnforcer,
        InvocationSubject,
        RendezvousEntry,
        invoke_with_rendezvous,
    },
    spike_components::{CatalogueResource, DependencyState},
};

/// Converts a Googletest assertion failure into the error the harness prints.
///
/// `verify_that!` reports failures as `googletest::Result`, whose failure type
/// is not `std::error::Error`. Tests here also propagate domain errors, so the
/// assertion failure travels back to the harness as its rendered message.
pub fn verified(outcome: googletest::Result<()>) -> Result<(), Box<dyn Error>> {
    outcome.map_err(|failure| failure.to_string().into())
}

/// The dependency the catalogue fixture injects.
#[fixture]
pub fn catalogue_dependency() -> Arc<DependencyState> {
    DependencyState::new("catalogue", "catalogue-record")
}

/// The dependency the billing fixture injects.
#[fixture]
pub fn billing_dependency() -> Arc<DependencyState> {
    DependencyState::new("billing", "billing-record")
}

/// A catalogue resource whose policy is public.
#[fixture]
pub fn public_catalogue(catalogue_dependency: Arc<DependencyState>) -> Arc<CatalogueResource> {
    CatalogueResource::new("catalogue", Policy::Public, catalogue_dependency)
}

/// A catalogue resource whose policy requires an authenticated identity.
#[fixture]
pub fn protected_catalogue(catalogue_dependency: Arc<DependencyState>) -> Arc<CatalogueResource> {
    CatalogueResource::new("catalogue", Policy::Authenticated, catalogue_dependency)
}

/// Builds a catalogue resource whose policy requires the given scopes.
///
/// Both scoped fixtures differ only in the scopes they declare, so the
/// construction lives here once rather than being repeated per fixture.
fn scoped_catalogue_with(
    scopes: impl IntoIterator<Item = ScopeName>,
    dependency: Arc<DependencyState>,
) -> Result<Arc<CatalogueResource>, PolicyError> {
    Ok(CatalogueResource::new(
        "catalogue",
        Policy::Scopes(ScopeSet::new(scopes)?),
        dependency,
    ))
}

/// A catalogue resource whose policy requires one scope.
#[fixture]
pub fn scoped_catalogue(
    catalogue_dependency: Arc<DependencyState>,
) -> Result<Arc<CatalogueResource>, PolicyError> {
    scoped_catalogue_with([ScopeName::new("catalogue:read")?], catalogue_dependency)
}

/// A catalogue resource whose policy requires three distinct scopes.
///
/// The scopes are deliberately not in alphabetical order, so a witness that
/// compares the ordered contents distinguishes declaration order from any
/// incidental sorting. A policy that kept only the first scope would otherwise
/// satisfy a single-scope fixture just as well.
#[fixture]
pub fn multi_scoped_catalogue(
    catalogue_dependency: Arc<DependencyState>,
) -> Result<Arc<CatalogueResource>, PolicyError> {
    scoped_catalogue_with(
        [
            ScopeName::new("catalogue:write")?,
            ScopeName::new("catalogue:read")?,
            ScopeName::new("catalogue:admin")?,
        ],
        catalogue_dependency,
    )
}

/// Builds a suspended-at-admission enforcer and the receiver of its events.
pub fn enforcer_with(
    decision: Decision,
) -> (Arc<FixtureEnforcer>, UnboundedReceiver<AdmissionEvent>) {
    FixtureEnforcer::new(decision)
}

/// Invokes a registration across the enforcer's real suspension.
///
/// Every caller presents the same authenticated request, so the only variation
/// a caller supplies is the registration under test. The returned
/// [`RendezvousEntry`] reports whether the request reached the hook; a caller
/// that expects a suspension must assert it.
pub async fn invoke_through_rendezvous(
    enforcer: &Arc<FixtureEnforcer>,
    runner: &ResourceRunner,
    registered: &RegisteredResource,
    body: &mut ErasedBody,
) -> (InvocationOutcome, RendezvousEntry) {
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

/// Wraps an enforcer in the trait object the runner accepts.
///
/// The concrete clone is bound first so the unsizing coercion happens on the
/// value rather than being pushed back into `Arc::clone`'s type parameter.
pub fn as_enforcer(enforcer: &Arc<FixtureEnforcer>) -> Arc<dyn PolicyEnforcer> {
    let concrete: Arc<FixtureEnforcer> = Arc::clone(enforcer);
    concrete
}
