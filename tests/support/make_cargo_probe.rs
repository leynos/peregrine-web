//! Fake Cargo executable shared by the Make command-contract suites.
//!
//! The probe records what Make actually executes, so a test can assert the
//! selected executable, the argument vector, and the order of the steps without
//! running a real build. It also fails on demand, which is how the suites prove
//! that a failed step stops the ones after it.
//!
//! # Environment isolation
//!
//! The probe clears the inherited `TEST_FLAGS`, `MAKEFLAGS` and `MFLAGS`
//! variables. A targeted run such as `make test TEST_FLAGS='--test suite'`
//! exports its flags, and Make forwards command-line variables through
//! `MAKEFLAGS`, so a nested `make -n test` would otherwise be describing the
//! outer invocation rather than the repository defaults. Each contract binary
//! therefore passes both on its own and inside the full `make test` route.
//!
//! # Lint scope
//!
//! `make_cargo_routes` and `make_fetch_contract` each include this module
//! through `#[path]`, and Cargo compiles a separate copy into every test
//! binary. An item that only one of those binaries uses therefore looks unused
//! here. The allowance covers that duplication; it is not a place to leave
//! genuinely stale helpers.

#![allow(
    dead_code,
    reason = "each Make contract binary includes a different subset of these helpers"
)]

use std::{
    error::Error,
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

/// The result type shared by the probe helpers.
pub type Read<T> = Result<T, Box<dyn Error>>;

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

/// Owns a fake Cargo executable and records only its build invocations.
pub struct CargoProbe {
    parent: Dir,
    pub directory: Dir,
    name: String,
    pub path: Utf8PathBuf,
}

/// The independent Make branches exercised by one fake-Cargo invocation.
#[derive(Clone, Copy)]
pub struct MakeOptions<'a> {
    pub nextest_available: bool,
    pub dry_run: bool,
    pub with_act: bool,
    pub check: &'a str,
}

impl CargoProbe {
    /// Creates an isolated directory holding the fake Cargo and preflight.
    pub fn new() -> Read<Self> {
        let parent_path = Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .map_err(|path| io::Error::other(format!("non-UTF-8 temp path: {}", path.display())))?;
        let parent = Dir::open_ambient_dir(&parent_path, ambient_authority())?;
        let name = format!(
            "peregrine-cargo-probe-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        );
        parent.create_dir(&name)?;
        let directory = parent.open_dir(&name)?;
        // A `fetch` invocation is logged like any other so the fetch contract
        // can assert it, and `CARGO_FETCH_STATUS` lets a test make it fail.
        directory.write(
            "probe-cargo",
            "#!/bin/sh\nif [ \"$1\" = fetch ]; then printf '%s\\n' \"$*\" >> \"$CARGO_LOG\"; exit \
             \"${CARGO_FETCH_STATUS:-0}\"; fi\nif [ \"$1\" = nextest ] && [ \"$2\" = --version ]; \
             then\n  if [ \"${NEXTEST_AVAILABLE:-0}\" = 0 ]; then printf '%s\\n' \"cargo-nextest \
             ${NEXTEST_FAKE_VERSION:-0.9.100}\"; exit 0; fi\n  exit 1\nfi\nprintf '%s\\n' \"$*\" \
             >> \"$CARGO_LOG\"\nif [ -n \"${PRODUCTION_LOG:-}\" ]; then env | sort > \
             \"$PRODUCTION_LOG\"; fi\nif [ -n \"${CARGO_ENV_LOG:-}\" ]; then printf '%s\\n' \
             \"${RUSTFLAGS:-}\" > \"$CARGO_ENV_LOG\"; fi\nexit \"${CARGO_PROBE_STATUS:-0}\"\n",
        )?;
        directory.set_permissions("probe-cargo", Permissions::from_mode(0o700))?;
        directory.write(
            "preflight",
            "#!/bin/sh\nprintf '%s\\n' preflight >> \"$CARGO_LOG\"\n",
        )?;
        directory.set_permissions("preflight", Permissions::from_mode(0o700))?;
        Ok(Self {
            parent,
            directory,
            path: parent_path.join(&name),
            name,
        })
    }

    /// Builds a Make invocation that reaches the fake Cargo executable.
    pub fn make_command(&self, target: &str, options: MakeOptions<'_>) -> Command {
        let mut command = Command::new("make");
        if options.dry_run {
            command.arg("-n");
        }
        command.arg("-B");
        command
            .arg(target)
            .arg(format!("CARGO={}", self.path.join("probe-cargo")))
            .arg(format!("CHECK_BUILD_TOOLS={}", options.check))
            .arg("WHITAKER=true")
            .arg(format!("WITH_ACT={}", u8::from(options.with_act)))
            .current_dir(Utf8Path::new(env!("CARGO_MANIFEST_DIR")))
            // A targeted `make test TEST_FLAGS=...` exports TEST_FLAGS, and
            // Make forwards command-line variables through MAKEFLAGS, so the
            // nested `make -n test` would otherwise rewrite its own flags and
            // the assertions would describe the outer run instead of the
            // repository defaults. Clearing them keeps each contract binary
            // runnable on its own.
            .env_remove("TEST_FLAGS")
            .env_remove("MAKEFLAGS")
            .env_remove("MFLAGS")
            .env(
                "NEXTEST_AVAILABLE",
                if options.nextest_available { "0" } else { "1" },
            )
            .env("CARGO_LOG", self.path.join("cargo.log").as_str());
        command
    }

    /// Runs a Make target against the fake Cargo executable.
    pub fn make(&self, target: &str, options: MakeOptions<'_>) -> io::Result<std::process::Output> {
        self.make_command(target, options).output()
    }
}

impl Drop for CargoProbe {
    fn drop(&mut self) { drop(self.parent.remove_dir_all(&self.name)); }
}

/// Read executed Make commands, leaving shell probe calls out of the output.
pub fn dry_run(probe: &CargoProbe, target: &str, nextest: bool, with_act: bool) -> Read<String> {
    let output = probe.make(
        target,
        MakeOptions {
            nextest_available: nextest,
            dry_run: true,
            with_act,
            check: "scripts/check-build-tools.sh",
        },
    )?;
    if !output.status.success() {
        return Err(format!(
            "make -n {target} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

/// Detect an executable bare Cargo token rather than a printed diagnostic.
pub fn has_bare_cargo(commands: &str) -> bool {
    commands.lines().any(|line| {
        let trimmed = line.trim_start();
        !trimmed.starts_with("printf ")
            && !trimmed.starts_with("echo ")
            && (trimmed.starts_with("cargo ") || trimmed.contains(" cargo "))
    })
}
