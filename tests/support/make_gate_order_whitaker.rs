//! NUL-framed executor observations for the Make gate-order contract.

use super::records::{EnvironmentValue, Invocation, SecretPresence};

/// Shell helpers shared by each private executable in the gate probe.
pub(super) const RECORDING_HELPERS: &str = concat!(
    "record_env_value() {\n",
    "  if [ \"$2\" != x ]; then\n",
    "    printf 'env\\0%s\\0unset\\0\\0' \"$1\"\n",
    "  elif [ -z \"$3\" ]; then\n",
    "    printf 'env\\0%s\\0empty\\0\\0' \"$1\"\n",
    "  else\n",
    "    printf 'env\\0%s\\0value\\0%s\\0' \"$1\" \"$3\"\n",
    "  fi\n",
    "}\n",
    "record_secret_presence() {\n",
    "  if [ \"$2\" = x ]; then state=present; else state=absent; fi\n",
    "  printf 'secret\\0%s\\0%s\\0' \"$1\" \"$state\"\n",
    "}\n",
    "record_invocation() {\n",
    "  log_file=$1; recorded_stage=$2; executable=$3; cache_state=$4\n",
    "  result_state=$5; result_value=$6; shift 6\n",
    "  {\n",
    "    printf 'invocation-v1\\0%s\\0%s\\0' \"$executable\" \"$PWD\"\n",
    "    record_env_value WITH_ACT \"${WITH_ACT+x}\" \"${WITH_ACT-}\"\n",
    "    record_env_value RUSTFLAGS \"${RUSTFLAGS+x}\" \"${RUSTFLAGS-}\"\n",
    "    record_env_value RUSTDOCFLAGS \"${RUSTDOCFLAGS+x}\" \"${RUSTDOCFLAGS-}\"\n",
    "    record_env_value CARGO_ENCODED_RUSTFLAGS \"${CARGO_ENCODED_RUSTFLAGS+x}\" \
     \"${CARGO_ENCODED_RUSTFLAGS-}\"\n",
    "    record_env_value CARGO_PROFILE_DEV_CODEGEN_BACKEND \
     \"${CARGO_PROFILE_DEV_CODEGEN_BACKEND+x}\" \"${CARGO_PROFILE_DEV_CODEGEN_BACKEND-}\"\n",
    "    record_env_value CARGO_BUILD_TARGET \"${CARGO_BUILD_TARGET+x}\" \
     \"${CARGO_BUILD_TARGET-}\"\n",
    "    record_env_value CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER \
     \"${CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER+x}\" \
     \"${CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER-}\"\n",
    "    record_env_value CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS \
     \"${CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS+x}\" \
     \"${CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS-}\"\n",
    "    record_env_value CFLAGS \"${CFLAGS+x}\" \"${CFLAGS-}\"\n",
    "    record_env_value LDFLAGS \"${LDFLAGS+x}\" \"${LDFLAGS-}\"\n",
    "    record_env_value DYLINT_RUSTFLAGS \"${DYLINT_RUSTFLAGS+x}\" \"${DYLINT_RUSTFLAGS-}\"\n",
    "    record_env_value DYLINT_DRIVER_PATH \"${DYLINT_DRIVER_PATH+x}\" \
     \"${DYLINT_DRIVER_PATH-}\"\n",
    "    record_env_value PATH \"${PATH+x}\" \"${PATH-}\"\n",
    "    record_env_value BUILD_TOOLS_PREFIX \"${BUILD_TOOLS_PREFIX+x}\" \
     \"${BUILD_TOOLS_PREFIX-}\"\n",
    "    record_env_value CURDIR \"${CURDIR+x}\" \"${CURDIR-}\"\n",
    "    record_env_value GATE_FAIL_AT \"${GATE_FAIL_AT+x}\" \"${GATE_FAIL_AT-}\"\n",
    "    record_env_value CARGO \"${CARGO+x}\" \"${CARGO-}\"\n",
    "    record_env_value CHECK_BUILD_TOOLS \"${CHECK_BUILD_TOOLS+x}\" \"${CHECK_BUILD_TOOLS-}\"\n",
    "    record_env_value WHITAKER \"${WHITAKER+x}\" \"${WHITAKER-}\"\n",
    "    record_env_value MDTABLEFIX \"${MDTABLEFIX+x}\" \"${MDTABLEFIX-}\"\n",
    "    record_env_value MDLINT \"${MDLINT+x}\" \"${MDLINT-}\"\n",
    "    record_env_value ACT \"${ACT+x}\" \"${ACT-}\"\n",
    "    record_env_value TYPOS_CONFIG_BUILDER \"${TYPOS_CONFIG_BUILDER+x}\" \
     \"${TYPOS_CONFIG_BUILDER-}\"\n",
    "    record_secret_presence CS_ACCESS_TOKEN \"${CS_ACCESS_TOKEN+x}\"\n",
    "    record_secret_presence GITHUB_TOKEN \"${GITHUB_TOKEN+x}\"\n",
    "    printf 'stage\\0%s\\0cache-state\\0%s\\0argc\\0%s\\0argv\\0' \\\n",
    "      \"$recorded_stage\" \"$cache_state\" \"$#\"\n",
    "    if [ \"$#\" -gt 0 ]; then printf '%s\\0' \"$@\"; fi\n",
    "    printf 'result-state\\0%s\\0result\\0%s\\0end-invocation\\0' \\\n",
    "      \"$result_state\" \"$result_value\"\n",
    "  } >> \"$log_file\"\n",
    "}\n",
);

/// Verifies Whitaker's repository-side Cargo route and driver cache evidence.
pub(super) fn assert_repository_route(
    gate_invocations: &[Invocation],
    config_invocations: &[Invocation],
    cache_states: &[&str],
    driver_path: &str,
) {
    let whitaker_invocations = gate_invocations
        .iter()
        .filter(|invocation| invocation.stage == "whitaker")
        .collect::<Vec<_>>();
    assert_eq!(
        whitaker_invocations.len(),
        cache_states.len(),
        "each expected Whitaker run must have one structured invocation record"
    );
    for (invocation, expected_cache_state) in whitaker_invocations.iter().zip(cache_states) {
        assert_whitaker_command(invocation, expected_cache_state);
        assert_driver_environment(invocation);
        assert_lint_policy(invocation);
        assert_whitaker_context(invocation, driver_path);
    }
    assert_repository_config(config_invocations, cache_states.len());
}

/// Checks each effective repository Cargo setting and its probe invocation.
fn assert_repository_config(config_invocations: &[Invocation], run_count: usize) {
    let expected = [
        (
            "profile.dev.codegen-backend",
            "profile.dev.codegen-backend = \"cranelift\"",
        ),
        ("build.rustflags", "build.rustflags = [\"-Zthreads=8\"]"),
        (
            "target.x86_64-unknown-linux-gnu.linker",
            "target.x86_64-unknown-linux-gnu.linker = \"scripts/native-clang-linker.sh\"",
        ),
        (
            "target.x86_64-unknown-linux-gnu.rustflags",
            "target.x86_64-unknown-linux-gnu.rustflags = [\"-Zthreads=8\", \"-C\", \
             \"link-arg=-fuse-ld=mold\"]",
        ),
    ];
    assert_eq!(
        config_invocations.len(),
        run_count * expected.len(),
        "each Whitaker run must inspect all four effective repository Cargo settings"
    );
    for (invocation, expected_query) in config_invocations.iter().zip(expected.iter().cycle()) {
        assert_config_query(invocation, expected_query);
        assert_config_environment(invocation);
    }
}

/// Checks one selected environment entry with an expectation-specific diagnostic.
fn assert_env(invocation: &Invocation, name: &str, expected: &EnvironmentValue, reason: &str) {
    assert_eq!(
        invocation.environment.get(name),
        Some(expected),
        "{reason}; executable {}, stage {}",
        invocation.executable,
        invocation.stage
    );
}

/// Checks the Whitaker executable, workspace, arguments, and cache state.
fn assert_whitaker_command(invocation: &Invocation, expected_cache_state: &str) {
    assert_eq!(
        invocation.executable, "probe-whitaker",
        "Make must invoke the controlled Whitaker executable"
    );
    assert_eq!(
        invocation.working_directory,
        env!("CARGO_MANIFEST_DIR"),
        "Whitaker's repository check must run from the workspace root"
    );
    assert_eq!(
        invocation.arguments,
        vec![
            "--all".to_owned(),
            "--".to_owned(),
            "--all-targets".to_owned(),
            "--all-features".to_owned(),
        ],
        "the repository checking arguments must reach Whitaker unchanged"
    );
    assert_eq!(
        invocation.cache_state, expected_cache_state,
        "the private Dylint cache must prove the expected construction state"
    );
}

/// Checks inherited build overrides are absent from driver construction.
fn assert_driver_environment(invocation: &Invocation) {
    /// Common diagnostic for caller build-routing overrides.
    const OVERRIDE_REASON: &str = "Whitaker must not inherit caller build-selection overrides";
    let expectations = [
        (
            "RUSTFLAGS",
            "Whitaker must not receive caller RUSTFLAGS for its driver build",
        ),
        ("CARGO_ENCODED_RUSTFLAGS", OVERRIDE_REASON),
        ("CARGO_PROFILE_DEV_CODEGEN_BACKEND", OVERRIDE_REASON),
        ("CARGO_BUILD_TARGET", OVERRIDE_REASON),
        (
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
            OVERRIDE_REASON,
        ),
        ("CFLAGS", OVERRIDE_REASON),
        ("LDFLAGS", OVERRIDE_REASON),
    ];
    assert_unset_environment(invocation, &expectations);
}

/// Checks warnings-as-errors are forwarded through the supported driver input.
fn assert_lint_policy(invocation: &Invocation) {
    let Some(EnvironmentValue::Value(flags)) = invocation.environment.get("DYLINT_RUSTFLAGS")
    else {
        panic!("repository lint must assign DYLINT_RUSTFLAGS");
    };
    assert_eq!(
        flags.split_whitespace().collect::<Vec<_>>(),
        ["-D", "warnings"],
        "repository lint must pass exactly the warnings policy through DYLINT_RUSTFLAGS"
    );
}

/// Checks private cache, redacted secrets, and subprocess context.
fn assert_whitaker_context(invocation: &Invocation, driver_path: &str) {
    assert_env(
        invocation,
        "DYLINT_DRIVER_PATH",
        &EnvironmentValue::Value(driver_path.to_owned()),
        "Whitaker must use the test's private driver cache",
    );
    assert_eq!(
        invocation.secrets.get("CS_ACCESS_TOKEN"),
        Some(&SecretPresence::Present),
        "the executor record must retain secret presence without its value"
    );
    assert_eq!(
        invocation.secrets.get("GITHUB_TOKEN"),
        Some(&SecretPresence::Present),
        "the executor record must retain GitHub token presence without its value"
    );
    assert_env(
        invocation,
        "GATE_FAIL_AT",
        &EnvironmentValue::Empty,
        "successful Whitaker runs must record an empty failure selector distinctly",
    );
    assert!(
        matches!(
            invocation.environment.get("PATH"),
            Some(EnvironmentValue::Value(path)) if !path.is_empty()
        ),
        "Whitaker must retain the executable search path needed by its subprocesses"
    );
}

/// Checks the executable, workspace, arguments, and output of one Cargo query.
fn assert_config_query(invocation: &Invocation, expected_query: &(&str, &str)) {
    assert_eq!(
        invocation.executable, "cargo",
        "the real Cargo executable must report the repository configuration"
    );
    assert_eq!(
        invocation.working_directory,
        env!("CARGO_MANIFEST_DIR"),
        "Cargo must discover the repository configuration from the workspace root"
    );
    assert_eq!(
        invocation.stage, "repository-config",
        "configuration reads must have their own command record"
    );
    assert_eq!(
        invocation.arguments,
        vec![
            "-Z".to_owned(),
            "unstable-options".to_owned(),
            "config".to_owned(),
            "get".to_owned(),
            expected_query.0.to_owned(),
        ],
        "the configuration query arguments must be recorded exactly"
    );
    assert_eq!(
        invocation.result.as_deref(),
        Some(expected_query.1),
        "the repository Cargo query must return its expected configuration"
    );
}

/// Checks Cargo probes cannot inherit the caller build route.
fn assert_config_environment(invocation: &Invocation) {
    /// Common diagnostic for caller build-routing overrides.
    const OVERRIDE_REASON: &str =
        "Cargo configuration probes must not inherit caller route overrides";
    let expectations = [
        (
            "RUSTFLAGS",
            "the repository configuration query must not inherit development flags",
        ),
        ("CARGO_ENCODED_RUSTFLAGS", OVERRIDE_REASON),
        ("CARGO_PROFILE_DEV_CODEGEN_BACKEND", OVERRIDE_REASON),
        ("CARGO_BUILD_TARGET", OVERRIDE_REASON),
        (
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
            OVERRIDE_REASON,
        ),
    ];
    assert_unset_environment(invocation, &expectations);
}

/// Checks each named observation is explicitly unset at its contract boundary.
fn assert_unset_environment(invocation: &Invocation, expectations: &[(&str, &str)]) {
    for (variable, reason) in expectations {
        assert_env(invocation, variable, &EnvironmentValue::Unset, reason);
    }
}

/// Regression coverage for explicit unset observations and later contamination.
#[cfg(test)]
#[path = "make_gate_order_environment_tests.rs"]
mod environment_tests;
