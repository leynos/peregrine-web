//! Execute committed workflow shell scripts with a recording Make command.

use std::{
    io,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{
    ambient_authority,
    fs::{Permissions, PermissionsExt},
    fs_utf8::Dir,
};

use super::{ACT_VALIDATION_WORKFLOW, CI_WORKFLOW, contract};

/// Fallible workflow shell-fixture result.
type Read<T> = Result<T, Box<dyn std::error::Error>>;
/// Distinguishes private shell command fixtures within the test process.
static NEXT_PROBE_ID: AtomicUsize = AtomicUsize::new(0);

/// Owns a workflow-test directory and removes it when the fixture leaves scope.
struct StepScratch {
    parent: Dir,
    name: String,
}

impl StepScratch {
    /// Creates the directory and immediately takes responsibility for cleanup.
    fn create(parent: Dir) -> io::Result<Self> {
        let name = format!(
            "peregrine-workflow-step-{}-{}",
            std::process::id(),
            NEXT_PROBE_ID.fetch_add(1, Ordering::Relaxed)
        );
        parent.create_dir(&name)?;
        Ok(Self { parent, name })
    }
}

impl Drop for StepScratch {
    fn drop(&mut self) { drop(self.parent.remove_dir_all(&self.name)); }
}

/// Describes one committed workflow step and its expected Make invocation.
#[derive(Clone, Copy)]
struct WorkflowStepCase<'a> {
    name: &'a str,
    script: &'a str,
    target: &'a str,
    with_act: Option<&'a str>,
    root: Option<&'a Utf8Path>,
    should_fail: bool,
}

/// Run one unchanged workflow shell fragment through its controlled Make command.
fn run_make_step(step: WorkflowStepCase<'_>) -> Read<(std::process::Output, Vec<String>)> {
    let root = match step.root {
        Some(root) => root.to_path_buf(),
        None => Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .map_err(|path| io::Error::other(format!("non-UTF-8 temp path: {}", path.display())))?,
    };
    let parent = Dir::open_ambient_dir(&root, ambient_authority())?;
    let scratch = StepScratch::create(parent)?;
    let directory = scratch.parent.open_dir(&scratch.name)?;
    directory.write(
        "make",
        concat!(
            "#!/bin/sh\n",
            "if [ \"${GITHUB_TOKEN+x}\" != x ]; then token=unset; ",
            "elif [ -z \"$GITHUB_TOKEN\" ]; then token=empty; else token=present; fi\n",
            "printf '%s\\0' make \"$PWD\" \"${WITH_ACT+x}\" \"${WITH_ACT-}\" ",
            "\"${RUSTFLAGS+x}\" \"${RUSTFLAGS-}\" ",
            "\"${CARGO_PROFILE_DEV_CODEGEN_BACKEND+x}\" \"${CARGO_PROFILE_DEV_CODEGEN_BACKEND-}\" ",
            "\"$token\" \"${DYLINT_DRIVER_PATH+x}\" \"${DYLINT_DRIVER_PATH-}\" ",
            "\"$#\" \"$@\" '__END__' >> \"$STEP_LOG\"\n",
            "case \"$*\" in 'check-fmt'|'spelling'|'audit'|'lint'|'test'|'install-build-tools') ",
            ";; *) exit 97;; esac\n",
            "[ \"${FAIL_MAKE_TARGET:-}\" != \"$1\" ]\n",
        ),
    )?;
    directory.set_permissions("make", Permissions::from_mode(0o700))?;
    let path = root.join(&scratch.name);
    let mut command = Command::new("bash");
    command
        .args(["-e", "-o", "pipefail", "-c", step.script])
        .current_dir(Utf8Path::new(env!("CARGO_MANIFEST_DIR")))
        .env("PATH", format!("{path}:/usr/bin:/bin"))
        .env("STEP_LOG", path.join("step.log").as_str())
        .env_remove("WITH_ACT")
        .env("RUSTFLAGS", "")
        .env_remove("CARGO_PROFILE_DEV_CODEGEN_BACKEND")
        .env_remove("GITHUB_TOKEN")
        .env_remove("DYLINT_DRIVER_PATH");
    if let Some(value) = step.with_act {
        command.env("WITH_ACT", value);
    }
    if step.target == "lint" {
        command.env("DYLINT_DRIVER_PATH", path.join("cold-driver").as_str());
    }
    if step.should_fail {
        command.env("FAIL_MAKE_TARGET", step.target);
    }
    let output = command.output()?;
    let bytes = directory.read("step.log")?;
    let mut fields = bytes.split(|byte| *byte == 0).collect::<Vec<_>>();
    if fields.pop() != Some(&b""[..]) {
        return Err("unterminated workflow command record".into());
    }
    let record = fields
        .into_iter()
        .map(|part| String::from_utf8(part.to_vec()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((output, record))
}

/// Removes the step directory when execution cannot produce its command record.
#[test]
fn run_make_step_cleans_scratch_after_record_read_failure() {
    let root = Utf8PathBuf::from_path_buf(std::env::temp_dir())
        .expect("system temporary directory should be UTF-8");
    let parent =
        Dir::open_ambient_dir(&root, ambient_authority()).expect("open system temporary directory");
    let scratch = StepScratch::create(parent).expect("create isolated test directory");
    let private_root = root.join(&scratch.name);

    let result = run_make_step(WorkflowStepCase {
        name: "cleanup failure",
        script: ":",
        target: "test",
        with_act: None,
        root: Some(&private_root),
        should_fail: false,
    });

    assert!(
        result.is_err(),
        "a workflow fragment without the recording Make command must fail record reading"
    );
    let directory = scratch
        .parent
        .open_dir(&scratch.name)
        .expect("reopen isolated test directory after failed step");
    let entries = directory
        .read_dir(".")
        .expect("read isolated test directory")
        .collect::<io::Result<Vec<_>>>()
        .expect("read every scratch entry");
    assert!(
        entries.is_empty(),
        "the failed step must remove its scratch directory before returning"
    );
}

#[test]
fn committed_workflow_make_scripts_execute_exact_targets_and_fail_closed() {
    assert!(
        contract::contracts_hold(CI_WORKFLOW, ACT_VALIDATION_WORKFLOW)
            .expect("committed workflows must parse before policy validation"),
        "unknown workflow scripts must be rejected before execution"
    );
    let ci = contract::document(CI_WORKFLOW).expect("parse committed CI workflow");
    for (name, target) in [
        ("Install the build standard", "install-build-tools"),
        ("Format", "check-fmt"),
        ("Check spelling", "spelling"),
        ("Audit dependencies", "audit"),
        ("Lint", "lint"),
        ("Test under Act", "test"),
    ] {
        let item = contract::step(&ci, name).expect("required step").1;
        let script = contract::text(item, &["run"]).expect("step script");
        let with_act = contract::text(item, &["env", "WITH_ACT"]);
        let step = WorkflowStepCase {
            name,
            script,
            target,
            with_act,
            root: None,
            should_fail: false,
        };
        let (passed, record) = run_make_step(step).expect("execute workflow script");
        assert!(
            passed.status.success(),
            "{name} must execute its Make goal: {}",
            String::from_utf8_lossy(&passed.stderr)
        );
        assert_command_record(&record, step);
        let (failed, _) = run_make_step(WorkflowStepCase {
            should_fail: true,
            ..step
        })
        .expect("execute failing workflow script");
        assert!(
            !failed.status.success(),
            "{name} must propagate its Make failure"
        );
    }
}

/// Verifies executor identity, argument boundaries, and inherited environment.
fn assert_command_record(record: &[String], step: WorkflowStepCase<'_>) {
    assert_eq!(
        record.first().map(String::as_str),
        Some("make"),
        "{} must call Make",
        step.name
    );
    assert_eq!(
        record.get(1).map(String::as_str),
        Some(env!("CARGO_MANIFEST_DIR")),
        "{} must run at repository root",
        step.name
    );
    assert_eq!(
        record.get(2).map(String::as_str),
        Some(if step.with_act.is_some() { "x" } else { "" }),
        "{} must distinguish unset WITH_ACT",
        step.name
    );
    assert_eq!(
        record.get(3).map(String::as_str),
        Some(step.with_act.unwrap_or("")),
        "{} must carry its declared WITH_ACT value",
        step.name
    );
    assert_command_environment(record, step);
    assert_command_arguments(record, step);
}

/// Checks setup flags, backend isolation, secret redaction, and the cold cache.
fn assert_command_environment(record: &[String], step: WorkflowStepCase<'_>) {
    assert_eq!(
        record.get(4).map(String::as_str),
        Some("x"),
        "{} must retain empty setup-rust RUSTFLAGS",
        step.name
    );
    assert_eq!(
        record.get(5).map(String::as_str),
        Some(""),
        "{} must retain the empty RUSTFLAGS value",
        step.name
    );
    assert_eq!(
        record.get(6).map(String::as_str),
        Some(""),
        "{} must not inherit a development backend override",
        step.name
    );
    assert_eq!(
        record.get(8).map(String::as_str),
        Some("unset"),
        "{} must not receive a token",
        step.name
    );
    assert_eq!(
        record.get(9).map(String::as_str),
        Some(if step.target == "lint" { "x" } else { "" }),
        "only lint gets a private Dylint cache: {}",
        step.name
    );
}

/// Requires a single complete Make call with exactly the expected goal.
fn assert_command_arguments(record: &[String], step: WorkflowStepCase<'_>) {
    assert_eq!(
        record.get(11).map(String::as_str),
        Some("1"),
        "{} must request exactly one Make goal",
        step.name
    );
    assert_eq!(
        record.get(12).map(String::as_str),
        Some(step.target),
        "{} must pass the expected goal",
        step.name
    );
    assert_eq!(
        record.get(13).map(String::as_str),
        Some("__END__"),
        "{} must issue one complete Make invocation",
        step.name
    );
    assert_eq!(
        record.len(),
        14,
        "{} must issue exactly one Make invocation",
        step.name
    );
}
