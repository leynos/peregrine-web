//! Contracts for exact Cargo target option detection in Make preflights.

use std::{io, process::Command};

use camino::Utf8Path;

/// Rejects cross-target selection in the native build routes.
#[test]
fn rejects_native_target_options() {
    for (target, variable, option, diagnostic) in [
        (
            "check-build-tools",
            "CARGO_FLAGS",
            "--target=aarch64-unknown-linux-gnu",
            "native Make build",
        ),
        (
            "check-build-tools",
            "TEST_FLAGS",
            "--target aarch64-unknown-linux-gnu",
            "native Make build",
        ),
        (
            "check-build-tools",
            "BUILD_JOBS",
            "--target=aarch64-unknown-linux-gnu",
            "native Make build",
        ),
        (
            "lint-whitaker",
            "CARGO_FLAGS",
            "--target aarch64-unknown-linux-gnu",
            "native Make build",
        ),
        (
            "lint-whitaker",
            "CARGO_FLAGS",
            "--target=aarch64-unknown-linux-gnu",
            "native Make build",
        ),
        (
            "check-coverage-tools",
            "TEST_FLAGS",
            "--target aarch64-unknown-linux-gnu",
            "native Make coverage",
        ),
        (
            "check-coverage-tools",
            "TEST_FLAGS",
            "--target=aarch64-unknown-linux-gnu",
            "native Make coverage",
        ),
    ] {
        let output =
            run_make(target, variable, option).expect("run the real Make preflight target");
        assert!(
            !output.status.success(),
            "{target} must reject {variable}={option}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(diagnostic),
            "{target} must explain the rejected target option: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// Keeps Cargo's build-artifact directory option available on native routes.
#[test]
fn allows_target_directories() {
    for (target, variable, option) in [
        (
            "check-build-tools",
            "TEST_FLAGS",
            "--target-dir=target/cargo-output",
        ),
        (
            "check-build-tools",
            "TEST_FLAGS",
            "--target-dir target/cargo-output",
        ),
        (
            "check-coverage-tools",
            "TEST_FLAGS",
            "--target-dir=target/cargo-output",
        ),
        (
            "check-coverage-tools",
            "TEST_FLAGS",
            "--target-dir target/cargo-output",
        ),
        (
            "lint-whitaker",
            "CARGO_FLAGS",
            "--target-dir=target/cargo-output",
        ),
        (
            "lint-whitaker",
            "CARGO_FLAGS",
            "--target-dir target/cargo-output",
        ),
    ] {
        let output =
            run_make(target, variable, option).expect("run the real Make preflight target");
        assert!(
            output.status.success(),
            "{target} must allow the Cargo artifact directory option {option}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn run_make(target: &str, variable: &str, value: &str) -> io::Result<std::process::Output> {
    Command::new("make")
        .arg(target)
        .arg("CHECK_BUILD_TOOLS=true")
        .arg("WHITAKER=true")
        .arg(format!("{variable}={value}"))
        .current_dir(Utf8Path::new(env!("CARGO_MANIFEST_DIR")))
        .output()
}
