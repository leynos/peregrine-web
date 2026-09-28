//! Contracts for pinned native build tools and LLVM coverage routing.

use std::process::Command;

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};

/// The pinned nightly must include every development and coverage component.
#[test]
fn toolchain_pins_required_developer_components() {
    let root = Dir::open_ambient_dir(
        Utf8Path::new(env!("CARGO_MANIFEST_DIR")),
        ambient_authority(),
    )
    .expect("open the repository root through a capability");
    let source = root
        .read_to_string("rust-toolchain.toml")
        .expect("read the pinned Rust toolchain");
    let document: toml::Value = toml::from_str(&source).expect("parse the pinned Rust toolchain");
    let components = document
        .get("toolchain")
        .and_then(|toolchain| toolchain.get("components"))
        .and_then(toml::Value::as_array)
        .expect("the toolchain must pin its components");
    for required in [
        "clippy",
        "llvm-tools-preview",
        "rustc-codegen-cranelift-preview",
        "rust-analyzer",
        "rustfmt",
    ] {
        assert!(
            components
                .iter()
                .any(|component| component.as_str() == Some(required)),
            "the toolchain must pin required component {required}"
        );
    }
}

#[test]
fn mold_pin_covers_only_the_supported_native_archive() {
    let root = Dir::open_ambient_dir(
        Utf8Path::new(env!("CARGO_MANIFEST_DIR")),
        ambient_authority(),
    )
    .expect("open the repository root through a capability");
    let checksums = root
        .read_to_string("tools/mold/SHA256SUMS")
        .expect("read the pinned mold archive digest");
    let entries: Vec<_> = checksums.lines().collect();
    assert_eq!(
        entries.len(),
        1,
        "the pin must cover only the supported native target"
    );
    assert!(
        entries
            .first()
            .is_some_and(|entry| entry.ends_with("mold-2.41.0-x86_64-linux.tar.gz")),
        "the mold archive pin must match the native x86_64 installer: {entries:?}"
    );
}

/// Cargo's effective profile setting changes for the measured route.
#[test]
fn coverage_overrides_the_effective_dev_backend_to_llvm() {
    for (override_backend, expected) in [(None, "cranelift"), (Some("llvm"), "llvm")] {
        let mut command = Command::new("cargo");
        command
            .args([
                "-Z",
                "unstable-options",
                "config",
                "get",
                "profile.dev.codegen-backend",
            ])
            .current_dir(env!("CARGO_MANIFEST_DIR"));
        match override_backend {
            Some(backend) => {
                command.env("CARGO_PROFILE_DEV_CODEGEN_BACKEND", backend);
            }
            None => {
                command.env_remove("CARGO_PROFILE_DEV_CODEGEN_BACKEND");
            }
        }
        let output = command
            .output()
            .expect("read Cargo's selected development backend");
        assert!(
            output.status.success(),
            "Cargo must expose its profile backend: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let selected = String::from_utf8_lossy(&output.stdout);
        assert!(
            selected.contains(&format!("= \"{expected}\"")),
            "Cargo must select {expected} for override {override_backend:?}: {selected}"
        );
    }
}

/// Make must keep LLVM selected even when a caller sets its former override.
#[test]
fn make_coverage_cannot_select_cranelift() {
    let output = Command::new("make")
        .args(["-n", "coverage"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("inspect Make coverage routing");
    assert!(output.status.success(), "Make coverage must be inspectable");
    let recipe = String::from_utf8_lossy(&output.stdout);
    assert!(
        recipe.contains("CARGO_PROFILE_DEV_CODEGEN_BACKEND=llvm"),
        "Make coverage must explicitly select LLVM: {recipe}"
    );
    let mutated = Command::new("make")
        .args(["-n", "coverage", "COVERAGE_CODEGEN_BACKEND=cranelift"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("inspect a rejected coverage backend override");
    assert!(
        mutated.status.success(),
        "Make coverage must remain inspectable"
    );
    let mutated_recipe = String::from_utf8_lossy(&mutated.stdout);
    assert!(
        mutated_recipe.contains("CARGO_PROFILE_DEV_CODEGEN_BACKEND=llvm"),
        "a command-line backend variable must not change coverage: {mutated_recipe}"
    );
    assert!(
        !mutated_recipe.contains("CARGO_PROFILE_DEV_CODEGEN_BACKEND=cranelift"),
        "coverage must never pass Cranelift to Cargo: {mutated_recipe}"
    );
}
