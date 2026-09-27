//! Regression tests for the Linux Act-validation workflow contract.

use std::collections::BTreeMap;

use serde::Deserialize;

const ACT_VALIDATION_WORKFLOW: &str = include_str!("../.github/workflows/act-validation.yml");
const MAKEFILE: &str = include_str!("../Makefile");
const CI_WORKFLOW: &str = include_str!("../.github/workflows/ci.yml");

#[derive(Deserialize)]
struct Workflow {
    jobs: BTreeMap<String, Job>,
}

#[derive(Deserialize)]
struct Job {
    steps: Vec<Step>,
}

#[derive(Deserialize)]
struct Step {
    run: Option<String>,
}

#[test]
/// Verifies the committed workflow installs the host linker before Act.
fn installs_linker_prerequisites_before_act_validation() {
    let workflow = parse_workflow();

    assert!(
        workflow_has_linker_prerequisites_before_act_validation(&workflow),
        "an executable sudo apt-get install command for clang and mold must run before make test \
         WITH_ACT=1"
    );
}

#[test]
/// Proves a pre-bootstrap workflow cannot satisfy the contract.
fn rejects_workflow_without_linker_bootstrap() {
    let workflow = parse_workflow_source(
        "jobs:\n  act-validation:\n    steps:\n      - run: make test WITH_ACT=1\n",
    );

    assert!(
        !workflow_has_linker_prerequisites_before_act_validation(&workflow),
        "a workflow without a preceding linker installation must fail the contract"
    );
}

#[test]
/// Rejects ambiguous YAML workflow mappings.
fn rejects_duplicate_act_validation_jobs() {
    let duplicate_jobs =
        "jobs:\n  act-validation:\n    steps: []\n  act-validation:\n    steps: []\n";
    assert!(
        validate_mapping_keys(duplicate_jobs).is_err(),
        "duplicate job keys must be rejected"
    );
}

#[test]
/// Rejects comments, malformed continuations, and later shell commands.
fn ignores_inert_package_references() {
    assert!(
        !is_linker_install_command("# sudo apt-get install clang mold"),
        "comments must not satisfy the linker prerequisite contract"
    );
    assert!(
        !is_linker_install_command("echo sudo apt-get install clang mold"),
        "echo commands must not satisfy the linker prerequisite contract"
    );
    assert!(
        !is_linker_install_command("sudo apt-get install clang # mold"),
        "shell comments must not add packages to an installation command"
    );
    assert!(
        !is_linker_install_command("sudo apt-get install other || echo clang mold"),
        "packages in a later shell command must not satisfy the contract"
    );
    assert!(
        !is_linker_install_command("sudo apt-get install clang mold\\ "),
        "whitespace after a continuation backslash must be rejected"
    );
}

#[test]
/// Preserves the Makefile switch and nested-Act forwarding contract.
fn preserves_makefile_act_execution_contract() {
    assert!(
        MAKEFILE.contains("if [ \"$(WITH_ACT)\" = \"1\" ]; then $(MAKE) act-validation; fi"),
        "WITH_ACT=1 must invoke Act after outer tests"
    );
    assert!(
        MAKEFILE.contains("--secret GITHUB_TOKEN") && MAKEFILE.contains("--env ACT=true"),
        "Act must receive the token secret and ACT marker"
    );
}

#[test]
/// Preserves the hosted-coverage guard for nested Act.
fn preserves_ci_coverage_guard_for_nested_act() {
    assert!(
        CI_WORKFLOW.contains("if: env.ACT != 'true'"),
        "nested Act must skip hosted coverage collection"
    );
}

/// Parses the committed Act workflow.
fn parse_workflow() -> Workflow { parse_workflow_source(ACT_VALIDATION_WORKFLOW) }

/// Parses validated workflow text into the test model.
fn parse_workflow_source(workflow_source: &str) -> Workflow {
    if let Err(error) = validate_mapping_keys(workflow_source) {
        panic!("the Act validation workflow must have unique YAML mapping keys: {error}");
    }
    match serde_yaml::from_str(workflow_source) {
        Ok(workflow) => workflow,
        Err(error) => panic!("the Act validation workflow must be valid YAML: {error}"),
    }
}

/// Validates YAML mappings before typed deserialisation.
fn validate_mapping_keys(workflow_source: &str) -> Result<(), serde_yaml::Error> {
    serde_yaml::from_str::<serde_yaml::Mapping>(workflow_source).map(|_| ())
}

/// Determines whether prerequisites precede the Act test step.
fn workflow_has_linker_prerequisites_before_act_validation(workflow: &Workflow) -> bool {
    let Some(act_job) = workflow.jobs.get("act-validation") else {
        return false;
    };
    let Some(act_test_step) = act_job.steps.iter().position(step_runs_act_tests) else {
        return false;
    };
    let Some(prerequisite_steps) = act_job.steps.get(..act_test_step) else {
        return false;
    };

    prerequisite_steps
        .iter()
        .filter_map(|step| step.run.as_deref())
        .any(script_installs_linker_prerequisites)
}

/// Identifies the outer Act test command.
fn step_runs_act_tests(step: &Step) -> bool {
    step.run.as_deref().is_some_and(|script| {
        script
            .lines()
            .map(normalise_command)
            .any(is_act_test_command)
    })
}

/// Identifies a valid linker-installation command in a script.
fn script_installs_linker_prerequisites(script: &str) -> bool {
    script
        .lines()
        .map(normalise_command)
        .any(is_linker_install_command)
}

/// Normalises a valid shell command line.
fn normalise_command(raw_command: &str) -> &str {
    if raw_command.ends_with([' ', '\t']) && raw_command.trim_end().ends_with('\\') {
        return "";
    }
    let trimmed_command = raw_command.trim().trim_end_matches('\\').trim_end();

    trimmed_command
        .strip_prefix("&&")
        .map_or(trimmed_command, str::trim_start)
}

/// Matches the exact Make invocation that enables Act.
fn is_act_test_command(raw_command: &str) -> bool {
    let mut words = raw_command.split_whitespace();

    matches!(
        (words.next(), words.next(), words.next(), words.next()),
        (Some("make"), Some("test"), Some("WITH_ACT=1"), None)
    )
}

/// Matches an executable apt installation of both linker packages.
fn is_linker_install_command(raw_command: &str) -> bool {
    let normalised_command = normalise_command(raw_command);
    let mut words = normalised_command.split_whitespace();
    let is_apt_install = matches!(
        (words.next(), words.next(), words.next()),
        (Some("sudo"), Some("apt-get"), Some("install"))
    );
    let arguments = words
        .take_while(|word| !word.starts_with('#') && !matches!(*word, "&&" | "||" | ";" | "|"))
        .collect::<Vec<_>>();

    is_apt_install && arguments.contains(&"clang") && arguments.contains(&"mold")
}
