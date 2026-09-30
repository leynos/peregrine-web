//! Behavioural tests for the Makefile's outer Cargo and nested Act sequence.

use std::process::Command;

use camino::{Utf8Path, Utf8PathBuf};

#[path = "support/act_make_assertions.rs"]
mod assertions;
#[path = "support/act_make_contract_tests.rs"]
mod contract_tests;
#[path = "support/act_make_harness.rs"]
mod harness;
#[path = "support/act_make_records.rs"]
mod records;

use assertions::{
    assert_act_invocation_matches,
    assert_cargo_probe,
    assert_invocations_use_repository_directory,
    assert_outer_invocations_are_ordered,
    assert_preflight_invocation,
    invocation_at,
    unwrap_test_result,
};
use harness::{ExecutorFailure, MakeHarness, MakeOptions};
use records::{EnvironmentValue, GitHubToken};

/// Token value supplied only to the controlled Act child process.
const ACT_TOKEN: &str = "controlled-act-token";
/// Runner image passed through the production Makefile option.
const RUNNER_IMAGE: &str = "controlled/ubuntu:act-test";

/// Runs the normal outer tests without invoking Act when disabled.
#[test]
fn with_act_disabled_runs_only_outer_cargo_commands() {
    let harness = unwrap_test_result(MakeHarness::new(), "create the Make harness");
    let output = unwrap_test_result(
        harness.run_make_test(MakeOptions::default()),
        "run make test",
    );
    let invocations = unwrap_test_result(harness.invocations(), "read controlled invocations");

    assert!(output.status.success(), "make test WITH_ACT=0 must succeed");
    assert_eq!(
        invocations
            .iter()
            .map(|invocation| invocation.executable.as_str())
            .collect::<Vec<_>>(),
        vec!["cargo", "check-build-tools", "cargo", "cargo"],
        "WITH_ACT=0 must probe Cargo, preflight, run tests, and run doctests"
    );
    assert_cargo_probe(invocation_at(
        &invocations,
        0,
        "Cargo probe must be recorded",
    ));
    assert_preflight_invocation(invocation_at(
        &invocations,
        1,
        "build preflight must be recorded",
    ));
    assert_outer_invocations_are_ordered(
        invocations
            .get(2..)
            .expect("both repository Cargo commands must be recorded"),
    );
    assert_invocations_use_repository_directory(&invocations);
}

/// Runs outer Cargo commands before Act and checks every forwarded setting.
#[test]
fn with_act_enabled_runs_outer_tests_then_configured_act() {
    let harness = unwrap_test_result(MakeHarness::new(), "create the Make harness");
    let output = unwrap_test_result(
        harness.run_make_test(MakeOptions {
            with_act: true,
            caller_rustflags: Some("-C debuginfo=0"),
            ..MakeOptions::default()
        }),
        "run make test",
    );
    let invocations = unwrap_test_result(harness.invocations(), "read controlled invocations");
    let expected_common_directory = unwrap_test_result(
        git_common_directory(Utf8Path::new(env!("CARGO_MANIFEST_DIR"))),
        "resolve Git common directory",
    );

    assert!(output.status.success(), "make test WITH_ACT=1 must succeed");
    assert_eq!(
        invocations.len(),
        6,
        "recursive Make must probe Cargo again before invoking Act"
    );
    assert_cargo_probe(invocation_at(
        &invocations,
        0,
        "Cargo probe must be recorded",
    ));
    assert_preflight_invocation(invocation_at(
        &invocations,
        1,
        "build preflight must be recorded",
    ));
    assert_outer_invocations_are_ordered(
        invocations
            .get(2..4)
            .expect("both repository Cargo commands must be recorded"),
    );
    assert_invocations_use_repository_directory(&invocations);
    assert_cargo_probe(invocation_at(
        &invocations,
        4,
        "recursive Make must repeat the Cargo probe",
    ));
    assert_act_invocation_matches(
        invocation_at(&invocations, 5, "Act invocation must be recorded"),
        &expected_common_directory,
    );
    assertions::assert_forwarded_environment(&invocations);
}

/// Proves caller flags distinguish unset and empty values in child records.
#[test]
fn command_records_distinguish_empty_and_unset_environment_values() {
    let harness = unwrap_test_result(MakeHarness::new(), "create the Make harness");
    let output = unwrap_test_result(
        harness.run_make_test(MakeOptions {
            caller_rustflags: Some(""),
            ..MakeOptions::default()
        }),
        "run make test",
    );
    let invocations = unwrap_test_result(harness.invocations(), "read controlled invocations");

    assert!(output.status.success(), "make test WITH_ACT=0 must succeed");
    assert_eq!(
        invocation_at(&invocations, 0, "Cargo probe must be recorded")
            .environment
            .get("CARGO_ENCODED_RUSTFLAGS"),
        Some(&EnvironmentValue::Unset),
        "an absent encoded-flags variable must remain distinguishable"
    );
    assert_eq!(
        invocation_at(&invocations, 1, "build preflight must be recorded")
            .environment
            .get("RUSTFLAGS"),
        Some(&EnvironmentValue::Empty),
        "unset and empty caller flags must remain distinguishable"
    );
    assert_eq!(
        invocation_at(&invocations, 1, "build preflight must be recorded").github_token,
        GitHubToken::Unset,
        "the preflight process must not receive a token"
    );
}

/// An inherited `WITH_ACT` value must not override the command-line setting.
#[test]
fn explicit_workflow_test_setting_prevents_nested_act_recursion() {
    let harness = unwrap_test_result(MakeHarness::new(), "create the Make harness");
    let output = unwrap_test_result(
        harness.run_make_test(MakeOptions {
            caller_with_act: Some("1"),
            ..MakeOptions::default()
        }),
        "run make test with the workflow environment",
    );
    let invocations = unwrap_test_result(harness.invocations(), "read controlled invocations");

    assert!(
        output.status.success(),
        "explicit WITH_ACT=0 must win over the caller"
    );
    assert_eq!(
        invocations
            .iter()
            .map(|invocation| invocation.executable.as_str())
            .collect::<Vec<_>>(),
        vec!["cargo", "check-build-tools", "cargo", "cargo"],
        "workflow test execution must not start Act recursively"
    );
    assert_outer_invocations_are_ordered(
        invocations
            .get(2..)
            .expect("both repository Cargo commands must be recorded"),
    );
    assert_eq!(
        invocation_at(&invocations, 1, "build preflight must be recorded")
            .environment
            .get("WITH_ACT"),
        Some(&EnvironmentValue::Value("0".to_owned())),
        "the command-line value must reach subprocesses after overriding the caller"
    );
}

/// Proves failures in either outer Cargo phase prevent Act from running.
#[test]
fn failed_outer_cargo_commands_prevent_act() {
    for failed_invocation in [1, 2] {
        let harness = unwrap_test_result(MakeHarness::new(), "create the Make harness");
        let output = unwrap_test_result(
            harness.run_make_test(MakeOptions {
                with_act: true,
                fail_cargo_invocation: Some(failed_invocation),
                ..MakeOptions::default()
            }),
            "run make test with a controlled failure",
        );
        let invocations = unwrap_test_result(harness.invocations(), "read controlled invocations");

        assert!(
            !output.status.success(),
            "a failed outer Cargo command must fail make"
        );
        assert_eq!(
            invocations.len(),
            failed_invocation + 2,
            "Act must not run after outer Cargo invocation {failed_invocation} fails"
        );
        assert!(
            invocations
                .iter()
                .all(|invocation| invocation.executable != "act"),
            "only Cargo and preflight commands may be recorded before the failure"
        );
    }
}

/// Proves an Act failure propagates through both Make recipes.
#[test]
fn failed_act_command_fails_make_test() {
    let harness = unwrap_test_result(MakeHarness::new(), "create the Make harness");
    let output = unwrap_test_result(
        harness.run_make_test(MakeOptions {
            with_act: true,
            failure: Some(ExecutorFailure::Act),
            ..MakeOptions::default()
        }),
        "run make test with a controlled Act failure",
    );
    let invocations = unwrap_test_result(harness.invocations(), "read controlled invocations");

    assert!(
        !output.status.success(),
        "a failed Act command must fail make test"
    );
    assert_eq!(
        invocations
            .last()
            .map(|invocation| invocation.executable.as_str()),
        Some("act"),
        "the controlled failure must occur in the Act invocation"
    );
}

/// Resolves Git's common directory from the tested checkout.
fn git_common_directory(repository: &Utf8Path) -> std::io::Result<Utf8PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .current_dir(repository)
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "git could not resolve its common directory: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let common_directory = String::from_utf8(output.stdout)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    Ok(Utf8PathBuf::from(common_directory.trim()))
}
