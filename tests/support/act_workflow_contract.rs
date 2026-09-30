//! Contracts for the committed hosted and optional Act workflow routes.

use serde_yaml::Value;

/// Exact conditions, environment, and source scripts for the known CI steps.
#[path = "act_workflow_policy.rs"]
mod policy;
/// Mutations proving the workflow clauses reject changed routes.
#[path = "act_workflow_contract_tests.rs"]
mod tests;

use super::{ACT_VALIDATION_WORKFLOW, CI_WORKFLOW, WorkflowSource, validate_mapping_keys};

/// Fallible workflow parsing and fixture execution result.
type Read<T> = Result<T, Box<dyn std::error::Error>>;

/// One action revision, display name, and its required input pairs.
type ActionContract = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
);

/// Expected revisions and inputs at every external action boundary in CI.
pub(super) const ACTIONS: &[ActionContract] = &[
    (
        "actions/checkout@900f2210b1d28bbbd0bd22d17926b9e224e8f231",
        "checkout",
        &[("persist-credentials", "false")],
    ),
    (
        "leynos/shared-actions/.github/actions/setup-rust@47b337e4f230b591891656534d4ffad868131740",
        "Setup Rust",
        &[("rustflags", "")],
    ),
    (
        "leynos/shared-actions/.github/actions/install-mdtablefix@\
         4fb8eb7ad52454678a0662865d81d3cd17aa6e0e",
        "Install mdtablefix",
        &[("version", "0.6.0")],
    ),
    (
        "DavidAnson/markdownlint-cli2-action@2df9e28eb87988518ef3880c34edad45d65b1668",
        "Markdown lint",
        &[("globs", "**/*.md")],
    ),
    (
        "astral-sh/setup-uv@12d13f90bc3a5a1971bebad4beb09a4dfa962e91",
        "Setup uv",
        &[],
    ),
    (
        "actions/setup-python@a309ff8b426b58ec0e2a45f0f869d46889d02405",
        "Setup Python for audit manifest extraction",
        &[("python-version", "3.x")],
    ),
    (
        "leynos/shared-actions/.github/actions/install-whitaker@\
         6dea5677a84fec60ca51b07202570e3af12ffdb4",
        "Install Whitaker",
        &[("cranelift", "true")],
    ),
    (
        "leynos/shared-actions/.github/actions/generate-coverage@\
         dbe2e22ceaf498d85512679ccded38be9dbe7777",
        "Test and Measure Coverage",
        &[
            ("output-path", "lcov.info"),
            ("format", "lcov"),
            ("with-ratchet", "true"),
            ("publish-artefact", "false"),
        ],
    ),
];

/// Parses the exact source while rejecting duplicate YAML mapping keys.
pub(super) fn document(source: &str) -> Read<Value> {
    validate_mapping_keys(&WorkflowSource(source))?;
    Ok(serde_yaml::from_str(source)?)
}

/// Reads a nested mapping key without making absent structure pass vacuously.
fn at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(value, |node, key| {
        node.as_mapping()?.get(Value::String((*key).into()))
    })
}

/// Reads one textual YAML scalar; quoted booleans remain textual inputs.
pub(super) fn text<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    at(value, path)?.as_str()
}

/// Converts only the YAML scalar forms accepted by known action inputs.
fn action_input(value: &Value, key: &str) -> Option<String> {
    match at(value, &["with", key])? {
        Value::String(text) => Some(text.clone()),
        Value::Bool(boolean) => Some(boolean.to_string()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

/// Finds the named step in the build-test job.
pub(super) fn step<'a>(document: &'a Value, name: &str) -> Option<(usize, &'a Value)> {
    at(document, &["jobs", "build-test", "steps"])?
        .as_sequence()?
        .iter()
        .enumerate()
        .find(|(_, item)| text(item, &["name"]) == Some(name))
}

/// Checks the two event routes and the one manually requested Act route.
fn valid_triggers(ci: &Value, act: &Value) -> bool {
    let Some(ci_events) = at(ci, &["on"]).and_then(Value::as_mapping) else {
        return false;
    };
    let Some(act_events) = at(act, &["on"]).and_then(Value::as_mapping) else {
        return false;
    };
    let Some(pr) = at(ci, &["on", "pull_request"]).and_then(Value::as_mapping) else {
        return false;
    };
    let expected_types = ["opened", "synchronize", "reopened"];
    let Some(types) = at(ci, &["on", "pull_request", "types"]).and_then(Value::as_sequence) else {
        return false;
    };
    ci_events.len() == 2
        && ci_events.contains_key(Value::String("pull_request".into()))
        && ci_events.contains_key(Value::String("workflow_dispatch".into()))
        && pr.len() == 1
        && types
            .iter()
            .map(Value::as_str)
            .eq(expected_types.into_iter().map(Some))
        && at(ci, &["on", "workflow_dispatch"]).is_some_and(Value::is_null)
        && act_events.len() == 1
        && act_events.contains_key(Value::String("workflow_dispatch".into()))
        && at(act, &["on", "workflow_dispatch"]).is_some_and(Value::is_null)
}

/// Checks the known action set before a fixture could replace any action.
pub(super) fn valid_actions(ci: &Value) -> bool {
    let Some(steps) = at(ci, &["jobs", "build-test", "steps"]).and_then(Value::as_sequence) else {
        return false;
    };
    let actions = steps
        .iter()
        .filter(|item| text(item, &["uses"]).is_some())
        .collect::<Vec<_>>();
    actions.len() == ACTIONS.len()
        && actions
            .iter()
            .zip(ACTIONS)
            .all(|(item, (revision, name, inputs))| {
                text(item, &["uses"]) == Some(*revision)
                    && (name == &"checkout" || text(item, &["name"]) == Some(*name))
                    && inputs
                        .iter()
                        .all(|(key, value)| action_input(item, key).as_deref() == Some(*value))
                    && at(item, &["with"])
                        .and_then(Value::as_mapping)
                        .map_or(0, serde_yaml::Mapping::len)
                        == inputs.len()
            })
}

/// Checks the hosted suite path, step order, guards, and failure semantics.
fn valid_ci_route(ci: &Value) -> bool {
    let Some(jobs) = at(ci, &["jobs"]).and_then(Value::as_mapping) else {
        return false;
    };
    if !valid_hosted_job_policy(ci, jobs) {
        return false;
    }
    let Some(steps) = at(ci, &["jobs", "build-test", "steps"]).and_then(Value::as_sequence) else {
        return false;
    };
    if steps.len() != 19 || !steps.iter().all(policy::known_step) {
        return false;
    }
    let Some(job_env) = at(ci, &["jobs", "build-test", "env"]).and_then(Value::as_mapping) else {
        return false;
    };
    if !valid_hosted_environment(ci, job_env) {
        return false;
    }
    valid_ci_order(ci, steps) && valid_ci_commands(ci, steps)
}

/// Requires one enabled hosted job with no shell or directory defaults.
fn valid_hosted_job_policy(ci: &Value, jobs: &serde_yaml::Mapping) -> bool {
    jobs.len() == 1
        && jobs.contains_key(Value::String("build-test".into()))
        && at(ci, &["jobs", "build-test", "if"]).is_none()
        && at(ci, &["defaults"]).is_none()
        && at(ci, &["jobs", "build-test", "defaults"]).is_none()
}

/// Requires exactly the two declared hosted job environment assignments.
fn valid_hosted_environment(ci: &Value, environment: &serde_yaml::Mapping) -> bool {
    environment.len() == 2
        && text(ci, &["jobs", "build-test", "env", "CARGO_TERM_COLOR"]) == Some("always")
        && text(ci, &["jobs", "build-test", "env", "BUILD_PROFILE"]) == Some("debug")
}

/// Pins the full manual Act step count, permission, and runner version.
fn valid_manual_job_policy(act: &Value, steps: &[Value]) -> bool {
    steps.len() == 8
        && text(act, &["jobs", "act-validation", "permissions", "contents"]) == Some("read")
        && text(act, &["jobs", "act-validation", "env", "ACT_VERSION"]) == Some("v0.2.89")
}

/// Pins provisioning order, read-only permissions, and unsuppressed failures.
fn valid_ci_order(ci: &Value, steps: &[Value]) -> bool {
    let expected_order = [
        None,
        Some("Setup Rust"),
        Some("Log Rust compiler configuration"),
        Some("Install linker prerequisites"),
        Some("Install the build standard"),
        Some("Install mdtablefix"),
        Some("Format"),
        Some("Markdown lint"),
        Some("Setup uv"),
        Some("Check spelling"),
        Some("Install test runner"),
        Some("Install cargo-audit"),
        Some("Setup Python for audit manifest extraction"),
        Some("Install Whitaker"),
        Some("Audit dependencies"),
        Some("Lint"),
        Some("Log coverage linker configuration"),
        Some("Test and Measure Coverage"),
        Some("Test under Act"),
    ];
    steps
        .iter()
        .zip(expected_order)
        .all(|(item, name)| text(item, &["name"]) == name)
        && text(ci, &["jobs", "build-test", "permissions", "contents"]) == Some("read")
        && steps
            .iter()
            .all(|item| at(item, &["continue-on-error"]).is_none())
}

/// Pins the command, cache, coverage, and Act recursion boundaries.
fn valid_ci_commands(ci: &Value, steps: &[Value]) -> bool {
    text(step(ci, "Format").map_or(ci, |(_, item)| item), &["run"]) == Some("make check-fmt")
        && text(step(ci, "Lint").map_or(ci, |(_, item)| item), &["run"]).is_some_and(|script| {
            script.trim_end() == "mkdir -p \"$DYLINT_DRIVER_PATH\"\nmake lint"
        })
        && text(
            step(ci, "Lint").map_or(ci, |(_, item)| item),
            &["env", "DYLINT_DRIVER_PATH"],
        ) == Some("${{ runner.temp }}/peregrine-whitaker-driver")
        && text(
            step(ci, "Test under Act").map_or(ci, |(_, item)| item),
            &["run"],
        ) == Some("make test")
        && text(
            step(ci, "Test under Act").map_or(ci, |(_, item)| item),
            &["env", "WITH_ACT"],
        ) == Some("0")
        && text(
            step(ci, "Test under Act").map_or(ci, |(_, item)| item),
            &["if"],
        ) == Some("env.ACT == 'true'")
        && valid_coverage_route(ci)
        && !steps
            .iter()
            .filter_map(|item| text(item, &["run"]))
            .any(|script| script.contains("WITH_ACT=1"))
}

/// Requires hosted coverage to use LLVM and remain skipped in Act.
fn valid_coverage_route(ci: &Value) -> bool {
    text(
        step(ci, "Test and Measure Coverage").map_or(ci, |(_, item)| item),
        &["if"],
    ) == Some("env.ACT != 'true'")
        && text(
            step(ci, "Test and Measure Coverage").map_or(ci, |(_, item)| item),
            &["env", "CARGO_PROFILE_DEV_CODEGEN_BACKEND"],
        ) == Some("llvm")
        && text(
            step(ci, "Test and Measure Coverage").map_or(ci, |(_, item)| item),
            &["with", "publish-artefact"],
        ) == Some("false")
}

/// Checks that the manual full Act job still reaches its supported Make route.
fn valid_manual_act_route(act: &Value) -> bool {
    let Some(steps) = at(act, &["jobs", "act-validation", "steps"]).and_then(Value::as_sequence)
    else {
        return false;
    };
    if !valid_manual_job_policy(act, steps) {
        return false;
    }
    let expected_actions = [
        (
            "actions/checkout@900f2210b1d28bbbd0bd22d17926b9e224e8f231",
            "persist-credentials",
            "false",
        ),
        (
            "leynos/shared-actions/.github/actions/install-mdtablefix@\
             4fb8eb7ad52454678a0662865d81d3cd17aa6e0e",
            "version",
            "0.6.0",
        ),
        (
            "leynos/shared-actions/.github/actions/setup-rust@\
             47b337e4f230b591891656534d4ffad868131740",
            "rustflags",
            "",
        ),
    ];
    if !steps
        .iter()
        .take(3)
        .zip(expected_actions)
        .all(|(item, (revision, key, value))| {
            text(item, &["uses"]) == Some(revision)
                && action_input(item, key).as_deref() == Some(value)
                && at(item, &["with"])
                    .and_then(Value::as_mapping)
                    .is_some_and(|mapping| mapping.len() == 1)
        })
    {
        return false;
    }
    let Some(index) = steps
        .iter()
        .position(|item| text(item, &["run"]) == Some("make test WITH_ACT=1"))
    else {
        return false;
    };
    index + 1 == steps.len()
        && steps
            .iter()
            .all(|item| at(item, &["continue-on-error"]).is_none())
}

/// Combines the independent static clauses for one production workflow pair.
pub(super) fn contracts_hold(ci_source: &str, act_source: &str) -> bool {
    let (Ok(ci), Ok(act)) = (document(ci_source), document(act_source)) else {
        return false;
    };
    valid_triggers(&ci, &act)
        && valid_actions(&ci)
        && valid_ci_route(&ci)
        && valid_manual_act_route(&act)
}
