//! Semantic assertions for the real Act fixture action and shell traces.

use super::contract;

/// Checks both action interfaces and shell routing after a successful job.
pub(super) fn assert_success_trace(trace: &[Vec<String>]) {
    assert_action_inputs(trace);
    assert_lint_environment(trace);
    assert_executor_guards(trace);
}

/// Pins the action order and every declared input consumed by the fixtures.
fn assert_action_inputs(trace: &[Vec<String>]) {
    let actions = trace
        .iter()
        .filter(|event| {
            event
                .first()
                .is_some_and(|name| name.starts_with("action-"))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actions.len(),
        contract::ACTIONS.len() - 1,
        "Act must execute every unguarded fixture action"
    );
    for (index, (action, (_, _, inputs))) in actions.iter().zip(contract::ACTIONS).enumerate() {
        assert_eq!(
            action.first().map(String::as_str),
            Some(format!("action-{index}").as_str()),
            "action order must match the committed workflow"
        );
        assert_eq!(
            action.len(),
            inputs.len() + 2,
            "fixture action {index} must expose every declared input"
        );
        for (offset, (_, expected)) in inputs.iter().enumerate() {
            assert_eq!(
                action.get(offset + 2).map(String::as_str),
                Some(*expected),
                "fixture action {index} must receive the pinned input"
            );
        }
    }
}

/// Requires setup flags and cold driver environment to reach the lint shell step.
fn assert_lint_environment(trace: &[Vec<String>]) {
    let Some(lint) = trace.iter().find(|event| {
        event.first().map(String::as_str) == Some("make")
            && event.get(7).map(String::as_str) == Some("lint")
    }) else {
        panic!("the lint shell step must execute");
    };
    assert_eq!(
        lint.get(2).map(String::as_str),
        Some(""),
        "setup-rust's empty RUSTFLAGS must reach lint"
    );
    assert_eq!(
        lint.get(3).map(String::as_str),
        Some("<unset>"),
        "lint must not receive coverage's LLVM override"
    );
    assert!(
        lint.get(6)
            .is_some_and(|path| path.ends_with("peregrine-whitaker-driver")),
        "lint must see its created private driver cache"
    );
}

/// Checks secret redaction, Act fallback evaluation, and coverage skipping.
fn assert_executor_guards(trace: &[Vec<String>]) {
    assert!(
        trace
            .iter()
            .filter(|event| matches!(
                event.first().map(String::as_str),
                Some("make" | "cargo" | "rustc" | "sudo")
            ))
            .all(|event| matches!(
                event.get(4).map(String::as_str),
                Some("unset" | "empty" | "present")
            )),
        "executor records may disclose only token presence"
    );
    assert!(
        trace
            .iter()
            .any(|event| event.first().map(String::as_str) == Some("make")
                && event.get(7).map(String::as_str) == Some("test")
                && event.get(5).map(String::as_str) == Some("0")),
        "Act must evaluate the test fallback guard and prevent recursion"
    );
    assert!(
        !trace
            .iter()
            .any(|event| event.first().map(String::as_str) == Some("action-7")),
        "the hosted coverage action must be skipped by Act's real expression evaluation"
    );
}

/// Requires a controlled lint failure to prevent the later test fallback.
pub(super) fn assert_failure_trace(new_events: &[Vec<String>]) {
    assert!(
        new_events
            .iter()
            .any(|event| event.first().map(String::as_str) == Some("make")
                && event.get(7).map(String::as_str) == Some("lint")),
        "failure run must reach lint"
    );
    assert!(
        !new_events
            .iter()
            .any(|event| event.first().map(String::as_str) == Some("make")
                && event.get(7).map(String::as_str) == Some("test")),
        "failure must prevent the later Act test fallback"
    );
}
