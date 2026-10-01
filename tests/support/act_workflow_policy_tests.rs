//! Direct regressions for workflow step predicates and scalar action inputs.

use rstest::rstest;
use serde_yaml::Value;

use super::super::{CI_WORKFLOW, action_inputs_match, document, policy, step, valid_actions};

#[rstest]
#[case::linker("Install linker prerequisites")]
#[case::audit_installer("Install cargo-audit")]
#[case::audit_python("Setup Python for audit manifest extraction")]
#[case::audit("Audit dependencies")]
#[case::coverage("Test and Measure Coverage")]
#[case::act("Test under Act")]
fn required_conditions_reject_missing_wrong_and_nontext_values(#[case] name: &str) {
    let ci = document(CI_WORKFLOW).expect("parse CI steps for condition fixture");
    let original = step(&ci, name).expect("find guarded production step").1;
    assert!(
        policy::known_step(original),
        "production guarded step must be accepted: {name}"
    );
    for replacement in [
        None,
        Some(Value::String("wrong".into())),
        Some(Value::Null),
        Some(Value::Bool(true)),
        Some(Value::Bool(false)),
    ] {
        let mut changed = original.clone();
        let mapping = changed
            .as_mapping_mut()
            .expect("guarded step must be a mapping");
        mapping.remove(Value::String("if".into()));
        if let Some(condition) = replacement {
            mapping.insert(Value::String("if".into()), condition);
        }
        assert_ne!(changed, *original, "condition mutation must change {name}");
        assert!(
            !policy::known_step(&changed),
            "guarded step must reject altered condition: {name}"
        );
    }
}

#[rstest]
#[case::approved("name: Format\nrun: make check-fmt", true)]
#[case::trailing_whitespace("name: Format\nrun: 'make check-fmt  '", true)]
#[case::leading_whitespace("name: Format\nrun: ' make check-fmt'", false)]
#[case::changed_script("name: Format\nrun: make test", false)]
#[case::unknown_script("name: Unknown\nrun: make check-fmt", false)]
#[case::shell("name: Format\nrun: make check-fmt\nshell: bash", false)]
#[case::null_shell("name: Format\nrun: make check-fmt\nshell: null", false)]
#[case::workdir("name: Format\nrun: make check-fmt\nworking-directory: /tmp", false)]
#[case::null_workdir("name: Format\nrun: make check-fmt\nworking-directory: null", false)]
#[case::unexpected_text_if("name: Format\nrun: make check-fmt\nif: 'true'", false)]
#[case::unexpected_bool_if("name: Format\nrun: make check-fmt\nif: false", false)]
#[case::unexpected_null_if("name: Format\nrun: make check-fmt\nif: null", false)]
#[case::absent_run_text_uses("name: Unknown\nuses: fixture", true)]
#[case::null_run_text_uses("name: Unknown\nrun: null\nuses: fixture", true)]
#[case::bool_run_text_uses("name: Unknown\nrun: true\nuses: fixture", true)]
#[case::number_run_text_uses("name: Unknown\nrun: 42\nuses: fixture", true)]
#[case::sequence_run_text_uses("name: Unknown\nrun: []\nuses: fixture", true)]
#[case::mapping_run_text_uses("name: Unknown\nrun: {}\nuses: fixture", true)]
#[case::empty_uses("name: Unknown\nuses: ''", true)]
#[case::missing_both("name: Unknown", false)]
#[case::null_uses("name: Unknown\nrun: null\nuses: null", false)]
#[case::bool_uses("name: Unknown\nrun: true\nuses: true", false)]
#[case::text_run_wins("name: Unknown\nrun: anything\nuses: fixture", false)]
#[case::empty_text_run_wins("name: Unknown\nrun: ''\nuses: fixture", false)]
#[case::unexpected_env("name: Format\nrun: make check-fmt\nenv: {RUSTFLAGS: ''}", false)]
#[case::nonmapping_env("name: Format\nrun: make check-fmt\nenv: malformed", true)]
fn step_policy_retains_scripts_guards_and_action_fallback(
    #[case] source: &str,
    #[case] expected: bool,
) {
    let value: Value = serde_yaml::from_str(source).expect("parse direct step fixture");
    assert_eq!(
        policy::known_step(&value),
        expected,
        "step policy must classify {source}"
    );
}

#[rstest]
#[case::string("with: {key: 'true'}", &[("key", "true")], true)]
#[case::boolean("with: {key: true}", &[("key", "true")], true)]
#[case::number("with: {key: 42}", &[("key", "42")], true)]
#[case::wrong_value("with: {key: false}", &[("key", "true")], false)]
#[case::null_input("with: {key: null}", &[("key", "true")], false)]
#[case::sequence_input("with: {key: []}", &[("key", "true")], false)]
#[case::mapping_input("with: {key: {}}", &[("key", "true")], false)]
#[case::extra_input("with: {key: true, other: true}", &[("key", "true")], false)]
#[case::absent_required("{}", &[("key", "true")], false)]
#[case::malformed_required("with: malformed", &[("key", "true")], false)]
#[case::absent_empty("{}", &[], true)]
#[case::null_empty("with: null", &[], true)]
#[case::scalar_empty("with: malformed", &[], true)]
#[case::sequence_empty("with: []", &[], true)]
#[case::mapping_empty("with: {}", &[], true)]
fn action_inputs_preserve_scalar_conversions_and_mapping_count(
    #[case] source: &str,
    #[case] inputs: &[(&str, &str)],
    #[case] expected: bool,
) {
    let value: Value = serde_yaml::from_str(source).expect("parse action input fixture");
    assert_eq!(
        action_inputs_match(&value, inputs),
        expected,
        "input matcher must classify {source}"
    );
}

#[test]
fn hosted_actions_reject_changed_order_and_extra_inputs() {
    let ci = document(CI_WORKFLOW).expect("parse hosted action fixture");
    assert!(
        valid_actions(&ci),
        "production action sequence must be accepted"
    );
    let mut reordered = ci.clone();
    let steps = hosted_steps_mut(&mut reordered).expect("hosted reorder fixture steps");
    steps.swap(0, 1);
    assert_ne!(
        reordered, ci,
        "action order mutation must change source model"
    );
    assert!(
        !valid_actions(&reordered),
        "hosted action sequence must preserve order"
    );
    let mut extra_input = ci.clone();
    let checkout = hosted_steps_mut(&mut extra_input)
        .expect("hosted input fixture steps")
        .first_mut()
        .expect("checkout step");
    let inputs = checkout
        .get_mut("with")
        .and_then(Value::as_mapping_mut)
        .expect("checkout input mapping");
    inputs.insert(Value::String("extra".into()), Value::String("true".into()));
    assert_ne!(
        extra_input, ci,
        "extra input mutation must change source model"
    );
    assert!(
        !valid_actions(&extra_input),
        "extra action inputs must be rejected"
    );
}

/// Selects hosted step fixtures for direct action mutations.
fn hosted_steps_mut(value: &mut Value) -> Option<&mut Vec<Value>> {
    value
        .get_mut("jobs")
        .and_then(|jobs| jobs.get_mut("build-test"))
        .and_then(|job| job.get_mut("steps"))
        .and_then(Value::as_sequence_mut)
}

#[rstest]
#[case::absent(None)]
#[case::null(Some(Value::Null))]
#[case::scalar(Some(Value::String("malformed".into())))]
#[case::sequence(Some(Value::Sequence(Vec::new())))]
fn hosted_no_input_action_preserves_nonmapping_with(#[case] replacement: Option<Value>) {
    let mut ci = document(CI_WORKFLOW).expect("parse no-input hosted fixture");
    let steps = hosted_steps_mut(&mut ci).expect("hosted no-input fixture steps");
    let setup_uv = steps
        .iter_mut()
        .find(|item| super::super::text(item, &["name"]) == Some("Setup uv"))
        .expect("Setup uv is the no-input action");
    let mapping = setup_uv.as_mapping_mut().expect("Setup uv mapping");
    mapping.remove(Value::String("with".into()));
    if let Some(value) = replacement {
        mapping.insert(Value::String("with".into()), value);
    }
    assert!(
        valid_actions(&ci),
        "absent and malformed with retain zero-input hosted semantics"
    );
}

#[test]
fn checkout_name_exception_remains_accepted() {
    let mut ci = document(CI_WORKFLOW).expect("parse checkout name fixture");
    let checkout = hosted_steps_mut(&mut ci)
        .expect("checkout fixture steps")
        .first_mut()
        .expect("checkout fixture step")
        .as_mapping_mut()
        .expect("checkout fixture mapping");
    checkout.insert(
        Value::String("name".into()),
        Value::String("custom checkout name".into()),
    );
    assert!(valid_actions(&ci), "checkout retains its name exception");
}
