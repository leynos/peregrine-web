//! Executable contracts for sequential Make gates and Whitaker's flag boundary.

use std::process::Command;

/// Cold and failing paths for the dedicated Whitaker integration target.
#[path = "support/make_gate_order_driver_target.rs"]
mod driver_target;
/// Private command executors and their real Make harness.
#[path = "support/make_gate_order_probe.rs"]
mod probe;
/// NUL-framed invocation records shared by the test and its support modules.
#[path = "support/make_gate_order_records.rs"]
mod records;
/// NUL record parsing and repository configuration assertions.
#[path = "support/make_gate_order_whitaker.rs"]
mod whitaker;

use probe::{CallerEnvironment, GateProbe};
use records::{EnvironmentValue, Invocation};

/// Expected command stages for the complete ordered Make target.
const ALL_STAGES: &[&str] = &[
    "fmt",
    "markdown-formatting",
    "preflight",
    "doc",
    "clippy",
    "preflight",
    "whitaker",
    "preflight",
    "nextest",
    "doctest",
    "spelling",
];

/// Proves the parallel composite invokes each required command in order.
#[test]
fn parallel_all_runs_each_gate_in_order() {
    let probe = GateProbe::new().expect("create private gate probes");
    let output = probe
        .run("all", None)
        .expect("execute parallel Make composite");
    assert_eq!(
        probe.stages().expect("read gate order"),
        ALL_STAGES
            .iter()
            .map(|stage| (*stage).to_owned())
            .collect::<Vec<_>>(),
        "make -j all must finish each gate before starting the next"
    );
    assert_all_commands(&probe.invocations().expect("parse complete gate records"));
    assert!(
        output.status.success(),
        "all gates must pass: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Checks the real Make target and confirms every selected executor call.
fn assert_all_commands(invocations: &[Invocation]) {
    for (stage, executable, arguments) in [
        ("fmt", "probe-cargo", &["fmt", "--all", "--", "--check"][..]),
        (
            "markdown-formatting",
            "probe-mdtablefix",
            &[
                "--check",
                "--git",
                "--include-untracked",
                "--wrap",
                "--renumber",
                "--breaks",
                "--ellipsis",
                "--fences",
            ][..],
        ),
        ("preflight", "probe-check", &[][..]),
        ("doc", "probe-cargo", &["doc", "--no-deps"][..]),
        (
            "clippy",
            "probe-cargo",
            &[
                "clippy",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ][..],
        ),
        (
            "whitaker",
            "probe-whitaker",
            &["--all", "--", "--all-targets", "--all-features"][..],
        ),
        (
            "nextest-version-probe",
            "probe-cargo",
            &["nextest", "--version"][..],
        ),
        (
            "nextest",
            "probe-cargo",
            &["nextest", "run", "--all-targets", "--all-features"][..],
        ),
        (
            "doctest",
            "probe-cargo",
            &["test", "--doc", "--workspace", "--all-features"][..],
        ),
        ("spelling", "probe-spelling", &["gate"][..]),
    ] {
        GateProbe::assert_command(invocations, stage, executable, arguments);
    }
}

/// Confirms nested Make clears inherited Act mode before running the composite.
#[test]
fn inherited_with_act_does_not_reach_nested_make_all() {
    let test_binary = std::env::current_exe().expect("find current test executable");
    let output = Command::new(test_binary)
        .args([
            "--exact",
            "parallel_all_runs_each_gate_in_order",
            "--nocapture",
        ])
        .env("WITH_ACT", "1")
        .output()
        .expect("run exact gate-order test with Act enabled in its environment");
    assert!(
        output.status.success(),
        "the gate-order test must clear inherited WITH_ACT before nested Make: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Checks Make stops at each representative failing stage without hiding errors.
#[test]
fn parallel_all_stops_after_the_first_failing_gate() {
    for failure in [
        "fmt",
        "markdown-formatting",
        "preflight",
        "clippy",
        "whitaker",
        "nextest",
        "doctest",
        "spelling",
    ] {
        let expected = expected_stages_through(failure);
        let probe = GateProbe::new().expect("create private gate probes");
        let output = probe
            .run("all", Some(failure))
            .expect("execute a failing parallel composite");
        assert!(
            !output.status.success(),
            "a failing {failure} gate must fail make all"
        );
        assert_eq!(
            probe.stages().expect("read gate order"),
            expected,
            "nothing may run after {failure}"
        );
    }
}

/// Returns the composite stages through the selected failure boundary.
fn expected_stages_through(failure: &str) -> Vec<String> {
    ALL_STAGES
        .iter()
        .take_while(|stage| **stage != failure)
        .copied()
        .chain(std::iter::once(failure))
        .map(str::to_owned)
        .collect()
}

/// Proves caller contamination is cleared for Whitaker while Cargo reads repo config.
#[test]
fn parallel_lint_preserves_repository_route_for_clean_and_coverage_callers() {
    let probe = GateProbe::new().expect("create private gate probes");
    let output = probe.run("lint", None).expect("execute parallel Make lint");
    assert!(
        output.status.success(),
        "lint stages must pass: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        probe.stages().expect("read lint order"),
        ["preflight", "doc", "clippy", "preflight", "whitaker"],
        "make -j lint must finish Clippy before Whitaker"
    );
    let first_invocations = probe.invocations().expect("parse coverage caller records");
    assert_coverage_contamination(&first_invocations);
    let cold_config_invocations = records::parse_invocations(
        &probe
            .whitaker_records()
            .expect("read cold-cache Cargo config records"),
    )
    .expect("parse cold-cache Cargo config records");
    whitaker::assert_repository_route(
        &first_invocations,
        &cold_config_invocations,
        &["cold"],
        probe.executable("dylint-drivers").as_str(),
    );

    let clean_output = probe
        .run_with_environment("lint", None, CallerEnvironment::Clean)
        .expect("execute Make lint from a clean caller environment");
    assert!(
        clean_output.status.success(),
        "lint must pass with a clean caller environment: {}",
        String::from_utf8_lossy(&clean_output.stderr)
    );
    let all_invocations = probe
        .invocations()
        .expect("parse clean and coverage records");
    let all_config_invocations = records::parse_invocations(
        &probe
            .whitaker_records()
            .expect("read cold- and warm-cache Cargo config records"),
    )
    .expect("parse cold- and warm-cache Cargo config records");
    whitaker::assert_repository_route(
        &all_invocations,
        &all_config_invocations,
        &["cold", "warm"],
        probe.executable("dylint-drivers").as_str(),
    );
    assert_target_rustflags_isolated(&all_invocations);
    assert_clean_caller(&all_invocations);
    assert_secret_values_redacted(
        &probe.gate_records().expect("read NUL-framed gate records"),
        &probe
            .whitaker_records()
            .expect("read NUL-framed Whitaker records"),
    );
}

/// Checks coverage-only caller values reached Clippy before Whitaker isolation.
fn assert_coverage_contamination(invocations: &[Invocation]) {
    for invocation in invocations
        .iter()
        .filter(|invocation| matches!(invocation.stage.as_str(), "doc" | "clippy"))
    {
        assert_env_value(
            invocation,
            "CARGO_PROFILE_DEV_CODEGEN_BACKEND",
            "llvm",
            "coverage test setup must contaminate the caller child process",
        );
        assert_env_value(
            invocation,
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
            "clang",
            "coverage test setup must retain its linker override before Whitaker",
        );
        assert_env_value(
            invocation,
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS",
            "-C link-arg=-fuse-ld=lld",
            "coverage setup must contaminate the child with target-specific lld flags",
        );
        assert_env_value(
            invocation,
            "CARGO_ENCODED_RUSTFLAGS",
            "-D\u{1f}warnings\u{1f}-C\u{1f}link-arg=-fuse-ld=lld",
            "coverage flags must reach the caller child process for the contract probe",
        );
    }
}

/// Confirms target-specific lld flags stop at the repository build boundary.
fn assert_target_rustflags_isolated(invocations: &[Invocation]) {
    let checks = invocations
        .iter()
        .filter(|invocation| invocation.stage == "whitaker")
        .collect::<Vec<_>>();
    assert_eq!(
        checks.len(),
        2,
        "cold and warm Whitaker runs must both be recorded"
    );
    for invocation in checks {
        assert_eq!(
            invocation
                .environment
                .get("CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS"),
            Some(&EnvironmentValue::Unset),
            "Whitaker must not inherit target-specific lld flags after preflight"
        );
    }
}

/// Confirms an empty inherited flag value stays distinct from an unset route.
fn assert_clean_caller(invocations: &[Invocation]) {
    for stage in ["doc", "clippy"] {
        let matching = invocations
            .iter()
            .filter(|invocation| invocation.stage == stage)
            .collect::<Vec<_>>();
        assert_eq!(
            matching.len(),
            2,
            "the coverage and clean caller must each execute {stage} once"
        );
        for clean in matching.into_iter().skip(1) {
            assert_eq!(
                clean.environment.get("CARGO_ENCODED_RUSTFLAGS"),
                Some(&EnvironmentValue::Empty),
                "the clean caller's explicitly empty encoded flags must remain observable"
            );
            for variable in [
                "CARGO_PROFILE_DEV_CODEGEN_BACKEND",
                "CARGO_BUILD_TARGET",
                "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
            ] {
                assert_eq!(
                    clean.environment.get(variable),
                    Some(&EnvironmentValue::Unset),
                    "clean caller routing variable {variable} must remain unset"
                );
            }
        }
    }
}

/// Checks a recorded environment value with its stage in the failure message.
fn assert_env_value(invocation: &Invocation, name: &str, value: &str, reason: &str) {
    assert_eq!(
        invocation.environment.get(name),
        Some(&EnvironmentValue::Value(value.to_owned())),
        "{reason}; stage {}",
        invocation.stage
    );
}

/// Confirms records retain only secret presence, never the supplied values.
fn assert_secret_values_redacted(gate_records: &[u8], whitaker_records: &[u8]) {
    let record_bytes = [gate_records, whitaker_records].concat();
    let records = String::from_utf8_lossy(&record_bytes);
    assert!(
        !records.contains("gate-probe-secret-must-not-be-logged"),
        "coverage tokens must never be written to probe records"
    );
    assert!(
        !records.contains("gate-probe-github-secret-must-not-be-logged"),
        "GitHub tokens must never be written to probe records"
    );
}

/// Confirms no later lint stage runs after a Clippy failure.
#[test]
fn parallel_lint_never_starts_whitaker_after_clippy_failure() {
    let probe = GateProbe::new().expect("create private gate probes");
    let output = probe
        .run("lint", Some("clippy"))
        .expect("execute failing Make lint");
    assert!(
        !output.status.success(),
        "Clippy failure must fail make lint"
    );
    assert_eq!(
        probe.stages().expect("read lint order"),
        ["preflight", "doc", "clippy"],
        "Whitaker must not start after Clippy fails"
    );
    assert!(
        probe.whitaker_records().is_err(),
        "Whitaker must not run after Clippy fails"
    );
}
