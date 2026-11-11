//! Runs the published example as an executable, as the user guide promises.
//!
//! `resource_policy_example` loads the example as a module, which checks its
//! assertions but not the process contract the user guide states: the command
//! exits zero and prints nothing on success. This test builds the example,
//! runs the resulting executable directly, and requires silence on both
//! standard output and standard error.
//!
//! Running the executable directly rather than through `cargo run` keeps
//! Cargo's own status and package-cache lock messages out of the observed
//! standard error. Cargo reports the executable's path in its JSON artefact
//! stream, so the test reads the path from there instead of guessing the
//! target-directory layout.
//!
//! The nested Cargo invocation passes `--offline` because the example needs
//! nothing from the network, and it shares the `nested-cargo-builds` test group
//! so it never stacks with the other tests that start a child Cargo.

use std::{error::Error, path::PathBuf, process::Command};

use serde_json::Value;

/// The example the user guide documents and `--example` selects.
const EXAMPLE: &str = "resource_policy_spike";

/// Builds the example and returns the executable Cargo produced.
fn build_example() -> Result<PathBuf, Box<dyn Error>> {
    let output = Command::new(env!("CARGO"))
        .args([
            "build",
            "--offline",
            "--example",
            EXAMPLE,
            "--message-format",
            "json",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "building the documented example must succeed, got {:?}; stderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    example_executable(&String::from_utf8_lossy(&output.stdout))
}

/// Extracts the example's executable path from Cargo's artefact messages.
fn example_executable(artifacts: &str) -> Result<PathBuf, Box<dyn Error>> {
    for line in artifacts.lines() {
        let message: Value = serde_json::from_str(line)
            .map_err(|error| format!("Cargo emitted a non-JSON build message `{line}`: {error}"))?;
        if let Some(executable) = example_artifact(&message) {
            return Ok(PathBuf::from(executable));
        }
    }
    Err(format!("Cargo's build messages must name the {EXAMPLE} executable").into())
}

/// Returns the executable path when the message reports the example artefact.
///
/// A dependency can produce several artefacts, and build-script messages carry
/// no executable at all, so the test matches on the target's name and kind.
fn example_artifact(message: &Value) -> Option<&str> {
    let is_example_artifact = message.get("reason").and_then(Value::as_str)
        == Some("compiler-artifact")
        && message.pointer("/target/name").and_then(Value::as_str) == Some(EXAMPLE)
        && message
            .pointer("/target/kind")
            .and_then(Value::as_array)
            .is_some_and(|kinds| kinds.iter().any(|kind| kind == "example"));
    if !is_example_artifact {
        return None;
    }
    message.get("executable").and_then(Value::as_str)
}

/// Checks that the example exits zero and writes nothing to either stream.
#[test]
fn the_published_example_exits_zero_and_prints_nothing() -> Result<(), Box<dyn Error>> {
    let executable = build_example()?;
    let output = Command::new(executable)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "the documented example must exit zero, got {:?}; stderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    if !output.stdout.is_empty() {
        return Err(format!(
            "the documented example must print nothing on standard output, got:\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
        .into());
    }
    if !output.stderr.is_empty() {
        return Err(format!(
            "the documented example must print nothing on standard error, got:\n{}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(())
}
