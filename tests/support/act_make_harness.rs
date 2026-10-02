//! Controlled executors and NUL-delimited records for Makefile behaviour tests.

use std::{
    io,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{
    ambient_authority,
    fs::{Dir, Permissions, PermissionsExt},
};

use super::{
    ACT_TOKEN,
    RUNNER_IMAGE,
    records::{Invocation, parse_invocations},
};

/// Act executor script that validates its fixed workflow invocation and can fail.
const ACT_SCRIPT: &str = concat!(
    "#!/bin/sh\n",
    include_str!("act_make_record.sh"),
    "record_invocation \"$@\"\n",
    "if [ \"$#\" -ne 14 ] || [ \"$1\" != pull_request ] || \\\n",
    "       [ \"$2\" != --bind ] || [ \"$3\" != --container-options ] || \\\n",
    "       [ \"$5\" != --secret ] || [ \"$6\" != GITHUB_TOKEN ] || \\\n",
    "       [ \"$7\" != --env ] || [ \"$8\" != ACT=true ] || \\\n",
    "       [ \"$9\" != --platform ] || [ \"${11}\" != --workflows ] || \\\n",
    "       [ \"${12}\" != .github/workflows/ci.yml ] || \\\n",
    "       [ \"${13}\" != --job ] || [ \"${14}\" != build-test ]; then\n",
    "  exit 64\n",
    "fi\n",
    "case \"$4\" in \"--volume \"*:ro) ;; *) exit 64 ;; esac\n",
    "case \"${10}\" in ubuntu-latest=*) ;; *) exit 64 ;; esac\n",
    "[ \"${FAIL_ACT:-}\" != 1 ] || exit 31\n",
);

/// PATH fallback that makes a literal `act` command visible as a failed bypass.
const ACT_PATH_SHIM: &str = concat!(
    "#!/bin/sh\n",
    include_str!("act_make_record.sh"),
    "INVOCATION_EXECUTABLE=act-path-shim\n",
    "export INVOCATION_EXECUTABLE\n",
    "record_invocation \"$@\"\n",
    "printf '%s\\n' 'Make bypassed the controlled ACT executor' >&2\n",
    "exit 65\n",
);

/// Cargo executor script that selects Cargo tests and can fail by stage.
const CARGO_SCRIPT: &str = concat!(
    "#!/bin/sh\n",
    include_str!("act_make_record.sh"),
    "record_invocation \"$@\"\n",
    "if [ \"$#\" -eq 2 ] && [ \"$1\" = nextest ] && [ \"$2\" = --version ]; then\n",
    "  if [ \"${NEXTTEST_AVAILABLE:-}\" = 1 ]; then\n",
    "    printf '%s\\n' 'cargo-nextest fixture 0.1'\n",
    "    exit 0\n",
    "  fi\n",
    "  exit 1\n",
    "fi\n",
    "if [ \"$#\" -eq 4 ] && [ \"$1\" = nextest ] && \\\n",
    "       [ \"$2\" = run ] && [ \"$3\" = --all-targets ] && \\\n",
    "       [ \"$4\" = --all-features ]; then\n",
    "  :\n",
    "elif [ \"$#\" -eq 3 ] && [ \"$1\" = test ] && \\\n",
    "       [ \"$2\" = --all-targets ] && [ \"$3\" = --all-features ]; then\n",
    "  :\n",
    "elif [ \"$#\" -eq 4 ] && [ \"$1\" = test ] && [ \"$2\" = --doc ] && \\\n",
    "         [ \"$3\" = --workspace ] && [ \"$4\" = --all-features ]; then\n",
    "  :\n",
    "else\n",
    "  exit 64\n",
    "fi\n",
    "count=0\n",
    "if [ -f \"$CARGO_COUNT\" ]; then IFS= read -r count < \"$CARGO_COUNT\"; fi\n",
    "count=$((count + 1))\n",
    "printf '%s' \"$count\" > \"$CARGO_COUNT\"\n",
    "[ \"${FAIL_CARGO_INVOCATION:-}\" != \"$count\" ] || exit 23\n",
);

/// PATH fallback that rejects Make recipes which bypass the injected Cargo.
const CARGO_PATH_SHIM: &str = concat!(
    "#!/bin/sh\n",
    include_str!("act_make_record.sh"),
    "INVOCATION_EXECUTABLE=cargo-path-shim\n",
    "export INVOCATION_EXECUTABLE\n",
    "record_invocation \"$@\"\n",
    "printf '%s\\n' 'Make bypassed the controlled CARGO executor' >&2\n",
    "exit 65\n",
);

/// Build-preflight executor script that rejects unexpected arguments.
const PREFLIGHT_SCRIPT: &str = concat!(
    "#!/bin/sh\n",
    include_str!("act_make_record.sh"),
    "record_invocation \"$@\"\n",
    "[ \"$#\" -eq 0 ] || exit 64\n",
    "[ \"${FAIL_PREFLIGHT:-}\" != 1 ] || exit 32\n",
    "if [ \"${CARGO_PROFILE_DEV_CODEGEN_BACKEND+x}\" = x ] && \\\n",
    "       [ \"$CARGO_PROFILE_DEV_CODEGEN_BACKEND\" != cranelift ]; then exit 33; fi\n",
);

/// Minimal recorder used to prove exact NUL-delimited argument preservation.
const RECORD_PROBE_SCRIPT: &str = concat!(
    "#!/bin/sh\n",
    include_str!("act_make_record.sh"),
    "record_invocation \"$@\"\n",
);

/// Generates distinct fixture directories when tests run concurrently.
static NEXT_HARNESS_ID: AtomicUsize = AtomicUsize::new(0);

/// External executor whose failure is injected for a test scenario.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum ExecutorFailure {
    /// Fail the nested Act invocation.
    Act,
    /// Fail the build-tools preflight before repository testing.
    Preflight,
}

/// Values controlled by each Make harness invocation.
#[derive(Clone, Copy, Default)]
pub(super) struct MakeOptions {
    /// Whether Make receives the command-line `WITH_ACT=1` assignment.
    pub(super) with_act: bool,
    /// Which real Cargo test invocation should fail, if any.
    pub(super) fail_cargo_invocation: Option<usize>,
    /// Controlled executor stage that should fail, if any.
    pub(super) failure: Option<ExecutorFailure>,
    /// Whether the Cargo version probe reports an installed Nextest binary.
    pub(super) nextest_available: bool,
    /// Caller value intentionally overridden by Make's `WITH_ACT=0` assignment.
    pub(super) caller_with_act: Option<&'static str>,
    /// Supported caller flags forwarded to repository Cargo commands.
    pub(super) caller_rustflags: Option<&'static str>,
    /// Backend override used to prove preflight rejects a contaminated caller.
    pub(super) caller_backend: Option<&'static str>,
    /// Linker override used to prove preflight sees coverage contamination.
    pub(super) caller_linker: Option<&'static str>,
}

impl MakeOptions {
    /// Applies caller values after the harness removes unsupported ambient routing.
    fn apply_caller_environment(&self, command: &mut Command) {
        if let Some(caller_with_act) = self.caller_with_act {
            command.env("WITH_ACT", caller_with_act);
        } else {
            command.env_remove("WITH_ACT");
        }
        if let Some(rustflags) = self.caller_rustflags {
            command.env("RUSTFLAGS", rustflags);
        }
        if let Some(backend) = self.caller_backend {
            command.env("CARGO_PROFILE_DEV_CODEGEN_BACKEND", backend);
        }
        if let Some(linker) = self.caller_linker {
            command.env("CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER", linker);
        }
    }

    /// Enables only the executor controls selected for this invocation.
    fn apply_executor_controls(&self, command: &mut Command) {
        if let Some(invocation) = self.fail_cargo_invocation {
            command.env("FAIL_CARGO_INVOCATION", invocation.to_string());
        }
        if self.failure == Some(ExecutorFailure::Act) {
            command.env("FAIL_ACT", "1");
        }
        if self.failure == Some(ExecutorFailure::Preflight) {
            command.env("FAIL_PREFLIGHT", "1");
        }
        if self.nextest_available {
            command.env("NEXTTEST_AVAILABLE", "1");
        }
    }
}

/// Runs real Make recipes while replacing their external executors.
pub(super) struct MakeHarness {
    /// Parent capability used to remove the private fixture directory.
    parent_directory: Dir,
    /// Capability rooted at the private fixture directory.
    directory: Dir,
    /// Directory name relative to the parent capability.
    directory_name: String,
    /// Absolute path passed to Make as each controlled executor.
    directory_path: Utf8PathBuf,
}

impl MakeHarness {
    /// Creates a private directory and executable stand-ins for Make commands.
    pub(super) fn new() -> io::Result<Self> {
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

    /// Runs the real `make test` target with child-process environment controls.
    pub(super) fn run_make_test(&self, options: MakeOptions) -> io::Result<Output> {
        let manifest_directory = Utf8Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut command = Command::new("make");
        command
            .arg("--no-print-directory")
            .arg("test")
            .arg(format!("WITH_ACT={}", u8::from(options.with_act)))
            .arg(format!("CARGO={}", self.directory_path.join("cargo")))
            .arg(format!("ACT={}", self.directory_path.join("act")))
            .arg(format!(
                "CHECK_BUILD_TOOLS={}",
                self.directory_path.join("check-build-tools")
            ))
            .arg(format!(
                "BUILD_TOOLS_PREFIX={}",
                self.directory_path.join("tool-prefix")
            ))
            .arg(format!("ACT_RUNNER_IMAGE={RUNNER_IMAGE}"))
            .arg(format!("ACT_GITHUB_TOKEN={ACT_TOKEN}"))
            .current_dir(manifest_directory)
            .env(
                "INVOCATION_LOG",
                self.directory_path.join("invocations.bin").as_str(),
            )
            .env(
                "CARGO_COUNT",
                self.directory_path.join("cargo-count").as_str(),
            )
            .env_remove("CARGO_ENCODED_RUSTFLAGS")
            .env_remove("CARGO_PROFILE_DEV_CODEGEN_BACKEND")
            .env_remove("CARGO_BUILD_TARGET")
            .env_remove("CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER")
            .env_remove("GITHUB_TOKEN")
            .env_remove("FAIL_CARGO_INVOCATION")
            .env_remove("FAIL_ACT")
            .env_remove("FAIL_PREFLIGHT")
            .env_remove("NEXTTEST_AVAILABLE")
            .env_remove("INVOCATION_EXECUTABLE")
            .env_remove("ACT");

        options.apply_caller_environment(&mut command);
        options.apply_executor_controls(&mut command);
        command.output()
    }

    /// Parses NUL-delimited records without confusing tabs or newlines in args.
    pub(super) fn invocations(&self) -> io::Result<Vec<Invocation>> {
        self.read_invocation_file("invocations.bin")
    }

    /// Records unusual arguments and environment values in a child-only process.
    pub(super) fn record_probe(
        &self,
        arguments: &[&str],
        rustflags: &str,
    ) -> io::Result<Vec<Invocation>> {
        let record_path = self.directory_path.join("record-only");
        let output = Command::new(record_path.as_str())
            .args(arguments)
            .current_dir(Utf8Path::new(env!("CARGO_MANIFEST_DIR")))
            .env_clear()
            .env(
                "INVOCATION_LOG",
                self.directory_path.join("round-trip.bin").as_str(),
            )
            .env("RUSTFLAGS", rustflags)
            .env("ACT", "")
            .output()?;
        if !output.status.success() {
            return Err(io::Error::other(format!(
                "record-only executor failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        self.read_invocation_file("round-trip.bin")
    }

    /// Reads and parses a named NUL-delimited record file.
    fn read_invocation_file(&self, file_name: &str) -> io::Result<Vec<Invocation>> {
        let log = self.directory.read(file_name)?;
        let text = std::str::from_utf8(&log)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        parse_invocations(text)
    }

    /// Writes the controlled Cargo, Act, and build-preflight commands.
    fn write_command_stubs(&self) -> io::Result<()> {
        write_executable(&self.directory, "cargo", CARGO_SCRIPT)?;
        write_executable(&self.directory, "act", ACT_SCRIPT)?;
        write_executable(&self.directory, "check-build-tools", PREFLIGHT_SCRIPT)?;
        write_executable(&self.directory, "record-only", RECORD_PROBE_SCRIPT)?;
        self.write_path_shims()
    }

    /// Installs rejecting PATH fallbacks for literal Cargo and Act commands.
    fn write_path_shims(&self) -> io::Result<()> {
        self.directory.create_dir("tool-prefix")?;
        let tool_prefix = self.directory.open_dir("tool-prefix")?;
        tool_prefix.create_dir("bin")?;
        let bin_directory = tool_prefix.open_dir("bin")?;
        write_executable(&bin_directory, "cargo", CARGO_PATH_SHIM)?;
        write_executable(&bin_directory, "act", ACT_PATH_SHIM)
    }
}

impl Drop for MakeHarness {
    /// Removes the private command records and fixture executables.
    fn drop(&mut self) { drop(self.parent_directory.remove_dir_all(&self.directory_name)); }
}

/// Writes a script and marks it executable for controlled command execution.
fn write_executable(directory: &Dir, file_name: &str, contents: &str) -> io::Result<()> {
    directory.write(file_name, contents)?;
    directory.set_permissions(file_name, Permissions::from_mode(0o700))
}

/// Direct configuration contracts for the private option helpers.
#[cfg(test)]
#[path = "act_make_options_tests.rs"]
mod options_tests;
