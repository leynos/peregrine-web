//! Regression tests for how CI provisions Whitaker (concordat QG-002).
//!
//! The estate provisions Whitaker one way: the shared-actions
//! `install-whitaker` action, pinned to a listed revision, with the lint suite
//! left to the rolling release. These tests hold the CI workflow to it.

use std::collections::BTreeMap;

use serde::Deserialize;

const CI_WORKFLOW: &str = include_str!("../.github/workflows/ci.yml");
const INSTALL_ACTION: &str = "leynos/shared-actions/.github/actions/install-whitaker@";
const PINNED_REVISION: &str = "6cec89bac47a21cf756d68d638a9a510998e57f8";

#[derive(Deserialize)]
struct Workflow {
    jobs: BTreeMap<String, Job>,
}

#[derive(Deserialize)]
struct Job {
    steps: Vec<Step>,
}

#[derive(Deserialize)]
struct Step {
    uses: Option<String>,
    run: Option<String>,
    with: Option<BTreeMap<String, serde_yaml::Value>>,
}

/// Verifies the CI job installs Whitaker through the pinned action.
#[test]
fn ci_provisions_whitaker_through_the_pinned_action_before_linting() {
    let workflow = parse(CI_WORKFLOW);

    assert!(
        provisions_whitaker_before_lint(&workflow),
        "build-test must run {INSTALL_ACTION}{PINNED_REVISION} with cranelift: true before make \
         lint"
    );
}

/// Verifies no step provisions Whitaker by hand.
#[test]
fn ci_does_not_install_whitaker_by_hand() {
    let workflow = parse(CI_WORKFLOW);

    assert!(
        !installs_whitaker_by_hand(&workflow),
        "no step may run whitaker-installer or binstall a Whitaker tool"
    );
}

/// Rejects a workflow whose Whitaker install is missing, late, unpinned or
/// without Cranelift.
#[test]
fn rejects_each_way_of_missing_the_contract() {
    let action = format!("{INSTALL_ACTION}{PINNED_REVISION}");
    let cases = [
        (
            "no install",
            "jobs:\n  build-test:\n    steps:\n      - run: make lint\n".to_owned(),
        ),
        (
            "install after lint",
            format!(
                "jobs:\n  build-test:\n    steps:\n      - run: make lint\n      - uses: \
                 {action}\n        with:\n          cranelift: true\n"
            ),
        ),
        (
            "a different pin",
            format!(
                "jobs:\n  build-test:\n    steps:\n      - uses: {INSTALL_ACTION}{}\n        \
                 with:\n          cranelift: true\n      - run: make lint\n",
                "0".repeat(40)
            ),
        ),
        (
            "no cranelift",
            format!(
                "jobs:\n  build-test:\n    steps:\n      - uses: {action}\n      - run: make \
                 lint\n"
            ),
        ),
    ];

    for (label, source) in cases {
        assert!(
            !provisions_whitaker_before_lint(&parse(&source)),
            "{label} must fail the contract"
        );
    }
}

/// Rejects hand-rolled installs of the Whitaker tools.
#[test]
fn rejects_a_hand_rolled_install() {
    for run in [
        "cargo binstall --no-confirm whitaker-installer@0.2.6",
        "cargo install whitaker-installer",
        "whitaker-installer --cranelift",
    ] {
        let source = format!("jobs:\n  build-test:\n    steps:\n      - run: {run}\n");
        assert!(
            installs_whitaker_by_hand(&parse(&source)),
            "{run} must be reported as a hand-rolled install"
        );
    }
}

/// Parses a workflow, failing the test when the YAML is invalid.
fn parse(source: &str) -> Workflow {
    match serde_yaml::from_str(source) {
        Ok(workflow) => workflow,
        Err(error) => panic!("the workflow must be valid YAML: {error}"),
    }
}

/// Reports whether `build-test` runs the pinned action, with Cranelift on,
/// before the first step that runs `make lint`.
fn provisions_whitaker_before_lint(workflow: &Workflow) -> bool {
    let Some(job) = workflow.jobs.get("build-test") else {
        return false;
    };
    let install_at = job.steps.iter().position(is_pinned_install);
    let lint_at = job.steps.iter().position(|step| {
        step.run
            .as_deref()
            .is_some_and(|run| run.contains("make lint"))
    });
    matches!((install_at, lint_at), (Some(install), Some(lint)) if install < lint)
}

/// Reports whether a step is the pinned action with `cranelift: true`.
fn is_pinned_install(step: &Step) -> bool {
    let pinned = step
        .uses
        .as_deref()
        .and_then(|uses| uses.strip_prefix(INSTALL_ACTION))
        .is_some_and(|revision| revision == PINNED_REVISION);
    let cranelift = step
        .with
        .as_ref()
        .and_then(|inputs| inputs.get("cranelift"))
        .is_some_and(|value| value.as_bool() == Some(true));
    pinned && cranelift
}

/// Reports whether any step installs a Whitaker tool outside the action.
fn installs_whitaker_by_hand(workflow: &Workflow) -> bool {
    workflow
        .jobs
        .values()
        .flat_map(|job| job.steps.iter())
        .filter_map(|step| step.run.as_deref())
        .any(|run| run.contains("whitaker-installer"))
}
