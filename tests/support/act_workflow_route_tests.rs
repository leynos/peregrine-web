//! Direct regressions for the composed hosted CI job contract.

use std::error::Error;

use serde_yaml::{Mapping, Value};

use super::super::{
    super::{ACT_VALIDATION_WORKFLOW, CI_WORKFLOW},
    contracts_hold,
    document,
    valid_ci_route,
};

/// The committed route must satisfy both its direct boundary and composition.
#[test]
fn committed_ci_route_satisfies_its_contract() {
    let ci = committed_ci().expect("parse committed CI workflow");
    assert!(
        valid_ci_route(&ci),
        "the committed hosted route must satisfy its direct policy boundary"
    );
    assert!(
        contracts_hold(CI_WORKFLOW, ACT_VALIDATION_WORKFLOW),
        "the full hosted and manual workflow composition must remain valid"
    );
}

/// Missing and wrongly typed route structures fail before nested policy checks.
#[test]
fn route_rejects_missing_and_wrongly_typed_jobs() {
    let original = committed_ci().expect("parse committed CI workflow");

    let mut missing_jobs = original.clone();
    assert!(
        remove_mapping_key(&mut missing_jobs, "jobs").is_some(),
        "the jobs removal must alter the fixture"
    );
    rejects_changed_route(&original, &missing_jobs, "missing jobs");
    rejects_route_value(&original, "jobs", Value::Null, "null jobs");
    rejects_route_value(
        &original,
        "jobs",
        Value::String("build-test".to_owned()),
        "scalar jobs",
    );
    rejects_route_value(
        &original,
        "jobs",
        Value::Sequence(Vec::new()),
        "sequence jobs",
    );

    let mut missing_job = original.clone();
    assert!(
        remove_mapping_key(
            mapping_value_mut(&mut missing_job, "jobs")
                .expect("the committed jobs field must be a mapping"),
            "build-test",
        )
        .is_some(),
        "the build-test removal must alter the fixture"
    );
    rejects_changed_route(&original, &missing_job, "missing build-test job");
    rejects_job_value(&original, Value::Null, "null build-test job");

    let mut extra_job = original.clone();
    mapping_value_mut(&mut extra_job, "jobs")
        .and_then(Value::as_mapping_mut)
        .expect("the committed jobs field must be a mapping")
        .insert(
            Value::String("unexpected-job".to_owned()),
            Value::Mapping(Mapping::default()),
        );
    rejects_changed_route(&original, &extra_job, "extra job");
}

/// Missing and wrongly typed hosted steps fail closed.
#[test]
fn route_rejects_missing_and_wrongly_typed_steps() {
    let original = committed_ci().expect("parse committed CI workflow");
    let mut missing_steps = original.clone();
    assert!(
        remove_mapping_key(
            build_test_mut(&mut missing_steps)
                .expect("the committed build-test job must be a mapping"),
            "steps",
        )
        .is_some(),
        "the steps removal must alter the fixture"
    );
    rejects_changed_route(&original, &missing_steps, "missing steps");
    rejects_job_field_value(&original, "steps", Value::Null, "null steps");
    rejects_job_field_value(
        &original,
        "steps",
        Value::String("not a sequence".to_owned()),
        "scalar steps",
    );
    rejects_job_field_value(
        &original,
        "steps",
        Value::Mapping(Mapping::default()),
        "mapping steps",
    );
}

/// Missing and wrongly typed hosted environment declarations fail closed.
#[test]
fn route_rejects_missing_and_wrongly_typed_environment() {
    let original = committed_ci().expect("parse committed CI workflow");
    let mut missing_env = original.clone();
    assert!(
        remove_mapping_key(
            build_test_mut(&mut missing_env)
                .expect("the committed build-test job must be a mapping"),
            "env",
        )
        .is_some(),
        "the env removal must alter the fixture"
    );
    rejects_changed_route(&original, &missing_env, "missing hosted environment");
    rejects_job_field_value(&original, "env", Value::Null, "null hosted environment");
    rejects_job_field_value(
        &original,
        "env",
        Value::String("not a mapping".to_owned()),
        "scalar hosted environment",
    );
    rejects_job_field_value(
        &original,
        "env",
        Value::Sequence(Vec::new()),
        "sequence hosted environment",
    );
}

/// Exact environment values and cardinality remain fixed.
#[test]
fn route_rejects_environment_mutations() {
    let original = committed_ci().expect("parse committed CI workflow");
    for (name, replacement, reason) in [
        ("CARGO_TERM_COLOR", None, "missing CARGO_TERM_COLOR"),
        ("BUILD_PROFILE", None, "missing BUILD_PROFILE"),
        (
            "EXTRA",
            Some(Value::String("unexpected".to_owned())),
            "extra environment",
        ),
        (
            "CARGO_TERM_COLOR",
            Some(Value::Bool(true)),
            "wrong CARGO_TERM_COLOR",
        ),
        (
            "BUILD_PROFILE",
            Some(Value::Number(1.into())),
            "wrong BUILD_PROFILE",
        ),
    ] {
        let mut changed = original.clone();
        let environment = hosted_environment_mut(&mut changed)
            .expect("the committed hosted environment must be a mapping");
        if let Some(replacement_value) = replacement {
            environment.insert(Value::String(name.to_owned()), replacement_value);
        } else {
            assert!(
                environment.remove(Value::String(name.to_owned())).is_some(),
                "the {name} removal must alter the fixture"
            );
        }
        rejects_changed_route(&original, &changed, reason);
    }
}

/// Step count, known commands, and ordering remain fixed.
#[test]
fn route_rejects_step_mutations() {
    let original = committed_ci().expect("parse committed CI workflow");

    for (longer, reason) in [(false, "shorter steps"), (true, "longer steps")] {
        let mut changed = original.clone();
        let steps = route_steps_mut(&mut changed).expect("the committed steps must be a sequence");
        if longer {
            steps.push(Value::String("unexpected step".to_owned()));
        } else {
            steps.pop();
        }
        rejects_changed_route(&original, &changed, reason);
    }

    let mut unknown_step = original.clone();
    replace_mapping_value(
        named_step_mut(&mut unknown_step, "Lint")
            .expect("the committed workflow must contain the Lint step"),
        "run",
        Value::String("make unknown".to_owned()),
    )
    .expect("the Lint step must accept the unknown-command mutation");
    rejects_changed_route(&original, &unknown_step, "unknown step script");

    let mut changed_order = original.clone();
    route_steps_mut(&mut changed_order)
        .expect("the committed steps must be a sequence")
        .swap(0, 1);
    rejects_changed_route(&original, &changed_order, "changed step order");

    let mut changed_command = original.clone();
    replace_mapping_value(
        named_step_mut(&mut changed_command, "Lint")
            .expect("the committed workflow must contain the Lint step"),
        "run",
        Value::String("make typecheck".to_owned()),
    )
    .expect("the Lint step must accept the changed-command mutation");
    rejects_changed_route(&original, &changed_command, "changed lint command");
}

/// Disabled jobs, defaults, and suppressed failures remain rejected.
#[test]
fn route_rejects_execution_policy_mutations() {
    let original = committed_ci().expect("parse committed CI workflow");

    let mut disabled = original.clone();
    replace_mapping_value(
        build_test_mut(&mut disabled).expect("the committed build-test job must be a mapping"),
        "if",
        Value::String("false".into()),
    )
    .expect("the committed job must accept the condition mutation");
    rejects_changed_route(&original, &disabled, "disabled job");

    let mut defaults = original.clone();
    let defaults_value: Value =
        serde_yaml::from_str("{run: {shell: sh}}").expect("parse job defaults mutation");
    replace_mapping_value(&mut defaults, "defaults", defaults_value)
        .expect("the workflow root must accept a defaults mutation");
    rejects_changed_route(&original, &defaults, "job defaults");

    let mut suppressed = original.clone();
    replace_mapping_value(
        named_step_mut(&mut suppressed, "Lint")
            .expect("the committed workflow must contain the Lint step"),
        "continue-on-error",
        Value::Bool(true),
    )
    .expect("the Lint step must accept the failure-suppression mutation");
    rejects_changed_route(&original, &suppressed, "suppressed failure");
}

fn committed_ci() -> Result<Value, Box<dyn Error>> { document(CI_WORKFLOW) }

fn mapping_value_mut<'a>(value: &'a mut Value, key: &str) -> Option<&'a mut Value> {
    value
        .as_mapping_mut()?
        .get_mut(Value::String(key.to_owned()))
}

fn build_test_mut(ci: &mut Value) -> Option<&mut Value> {
    mapping_value_mut(mapping_value_mut(ci, "jobs")?, "build-test")
}

fn hosted_environment_mut(ci: &mut Value) -> Option<&mut Mapping> {
    mapping_value_mut(build_test_mut(ci)?, "env")?.as_mapping_mut()
}

fn route_steps_mut(ci: &mut Value) -> Option<&mut Vec<Value>> {
    mapping_value_mut(build_test_mut(ci)?, "steps")?.as_sequence_mut()
}

fn named_step_mut<'a>(ci: &'a mut Value, name: &str) -> Option<&'a mut Value> {
    route_steps_mut(ci)?
        .iter_mut()
        .find(|step| step.get("name").and_then(Value::as_str) == Some(name))
}

fn remove_mapping_key(value: &mut Value, key: &str) -> Option<Value> {
    value
        .as_mapping_mut()?
        .remove(Value::String(key.to_owned()))
}

fn replace_mapping_value(value: &mut Value, key: &str, replacement: Value) -> Option<()> {
    value
        .as_mapping_mut()?
        .insert(Value::String(key.to_owned()), replacement);
    Some(())
}

fn rejects_route_value(original: &Value, key: &str, replacement: Value, reason: &str) {
    let mut changed = original.clone();
    assert!(
        replace_mapping_value(&mut changed, key, replacement).is_some(),
        "the {key} replacement must alter the fixture"
    );
    rejects_changed_route(original, &changed, reason);
}

fn rejects_job_value(original: &Value, replacement: Value, reason: &str) {
    let mut changed = original.clone();
    let jobs_field = mapping_value_mut(&mut changed, "jobs");
    assert!(
        jobs_field
            .and_then(|jobs_mapping| {
                replace_mapping_value(jobs_mapping, "build-test", replacement)
            })
            .is_some(),
        "the build-test replacement must alter the fixture"
    );
    rejects_changed_route(original, &changed, reason);
}

fn rejects_job_field_value(original: &Value, key: &str, replacement: Value, reason: &str) {
    let mut changed = original.clone();
    assert!(
        build_test_mut(&mut changed)
            .and_then(|job| replace_mapping_value(job, key, replacement))
            .is_some(),
        "the {key} replacement must alter the fixture"
    );
    rejects_changed_route(original, &changed, reason);
}

fn rejects_changed_route(original: &Value, changed: &Value, reason: &str) {
    assert_ne!(
        changed, original,
        "route mutation must change the parsed source: {reason}"
    );
    assert!(
        !valid_ci_route(changed),
        "hosted route must reject {reason}"
    );
}
