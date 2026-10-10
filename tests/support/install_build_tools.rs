//! Runs the real build-tools installer against controlled child executors.

use std::{
    io::{self, Read},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

use camino::{Utf8Path, Utf8PathBuf};
use cap_std::{
    ambient_authority,
    fs::{Permissions, PermissionsExt},
    fs_utf8::Dir,
};

/// Gives each fixture a distinct path inside the shared temporary directory.
static NEXT_FIXTURE_ID: AtomicUsize = AtomicUsize::new(0);
/// The archive basename selected by the pinned tool version.
const MOLD_ARCHIVE: &str = "mold-2.41.0-x86_64-linux.tar.gz";
/// A stable payload that the fake downloader writes to its output path.
const ARCHIVE_CONTENT: &str = "fixture mold archive\n";
/// The configurable URL deliberately contains a credential-shaped test value.
const RELEASE_BASE_URL: &str =
    "https://download.example.invalid/private?credential=fixture-url-secret";

/// The checksum-file state to present to the installer.
#[derive(Clone, Copy)]
pub(super) enum ChecksumRecord {
    /// One correct checksum is recorded.
    Correct,
    /// No checksum is recorded.
    Missing,
    /// Two checksum rows name the same artifact.
    Duplicate,
    /// A single incorrect checksum is recorded.
    Wrong,
}

/// Owns a scratch repository and fake command executors for one installer run.
pub(super) struct InstallerFixture {
    /// Capability used to remove the fixture root when the test ends.
    parent: Dir,
    /// Capability rooted at the fixture's private directory.
    directory: Dir,
    /// Directory name relative to the system temporary directory.
    name: String,
    /// UTF-8 path used to configure the controlled child environment.
    root: Utf8PathBuf,
}

impl InstallerFixture {
    /// Creates isolated pins, checksum data, and fake external executors.
    pub(super) fn new() -> io::Result<Self> {
        let parent_path = Utf8PathBuf::from_path_buf(std::env::temp_dir()).map_err(|path| {
            io::Error::other(format!(
                "temporary directory is not UTF-8: {}",
                path.display()
            ))
        })?;
        let parent = Dir::open_ambient_dir(&parent_path, ambient_authority())?;
        let name = format!(
            "peregrine-install-tools-{}-{}",
            std::process::id(),
            NEXT_FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        );
        parent.create_dir(&name)?;
        let directory = parent.open_dir(&name)?;
        directory.create_dir("bin")?;
        directory.create_dir("scratch")?;
        directory.write("version", "2.41.0\n")?;
        directory.write(
            "toolchain.toml",
            r#"[toolchain]
channel = "nightly-2030-01-01"
components = [
  "clippy",
  "rustfmt",
]
"#,
        )?;
        directory.write("archive.fixture", ARCHIVE_CONTENT)?;
        for (executable, script) in [
            ("bin/curl", CURL_FIXTURE),
            ("bin/tar", TAR_FIXTURE),
            ("bin/rustup", RUSTUP_FIXTURE),
            ("bin/uname", UNAME_FIXTURE),
        ] {
            directory.write(executable, script)?;
            directory.set_permissions(executable, Permissions::from_mode(0o700))?;
        }

        let root = parent_path.join(&name);
        let fixture = Self {
            parent,
            directory,
            name,
            root,
        };
        fixture.set_checksum(ChecksumRecord::Correct)?;
        Ok(fixture)
    }

    /// Runs the repository installer with child-only host and failure controls.
    pub(super) fn run(
        &self,
        system: &str,
        machine: &str,
        failure: Option<&str>,
    ) -> io::Result<Output> {
        self.installer_command(system, machine, failure).output()
    }

    /// Runs with system curl and tar while keeping rustup under fixture control.
    pub(super) fn run_with_real_download(&self, release_base_url: &str) -> io::Result<Output> {
        self.directory
            .rename("bin/curl", &self.directory, "bin/curl.fixture")?;
        self.directory
            .rename("bin/tar", &self.directory, "bin/tar.fixture")?;
        self.installer_command("Linux", "x86_64", None)
            .env("MOLD_RELEASE_BASE_URL", release_base_url)
            .env(
                "INSTALLER_EXPECTED_URL",
                format!("{release_base_url}/v2.41.0/{MOLD_ARCHIVE}"),
            )
            .env("NO_PROXY", "127.0.0.1,localhost")
            .env("no_proxy", "127.0.0.1,localhost")
            .output()
    }

    /// Creates the small, valid mold archive served by the local HTTP fixture.
    pub(super) fn create_real_archive(&self) -> io::Result<()> {
        let archive_root = "archive-root/mold-2.41.0-x86_64-linux/bin";
        self.directory.create_dir_all(archive_root)?;
        self.directory.write(
            "archive-root/mold-2.41.0-x86_64-linux/bin/ld.mold",
            "fixture mold executable\n",
        )?;
        let output = Command::new("/usr/bin/tar")
            .arg("--create")
            .arg("--gzip")
            .arg("--file")
            .arg(self.root.join("archive.fixture").as_str())
            .arg("--directory")
            .arg(self.root.join("archive-root").as_str())
            .arg("mold-2.41.0-x86_64-linux/bin/ld.mold")
            .output()?;
        if !output.status.success() {
            return Err(io::Error::other(format!(
                "real tar could not create the fixture archive: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        Ok(())
    }

    /// Reads the archive bytes for the loopback HTTP response.
    pub(super) fn archive_bytes(&self) -> io::Result<Vec<u8>> {
        let mut archive = self.directory.open("archive.fixture")?;
        let mut contents = Vec::new();
        archive.read_to_end(&mut contents)?;
        Ok(contents)
    }

    /// Reads the file the real tar extractor should install under the prefix.
    pub(super) fn extracted_payload(&self) -> io::Result<String> {
        self.directory
            .open_dir("prefix")?
            .open_dir("bin")?
            .read_to_string("ld.mold")
    }

    /// Builds the installer child command with an explicit executor precedence.
    fn installer_command(&self, system: &str, machine: &str, failure: Option<&str>) -> Command {
        let repository = Utf8Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut command = Command::new("/usr/bin/bash");
        let fixture_bin = self.root.join("bin");
        let path = format!("{fixture_bin}:/usr/bin:/bin");
        command
            .arg(repository.join("scripts/install-build-tools.sh"))
            .env("PATH", path)
            .env("HOME", self.root.as_str())
            .env("TMPDIR", self.root.join("scratch").as_str())
            .env("INSTALLER_TRACE", self.root.join("trace").as_str())
            .env("INSTALLER_FAILURE", failure.unwrap_or("none"))
            .env("INSTALLER_UNAME_SYSTEM", system)
            .env("INSTALLER_UNAME_MACHINE", machine)
            .env("MOLD_VERSION_FILE", self.root.join("version").as_str())
            .env(
                "MOLD_SHA256SUMS_FILE",
                self.root.join("sha256sums").as_str(),
            )
            .env(
                "RUST_TOOLCHAIN_FILE",
                self.root.join("toolchain.toml").as_str(),
            )
            .env("BUILD_TOOLS_PREFIX", self.root.join("prefix").as_str())
            .env("MOLD_RELEASE_BASE_URL", RELEASE_BASE_URL)
            .env(
                "INSTALLER_EXPECTED_URL",
                format!("{RELEASE_BASE_URL}/v2.41.0/{MOLD_ARCHIVE}"),
            )
            .env("INSTALLER_ARCHIVE_NAME", MOLD_ARCHIVE)
            .env(
                "INSTALLER_EXPECTED_PREFIX",
                self.root.join("prefix").as_str(),
            )
            .env("CURL_CONNECT_TIMEOUT", "2")
            .env("CURL_MIN_BYTES_PER_SECOND", "1")
            .env("CURL_STALL_SECONDS", "2")
            .env_remove("BASH_ENV");
        command
    }

    /// Writes the selected checksum-file state using a real sha256sum result.
    pub(super) fn set_checksum(&self, record: ChecksumRecord) -> io::Result<()> {
        let digest = self.archive_digest()?;
        let rows = match record {
            ChecksumRecord::Correct => format!("{digest}  {MOLD_ARCHIVE}\n"),
            ChecksumRecord::Missing => String::new(),
            ChecksumRecord::Duplicate => {
                format!("{digest}  {MOLD_ARCHIVE}\n{digest}  {MOLD_ARCHIVE}\n")
            }
            ChecksumRecord::Wrong => {
                let wrong = if digest.starts_with('0') {
                    "f".repeat(64)
                } else {
                    "0".repeat(64)
                };
                format!("{wrong}  {MOLD_ARCHIVE}\n")
            }
        };
        self.directory.write("sha256sums", rows)
    }

    /// Returns the executor names in their recorded invocation order.
    pub(super) fn command_names(&self) -> io::Result<String> {
        let trace = self.directory.read_to_string("trace")?;
        let names = trace
            .lines()
            .map(|line| line.split_once(':').map_or(line, |(name, _)| name))
            .collect::<Vec<_>>();
        Ok(names.join(","))
    }

    /// Checks whether a fake executor recorded the requested arguments.
    pub(super) fn trace_contains(&self, expected: &str) -> io::Result<bool> {
        Ok(self.directory.read_to_string("trace")?.contains(expected))
    }

    /// Counts remaining installer temporary directories after the EXIT trap.
    pub(super) fn scratch_entries(&self) -> io::Result<usize> {
        Ok(self.directory.read_dir("scratch")?.count())
    }

    /// Hashes the controlled payload with the host sha256sum executable.
    fn archive_digest(&self) -> io::Result<String> {
        let output = Command::new("/usr/bin/sha256sum")
            .arg(self.root.join("archive.fixture"))
            .output()?;
        if !output.status.success() {
            return Err(io::Error::other(
                "real sha256sum could not hash the fixture",
            ));
        }
        String::from_utf8(output.stdout)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?
            .split_ascii_whitespace()
            .next()
            .map(str::to_owned)
            .ok_or_else(|| io::Error::other("real sha256sum returned no digest"))
    }
}

impl Drop for InstallerFixture {
    /// Removes the isolated fixture tree after every scenario.
    fn drop(&mut self) { drop(self.parent.remove_dir_all(&self.name)); }
}

/// A curl replacement that validates its full contract and can fail privately.
const CURL_FIXTURE: &str = r#"#!/bin/sh
printf 'curl:%s\n' "$*" >> "$INSTALLER_TRACE"
if [ "$#" -ne 13 ] || [ "$1" != --fail ] || [ "$2" != --silent ] || \
   [ "$3" != --show-error ] || [ "$4" != --location ] || \
   [ "$5" != --connect-timeout ] || [ "$6" != 2 ] || \
   [ "$7" != --speed-limit ] || [ "$8" != 1 ] || \
   [ "$9" != --speed-time ] || [ "${10}" != 2 ] || \
   [ "${11}" != --output ] || [ "${12##*/}" != "$INSTALLER_ARCHIVE_NAME" ] || \
   [ "${13}" != "$INSTALLER_EXPECTED_URL" ]; then
  printf '%s\n' 'unexpected curl invocation' >&2
  exit 97
fi
case "${12}" in
  "$TMPDIR"/*/"$INSTALLER_ARCHIVE_NAME") ;;
  *) printf '%s\n' 'unexpected curl output path' >&2; exit 97 ;;
esac
if [ "$INSTALLER_FAILURE" = curl ]; then
  printf '%s\n' 'fixture-curl-secret' >&2
  exit 22
fi
printf '%s\n' 'fixture mold archive' > "${12}"
"#;

/// A tar replacement that validates extraction arguments and supports failure.
const TAR_FIXTURE: &str = r#"#!/bin/sh
printf 'tar:%s\n' "$*" >> "$INSTALLER_TRACE"
if [ "$#" -ne 7 ] || [ "$1" != --extract ] || [ "$2" != --gzip ] || \
   [ "$3" != --strip-components=1 ] || [ "$4" != --directory ] || \
   [ "$5" != "$INSTALLER_EXPECTED_PREFIX" ] || [ "$6" != --file ] || \
   [ "${7##*/}" != "$INSTALLER_ARCHIVE_NAME" ]; then
  printf '%s\n' 'unexpected tar invocation' >&2
  exit 97
fi
case "$7" in
  "$TMPDIR"/*/"$INSTALLER_ARCHIVE_NAME") ;;
  *) printf '%s\n' 'unexpected tar archive path' >&2; exit 97 ;;
esac
if [ "$INSTALLER_FAILURE" = tar ]; then
  printf '%s\n' 'fixture tar failure' >&2
  exit 19
fi
exit 0
"#;

/// A rustup replacement that returns host metadata and validates installation.
const RUSTUP_FIXTURE: &str = r#"#!/bin/sh
if [ "$#" -eq 1 ] && [ "$1" = show ]; then
  printf '%s\n' rustup-show >> "$INSTALLER_TRACE"
  printf '%s\n' 'Default host: x86_64-unknown-linux-gnu'
  exit 0
fi
if [ "$1" = toolchain ] && [ "$2" = install ]; then
  printf 'rustup-install:%s\n' "$*" >> "$INSTALLER_TRACE"
  if [ "$#" -ne 9 ] || [ "$3" != nightly-2030-01-01 ] || \
     [ "$4" != --profile ] || [ "$5" != minimal ] || \
     [ "$6" != --component ] || [ "$7" != clippy ] || \
     [ "$8" != --component ] || [ "$9" != rustfmt ]; then
    printf '%s\n' 'unexpected rustup installation arguments' >&2
    exit 97
  fi
  if [ "$INSTALLER_FAILURE" = rustup ]; then
    printf '%s\n' 'fixture rustup failure' >&2
    exit 18
  fi
  exit 0
fi
printf '%s\n' 'unexpected rustup invocation' >&2
exit 97
"#;

/// A uname replacement that supplies controlled host triples to the installer.
const UNAME_FIXTURE: &str = r#"#!/bin/sh
case "$1" in
  -s)
    printf '%s\n' uname-system >> "$INSTALLER_TRACE"
    printf '%s\n' "$INSTALLER_UNAME_SYSTEM"
    ;;
  -m)
    printf '%s\n' uname-machine >> "$INSTALLER_TRACE"
    printf '%s\n' "$INSTALLER_UNAME_MACHINE"
    ;;
  -sm)
    printf '%s\n' uname-system-machine >> "$INSTALLER_TRACE"
    printf '%s %s\n' "$INSTALLER_UNAME_SYSTEM" "$INSTALLER_UNAME_MACHINE"
    ;;
  *)
    printf '%s\n' 'unexpected uname invocation' >&2
    exit 97
    ;;
esac
"#;
