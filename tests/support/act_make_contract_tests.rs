//! Focused Make command-selection and NUL-record regression tests.

use camino::Utf8Path;

use super::{
    harness::{ExecutorFailure, MakeHarness, MakeOptions},
    records::{EnvironmentValue, GitHubToken},
};

/// Checks Make selects Nextest when the controlled version probe succeeds.
#[test]
fn available_nextest_selects_the_nextest_run_branch() {
    let harness = MakeHarness::new().expect("create the Make harness");
    let output = harness
        .run_make_test(MakeOptions {
            nextest_available: true,
            ..MakeOptions::default()
        })
        .expect("run Make with the Nextest probe enabled");
    let invocations = harness.invocations().expect("read controlled invocations");

    assert!(
        output.status.success(),
        "the Nextest Make route must succeed"
    );
    let probe_arguments = invocations.first().map(|call| {
        call.arguments
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    });
    assert_eq!(
        probe_arguments,
        Some(vec!["nextest", "--version"]),
        "the first Cargo call must probe Nextest"
    );
    let test_arguments = invocations.get(2).map(|call| {
        call.arguments
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    });
    assert_eq!(
        test_arguments,
        Some(vec!["nextest", "run", "--all-targets", "--all-features"]),
        "a successful Nextest probe must select its run command"
    );
    let doctest_arguments = invocations.get(3).map(|call| {
        call.arguments
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    });
    assert_eq!(
        doctest_arguments,
        Some(vec!["test", "--doc", "--workspace", "--all-features"]),
        "doctests must still follow Nextest"
    );
}

/// Checks a failing build preflight stops before repository commands or Act.
#[test]
fn failed_preflight_stops_before_cargo_tests_and_act() {
    let harness = MakeHarness::new().expect("create the Make harness");
    let output = harness
        .run_make_test(MakeOptions {
            with_act: true,
            failure: Some(ExecutorFailure::Preflight),
            ..MakeOptions::default()
        })
        .expect("run Make with a controlled preflight failure");
    let invocations = harness.invocations().expect("read controlled invocations");

    assert!(
        !output.status.success(),
        "preflight failure must fail make test"
    );
    assert_eq!(
        invocations
            .iter()
            .map(|call| call.executable.as_str())
            .collect::<Vec<_>>(),
        vec!["cargo", "check-build-tools"],
        "preflight failure must prevent Cargo test commands and nested Act"
    );
}

/// Checks a forbidden caller backend reaches preflight and is rejected there.
#[test]
fn contaminated_backend_reaches_preflight_and_fails() {
    let harness = MakeHarness::new().expect("create the Make harness");
    let output = harness
        .run_make_test(MakeOptions {
            with_act: true,
            caller_backend: Some("llvm"),
            caller_linker: Some("clang"),
            ..MakeOptions::default()
        })
        .expect("run Make with a contaminated caller backend");
    let invocations = harness.invocations().expect("read controlled invocations");

    assert!(
        !output.status.success(),
        "an LLVM caller override must fail preflight"
    );
    assert_eq!(
        invocations
            .iter()
            .map(|call| call.executable.as_str())
            .collect::<Vec<_>>(),
        vec!["cargo", "check-build-tools"],
        "contaminated backend must not be silently removed or reach tests"
    );
    assert_eq!(
        invocations
            .get(1)
            .and_then(|call| call.environment.get("CARGO_PROFILE_DEV_CODEGEN_BACKEND")),
        Some(&EnvironmentValue::Value("llvm".to_owned())),
        "preflight must observe the inherited forbidden backend"
    );
    assert_eq!(
        invocations.get(1).and_then(|call| call
            .environment
            .get("CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER")),
        Some(&EnvironmentValue::Value("clang".to_owned())),
        "preflight must observe the inherited non-wrapper coverage linker"
    );
}

/// Verifies NUL records round-trip empty, tabbed, and multiline values exactly.
#[test]
fn nul_records_preserve_unusual_arguments_and_environment_values() {
    let harness = MakeHarness::new().expect("create the Make harness");
    let arguments = ["", "tab\tinside", "line one\nline two"];
    let caller_flags = "-C\tdebuginfo=1\n-Zthreads=8";
    let invocations = harness
        .record_probe(&arguments, caller_flags)
        .expect("record unusual child arguments and environment");
    let invocation = invocations.first().expect("record-only call must exist");
    let expected_arguments = arguments
        .iter()
        .map(|argument| (*argument).to_owned())
        .collect::<Vec<_>>();

    assert_eq!(
        invocation.arguments, expected_arguments,
        "NUL framing must preserve empty, tabbed, and multiline arguments"
    );
    assert_eq!(
        invocation.environment.get("RUSTFLAGS"),
        Some(&EnvironmentValue::Value(caller_flags.to_owned())),
        "NUL framing must preserve tabs and newlines in environment values"
    );
    assert_eq!(
        invocation.environment.get("ACT"),
        Some(&EnvironmentValue::Empty),
        "the record must distinguish an empty value from an unset variable"
    );
    assert_eq!(
        invocation.environment.get("CARGO_ENCODED_RUSTFLAGS"),
        Some(&EnvironmentValue::Unset),
        "the record must retain unset state beside empty values"
    );
    assert_eq!(
        invocation.github_token,
        GitHubToken::Unset,
        "token recording must expose only presence state"
    );
    assert_eq!(
        Utf8Path::new(&invocation.working_directory),
        Utf8Path::new(env!("CARGO_MANIFEST_DIR")),
        "record-only execution must retain the requested working directory"
    );
}
