//! Regression tests for the Linux Act-validation workflow contract.

/// Executes source workflow shell commands with a bounded Make fixture.
#[path = "support/act_workflow_command.rs"]
mod command;
/// Hosted and manual workflow static and command contracts.
#[path = "support/act_workflow_contract.rs"]
mod contract;
/// Typed workflow fields used by the focused Act contracts.
#[path = "support/act_workflow_model.rs"]
mod model;
/// Optional real Act compatibility test over derived fixture actions.
#[path = "support/act_workflow_smoke.rs"]
mod smoke;
use model::{Step, Workflow};

/// Committed manual Act validation workflow.
const ACT_VALIDATION_WORKFLOW: &str = include_str!("../.github/workflows/act-validation.yml");
/// Committed hosted CI workflow.
const CI_WORKFLOW: &str = include_str!("../.github/workflows/ci.yml");

/// Workflow YAML awaiting strict mapping validation and typed parsing.
#[derive(Clone, Copy)]
struct WorkflowSource<'a>(&'a str);

/// Approved full-SHA mdtablefix installer.
const MDTABLEFIX_ACTION: &str = "leynos/shared-actions/.github/actions/install-mdtablefix@\
                                 4fb8eb7ad52454678a0662865d81d3cd17aa6e0e";
/// Invalid floating installer reference used by a mutation test.
const UNPINNED_MDTABLEFIX_ACTION: &str =
    "leynos/shared-actions/.github/actions/install-mdtablefix@main";
/// Approved mdtablefix version for the Act route.
const MDTABLEFIX_VERSION: &str = "0.6.0";

/// The ACT values relevant to hosted coverage and its local test fallback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ActValue {
    /// Act explicitly enabled.
    True,
    /// Act explicitly disabled.
    False,
    /// No Act override was supplied.
    Unset,
}

/// Verifies the committed workflow installs the prerequisites before Act.
#[test]
fn installs_linker_prerequisites_before_act_validation() {
    let workflow = parse_workflow();

    assert!(
        workflow_has_linker_prerequisites_before_act_validation(&workflow),
        "an executable sudo apt-get install command for clang and mold must run before make test \
         WITH_ACT=1"
    );
    assert!(
        workflow_has_mdtablefix_before_act_validation(&workflow),
        "the pinned mdtablefix action must install version 0.6.0 before make test WITH_ACT=1"
    );
}

/// Proves a pre-bootstrap workflow cannot satisfy the contract.
#[test]
fn rejects_workflow_without_linker_bootstrap() {
    let workflow = parse_workflow_source(WorkflowSource(
        "jobs:\n  act-validation:\n    steps:\n      - run: make test WITH_ACT=1\n",
    ));

    assert!(
        !workflow_has_linker_prerequisites_before_act_validation(&workflow),
        "a workflow without a preceding linker installation must fail the contract"
    );
}

/// Rejects missing, unpinned, or misordered formatter installation.
#[test]
fn rejects_invalid_mdtablefix_provisioning() {
    let fixtures = [
        ("missing installation", None, MDTABLEFIX_VERSION, true),
        (
            "wrong action revision",
            Some(UNPINNED_MDTABLEFIX_ACTION),
            MDTABLEFIX_VERSION,
            true,
        ),
        (
            "wrong binary version",
            Some(MDTABLEFIX_ACTION),
            "0.5.0",
            true,
        ),
        (
            "installation after tests",
            Some(MDTABLEFIX_ACTION),
            MDTABLEFIX_VERSION,
            false,
        ),
    ];

    for (reason, action, version, installs_before_test) in fixtures {
        let fixture = mdtablefix_workflow_fixture(action, version, installs_before_test);
        assert!(
            !workflow_has_mdtablefix_before_act_validation(&parse_workflow_source(WorkflowSource(
                &fixture
            ))),
            "the workflow contract must reject {reason}"
        );
    }
}

/// Rejects ambiguous YAML workflow mappings.
#[test]
#[should_panic(expected = "unique YAML mapping keys")]
fn rejects_duplicate_act_validation_jobs() {
    let duplicate_jobs =
        "jobs:\n  act-validation:\n    steps: []\n  act-validation:\n    steps: []\n";
    parse_workflow_source(WorkflowSource(duplicate_jobs));
}

/// Preserves the hosted-coverage guard for nested Act.
#[test]
fn preserves_ci_coverage_guard_for_nested_act() {
    assert_eq!(
        coverage_step_runs(
            &parse_ci_workflow(WorkflowSource(CI_WORKFLOW)),
            ActValue::True
        ),
        Some(false),
        "hosted coverage must be skipped when ACT is true"
    );
    assert_eq!(
        coverage_step_runs(
            &parse_ci_workflow(WorkflowSource(CI_WORKFLOW)),
            ActValue::Unset
        ),
        Some(true),
        "hosted coverage must run when ACT is unset"
    );
    assert_eq!(
        coverage_step_runs(
            &parse_ci_workflow(WorkflowSource(CI_WORKFLOW)),
            ActValue::False
        ),
        Some(true),
        "hosted coverage must run when ACT is not true"
    );
}

/// Rejects coverage guards missing from the coverage step.
#[test]
fn rejects_missing_coverage_guard() {
    let fixture = "jobs:\n  build-test:\n    steps:\n      - name: Test and Measure Coverage\n";
    assert_eq!(
        coverage_step_runs(&parse_ci_workflow(WorkflowSource(fixture)), ActValue::True),
        None,
        "a coverage step without its own guard must fail the contract"
    );
}

/// Rejects a guard attached to a different step.
#[test]
fn rejects_coverage_guard_on_wrong_step() {
    let fixture = "jobs:\n  build-test:\n    steps:\n      - name: Other step\n        if: \
                   env.ACT != 'true'\n      - name: Test and Measure Coverage\n";
    assert_eq!(
        coverage_step_runs(&parse_ci_workflow(WorkflowSource(fixture)), ActValue::True),
        None,
        "a guard on another step must not satisfy the coverage contract"
    );
}

/// Act must execute the ordinary tests when hosted coverage is unavailable.
#[test]
fn act_runs_tests_instead_of_coverage() {
    let workflow = parse_ci_workflow(WorkflowSource(CI_WORKFLOW));
    assert_eq!(
        act_test_step_runs(&workflow, ActValue::True),
        Some(true),
        "ACT=true must run the CI test fallback"
    );
    for act_value in [ActValue::Unset, ActValue::False] {
        assert_eq!(
            act_test_step_runs(&workflow, act_value),
            Some(false),
            "hosted CI must not run the Act test fallback for ACT={act_value:?}"
        );
    }
}

/// A missing, misplaced, or weakened fallback cannot satisfy the Act contract.
#[test]
fn rejects_ci_without_an_act_only_test_fallback() {
    let fixtures =
        [
            "jobs:\n  build-test:\n    steps: []\n",
            "jobs:\n  build-test:\n    steps:\n      - name: Test under Act\n        run: echo \
             make test\n        if: env.ACT == 'true'\n",
            "jobs:\n  build-test:\n    steps:\n      - name: Test under Act\n        run: make \
             test\n        if: env.ACT != 'true'\n",
            "jobs:\n  build-test:\n    steps:\n      - name: Other step\n        if: env.ACT == \
             'true'\n      - name: Test under Act\n        run: make test\n",
        ];
    for fixture in fixtures {
        assert_eq!(
            act_test_step_runs(&parse_ci_workflow(WorkflowSource(fixture)), ActValue::True),
            None,
            "a missing or ineffective Act fallback must fail: {fixture}"
        );
    }
}

/// Parses the CI workflow after validating its YAML mapping keys.
fn parse_ci_workflow(source: WorkflowSource<'_>) -> Workflow {
    if let Err(error) = validate_mapping_keys(&source) {
        panic!("the CI workflow must have unique YAML mapping keys: {error}");
    }
    match serde_yaml::from_str(source.0) {
        Ok(workflow) => workflow,
        Err(error) => panic!("the CI workflow must be valid YAML: {error}"),
    }
}

/// Evaluates the build-test coverage guard for one ACT environment value.
fn coverage_step_runs(workflow: &Workflow, act_value: ActValue) -> Option<bool> {
    let job = workflow.jobs.get("build-test")?;
    let coverage_step = job
        .steps
        .iter()
        .find(|step| step.name.as_deref() == Some("Test and Measure Coverage"))?;
    let condition = coverage_step.condition.as_deref()?;
    (condition.trim() == "env.ACT != 'true'").then_some(act_value != ActValue::True)
}

/// Evaluates the exact Act-only test step for one ACT environment value.
fn act_test_step_runs(workflow: &Workflow, act_value: ActValue) -> Option<bool> {
    let job = workflow.jobs.get("build-test")?;
    let test_step = job
        .steps
        .iter()
        .find(|step| step.name.as_deref() == Some("Test under Act"))?;
    (test_step.condition.as_deref()? == "env.ACT == 'true'"
        && test_step.run.as_deref()? == "make test")
        .then_some(act_value == ActValue::True)
}

/// Parses the committed Act workflow.
fn parse_workflow() -> Workflow { parse_workflow_source(WorkflowSource(ACT_VALIDATION_WORKFLOW)) }

/// Parses validated workflow text into the test model.
fn parse_workflow_source(workflow_source: WorkflowSource<'_>) -> Workflow {
    if let Err(error) = validate_mapping_keys(&workflow_source) {
        panic!("the Act validation workflow must have unique YAML mapping keys: {error}");
    }
    match serde_yaml::from_str(workflow_source.0) {
        Ok(workflow) => workflow,
        Err(error) => panic!("the Act validation workflow must be valid YAML: {error}"),
    }
}

/// Validates YAML mappings before typed deserialisation.
fn validate_mapping_keys(workflow_source: &WorkflowSource<'_>) -> Result<(), serde_yaml::Error> {
    serde_yaml::from_str::<serde_yaml::Mapping>(workflow_source.0).map(|_| ())
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

/// Determines whether the exact formatter action and binary precede the Act test.
fn workflow_has_mdtablefix_before_act_validation(workflow: &Workflow) -> bool {
    let Some(act_job) = workflow.jobs.get("act-validation") else {
        return false;
    };
    let Some(act_test_step) = act_job.steps.iter().position(step_runs_act_tests) else {
        return false;
    };

    act_job.steps.get(..act_test_step).is_some_and(|steps| {
        steps.iter().any(|step| {
            step.uses.as_deref() == Some(MDTABLEFIX_ACTION)
                && step.with.get("version").and_then(serde_yaml::Value::as_str)
                    == Some(MDTABLEFIX_VERSION)
        })
    })
}

/// Builds a small workflow with one formatter action ordering mutation.
fn mdtablefix_workflow_fixture(
    action: Option<&str>,
    version: &str,
    installs_before_test: bool,
) -> String {
    let install = action.map_or_else(String::new, |action_ref| {
        format!("      - uses: {action_ref}\n        with:\n          version: '{version}'\n")
    });
    let test = "      - run: make test WITH_ACT=1\n";
    let (first_step, second_step) = if installs_before_test {
        (install.as_str(), test)
    } else {
        (test, install.as_str())
    };

    format!("jobs:\n  act-validation:\n    steps:\n{first_step}{second_step}")
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
