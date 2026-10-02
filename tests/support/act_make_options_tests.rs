//! Direct child-command configuration contracts for private Make options.

use std::{collections::BTreeMap, ffi::OsString, process::Command};

use rstest::rstest;

use super::{ExecutorFailure, MakeOptions};

/// Reads explicit child overrides without consulting or mutating the parent environment.
fn environment(command: &Command) -> BTreeMap<OsString, Option<OsString>> {
    command
        .get_envs()
        .map(|(key, value)| (key.to_owned(), value.map(ToOwned::to_owned)))
        .collect()
}

/// Absent caller flags preserve inheritance while baseline routing removals stay removed.
#[test]
fn absent_caller_values_preserve_baseline_environment() {
    let mut command = Command::new("unused");
    command
        .env("WITH_ACT", "ambient")
        .env_remove("ACT")
        .env_remove("CARGO_PROFILE_DEV_CODEGEN_BACKEND")
        .env_remove("CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER");
    MakeOptions::default().apply_caller_environment(&mut command);
    let actual = environment(&command);
    for name in [
        "WITH_ACT",
        "ACT",
        "CARGO_PROFILE_DEV_CODEGEN_BACKEND",
        "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
    ] {
        assert_eq!(
            actual.get(&OsString::from(name)),
            Some(&None),
            "{name} must remain explicitly removed"
        );
    }
    assert!(
        !actual.contains_key(&OsString::from("RUSTFLAGS")),
        "absent caller flags must leave inheritance untouched"
    );
}

/// Explicit caller values, including empty strings, must replace baseline removals.
#[rstest]
#[case("")]
#[case("selected")]
fn selected_caller_values_are_explicit(#[case] value: &'static str) {
    let mut command = Command::new("unused");
    command
        .env_remove("WITH_ACT")
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_PROFILE_DEV_CODEGEN_BACKEND")
        .env_remove("CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER")
        .env_remove("ACT");
    MakeOptions {
        caller_with_act: Some(value),
        caller_rustflags: Some(value),
        caller_backend: Some(value),
        caller_linker: Some(value),
        ..MakeOptions::default()
    }
    .apply_caller_environment(&mut command);
    let actual = environment(&command);
    for name in [
        "WITH_ACT",
        "RUSTFLAGS",
        "CARGO_PROFILE_DEV_CODEGEN_BACKEND",
        "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
    ] {
        assert_eq!(
            actual.get(&OsString::from(name)),
            Some(&Some(OsString::from(value))),
            "{name} must retain its selected value"
        );
    }
    assert_eq!(
        actual.get(&OsString::from("ACT")),
        Some(&None),
        "ACT must always be removed from the child"
    );
}

/// Every control combination must retain absent removals and enable selected flags.
#[rstest]
#[case(None)]
#[case(Some(ExecutorFailure::Act))]
#[case(Some(ExecutorFailure::Preflight))]
fn executor_failure_selection_preserves_other_controls(#[case] failure: Option<ExecutorFailure>) {
    let mut command = Command::new("unused");
    for name in [
        "FAIL_CARGO_INVOCATION",
        "FAIL_ACT",
        "FAIL_PREFLIGHT",
        "NEXTTEST_AVAILABLE",
    ] {
        command.env_remove(name);
    }
    MakeOptions {
        failure,
        ..MakeOptions::default()
    }
    .apply_executor_controls(&mut command);
    let actual = environment(&command);
    for (name, selected) in [
        ("FAIL_ACT", failure == Some(ExecutorFailure::Act)),
        (
            "FAIL_PREFLIGHT",
            failure == Some(ExecutorFailure::Preflight),
        ),
    ] {
        let expected = selected.then(|| OsString::from("1"));
        assert_eq!(
            actual.get(&OsString::from(name)),
            Some(&expected),
            "{name} must reflect only the selected failure"
        );
    }
    for name in ["FAIL_CARGO_INVOCATION", "NEXTTEST_AVAILABLE"] {
        assert_eq!(
            actual.get(&OsString::from(name)),
            Some(&None),
            "absent {name} must remain removed"
        );
    }
}

/// Cargo failure numbers and Nextest availability are independently selected.
#[test]
fn cargo_and_nextest_controls_are_applied() {
    let mut command = Command::new("unused");
    command
        .env_remove("FAIL_CARGO_INVOCATION")
        .env_remove("NEXTTEST_AVAILABLE");
    MakeOptions {
        fail_cargo_invocation: Some(2),
        nextest_available: true,
        ..MakeOptions::default()
    }
    .apply_executor_controls(&mut command);
    let actual = environment(&command);
    assert_eq!(
        actual.get(&OsString::from("FAIL_CARGO_INVOCATION")),
        Some(&Some(OsString::from("2"))),
        "Cargo failure must retain its invocation number"
    );
    assert_eq!(
        actual.get(&OsString::from("NEXTTEST_AVAILABLE")),
        Some(&Some(OsString::from("1"))),
        "Nextest probe selection must be enabled"
    );
}

/// An absent flag override preserves an explicit value already on the child command.
#[test]
fn absent_caller_flags_leave_existing_child_value_untouched() {
    let mut command = Command::new("unused");
    command.env("RUSTFLAGS", "existing-child-flags");
    MakeOptions::default().apply_caller_environment(&mut command);
    assert_eq!(
        environment(&command).get(&OsString::from("RUSTFLAGS")),
        Some(&Some(OsString::from("existing-child-flags"))),
        "absent caller flags must leave a pre-existing child value untouched"
    );
}
