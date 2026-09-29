//! Executable contracts for sequential Make gates and Whitaker's flag boundary.

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

type Read<T> = Result<T, Box<dyn Error>>;
static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

/// Private fake tools write their observed order without compiling the crate.
struct GateProbe {
    parent: Dir,
    directory: Dir,
    name: String,
    path: Utf8PathBuf,
}

impl GateProbe {
    fn new() -> Read<Self> {
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
        Self::script(
            &directory,
            "probe-cargo",
            concat!(
                "#!/bin/sh\n",
                "if [ \"$1\" = nextest ] && [ \"$2\" = --version ]; then\n",
                "  printf '%s\\n' 'cargo-nextest 0.9.0'; exit 0\n",
                "fi\n",
                "stage=$1\n",
                "if [ \"$1\" = test ] && [ \"$2\" = --doc ]; then stage=doctest; fi\n",
                "printf '%s\\n' \"$stage\" >> \"$GATE_LOG\"\n",
                "[ \"${GATE_FAIL_AT:-}\" != \"$stage\" ]\n",
            ),
        )?;
        Self::script(
            &directory,
            "probe-check",
            "#!/bin/sh\nprintf '%s\\n' preflight >> \"$GATE_LOG\"\n[ \"${GATE_FAIL_AT:-}\" != \
             preflight ]\n",
        )?;
        Self::script(
            &directory,
            "probe-whitaker",
            concat!(
                "#!/bin/sh\n",
                "printf 'RUSTFLAGS=%s\\nENCODED=%s\\nBACKEND=%s\\n' ",
                "\"${RUSTFLAGS-<unset>}\" \"${CARGO_ENCODED_RUSTFLAGS-<unset>}\" ",
                "\"${CARGO_PROFILE_DEV_CODEGEN_BACKEND-<unset>}\" > \"$WHITAKER_ENV_LOG\"\n",
                "printf '%s\\n' whitaker >> \"$GATE_LOG\"\n",
                "[ \"${GATE_FAIL_AT:-}\" != whitaker ]\n",
            ),
        )?;
        Self::script(
            &directory,
            "probe-spelling",
            "#!/bin/sh\nprintf '%s\\n' spelling >> \"$GATE_LOG\"\n[ \"${GATE_FAIL_AT:-}\" != \
             spelling ]\n",
        )?;
        Self::script(
            &directory,
            "probe-act",
            "#!/bin/sh\nprintf '%s\\n' act >> \"$GATE_LOG\"\nexit 1\n",
        )?;
        let path = parent_path.join(&name);
        Ok(Self {
            parent,
            directory,
            name,
            path,
        })
    }

    fn script(directory: &Dir, name: &str, source: &str) -> Read<()> {
        directory.write(name, source)?;
        directory.set_permissions(name, Permissions::from_mode(0o700))?;
        Ok(())
    }

    fn run(&self, target: &str, failure: Option<&str>) -> io::Result<std::process::Output> {
        let mut command = Command::new("make");
        command
            .args(["-j", "4", target])
            .arg(format!("CARGO={}", self.path.join("probe-cargo")))
            .arg(format!(
                "CHECK_BUILD_TOOLS={}",
                self.path.join("probe-check")
            ))
            .arg(format!("WHITAKER={}", self.path.join("probe-whitaker")))
            .arg(format!(
                "TYPOS_CONFIG_BUILDER={}",
                self.path.join("probe-spelling")
            ))
            .arg(format!("ACT={}", self.path.join("probe-act")))
            .arg("WITH_ACT=0")
            .current_dir(Utf8Path::new(env!("CARGO_MANIFEST_DIR")))
            .env("GATE_LOG", self.path.join("gate.log").as_str())
            .env(
                "WHITAKER_ENV_LOG",
                self.path.join("whitaker-env.log").as_str(),
            )
            .env("RUSTFLAGS", "-Zthreads=8 -C link-arg=-fuse-ld=mold")
            .env("CARGO_ENCODED_RUSTFLAGS", "-Zthreads=8");
        if let Some(stage) = failure {
            command.env("GATE_FAIL_AT", stage);
        }
        command.output()
    }

    fn stages(&self) -> Read<Vec<String>> {
        Ok(self
            .directory
            .read_to_string("gate.log")?
            .lines()
            .map(str::to_owned)
            .collect())
    }
}

impl Drop for GateProbe {
    fn drop(&mut self) { drop(self.parent.remove_dir_all(&self.name)); }
}

#[test]
fn parallel_all_runs_each_gate_in_order() {
    let probe = GateProbe::new().expect("create private gate probes");
    let output = probe
        .run("all", None)
        .expect("execute parallel Make composite");
    assert_eq!(
        probe.stages().expect("read gate order"),
        [
            "fmt",
            "preflight",
            "doc",
            "clippy",
            "preflight",
            "whitaker",
            "preflight",
            "nextest",
            "doctest",
            "spelling"
        ],
        "make -j all must finish each gate before starting the next"
    );
    assert!(
        output.status.success(),
        "all gates must pass: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn inherited_with_act_does_not_reach_nested_make_all() {
    let test_binary = std::env::current_exe().expect("find current test executable");
    let output = Command::new(test_binary)
        .args([
            "--exact",
            "parallel_all_runs_each_gate_in_order",
            "--nocapture",
        ])
        .env("WITH_ACT", "1")
        .output()
        .expect("run exact gate-order test with Act enabled in its environment");
    assert!(
        output.status.success(),
        "the gate-order test must clear inherited WITH_ACT before nested Make: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn parallel_all_stops_after_the_first_failing_gate() {
    for (failure, expected) in [
        ("fmt", &["fmt"][..]),
        ("clippy", &["fmt", "preflight", "doc", "clippy"][..]),
        (
            "whitaker",
            &["fmt", "preflight", "doc", "clippy", "preflight", "whitaker"][..],
        ),
        (
            "nextest",
            &[
                "fmt",
                "preflight",
                "doc",
                "clippy",
                "preflight",
                "whitaker",
                "preflight",
                "nextest",
            ][..],
        ),
        (
            "doctest",
            &[
                "fmt",
                "preflight",
                "doc",
                "clippy",
                "preflight",
                "whitaker",
                "preflight",
                "nextest",
                "doctest",
            ][..],
        ),
        (
            "spelling",
            &[
                "fmt",
                "preflight",
                "doc",
                "clippy",
                "preflight",
                "whitaker",
                "preflight",
                "nextest",
                "doctest",
                "spelling",
            ][..],
        ),
    ] {
        let probe = GateProbe::new().expect("create private gate probes");
        let output = probe
            .run("all", Some(failure))
            .expect("execute a failing parallel composite");
        assert!(
            !output.status.success(),
            "a failing {failure} gate must stop make all"
        );
        assert_eq!(
            probe.stages().expect("read gate order"),
            expected,
            "nothing may run after {failure}"
        );
    }
}

#[test]
fn parallel_lint_is_sequential_and_whitaker_clears_inherited_flags() {
    let probe = GateProbe::new().expect("create private gate probes");
    let output = probe.run("lint", None).expect("execute parallel Make lint");
    assert!(
        output.status.success(),
        "lint stages must pass: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        probe.stages().expect("read lint order"),
        ["preflight", "doc", "clippy", "preflight", "whitaker"],
        "make -j lint must finish Clippy before Whitaker"
    );
    assert_eq!(
        probe
            .directory
            .read_to_string("whitaker-env.log")
            .expect("read Whitaker environment"),
        "RUSTFLAGS=\nENCODED=<unset>\nBACKEND=llvm\n",
        "Whitaker must not inherit development or encoded flags and must use LLVM"
    );
}

#[test]
fn parallel_lint_never_starts_whitaker_after_clippy_failure() {
    let probe = GateProbe::new().expect("create private gate probes");
    let output = probe
        .run("lint", Some("clippy"))
        .expect("execute failing Make lint");
    assert!(
        !output.status.success(),
        "Clippy failure must fail make lint"
    );
    assert_eq!(
        probe.stages().expect("read lint order"),
        ["preflight", "doc", "clippy"],
        "Whitaker must not start after Clippy fails"
    );
    assert!(
        probe.directory.read_to_string("whitaker-env.log").is_err(),
        "Whitaker must not run after Clippy fails"
    );
}
