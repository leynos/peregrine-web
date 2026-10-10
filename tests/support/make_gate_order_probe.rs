//! Controlled command executors and the real Make gate harness.

use std::{
    error::Error,
    io,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{
    ambient_authority,
    fs::{Permissions, PermissionsExt},
    fs_utf8::Dir,
};

use super::{records, records::Invocation, whitaker};

/// Controlled shell programs, including the test-local mktemp command.
#[path = "make_gate_order_scripts.rs"]
mod scripts;

/// Boxed errors from private fixture setup and NUL record decoding.
type Read<T> = Result<T, Box<dyn Error>>;

/// Caller variables that can change the Make gate route or its child flags.
const CONTROLLED_CALLER_VARIABLES: &[&str] = &[
    "WITH_ACT",
    "RUSTFLAGS",
    "RUSTDOCFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_PROFILE_DEV_CODEGEN_BACKEND",
    "CARGO_BUILD_TARGET",
    "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
    "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS",
    "CFLAGS",
    "LDFLAGS",
];

/// Supplies unique fixture names when tests run concurrently.
static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

/// A caller environment that is either clean or carries the coverage route.
#[derive(Clone, Copy)]
pub(super) enum CallerEnvironment {
    /// Selected routing variables are removed before Make starts.
    Clean,
    /// Coverage's LLVM and lld selections contaminate the Make parent.
    Coverage,
}

/// Private fake tools record their invocation without compiling the crate.
pub(super) struct GateProbe {
    /// Ambient parent directory used only to remove the private fixture.
    parent: Dir,
    /// Capability handle for the private fixture directory.
    directory: Dir,
    /// Unique directory component beneath the system temporary directory.
    name: String,
    /// Absolute UTF-8 path passed to Make's tool variables.
    path: Utf8PathBuf,
}

impl GateProbe {
    /// Creates the controlled executors in a unique private directory.
    pub(super) fn new() -> Read<Self> {
        let parent_path = Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .map_err(|path| io::Error::other(format!("non-UTF-8 temp path: {}", path.display())))?;
        let parent = Dir::open_ambient_dir(&parent_path, ambient_authority())?;
        let name = format!(
            "peregrine-gate-probe-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        );
        parent.create_dir(&name)?;
        let directory = parent.open_dir(&name)?;
        let probe = Self {
            parent,
            directory,
            name: name.clone(),
            path: parent_path.join(&name),
        };
        probe.write_executors()?;
        Ok(probe)
    }

    /// Runs the selected real Make target with a coverage-contaminated caller.
    pub(super) fn run(&self, target: &str, failure: Option<&str>) -> io::Result<Output> {
        self.run_with_environment(target, failure, CallerEnvironment::Coverage)
    }

    /// Runs Make with child-only control over caller routing values.
    pub(super) fn run_with_environment(
        &self,
        target: &str,
        failure: Option<&str>,
        caller_environment: CallerEnvironment,
    ) -> io::Result<Output> {
        self.make_command(target, failure, caller_environment)
            .output()
    }

    /// Runs a real Make target from the fixture root with a private Makefile copy.
    pub(super) fn run_in_private_scratch(
        &self,
        target: &str,
        failure: Option<&str>,
    ) -> Read<Output> {
        self.write_private_makefile()?;
        let mut command = self.make_command(target, failure, CallerEnvironment::Clean);
        command.current_dir(&self.path);
        Ok(command.output()?)
    }

    /// Constructs real Make with command-line substitutions for every executor.
    fn make_command(
        &self,
        target: &str,
        failure: Option<&str>,
        caller_environment: CallerEnvironment,
    ) -> Command {
        let mut command = Command::new("make");
        command
            .args(["-j", "4", target])
            .arg(format!("CARGO={}", self.executable("probe-cargo")))
            .arg(format!(
                "CHECK_BUILD_TOOLS={}",
                self.executable("probe-check")
            ))
            .arg(format!("WHITAKER={}", self.executable("probe-whitaker")))
            .arg(format!(
                "MDTABLEFIX={}",
                self.executable("probe-mdtablefix")
            ))
            .arg(format!("MDLINT={}", self.executable("probe-mdlint")))
            .arg(format!(
                "TYPOS_CONFIG_BUILDER={}",
                self.executable("probe-spelling")
            ))
            .arg(format!("ACT={}", self.executable("probe-act")))
            .arg(format!("BUILD_TOOLS_PREFIX={}", self.path))
            .arg(format!("CURDIR={}", self.path))
            .args([
                "WITH_ACT=0",
                "BUILD_JOBS=",
                "CARGO_FLAGS=--all-targets --all-features",
                "TEST_FLAGS=--all-targets --all-features",
            ])
            .current_dir(Utf8Path::new(env!("CARGO_MANIFEST_DIR")))
            .env("GATE_LOG", self.path.join("gate.bin").as_str())
            .env("WHITAKER_ENV_LOG", self.path.join("whitaker.bin").as_str())
            .env(
                "DYLINT_DRIVER_PATH",
                self.path.join("dylint-drivers").as_str(),
            );
        for variable in CONTROLLED_CALLER_VARIABLES {
            command.env_remove(variable);
        }
        command
            .env("GATE_FAIL_AT", failure.unwrap_or_default())
            .env("CS_ACCESS_TOKEN", "gate-probe-secret-must-not-be-logged")
            .env(
                "GITHUB_TOKEN",
                "gate-probe-github-secret-must-not-be-logged",
            );

        if let CallerEnvironment::Coverage = caller_environment {
            command
                .env("RUSTFLAGS", "-D warnings -C link-arg=-fuse-ld=lld")
                .env(
                    "CARGO_ENCODED_RUSTFLAGS",
                    "-D\u{1f}warnings\u{1f}-C\u{1f}link-arg=-fuse-ld=lld",
                )
                .env("CARGO_PROFILE_DEV_CODEGEN_BACKEND", "llvm")
                .env("CARGO_BUILD_TARGET", "x86_64-unknown-linux-gnu")
                .env("CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER", "clang")
                .env(
                    "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS",
                    "-C link-arg=-fuse-ld=lld",
                )
                .env("CFLAGS", "-fuse-ld=lld")
                .env("LDFLAGS", "-fuse-ld=lld");
        } else {
            command.env("CARGO_ENCODED_RUSTFLAGS", "");
        }
        command
    }

    /// Copies the production Makefile into this test's capability directory.
    fn write_private_makefile(&self) -> Read<()> {
        let repository = Dir::open_ambient_dir(
            Utf8Path::new(env!("CARGO_MANIFEST_DIR")),
            ambient_authority(),
        )?;
        self.directory
            .write("Makefile", repository.read("Makefile")?)?;
        Ok(())
    }

    /// Returns structured records from every controlled Make executor.
    pub(super) fn invocations(&self) -> Read<Vec<Invocation>> {
        records::parse_invocations(&self.gate_records()?)
    }

    /// Reads the NUL-framed stream without converting it to text.
    pub(super) fn gate_records(&self) -> io::Result<Vec<u8>> { self.directory.read("gate.bin") }

    /// Returns executed gate stages while omitting Cargo's version probe.
    pub(super) fn stages(&self) -> Read<Vec<String>> {
        Ok(self
            .invocations()?
            .into_iter()
            .filter(|invocation| invocation.stage != "nextest-version-probe")
            .map(|invocation| invocation.stage)
            .collect())
    }

    /// Reads the dedicated records written by the fake Whitaker repository check.
    pub(super) fn whitaker_records(&self) -> io::Result<Vec<u8>> {
        self.directory.read("whitaker.bin")
    }

    /// Returns the exact private executable path passed to Make.
    pub(super) fn executable(&self, name: &str) -> Utf8PathBuf { self.path.join(name) }

    /// Returns the capability root used for target-local scratch paths.
    pub(super) fn root_path(&self) -> &Utf8Path { &self.path }

    /// Checks a directory beneath the private fixture without ambient access.
    pub(super) fn directory_exists(&self, path: &Utf8Path) -> io::Result<bool> {
        match self.directory.open_dir(path) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// Confirms each record for one stage has the expected executable and argv.
    pub(super) fn assert_command(
        invocations: &[Invocation],
        stage: &str,
        executable: &str,
        arguments: &[&str],
    ) {
        let matching = invocations
            .iter()
            .filter(|invocation| invocation.stage == stage)
            .collect::<Vec<_>>();
        assert!(
            !matching.is_empty(),
            "the {stage} stage must produce an invocation record"
        );
        for invocation in matching {
            assert_eq!(
                invocation.executable, executable,
                "the {stage} stage must use its controlled executable"
            );
            assert_eq!(
                invocation.arguments,
                arguments
                    .iter()
                    .map(|argument| (*argument).to_owned())
                    .collect::<Vec<_>>(),
                "the {stage} stage must preserve its exact argument vector"
            );
            assert_eq!(
                invocation.working_directory,
                env!("CARGO_MANIFEST_DIR"),
                "the {stage} stage must run from the workspace root"
            );
        }
    }

    /// Writes every controlled executable through the same NUL recorder.
    fn write_executors(&self) -> Read<()> {
        self.write_executor("probe-cargo", scripts::CARGO_SCRIPT)?;
        self.write_executor("probe-check", scripts::PREFLIGHT_SCRIPT)?;
        self.write_executor("probe-whitaker", scripts::WHITAKER_SCRIPT)?;
        self.write_executor("probe-mdtablefix", scripts::MDTABLEFIX_SCRIPT)?;
        self.write_executor("probe-mdlint", scripts::MDLINT_SCRIPT)?;
        self.write_executor("probe-spelling", scripts::SPELLING_SCRIPT)?;
        self.write_executor("probe-act", scripts::ACT_SCRIPT)?;
        self.directory.create_dir("bin")?;
        let private_bin = self.directory.open_dir("bin")?;
        Self::write_executor_in(&private_bin, "mktemp", scripts::MKTEMP_SCRIPT)
    }

    /// Creates one executable fixture with the shared structured recorder.
    fn write_executor(&self, name: &str, body: &str) -> Read<()> {
        Self::write_executor_in(&self.directory, name, body)
    }

    /// Creates one executable under a private capability directory.
    fn write_executor_in(directory: &Dir, name: &str, body: &str) -> Read<()> {
        directory.write(
            name,
            format!("#!/bin/sh\n{}\n{}", whitaker::RECORDING_HELPERS, body),
        )?;
        directory.set_permissions(name, Permissions::from_mode(0o700))?;
        Ok(())
    }
}

impl Drop for GateProbe {
    /// Removes only this probe's private scratch directory.
    fn drop(&mut self) { drop(self.parent.remove_dir_all(&self.name)); }
}
