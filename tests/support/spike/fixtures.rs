//! Resources, dependencies, and helpers shared by the spike's runtime suites.
//!
//! The settings are mundane on purpose: each fixture owns its typed policy and
//! its injected dependency, so a test can assert that an observation came from
//! the registration that retained the resource rather than from a global.
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

use std::{error::Error, sync::Arc};

use peregrine_web::spike::{Decision, Policy, PolicyEnforcer, PolicyError, ScopeName, ScopeSet};
use rstest::fixture;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::{
    spike_admission::{AdmissionEvent, FixtureEnforcer},
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

/// A catalogue resource whose policy requires one scope.
#[fixture]
pub fn scoped_catalogue(
    catalogue_dependency: Arc<DependencyState>,
) -> Result<Arc<CatalogueResource>, PolicyError> {
    let scope = ScopeName::new("catalogue:read")?;
    let scopes = ScopeSet::new([scope])?;
    Ok(CatalogueResource::new(
        "catalogue",
        Policy::Scopes(scopes),
        catalogue_dependency,
    ))
}

/// Builds a suspended-at-admission enforcer and the receiver of its events.
pub fn enforcer_with(
    decision: Decision,
) -> (Arc<FixtureEnforcer>, UnboundedReceiver<AdmissionEvent>) {
    FixtureEnforcer::new(decision)
}

/// Wraps an enforcer in the trait object the runner accepts.
///
/// The concrete clone is bound first so the unsizing coercion happens on the
/// value rather than being pushed back into `Arc::clone`'s type parameter.
pub fn as_enforcer(enforcer: &Arc<FixtureEnforcer>) -> Arc<dyn PolicyEnforcer> {
    let concrete: Arc<FixtureEnforcer> = Arc::clone(enforcer);
    concrete
}
