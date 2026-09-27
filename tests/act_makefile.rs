//! Behavioural tests for the Makefile's outer Cargo and nested Act sequence.

use std::{
    fmt::Display,
    io,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{
    ambient_authority,
    fs::{Dir, Permissions, PermissionsExt},
};

const ACT_TOKEN: &str = "controlled-act-token";
const RUNNER_IMAGE: &str = "controlled/ubuntu:act-test";
const CARGO_SCRIPT: &str =
    "#!/bin/sh\nif [ \"$1\" = nextest ]; then exit 1; fi\ncount=0\nif [ -f \"$CARGO_COUNT\" ]; \
     then count=$(cat \"$CARGO_COUNT\"); fi\ncount=$((count + 1))\nprintf '%s' \"$count\" > \
     \"$CARGO_COUNT\"\nprintf cargo >> \"$INVOCATION_LOG\"\nprintf '\\t%s' \"$@\" >> \
     \"$INVOCATION_LOG\"\nprintf '\\n' >> \"$INVOCATION_LOG\"\nif [ \
     \"${FAIL_CARGO_INVOCATION:-}\" = \"$count\" ]; then exit 23; fi\nexit 0\n";
const ACT_SCRIPT: &str = "#!/bin/sh\nprintf act >> \"$INVOCATION_LOG\"\nprintf '\\t%s' \"$@\" >> \
                          \"$INVOCATION_LOG\"\nprintf '\\n' >> \"$INVOCATION_LOG\"\nprintf '%s' \
                          \"${GITHUB_TOKEN:-}\" > \"$TOKEN_CAPTURE\"\nexit 0\n";
static NEXT_HARNESS_ID: AtomicUsize = AtomicUsize::new(0);

/// Records controlled command executions without running Cargo or Act.
struct MakeHarness {
    parent_directory: Dir,
    directory: Dir,
    directory_name: String,
    directory_path: Utf8PathBuf,
}

impl MakeHarness {
    /// Creates executable stand-ins for Cargo and Act in a unique directory.
    fn new() -> io::Result<Self> {
        let harness_id = NEXT_HARNESS_ID.fetch_add(1, Ordering::Relaxed);
        let parent_path = Utf8PathBuf::from_path_buf(std::env::temp_dir()).map_err(|path| {
            io::Error::new(io::ErrorKind::InvalidData, path.display().to_string())
        })?;
        let parent_directory = Dir::open_ambient_dir(&parent_path, ambient_authority())?;
        let directory_name = format!("peregrine-act-make-{}-{harness_id}", std::process::id());
        parent_directory.create_dir(&directory_name)?;
        let directory = parent_directory.open_dir(&directory_name)?;

        let harness = Self {
            parent_directory,
            directory,
            directory_path: parent_path.join(&directory_name),
            directory_name,
        };
        harness.write_command_stubs()?;
        Ok(harness)
    }

    /// Runs the repository's real `make test` target with stub executables.
    fn run_make_test(
        &self,
        with_act: bool,
        fail_cargo_invocation: Option<usize>,
    ) -> io::Result<Output> {
        let manifest_directory = Utf8Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut command = Command::new("make");
        command
            .arg("--no-print-directory")
            .arg("test")
            .arg(format!("WITH_ACT={}", u8::from(with_act)))
            .arg(format!("CARGO={}", self.directory_path.join("cargo")))
            .arg(format!("ACT={}", self.directory_path.join("act")))
            .arg(format!("ACT_RUNNER_IMAGE={RUNNER_IMAGE}"))
            .arg(format!("ACT_GITHUB_TOKEN={ACT_TOKEN}"))
            .current_dir(manifest_directory)
            .env(
                "INVOCATION_LOG",
                self.directory_path.join("invocations.log").as_str(),
            )
            .env(
                "TOKEN_CAPTURE",
                self.directory_path.join("act-token.capture").as_str(),
            )
            .env(
                "CARGO_COUNT",
                self.directory_path.join("cargo-count").as_str(),
            );

        if let Some(invocation) = fail_cargo_invocation {
            command.env("FAIL_CARGO_INVOCATION", invocation.to_string());
        }

        command.output()
    }

    /// Reads command records as tab-separated argument vectors.
    fn invocations(&self) -> io::Result<Vec<Vec<String>>> {
        Ok(self
            .directory
            .read_to_string("invocations.log")?
            .lines()
            .map(|line| line.split('\t').map(str::to_owned).collect())
            .collect())
    }

    /// Reads the token captured from the controlled Act process environment.
    fn captured_token(&self) -> io::Result<String> {
        self.directory.read_to_string("act-token.capture")
    }

    /// Writes silent scripts that log arguments and optionally fail Cargo.
    fn write_command_stubs(&self) -> io::Result<()> {
        write_executable(&self.directory, "cargo", CARGO_SCRIPT)?;
        write_executable(&self.directory, "act", ACT_SCRIPT)
    }
}

impl Drop for MakeHarness {
    /// Removes the per-test command logs and stub executables.
    fn drop(&mut self) { drop(self.parent_directory.remove_dir_all(&self.directory_name)); }
}

/// Runs the normal outer tests without invoking Act when disabled.
#[test]
fn with_act_disabled_runs_only_outer_cargo_commands() {
    let harness = unwrap_test_result(MakeHarness::new(), "create the Make harness");
    let output = unwrap_test_result(harness.run_make_test(false, None), "run make test");

    assert!(output.status.success(), "make test WITH_ACT=0 must succeed");
    assert_eq!(
        unwrap_test_result(harness.invocations(), "read controlled invocations"),
        vec![
            vec!["cargo", "test", "--all-targets", "--all-features"],
            vec!["cargo", "test", "--doc", "--workspace", "--all-features"],
        ],
        "WITH_ACT=0 must run tests then doctests without starting Act"
    );
}

/// Runs outer Cargo commands before Act and checks every forwarded setting.
#[test]
fn with_act_enabled_runs_outer_tests_then_configured_act() {
    let harness = unwrap_test_result(MakeHarness::new(), "create the Make harness");
    let output = unwrap_test_result(harness.run_make_test(true, None), "run make test");
    let invocations = unwrap_test_result(harness.invocations(), "read controlled invocations");
    let expected_common_directory = unwrap_test_result(
        git_common_directory(Utf8Path::new(env!("CARGO_MANIFEST_DIR"))),
        "resolve Git common directory",
    );

    assert!(output.status.success(), "make test WITH_ACT=1 must succeed");
    assert_eq!(
        invocations.len(),
        3,
        "Act must run after the two Cargo commands"
    );
    assert_outer_invocations_are_ordered(&invocations);
    assert_act_invocation_matches(&invocations, &expected_common_directory);
    assert!(
        unwrap_test_result(harness.captured_token(), "read controlled Act token") == ACT_TOKEN,
        "the configured token must reach the controlled Act process"
    );
}

/// Proves failures in either outer Cargo phase prevent Act from running.
#[test]
fn failed_outer_cargo_commands_prevent_act() {
    for failed_invocation in [1, 2] {
        let harness = unwrap_test_result(MakeHarness::new(), "create the Make harness");
        let output = unwrap_test_result(
            harness.run_make_test(true, Some(failed_invocation)),
            "run make test with a controlled failure",
        );
        let invocations = unwrap_test_result(harness.invocations(), "read controlled invocations");

        assert!(
            !output.status.success(),
            "a failed outer Cargo command must fail make"
        );
        assert_eq!(
            invocations.len(),
            failed_invocation,
            "Act must not run when outer Cargo invocation {failed_invocation} fails"
        );
        assert!(
            invocations
                .iter()
                .all(|invocation| invocation.first().is_some_and(|name| name == "cargo")),
            "only the Cargo commands before the failure may be recorded"
        );
    }
}

/// Returns a test result value or reports its setup failure clearly.
fn unwrap_test_result<T, E: Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error}"),
    }
}

/// Compares the recorded Cargo and Act commands with the required order.
fn assert_outer_invocations_are_ordered(invocations: &[Vec<String>]) {
    let outer_invocations = invocations
        .iter()
        .take(2)
        .map(|arguments| arguments.iter().map(String::as_str).collect::<Vec<_>>())
        .collect::<Vec<_>>();

    assert_eq!(
        outer_invocations,
        vec![
            vec!["cargo", "test", "--all-targets", "--all-features"],
            vec!["cargo", "test", "--doc", "--workspace", "--all-features"],
        ],
        "unit and integration tests must precede documentation tests"
    );
}

/// Checks the selected CI job and all Act arguments controlled by the Makefile.
fn assert_act_invocation_matches(invocations: &[Vec<String>], common_directory: &Utf8Path) {
    let Some(act_invocation) = invocations.get(2) else {
        panic!("the third recorded command must be Act");
    };
    let common_mount = format!("--volume {common_directory}:{common_directory}:ro");

    assert_eq!(
        act_invocation.first().map(String::as_str),
        Some("act"),
        "the third command must be Act"
    );
    assert!(
        has_argument_pair(act_invocation, "--job", "build-test"),
        "Act must select the CI build-test job"
    );
    assert!(
        has_argument_pair(
            act_invocation,
            "--platform",
            &format!("ubuntu-latest={RUNNER_IMAGE}")
        ),
        "Act must use the configured runner image"
    );
    assert!(
        has_argument_pair(act_invocation, "--container-options", &common_mount),
        "Act must mount the Git common directory read-only"
    );
    assert!(
        has_argument_pair(act_invocation, "--env", "ACT=true"),
        "Act must mark nested execution with ACT=true"
    );
    assert!(
        has_argument_pair(act_invocation, "--secret", "GITHUB_TOKEN"),
        "Act must forward the token as a secret"
    );
}

/// Finds an option followed by its expected argument value.
fn has_argument_pair(arguments: &[String], option: &str, value: &str) -> bool {
    arguments.windows(2).any(|pair| {
        pair.first().map(String::as_str) == Some(option)
            && pair.get(1).map(String::as_str) == Some(value)
    })
}

/// Resolves Git's common directory from the tested checkout.
fn git_common_directory(repository: &Utf8Path) -> io::Result<Utf8PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .current_dir(repository)
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "git could not resolve its common directory: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let common_directory = String::from_utf8(output.stdout)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(Utf8PathBuf::from(common_directory.trim()))
}

/// Writes a script and marks it executable for controlled command execution.
fn write_executable(directory: &Dir, file_name: &str, contents: &str) -> io::Result<()> {
    directory.write(file_name, contents)?;
    directory.set_permissions(file_name, Permissions::from_mode(0o700))
}
