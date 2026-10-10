//! Exact known CI shell scripts, step guards, and environment contracts.

use serde_yaml::Value;

use super::{at, text};

/// Matches each command and conditional to its one approved CI step.
pub(super) fn known_step(item: &Value) -> bool {
    if at(item, &["working-directory"]).is_some() || at(item, &["shell"]).is_some() {
        return false;
    }
    let name = text(item, &["name"]);
    if !valid_step_env(item, name) {
        return false;
    }
    valid_step_condition(item, name) && valid_step_command(item, name)
}

/// Pins each guarded step's textual condition and explicit condition presence.
fn valid_step_condition(item: &Value, name: Option<&str>) -> bool {
    let condition = text(item, &["if"]);
    let expected_condition = match name {
        Some("Install linker prerequisites") => Some("runner.os == 'Linux'"),
        Some(
            "Install cargo-audit"
            | "Setup Python for audit manifest extraction"
            | "Audit dependencies",
        ) => Some("github.actor != 'dependabot[bot]'"),
        Some("Test and Measure Coverage") => Some("env.ACT != 'true'"),
        Some("Test under Act") => Some("env.ACT == 'true'"),
        _ => None,
    };
    condition == expected_condition && at(item, &["if"]).is_some() == expected_condition.is_some()
}

/// Matches known scripts, retaining the textual action fallback for nontext runs.
fn valid_step_command(item: &Value, name: Option<&str>) -> bool {
    let Some(script) = text(item, &["run"]) else {
        return text(item, &["uses"]).is_some();
    };
    let expected = match name {
        Some("Log Rust compiler configuration") => concat!(
            "rustc --version\n",
            "printf 'Base RUSTFLAGS: %s\\n' \"$RUSTFLAGS\""
        ),
        Some("Install linker prerequisites") => concat!(
            "set -euo pipefail\n",
            "export DEBIAN_FRONTEND=noninteractive\n",
            "sudo apt-get update \\\n  && sudo apt-get install --yes --no-install-recommends ",
            "clang lld mold"
        ),
        Some("Install the build standard") => "make install-build-tools",
        Some("Format") => "make check-fmt",
        Some("Check spelling") => "make spelling",
        Some("Install test runner") => "cargo binstall --no-confirm --locked cargo-nextest",
        Some("Install cargo-audit") => "cargo binstall --no-confirm cargo-audit",
        Some("Audit dependencies") => "make audit",
        Some("Lint") => "mkdir -p \"$DYLINT_DRIVER_PATH\"\nmake lint",
        Some("Log coverage linker configuration") => concat!(
            "echo \"Coverage linker: clang\"\n",
            "echo \"Coverage RUSTFLAGS: -D warnings -C link-arg=-fuse-ld=lld\"\n",
            "echo \"Coverage CFLAGS: -fuse-ld=lld\"\n",
            "echo \"Coverage LDFLAGS: -fuse-ld=lld\""
        ),
        Some("Test under Act") => "make test",
        _ => return false,
    };
    script.trim_end() == expected
}

/// Requires only the known per-step environment, including explicit LLVM coverage.
fn valid_step_env(item: &Value, name: Option<&str>) -> bool {
    let expected: &[(&str, &str)] = match name {
        Some("Install test runner" | "Install cargo-audit") => &[("RUSTFLAGS", "")],
        Some("Lint") => &[(
            "DYLINT_DRIVER_PATH",
            "${{ runner.temp }}/peregrine-whitaker-driver",
        )],
        Some("Test under Act") => &[("WITH_ACT", "0")],
        Some("Test and Measure Coverage") => &[
            ("CARGO_PROFILE_DEV_CODEGEN_BACKEND", "llvm"),
            ("CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER", "clang"),
            ("RUSTFLAGS", "-D warnings -C link-arg=-fuse-ld=lld"),
            ("CFLAGS", "-fuse-ld=lld"),
            ("LDFLAGS", "-fuse-ld=lld"),
        ],
        _ => &[],
    };
    at(item, &["env"])
        .and_then(Value::as_mapping)
        .map_or(0, serde_yaml::Mapping::len)
        == expected.len()
        && expected
            .iter()
            .all(|(key, value)| text(item, &["env", key]) == Some(*value))
}
