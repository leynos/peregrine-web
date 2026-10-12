//! Regression tests for explicitly recorded unset environment expectations.

use std::collections::BTreeMap;

use rstest::{fixture, rstest};

use super::{EnvironmentValue, Invocation, assert_unset_environment};

/// Two observations ensure traversal reaches expectations after the first one.
const EXPECTATIONS: &[(&str, &str)] = &[("FIRST", "first reason"), ("LATER", "later reason")];

/// An executor record with both selected variables explicitly absent.
#[fixture]
fn invocation() -> Invocation {
    Invocation {
        executable: "controlled-executor".to_owned(),
        working_directory: "/fixture".to_owned(),
        environment: EXPECTATIONS
            .iter()
            .map(|(name, _)| ((*name).to_owned(), EnvironmentValue::Unset))
            .collect(),
        secrets: BTreeMap::new(),
        stage: "environment-check".to_owned(),
        cache_state: String::new(),
        arguments: Vec::new(),
        result: None,
    }
}

/// Explicit unset records satisfy every selected expectation.
#[rstest]
fn explicitly_unset_observations_pass(invocation: Invocation) {
    assert_unset_environment(&invocation, EXPECTATIONS);
}

/// Empty, populated, and missing observations fail at either record position.
#[rstest]
#[should_panic(expected = "first reason; executable controlled-executor, stage environment-check")]
#[case("FIRST", Some(EnvironmentValue::Empty))]
#[should_panic(expected = "first reason; executable controlled-executor, stage environment-check")]
#[case("FIRST", Some(EnvironmentValue::Value("caller".to_owned())))]
#[should_panic(expected = "first reason; executable controlled-executor, stage environment-check")]
#[case("FIRST", None)]
#[should_panic(expected = "later reason; executable controlled-executor, stage environment-check")]
#[case("LATER", Some(EnvironmentValue::Empty))]
#[should_panic(expected = "later reason; executable controlled-executor, stage environment-check")]
#[case("LATER", Some(EnvironmentValue::Value("caller".to_owned())))]
#[should_panic(expected = "later reason; executable controlled-executor, stage environment-check")]
#[case("LATER", None)]
fn contaminated_observation_fails(
    mut invocation: Invocation,
    #[case] variable: &str,
    #[case] observation: Option<EnvironmentValue>,
) {
    if let Some(value) = observation {
        invocation.environment.insert(variable.to_owned(), value);
    } else {
        invocation.environment.remove(variable);
    }
    assert_unset_environment(&invocation, EXPECTATIONS);
}
