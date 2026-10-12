//! Contract for the approved binary-only Whitaker installer in pull-request CI.

use serde_yaml::Value;

const CI: &str = include_str!("../.github/workflows/ci.yml");
// This revision contains shared-actions #522. The approved pin is a policy
// boundary: earlier revisions can build the rolling suite from source.
const INSTALL_ACTION: &str = concat!(
    "leynos/shared-actions/.github/actions/install-whitaker@",
    "6dea5677a84fec60ca51b07202570e3af12ffdb4",
);

fn check_install_step(step: &Value) -> Result<(), String> {
    if step.get("uses").and_then(Value::as_str) != Some(INSTALL_ACTION) {
        return Err("Whitaker action does not use the approved #522 pin".into());
    }
    if step.get("if").is_some() || step.get("continue-on-error").is_some() {
        return Err("Whitaker installation can be skipped or softened".into());
    }
    let inputs = step
        .get("with")
        .and_then(Value::as_mapping)
        .ok_or("Whitaker action inputs are missing")?;
    if inputs.get("cranelift").and_then(Value::as_str) != Some("true") {
        return Err("Whitaker must install its Cranelift component".into());
    }
    for forbidden in [
        "suite-version",
        "allow-suite-pin",
        "installer-version",
        "ci-mode",
        "source-fallback",
        "allow-source-fallback",
    ] {
        if inputs.contains_key(forbidden) {
            return Err(format!(
                "Whitaker input {forbidden} overrides shared policy"
            ));
        }
    }
    Ok(())
}

fn check_installer(source: &str) -> Result<(), String> {
    // serde_yaml rejects duplicate mapping keys rather than choosing one.
    let workflow: Value = serde_yaml::from_str(source).map_err(|error| error.to_string())?;
    let job = workflow
        .get("jobs")
        .and_then(|jobs| jobs.get("build-test"))
        .ok_or("build-test job is missing")?;
    check_job_versions(job)?;
    let steps = job
        .get("steps")
        .and_then(Value::as_sequence)
        .ok_or("build-test steps are missing")?;
    check_steps(steps)
}

fn check_job_versions(job: &Value) -> Result<(), String> {
    if job.get("env").is_some_and(|env| {
        env.get("WHITAKER_INSTALLER_VERSION").is_some()
            || env.get("WHITAKER_SUITE_VERSION").is_some()
    }) {
        return Err("job overrides the shared Whitaker versions".into());
    }
    Ok(())
}

fn check_steps(steps: &[Value]) -> Result<(), String> {
    let mut installer = None;
    let mut lint = None;
    for (index, step) in steps.iter().enumerate() {
        record_installer(step, index, &mut installer)?;
        record_lint(step, index, &mut lint)?;
        check_no_bypass(step)?;
    }
    match (installer, lint) {
        (Some(install), Some(gate)) if install < gate => Ok(()),
        _ => Err("Whitaker action must precede the binding lint gate".into()),
    }
}

fn record_installer(
    step: &Value,
    index: usize,
    installer: &mut Option<usize>,
) -> Result<(), String> {
    let uses = step.get("uses").and_then(Value::as_str);
    if uses.is_some_and(|action| action.contains("install-whitaker")) {
        check_install_step(step)?;
        if installer.replace(index).is_some() {
            return Err("more than one Whitaker installer".into());
        }
    }
    Ok(())
}

fn record_lint(step: &Value, index: usize, lint: &mut Option<usize>) -> Result<(), String> {
    let run = step.get("run").and_then(Value::as_str).unwrap_or("");
    if matches!(
        run.trim(),
        "make lint" | "mkdir -p \"$DYLINT_DRIVER_PATH\"\nmake lint"
    ) {
        if lint.replace(index).is_some() {
            return Err("more than one CI lint gate".into());
        }
        if step.get("if").is_some() || step.get("continue-on-error").is_some() {
            return Err("CI lint gate can be skipped or softened".into());
        }
    }
    Ok(())
}

fn check_no_bypass(step: &Value) -> Result<(), String> {
    let run = step.get("run").and_then(Value::as_str).unwrap_or("");
    let cargo_installer = ["cargo binstall", "whitaker"]
        .into_iter()
        .all(|part| run.contains(part));
    if run.contains("whitaker-installer") || cargo_installer {
        return Err("direct Whitaker installation bypasses the shared action".into());
    }
    let uses = step.get("uses").and_then(Value::as_str);
    if uses.is_some_and(|action| action.contains("actions/cache"))
        && step
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|name| name.contains("Whitaker"))
    {
        return Err("a separate Whitaker cache bypasses action ownership".into());
    }
    Ok(())
}

#[test]
fn ci_installs_approved_whitaker_before_lint() {
    assert_eq!(
        check_installer(CI),
        Ok(()),
        "CI Whitaker provisioning contract"
    );
}

#[test]
fn weakened_whitaker_provisioning_is_rejected() {
    let cases = [
        (
            INSTALL_ACTION,
            concat!(
                "leynos/shared-actions/.github/actions/install-whitaker@",
                "8193dca5c1d1411e14108ec8654d4f43e7d06a30",
            ),
        ),
        ("cranelift: 'true'", "cranelift: 'false'"),
        (
            "          cranelift: 'true'",
            "          cranelift: 'true'\n          suite-version: 'rolling'",
        ),
        (
            "          cranelift: 'true'",
            "          cranelift: 'true'\n          installer-version: '0.2.6'",
        ),
        (
            "          cranelift: 'true'",
            "          cranelift: 'true'\n          source-fallback: 'true'",
        ),
        (
            "      BUILD_PROFILE: debug",
            "      BUILD_PROFILE: debug\n      WHITAKER_INSTALLER_VERSION: '0.2.6'",
        ),
        (
            "        with:\n          cranelift: 'true'",
            "        if: false\n        with:\n          cranelift: 'true'",
        ),
        (
            "        with:\n          cranelift: 'true'",
            "        continue-on-error: true\n        with:\n          cranelift: 'true'",
        ),
    ];
    for (before, after) in cases {
        assert!(CI.contains(before), "fixture anchor is missing: {before}");
        let fixture = CI.replacen(before, after, 1);
        assert!(
            check_installer(&fixture).is_err(),
            "weakened Whitaker route was accepted: {after}"
        );
    }
}

#[test]
fn missing_or_late_whitaker_installer_is_rejected() {
    let install_step = format!(
        concat!(
            "      - name: Install Whitaker\n",
            "        uses: {}\n",
            "        with:\n",
            "          cranelift: 'true'\n",
        ),
        INSTALL_ACTION
    );
    let removed = CI.replacen(&install_step, "", 1);
    assert_ne!(removed, CI, "the installer step fixture must exist");
    assert!(
        check_installer(&removed).is_err(),
        "a missing installer must fail"
    );
    let misplaced = removed.replacen(
        "          make lint\n",
        &format!("          make lint\n{install_step}"),
        1,
    );
    assert_ne!(misplaced, removed, "the lint step fixture must exist");
    assert!(
        check_installer(&misplaced).is_err(),
        "installation after lint must fail"
    );
}

#[test]
fn duplicate_workflow_keys_are_rejected() {
    let duplicate = CI.replacen(
        "      - name: Lint\n",
        "      - name: Lint\n        run: echo bypass\n",
        1,
    );
    assert!(
        check_installer(&duplicate).is_err(),
        "duplicate YAML keys must fail closed"
    );
}
