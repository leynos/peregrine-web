//! Keep the hosted Act fallback from recursively launching Act through Make.

use serde_yaml::Value;

const CI: &str = include_str!("../.github/workflows/ci.yml");

fn fallback_has_disabled_nested_act(source: &str) -> Result<bool, serde_yaml::Error> {
    // Parsing to Value rejects duplicate YAML mapping keys.
    let workflow: Value = serde_yaml::from_str(source)?;
    let Some(steps) = workflow
        .get("jobs")
        .and_then(|jobs| jobs.get("build-test"))
        .and_then(|job| job.get("steps"))
        .and_then(Value::as_sequence)
    else {
        return Ok(false);
    };
    let fallbacks: Vec<&Value> = steps
        .iter()
        .filter(|step| step.get("run").and_then(Value::as_str) == Some("make test"))
        .collect();
    let [step] = fallbacks.as_slice() else {
        return Ok(false);
    };
    let correct_step = step.get("name").and_then(Value::as_str) == Some("Test under Act");
    let act_only = step.get("if").and_then(Value::as_str) == Some("env.ACT == 'true'");
    let nested_act_disabled = step
        .get("env")
        .and_then(|env| env.get("WITH_ACT"))
        .and_then(Value::as_str)
        == Some("0");
    Ok([correct_step, act_only, nested_act_disabled]
        .into_iter()
        .all(|condition| condition))
}

#[test]
fn hosted_act_fallback_disables_recursive_act() {
    let is_disabled =
        fallback_has_disabled_nested_act(CI).expect("the committed CI workflow must parse as YAML");
    assert!(
        is_disabled,
        "the Act fallback must prevent nested Make tests from relaunching Act"
    );
}

#[test]
fn missing_or_enabled_nested_act_is_rejected() {
    let setting = "        env:\n          WITH_ACT: '0'\n";
    assert!(
        CI.contains(setting),
        "the committed fallback setting must exist"
    );
    let no_override = CI.replacen(setting, "", 1);
    let recursive_override = CI.replacen("WITH_ACT: '0'", "WITH_ACT: '1'", 1);
    for changed in [no_override, recursive_override] {
        let is_disabled = fallback_has_disabled_nested_act(&changed)
            .expect("the mutated CI workflow must parse as YAML");
        assert!(
            !is_disabled,
            "a recursive Act fallback must fail the contract"
        );
    }
}
