//! Contracts for exact Cargo target option detection in Make preflights.

use std::{io, process::Command};

use camino::Utf8Path;

/// Target selection is rejected in every Make variable inspected by preflight.
#[test]
fn rejects_target_options_but_allows_target_directories() {
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

    for (target, option) in [
        ("check-build-tools", "--target-dir=target/cargo-output"),
        ("check-build-tools", "--target-dir target/cargo-output"),
        ("check-coverage-tools", "--target-dir=target/cargo-output"),
        ("check-coverage-tools", "--target-dir target/cargo-output"),
    ] {
        let output =
            run_make(target, "TEST_FLAGS", option).expect("run the real Make preflight target");
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
        .arg(format!("{variable}={value}"))
        .current_dir(Utf8Path::new(env!("CARGO_MANIFEST_DIR")))
        .output()
}
