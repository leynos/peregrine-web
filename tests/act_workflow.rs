//! Regression tests for the Linux Act-validation workflow contract.

use std::collections::BTreeMap;

use serde::Deserialize;

const ACT_VALIDATION_WORKFLOW: &str = include_str!("../.github/workflows/act-validation.yml");

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
fn installs_linker_prerequisites_before_act_validation() {
    let workflow = parse_workflow();
    let Some(act_job) = workflow.jobs.get("act-validation") else {
        panic!("the workflow must define the act-validation job");
    };
    let Some(act_test_step) = act_job.steps.iter().position(step_runs_act_tests) else {
        panic!("the act-validation job must run make test WITH_ACT=1");
    };
    let Some(prerequisite_steps) = act_job.steps.get(..act_test_step) else {
        panic!("the Act test step index must refer to the workflow step list");
    };

    assert!(
        prerequisite_steps
            .iter()
            .filter_map(|step| step.run.as_deref())
            .any(script_installs_linker_prerequisites),
        "an executable sudo apt-get install command for clang and mold must run before make test \
         WITH_ACT=1"
    );
}

#[test]
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
}

fn parse_workflow() -> Workflow {
    match serde_yaml::from_str(ACT_VALIDATION_WORKFLOW) {
        Ok(workflow) => workflow,
        Err(error) => panic!("the Act validation workflow must be valid YAML: {error}"),
    }
}

fn step_runs_act_tests(step: &Step) -> bool {
    step.run.as_deref().is_some_and(|script| {
        script
            .lines()
            .map(normalise_command)
            .any(is_act_test_command)
    })
}

fn script_installs_linker_prerequisites(script: &str) -> bool {
    script
        .lines()
        .map(normalise_command)
        .any(is_linker_install_command)
}

fn normalise_command(raw_command: &str) -> &str {
    let trimmed_command = raw_command.trim().trim_end_matches('\\').trim_end();

    trimmed_command
        .strip_prefix("&&")
        .map_or(trimmed_command, str::trim_start)
}

fn is_act_test_command(raw_command: &str) -> bool {
    let mut words = raw_command.split_whitespace();

    matches!(
        (words.next(), words.next(), words.next(), words.next()),
        (Some("make"), Some("test"), Some("WITH_ACT=1"), None)
    )
}

fn is_linker_install_command(raw_command: &str) -> bool {
    let normalised_command = normalise_command(raw_command);
    let mut words = normalised_command.split_whitespace();
    let is_apt_install = matches!(
        (words.next(), words.next(), words.next()),
        (Some("sudo"), Some("apt-get"), Some("install"))
    );
    let arguments = words
        .take_while(|word| !word.starts_with('#'))
        .collect::<Vec<_>>();

    is_apt_install && arguments.contains(&"clang") && arguments.contains(&"mold")
}
