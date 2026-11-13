//! Contracts for the locked dependency fetch that precedes `make test`.
//!
//! `make test` fetches the locked dependency graph before it starts a test
//! runner, so an unusable lockfile or an unreachable registry fails at one
//! named step rather than inside a nested build. These witnesses pin the
//! `--locked` flag, the selected target, the position of the step, and what
//! happens when the fetch fails.
//!
//! The module is a child of `make_cargo_routes`, which owns the shared
//! `CargoProbe` fixture. Keeping the fetch contracts in the same binary lets
//! the fixture keep one definition with no lint suppression: every helper is
//! reachable from at least one test in that binary.

use std::process::Output;

use super::{CargoProbe, MakeOptions};

/// The target Make names when `CARGO_BUILD_TARGET` is unset.
const DEFAULT_TARGET: &str = "host-tuple";

/// Runs `make test` against the fake Cargo with optional extra environment.
fn run_test(probe: &CargoProbe, environment: &[(&str, &str)]) -> Output {
    let mut command = probe.make_command(
        "test",
        MakeOptions {
            nextest_available: true,
            dry_run: false,
            with_act: false,
            check: "true",
        },
    );
    for (key, value) in environment {
        command.env(key, value);
    }
    command.output().expect("run make test with fake Cargo")
}

/// Reads the commands the fake Cargo executed, in order.
fn executed(probe: &CargoProbe) -> Vec<String> {
    probe
        .directory
        .read_to_string("cargo.log")
        .expect("make test must reach the fake Cargo")
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The fetch runs first, locked, and for the native target by default.
#[test]
fn test_fetches_the_locked_graph_for_the_native_target() {
    let probe = CargoProbe::new().expect("create an isolated fake Cargo executable");
    let output = run_test(&probe, &[]);
    assert!(
        output.status.success(),
        "make test must succeed with fake Cargo: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let calls = executed(&probe);
    let expected = format!("fetch --locked --target {DEFAULT_TARGET}");
    assert_eq!(
        calls.first().map(String::as_str),
        Some(expected.as_str()),
        "the first step must be a locked fetch for the native target: {calls:?}"
    );
}

/// A configured build target is what the fetch resolves dependencies for.
#[test]
fn a_configured_build_target_is_used_by_the_fetch() {
    let probe = CargoProbe::new().expect("create an isolated fake Cargo executable");
    let output = run_test(
        &probe,
        &[("CARGO_BUILD_TARGET", "x86_64-unknown-linux-gnu")],
    );
    assert!(
        output.status.success(),
        "make test must succeed with a configured target: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let calls = executed(&probe);
    assert_eq!(
        calls.first().map(String::as_str),
        Some("fetch --locked --target x86_64-unknown-linux-gnu"),
        "the fetch must use the configured target rather than the native one: {calls:?}"
    );
}

/// Both test runners start only after the fetch has completed.
#[test]
fn the_fetch_precedes_every_test_runner() {
    let probe = CargoProbe::new().expect("create an isolated fake Cargo executable");
    run_test(&probe, &[]);
    let calls = executed(&probe);
    assert!(
        calls.first().is_some_and(|call| call.starts_with("fetch ")),
        "the fetch must be the first executed step: {calls:?}"
    );
    for runner in ["nextest run", "test --doc"] {
        assert!(
            calls.iter().any(|call| call.starts_with(runner)),
            "the {runner} runner must still execute after the fetch: {calls:?}"
        );
    }
}

/// A failed fetch stops the run instead of letting a runner start.
#[test]
fn a_failed_fetch_stops_the_test_runners() {
    let probe = CargoProbe::new().expect("create an isolated fake Cargo executable");
    let output = run_test(&probe, &[("CARGO_FETCH_STATUS", "1")]);
    assert!(
        !output.status.success(),
        "a failed fetch must fail make test rather than continue"
    );
    let calls = executed(&probe);
    assert_eq!(
        calls.len(),
        1,
        "no test runner may start after a failed fetch: {calls:?}"
    );
    assert!(
        calls
            .first()
            .is_some_and(|call| call.starts_with("fetch --locked --target ")),
        "the only executed step must be the locked fetch: {calls:?}"
    );
}
