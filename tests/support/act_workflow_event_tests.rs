//! Direct event-shape regressions for hosted validation and optional manual Act.

use rstest::rstest;
use serde_yaml::Value;

use super::super::{
    valid_hosted_events,
    valid_manual_events,
    valid_pull_request_types,
    valid_triggers,
};

/// The accepted hosted event mapping, independent of job and action policy.
const HOSTED: &str =
    "on: {pull_request: {types: [opened, synchronize, reopened]}, workflow_dispatch: null}";
/// The accepted optional Act event mapping.
const MANUAL: &str = "on: {workflow_dispatch: null}";

#[rstest]
#[case::accepted(HOSTED, true)]
#[case::missing_on("{}", false)]
#[case::null_on("on: null", false)]
#[case::scalar_on("on: pull_request", false)]
#[case::sequence_on("on: [pull_request, workflow_dispatch]", false)]
#[case::empty_on("on: {}", false)]
#[case::missing_pr("on: {workflow_dispatch: null}", false)]
#[case::missing_dispatch("on: {pull_request: {types: [opened, synchronize, reopened]}}", false)]
#[case::extra_event(
    "on: {pull_request: {types: [opened, synchronize, reopened]}, workflow_dispatch: null, push: \
     null}",
    false
)]
#[case::nonnull_dispatch(
    "on: {pull_request: {types: [opened, synchronize, reopened]}, workflow_dispatch: {}}",
    false
)]
#[case::invalid_pr("on: {pull_request: null, workflow_dispatch: null}", false)]
#[case::invalid_types(
    "on: {pull_request: {types: [reopened, synchronize, opened]}, workflow_dispatch: null}",
    false
)]
#[case::extra_pr_field(
    "on: {pull_request: {types: [opened, synchronize, reopened], branches: []}, \
     workflow_dispatch: null}",
    false
)]
#[case::scalar_dispatch(
    "on: {pull_request: {types: [opened, synchronize, reopened]}, workflow_dispatch: false}",
    false
)]
fn hosted_events_require_exact_mapping(#[case] source: &str, #[case] expected: bool) {
    let value: Value = serde_yaml::from_str(source).expect("parse hosted event fixture");
    assert_eq!(
        valid_hosted_events(&value),
        expected,
        "hosted event contract must classify {source}"
    );
}

#[rstest]
#[case::accepted("on: {pull_request: {types: [opened, synchronize, reopened]}}", true)]
#[case::missing("on: {}", false)]
#[case::null("on: {pull_request: null}", false)]
#[case::scalar("on: {pull_request: true}", false)]
#[case::sequence("on: {pull_request: [opened, synchronize, reopened]}", false)]
#[case::missing_types("on: {pull_request: {}}", false)]
#[case::null_types("on: {pull_request: {types: null}}", false)]
#[case::scalar_types("on: {pull_request: {types: opened}}", false)]
#[case::mapping_types("on: {pull_request: {types: {opened: null}}}", false)]
#[case::empty_types("on: {pull_request: {types: []}}", false)]
#[case::short_types("on: {pull_request: {types: [opened, synchronize]}}", false)]
#[case::extra_type(
    "on: {pull_request: {types: [opened, synchronize, reopened, closed]}}",
    false
)]
#[case::reordered("on: {pull_request: {types: [reopened, synchronize, opened]}}", false)]
#[case::duplicate("on: {pull_request: {types: [opened, opened, reopened]}}", false)]
#[case::wrong_type("on: {pull_request: {types: [opened, closed, reopened]}}", false)]
#[case::nonnull_type("on: {pull_request: {types: [opened, true, reopened]}}", false)]
#[case::number_type("on: {pull_request: {types: [opened, 42, reopened]}}", false)]
#[case::sequence_type("on: {pull_request: {types: [opened, [], reopened]}}", false)]
#[case::mapping_type("on: {pull_request: {types: [opened, {}, reopened]}}", false)]
#[case::null_type("on: {pull_request: {types: [opened, null, reopened]}}", false)]
#[case::extra_pr_field(
    "on: {pull_request: {types: [opened, synchronize, reopened], branches: [main]}}",
    false
)]
fn pull_request_types_require_exact_order(#[case] source: &str, #[case] expected: bool) {
    let value: Value = serde_yaml::from_str(source).expect("parse PR types fixture");
    assert_eq!(
        valid_pull_request_types(&value),
        expected,
        "PR types contract must classify {source}"
    );
}

#[rstest]
#[case::accepted(MANUAL, true)]
#[case::missing_on("{}", false)]
#[case::null_on("on: null", false)]
#[case::scalar_on("on: workflow_dispatch", false)]
#[case::sequence_on("on: [workflow_dispatch]", false)]
#[case::empty_on("on: {}", false)]
#[case::wrong_event("on: {push: null}", false)]
#[case::extra_event("on: {workflow_dispatch: null, pull_request: null}", false)]
#[case::nonnull_dispatch("on: {workflow_dispatch: {}}", false)]
#[case::scalar_dispatch("on: {workflow_dispatch: true}", false)]
#[case::sequence_dispatch("on: {workflow_dispatch: []}", false)]
fn manual_events_require_only_null_dispatch(#[case] source: &str, #[case] expected: bool) {
    let value: Value = serde_yaml::from_str(source).expect("parse manual event fixture");
    assert_eq!(
        valid_manual_events(&value),
        expected,
        "manual event contract must classify {source}"
    );
}

#[rstest]
#[case::both_valid(HOSTED, MANUAL, true)]
#[case::invalid_hosted("{}", MANUAL, false)]
#[case::invalid_manual(HOSTED, "{}", false)]
#[case::both_invalid("{}", "{}", false)]
fn trigger_boundary_composes_independent_predicates(
    #[case] hosted: &str,
    #[case] manual: &str,
    #[case] expected: bool,
) {
    let ci: Value = serde_yaml::from_str(hosted).expect("parse hosted composition fixture");
    let act: Value = serde_yaml::from_str(manual).expect("parse manual composition fixture");
    assert_eq!(
        valid_triggers(&ci, &act),
        expected,
        "both independent event boundaries must hold"
    );
}
