//! Runs the published example as an executable, as the user guide promises.
//!
//! `resource_policy_example` loads the example as a module, which checks its
//! assertions but not the process contract the user guide states: the command
//! exits zero and prints nothing on success. This test starts the example
//! through Cargo so the documented exit status and silence are observed where
//! a reader would see them.
//!
//! The nested Cargo invocation passes `--offline` because the example needs
//! nothing from the network, and it shares the `nested-cargo-builds` test group
//! so it never stacks with the other tests that start a child Cargo.

use std::process::Command;

/// Checks that the example exits zero and writes nothing to standard output.
///
/// Standard error is deliberately not asserted empty: Cargo reports a package
/// cache lock wait there, and this test shares the host with other builds.
#[test]
fn the_published_example_exits_zero_and_prints_nothing() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO"))
        .args([
            "run",
            "--quiet",
            "--offline",
            "--example",
            "resource_policy_spike",
        ])
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
            "the documented example must print nothing on success, got:\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
        .into());
    }
    Ok(())
}
