//! Behavioural contracts for Make's Cargo substitution, locked fetch, and
//! build preflight.

#[path = "support/make_act_contract_smoke_tests.rs"]
mod act_contract_smoke;
#[path = "support/make_fetch_contract_tests.rs"]
mod fetch_contract;
#[path = "support/make_cargo_probe.rs"]
mod probe_support;
#[path = "support/make_target_option_tests.rs"]
mod target_options;

use std::process::Command;

use camino::Utf8Path;
use probe_support::{CargoProbe, MakeOptions, dry_run, has_bare_cargo};

#[test]
fn every_make_cargo_route_uses_the_selected_executable() {
    let probe = CargoProbe::new().expect("create an isolated fake Cargo executable");
    for (target, calls) in [
        ("build", &["build"][..]),
        ("release", &["build"][..]),
        ("package", &["package"][..]),
        ("lint", &["doc", "clippy"][..]),
        ("typecheck", &["check"][..]),
        ("coverage", &["llvm-cov"][..]),
        ("check-fmt", &["fmt"][..]),
        ("fmt", &["fmt"][..]),
        ("clean", &["clean"][..]),
        ("rust-audit", &["metadata", "audit"][..]),
    ] {
        let commands =
            dry_run(&probe, target, true, false).expect("inspect Make's selected command");
        for call in calls {
            assert!(
                commands.contains(&format!("probe-cargo {call}")),
                "make {target} must use the selected Cargo for {call}: {commands}"
            );
        }
        assert!(
            !has_bare_cargo(&commands),
            "make {target} must not use bare Cargo: {commands}"
        );
    }
}

/// Production routes must replace even hostile inherited development settings.
#[test]
fn release_and_package_select_llvm_without_development_flags() {
    let probe = CargoProbe::new().expect("create an isolated fake Cargo executable");
    for (target, expected_command) in [
        ("release", "build --release"),
        ("package", "package --locked"),
    ] {
        let output = Command::new("make")
            .arg(target)
            .arg(format!("CARGO={}", probe.path.join("probe-cargo")))
            .current_dir(Utf8Path::new(env!("CARGO_MANIFEST_DIR")))
            .env("CARGO_LOG", probe.path.join("cargo.log").as_str())
            .env("PRODUCTION_LOG", probe.path.join("production.log").as_str())
            .env("RUSTFLAGS", "-Zthreads=8 -C link-arg=-fuse-ld=mold")
            .env("CARGO_ENCODED_RUSTFLAGS", "-Zthreads=8")
            .env("CARGO_PROFILE_DEV_CODEGEN_BACKEND", "cranelift")
            .env("CARGO_PROFILE_RELEASE_CODEGEN_BACKEND", "cranelift")
            .env(
                "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
                "other-linker",
            )
            .output()
            .expect("execute a production Make target with fake Cargo");
        assert!(
            output.status.success(),
            "make {target} must reach fake Cargo: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let calls = probe
            .directory
            .read_to_string("cargo.log")
            .expect("read the selected Cargo command");
        assert!(
            calls.contains(expected_command),
            "make {target} must execute {expected_command}: {calls}"
        );
        let environment = probe
            .directory
            .read_to_string("production.log")
            .expect("read the production Cargo environment");
        for required in [
            "CARGO_PROFILE_DEV_CODEGEN_BACKEND=llvm",
            "CARGO_PROFILE_RELEASE_CODEGEN_BACKEND=llvm",
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=clang",
        ] {
            assert!(
                environment.lines().any(|line| line == required),
                "make {target} must select {required}: {environment}"
            );
        }
        let rustflags = environment
            .lines()
            .find(|line| line.starts_with("RUSTFLAGS="))
            .expect("production Cargo must receive explicit RUSTFLAGS");
        assert!(
            rustflags.contains("-D warnings")
                && !rustflags.contains("-Zthreads=8")
                && !rustflags.contains("-fuse-ld=mold"),
            "make {target} must replace inherited development flags: {rustflags}"
        );
        assert!(
            !environment.contains("CARGO_ENCODED_RUSTFLAGS="),
            "make {target} must remove the encoded flags override: {environment}"
        );
        probe
            .directory
            .remove_file("cargo.log")
            .expect("reset the Cargo command log");
    }
}

#[test]
fn nextest_fallback_doctests_and_act_are_independent_routes() {
    let probe = CargoProbe::new().expect("create an isolated fake Cargo executable");
    for (nextest, selected, omitted) in [
        (
            true,
            "probe-cargo nextest run",
            "probe-cargo test --all-targets",
        ),
        (
            false,
            "probe-cargo test --all-targets",
            "probe-cargo nextest run",
        ),
    ] {
        let commands = dry_run(&probe, "test", nextest, false).expect("inspect Make's test route");
        assert!(
            commands.contains(selected),
            "selected test runner must appear: {commands}"
        );
        assert!(
            !commands.contains(omitted),
            "unselected test runner must stay out: {commands}"
        );
        assert!(
            commands.contains("probe-cargo test --doc"),
            "doctests must run separately: {commands}"
        );
        assert!(
            !has_bare_cargo(&commands),
            "test route must use selected Cargo: {commands}"
        );
    }
    let ordinary = dry_run(&probe, "test", true, false).expect("inspect ordinary tests");
    let nested = dry_run(&probe, "test", true, true).expect("inspect Act-enabled tests");
    assert!(
        !ordinary.contains("--env ACT=true"),
        "ordinary tests must not start Act: {ordinary}"
    );
    assert!(
        nested.contains("--env ACT=true"),
        "WITH_ACT=1 must reach nested Act: {nested}"
    );
}

#[test]
fn pinned_tool_preflight_precedes_development_cargo() {
    let probe = CargoProbe::new().expect("create an isolated fake Cargo executable");
    for target in ["build", "test", "lint", "typecheck", "coverage"] {
        let output = probe
            .make(
                target,
                MakeOptions {
                    nextest_available: true,
                    dry_run: false,
                    with_act: false,
                    check: probe.path.join("preflight").as_str(),
                },
            )
            .expect("execute a Make target with fake tools");
        assert!(
            output.status.success(),
            "make {target} must accept the fake tools: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let log = probe
            .directory
            .read_to_string("cargo.log")
            .expect("read executed tool order");
        let calls: Vec<_> = log.lines().collect();
        assert_eq!(
            calls.first().copied(),
            Some("preflight"),
            "make {target} must check tools before compilation: {log}"
        );
        assert!(
            calls.len() > 1,
            "make {target} must reach Cargo after preflight: {log}"
        );
        probe
            .directory
            .remove_file("cargo.log")
            .expect("reset the command log");
    }
}

/// An outdated nextest is an error: falling back to `cargo test` would lose
/// the nextest timeouts and test groups.
#[test]
fn outdated_nextest_fails_make_test_instead_of_falling_back() {
    let probe = CargoProbe::new().expect("create an isolated fake Cargo executable");
    let output = probe
        .make_command(
            "test",
            MakeOptions {
                nextest_available: true,
                dry_run: false,
                with_act: false,
                check: "true",
            },
        )
        .env("NEXTEST_FAKE_VERSION", "0.9.99")
        .output()
        .expect("run Make with an outdated fake nextest");
    assert!(
        !output.status.success(),
        "an outdated nextest must fail make test"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("0.9.99 is older than"),
        "the failure must name the installed version: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        probe.directory.read_to_string("cargo.log").is_err(),
        "no Cargo test command may run, nextest or otherwise"
    );
}

#[test]
fn failed_preflight_stops_cargo_execution() {
    let probe = CargoProbe::new().expect("create an isolated fake Cargo executable");
    let output = probe
        .make(
            "test",
            MakeOptions {
                nextest_available: true,
                dry_run: false,
                with_act: false,
                check: "false",
            },
        )
        .expect("run Make with an injected failing prerequisite");
    assert!(
        !output.status.success(),
        "a failed prerequisite must fail make test"
    );
    assert!(
        probe.directory.read_to_string("cargo.log").is_err(),
        "no Cargo compile or test command may run after the prerequisite fails"
    );
    assert!(
        has_bare_cargo("RUSTFLAGS=\"-D warnings\" cargo test"),
        "the substitution check must reject a hard-coded cargo command"
    );
}

#[test]
fn unsupported_make_target_override_fails_before_compilation() {
    let probe = CargoProbe::new().expect("create an isolated fake Cargo executable");
    let output = Command::new("make")
        .args([
            "test",
            "CHECK_BUILD_TOOLS=true",
            "TEST_FLAGS=--target=aarch64-unknown-linux-gnu",
        ])
        .arg(format!("CARGO={}", probe.path.join("probe-cargo")))
        .current_dir(Utf8Path::new(env!("CARGO_MANIFEST_DIR")))
        .env("CARGO_LOG", probe.path.join("cargo.log").as_str())
        .output()
        .expect("run Make with an unsupported cross target");
    assert!(
        !output.status.success(),
        "Make must reject the unsupported target"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("outside the supported native Make build"),
        "the target rejection must identify the native-only boundary"
    );
    assert!(
        probe.directory.read_to_string("cargo.log").is_err(),
        "unsupported target must stop compile and test commands"
    );
}
