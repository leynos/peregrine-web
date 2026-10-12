//! Checks that the repository's private-item documentation lint remains enforced.

use std::{path::PathBuf, process::Command};

/// Runs Clippy against a deliberately undocumented private item.
#[test]
fn missing_docs_in_private_items_remains_denied() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_manifest_path = repository.join("tests/fixtures/private_docs_lint/Cargo.toml");
    let root_manifest: toml::Value =
        toml::from_str(include_str!("../Cargo.toml")).expect("parse the repository manifest");
    let fixture_manifest: toml::Value =
        toml::from_str(include_str!("fixtures/private_docs_lint/Cargo.toml"))
            .expect("parse the private-docs fixture manifest");
    let root_level = root_manifest
        .get("lints")
        .and_then(|lints| lints.get("clippy"))
        .and_then(|clippy| clippy.get("missing_docs_in_private_items"))
        .and_then(toml::Value::as_str);
    let fixture_level = fixture_manifest
        .get("lints")
        .and_then(|lints| lints.get("clippy"))
        .and_then(|clippy| clippy.get("missing_docs_in_private_items"))
        .and_then(toml::Value::as_str);

    assert_eq!(
        root_level,
        Some("deny"),
        "the repository must keep private-item documentation failures denied"
    );
    assert_eq!(
        fixture_level, root_level,
        "the compile-fail fixture must use the repository's Clippy lint level"
    );

    let output = Command::new("cargo")
        .args(["clippy", "--manifest-path"])
        .arg(fixture_manifest_path)
        .args(["--locked", "--offline"])
        .current_dir(&repository)
        .output()
        .expect("run Clippy against the private-docs fixture");
    let diagnostics = String::from_utf8_lossy(&output.stderr);

    assert!(
        !output.status.success(),
        "Clippy accepted an undocumented private item; stderr: {diagnostics}"
    );
    assert!(
        diagnostics.contains("missing documentation for a function"),
        "Clippy must fail for the private-item documentation diagnostic; stderr: {diagnostics}"
    );
}
