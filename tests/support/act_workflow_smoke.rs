//! Optional real-Act smoke over a fixture rewrite of the committed CI workflow.

use std::{
    io,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

use camino::Utf8PathBuf;
use cap_std::{
    ambient_authority,
    fs::{Permissions, PermissionsExt},
    fs_utf8::Dir,
};
use serde_yaml::Value;

use super::{
    ACT_VALIDATION_WORKFLOW,
    CI_WORKFLOW,
    WorkflowSource,
    contract,
    validate_mapping_keys,
};

/// Controlled composite action interfaces.
#[path = "act_workflow_smoke_actions.rs"]
mod actions;
/// Semantic assertions over the recorded real Act trace.
#[path = "act_workflow_smoke_assertions.rs"]
mod assertions;

/// Fallible scratch workflow construction and real Act invocation result.
type Read<T> = Result<T, Box<dyn std::error::Error>>;
/// Distinguishes private Act scratch checkouts in a parallel test process.
static NEXT_SMOKE_ID: AtomicUsize = AtomicUsize::new(0);

/// Owns a scratch checkout containing the derived workflow and controlled tools.
struct ActSmoke {
    /// Capability for the scratch parent, used for final cleanup.
    parent: Dir,
    /// Private checkout name relative to its scratch parent.
    name: String,
    /// Absolute path from which Act discovers the generated workflow.
    path: Utf8PathBuf,
}

impl ActSmoke {
    /// Constructs fixture actions only after validating all original action interfaces.
    fn new() -> Read<Self> {
        if !contract::contracts_hold(CI_WORKFLOW, ACT_VALIDATION_WORKFLOW) {
            return Err("production workflow has an unmapped command, action, or guard".into());
        }
        validate_mapping_keys(&WorkflowSource(CI_WORKFLOW))?;
        let mut workflow: Value = serde_yaml::from_str(CI_WORKFLOW)?;
        if !contract::valid_actions(&workflow) {
            return Err("an action pin or input lacks its approved fixture mapping".into());
        }
        let original = workflow.clone();
        let root = Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .map_err(|path| io::Error::other(format!("non-UTF-8 temp path: {}", path.display())))?;
        let parent = Dir::open_ambient_dir(&root, ambient_authority())?;
        let name = format!(
            "peregrine-act-smoke-{}-{}",
            std::process::id(),
            NEXT_SMOKE_ID.fetch_add(1, Ordering::Relaxed)
        );
        parent.create_dir(&name)?;
        let path = root.join(&name);
        let fixture = Self { parent, name, path };
        let directory = fixture.parent.open_dir(&fixture.name)?;
        directory.create_dir_all(".github/workflows")?;
        directory.create_dir_all(".github/actions")?;
        directory.create_dir_all("fixture-bin")?;
        Self::replace_actions(&mut workflow, &directory)?;
        Self::verify_rewrite(&workflow, &original)?;
        fixture.write_tools(&directory, &workflow)?;
        Ok(fixture)
    }

    /// Replaces only approved action references with local interface fixtures.
    fn replace_actions(workflow: &mut Value, directory: &Dir) -> Read<()> {
        let steps = workflow
            .get_mut("jobs")
            .and_then(|jobs| jobs.get_mut("build-test"))
            .and_then(|job| job.get_mut("steps"))
            .and_then(Value::as_sequence_mut)
            .ok_or("the hosted build-test steps are missing")?;
        let mut action_index = 0;
        for step in steps {
            let Some(uses) = step.get("uses").and_then(Value::as_str) else {
                continue;
            };
            let (expected, _, inputs) = contract::ACTIONS
                .get(action_index)
                .ok_or("unmapped action")?;
            if uses != *expected {
                return Err(format!("unmapped action: {uses}").into());
            }
            let action_path = format!(".github/actions/fixture-{action_index}");
            directory.create_dir_all(&action_path)?;
            directory.write(
                format!("{action_path}/action.yml"),
                actions::fixture_action(action_index, inputs)?,
            )?;
            let mapping = step
                .as_mapping_mut()
                .ok_or("action step is not a mapping")?;
            mapping.insert(
                Value::String("uses".into()),
                Value::String(format!("./{action_path}")),
            );
            action_index += 1;
        }
        if action_index != contract::ACTIONS.len() {
            return Err("one or more fixture actions were not reached".into());
        }
        Ok(())
    }

    /// Restores action references to prove every other workflow field is unchanged.
    fn verify_rewrite(workflow: &Value, original: &Value) -> Read<()> {
        let mut restored = workflow.clone();
        let restored_steps = restored
            .get_mut("jobs")
            .and_then(|jobs| jobs.get_mut("build-test"))
            .and_then(|job| job.get_mut("steps"))
            .and_then(Value::as_sequence_mut)
            .ok_or("derived workflow lost its build-test steps")?;
        let mut restored_index = 0;
        for step in restored_steps {
            if step.get("uses").is_none() {
                continue;
            }
            let revision = contract::ACTIONS
                .get(restored_index)
                .ok_or("unmapped restored action")?
                .0;
            step.as_mapping_mut()
                .ok_or("restored action is not a mapping")?
                .insert(Value::String("uses".into()), Value::String(revision.into()));
            restored_index += 1;
        }
        if &restored != original {
            return Err("fixture rewrite changed a step field beyond uses".into());
        }
        Ok(())
    }

    /// Writes the derived YAML and rejecting shell tools into a scratch checkout.
    fn write_tools(&self, directory: &Dir, workflow: &Value) -> Read<()> {
        directory.write(".github/workflows/ci.yml", serde_yaml::to_string(workflow)?)?;
        for executable in ["make", "cargo", "rustc", "sudo"] {
            directory.write(format!("fixture-bin/{executable}"), STUB)?;
            directory.set_permissions(
                format!("fixture-bin/{executable}"),
                Permissions::from_mode(0o700),
            )?;
        }
        let status = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&self.path)
            .status()?;
        if !status.success() {
            return Err("could not initialise the scratch Act checkout".into());
        }
        Ok(())
    }

    /// Runs the derived job with real Act and optional controlled Make failure.
    fn run(&self, failed_target: Option<&str>) -> io::Result<std::process::Output> {
        let mut command = Command::new("act");
        command
            .args([
                "pull_request",
                "--bind",
                "--workflows",
                ".github/workflows/ci.yml",
                "--job",
                "build-test",
                "--env",
                "ACT=true",
                "--platform",
                "ubuntu-latest=catthehacker/ubuntu:act-latest",
            ])
            .current_dir(&self.path);
        if let Some(target) = failed_target {
            command.args(["--env", &format!("SMOKE_FAIL_MAKE={target}")]);
        }
        command.output()
    }

    /// Reads only the bounded fixture trace, which contains no secret values.
    fn trace(&self) -> Read<Vec<Vec<String>>> {
        let directory = self.parent.open_dir(&self.name)?;
        let bytes = directory.read("smoke.log")?;
        if !bytes.ends_with(&[0]) {
            return Err("unterminated Act fixture trace".into());
        }
        let mut events = Vec::new();
        let mut event = Vec::new();
        let parts = bytes.split(|byte| *byte == 0).collect::<Vec<_>>();
        for part in parts.iter().take(parts.len().saturating_sub(1)) {
            let value = String::from_utf8(part.to_vec())?;
            if value == "__END__" {
                events.push(std::mem::take(&mut event));
            } else {
                event.push(value);
            }
        }
        if !event.is_empty() {
            return Err("truncated Act fixture trace".into());
        }
        Ok(events)
    }
}

impl Drop for ActSmoke {
    /// Removes only this private scratch checkout.
    fn drop(&mut self) { drop(self.parent.remove_dir_all(&self.name)); }
}

/// Records exact argument boundaries, working directory, selected flags, and token presence.
const STUB: &str = concat!(
    "#!/bin/sh\n",
    "name=${0##*/}\n",
    "if [ \"${GITHUB_TOKEN+x}\" != x ]; then token=unset; ",
    "elif [ -z \"$GITHUB_TOKEN\" ]; then token=empty; else token=present; fi\n",
    "printf '%s\\0' \"$name\" \"$PWD\" \"${RUSTFLAGS-<unset>}\" ",
    "\"${CARGO_PROFILE_DEV_CODEGEN_BACKEND-<unset>}\" \"$token\" ",
    "\"${WITH_ACT-<unset>}\" \"${DYLINT_DRIVER_PATH-<unset>}\" \"$@\" ",
    "'__END__' >> \"$GITHUB_WORKSPACE/smoke.log\"\n",
    "case \"$name:$*\" in\n",
    "  'make:install-build-tools'|'make:check-fmt'|'make:spelling'|'make:audit'|'make:lint'|'make:\
     test') ;;\n",
    "  'cargo:binstall --no-confirm --locked cargo-nextest'|'cargo:binstall --no-confirm \
     cargo-audit') ;;\n",
    "  'rustc:--version') printf '%s\\n' 'rustc fixture'; exit 0 ;;\n",
    "  'sudo:apt-get update'|'sudo:apt-get install --yes --no-install-recommends clang lld mold') \
     ;;\n",
    "  *) exit 97 ;;\n",
    "esac\n",
    "if [ \"$name:$1\" = make:lint ]; then [ -d \"$DYLINT_DRIVER_PATH\" ] || exit 98; fi\n",
    "[ \"${SMOKE_FAIL_MAKE:-}\" != \"$name:$1\" ]\n",
);

/// Exercise Act's real expression and step runner against a derived fixture job.
#[test]
#[ignore = "requires local Act and Docker; run make act-contract-smoke"]
fn real_act_runs_derived_workflow_and_propagates_failure() {
    let fixture =
        ActSmoke::new().expect("create derived Act workflow and explicit action fixtures");
    let passed = fixture.run(None).expect("run real Act smoke");
    assert!(
        passed.status.success(),
        "real Act fixture job must pass: {}",
        String::from_utf8_lossy(&passed.stderr)
    );
    let trace = fixture.trace().expect("read action and shell trace");
    assertions::assert_success_trace(&trace);
    let failed = fixture
        .run(Some("make:lint"))
        .expect("run controlled Act failure");
    assert!(
        !failed.status.success(),
        "a failed lint shell step must fail the Act job"
    );
    let after_failure = fixture.trace().expect("read controlled failure trace");
    let new_events = after_failure
        .get(trace.len()..)
        .expect("second Act run must append trace records");
    assertions::assert_failure_trace(new_events);
}
