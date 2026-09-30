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

/// Run one unchanged workflow shell fragment through its controlled Make command.
fn run_make_step(
    script: &str,
    target: &str,
    with_act: Option<&str>,
    failure: bool,
) -> Read<(std::process::Output, Vec<String>)> {
    let root = Utf8PathBuf::from_path_buf(std::env::temp_dir())
        .map_err(|path| io::Error::other(format!("non-UTF-8 temp path: {}", path.display())))?;
    let parent = Dir::open_ambient_dir(&root, ambient_authority())?;
    let name = format!(
        "peregrine-workflow-step-{}-{}",
        std::process::id(),
        NEXT_PROBE_ID.fetch_add(1, Ordering::Relaxed)
    );
    parent.create_dir(&name)?;
    let directory = parent.open_dir(&name)?;
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
            "case \"$*\" in 'check-fmt'|'spelling'|'audit'|'lint'|'test'|'install-build-tools') \
             ;; *) exit 97;; esac\n",
            "[ \"${FAIL_MAKE_TARGET:-}\" != \"$1\" ]\n",
        ),
    )?;
    directory.set_permissions("make", Permissions::from_mode(0o700))?;
    let path = root.join(&name);
    let mut command = Command::new("bash");
    command
        .args(["-e", "-o", "pipefail", "-c", script])
        .current_dir(Utf8Path::new(env!("CARGO_MANIFEST_DIR")))
        .env("PATH", format!("{path}:/usr/bin:/bin"))
        .env("STEP_LOG", path.join("step.log").as_str())
        .env_remove("WITH_ACT")
        .env("RUSTFLAGS", "")
        .env_remove("CARGO_PROFILE_DEV_CODEGEN_BACKEND")
        .env_remove("GITHUB_TOKEN")
        .env_remove("DYLINT_DRIVER_PATH");
    if let Some(value) = with_act {
        command.env("WITH_ACT", value);
    }
    if target == "lint" {
        command.env("DYLINT_DRIVER_PATH", path.join("cold-driver").as_str());
    }
    if failure {
        command.env("FAIL_MAKE_TARGET", target);
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
    parent.remove_dir_all(name)?;
    Ok((output, record))
}

#[test]
fn committed_workflow_make_scripts_execute_exact_targets_and_fail_closed() {
    assert!(
        contract::contracts_hold(CI_WORKFLOW, ACT_VALIDATION_WORKFLOW),
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
        let (passed, record) =
            run_make_step(script, target, with_act, false).expect("execute workflow script");
        assert!(
            passed.status.success(),
            "{name} must execute its Make goal: {}",
            String::from_utf8_lossy(&passed.stderr)
        );
        assert_command_record(&record, name, target, with_act);
        let (failed, _) =
            run_make_step(script, target, with_act, true).expect("execute failing workflow script");
        assert!(
            !failed.status.success(),
            "{name} must propagate its Make failure"
        );
    }
}

/// Verifies executor identity, argument boundaries, and inherited environment.
fn assert_command_record(record: &[String], name: &str, target: &str, with_act: Option<&str>) {
    assert_eq!(
        record.first().map(String::as_str),
        Some("make"),
        "{name} must call Make"
    );
    assert_eq!(
        record.get(1).map(String::as_str),
        Some(env!("CARGO_MANIFEST_DIR")),
        "{name} must run at repository root"
    );
    assert_eq!(
        record.get(2).map(String::as_str),
        Some(if with_act.is_some() { "x" } else { "" }),
        "{name} must distinguish unset WITH_ACT"
    );
    assert_eq!(
        record.get(3).map(String::as_str),
        Some(with_act.unwrap_or("")),
        "{name} must carry its declared WITH_ACT value"
    );
    assert_command_environment(record, name, target);
    assert_command_arguments(record, name, target);
}

/// Checks setup flags, backend isolation, secret redaction, and the cold cache.
fn assert_command_environment(record: &[String], name: &str, target: &str) {
    assert_eq!(
        record.get(4).map(String::as_str),
        Some("x"),
        "{name} must retain empty setup-rust RUSTFLAGS"
    );
    assert_eq!(
        record.get(5).map(String::as_str),
        Some(""),
        "{name} must retain the empty RUSTFLAGS value"
    );
    assert_eq!(
        record.get(6).map(String::as_str),
        Some(""),
        "{name} must not inherit a development backend override"
    );
    assert_eq!(
        record.get(8).map(String::as_str),
        Some("unset"),
        "{name} must not receive a token"
    );
    assert_eq!(
        record.get(9).map(String::as_str),
        Some(if target == "lint" { "x" } else { "" }),
        "only lint gets a private Dylint cache"
    );
}

/// Requires a single complete Make call with exactly the expected goal.
fn assert_command_arguments(record: &[String], name: &str, target: &str) {
    assert_eq!(
        record.get(11).map(String::as_str),
        Some("1"),
        "{name} must request exactly one Make goal"
    );
    assert_eq!(
        record.get(12).map(String::as_str),
        Some(target),
        "{name} must pass the expected goal"
    );
    assert_eq!(
        record.get(13).map(String::as_str),
        Some("__END__"),
        "{name} must issue one complete Make invocation"
    );
    assert_eq!(
        record.len(),
        14,
        "{name} must issue exactly one Make invocation"
    );
}
