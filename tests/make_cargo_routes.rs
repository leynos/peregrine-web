//! Behavioural contracts for Make's Cargo substitution and build preflight.

#[path = "support/make_target_option_tests.rs"]
mod target_options;

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

/// Owns a fake Cargo executable and records only its build invocations.
struct CargoProbe {
    parent: Dir,
    directory: Dir,
    name: String,
    path: Utf8PathBuf,
}

/// The independent Make branches exercised by one fake-Cargo invocation.
#[derive(Clone, Copy)]
struct MakeOptions<'a> {
    nextest_available: bool,
    dry_run: bool,
    with_act: bool,
    check: &'a str,
}

impl CargoProbe {
    fn new() -> Read<Self> {
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
        directory.write(
            "probe-cargo",
            "#!/bin/sh\nif [ \"$1\" = nextest ] && [ \"$2\" = --version ]; then\n  if [ \
             \"${NEXTEST_AVAILABLE:-0}\" = 0 ]; then printf '%s\\n' 'cargo-nextest 0.9.0'; exit \
             0; fi\n  exit 1\nfi\nprintf '%s\\n' \"$*\" >> \"$CARGO_LOG\"\nif [ -n \
             \"${PRODUCTION_LOG:-}\" ]; then env | sort > \"$PRODUCTION_LOG\"; fi\n",
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

    fn make(&self, target: &str, options: MakeOptions<'_>) -> io::Result<std::process::Output> {
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
            .env(
                "NEXTEST_AVAILABLE",
                if options.nextest_available { "0" } else { "1" },
            )
            .env("CARGO_LOG", self.path.join("cargo.log").as_str())
            .output()
    }
}

impl Drop for CargoProbe {
    fn drop(&mut self) { drop(self.parent.remove_dir_all(&self.name)); }
}

/// Read executed Make commands, leaving shell probe calls out of the output.
fn dry_run(probe: &CargoProbe, target: &str, nextest: bool, with_act: bool) -> Read<String> {
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
fn has_bare_cargo(commands: &str) -> bool {
    commands.lines().any(|line| {
        let trimmed = line.trim_start();
        !trimmed.starts_with("printf ")
            && !trimmed.starts_with("echo ")
            && (trimmed.starts_with("cargo ") || trimmed.contains(" cargo "))
    })
}

#[test]
fn every_make_cargo_route_uses_the_selected_executable() {
    let probe = CargoProbe::new().expect("create an isolated fake Cargo executable");
    for (target, calls) in [
        ("build", &["build"][..]),
        ("release", &["build"][..]),
        ("package", &["package"][..]),
        ("lint", &["doc", "clippy"][..]),
        ("typecheck", &["check"][..]),
        ("coverage", &["llvm-cov"][..]),
        ("check-fmt", &["fmt"][..]),
        ("fmt", &["fmt"][..]),
        ("clean", &["clean"][..]),
        ("rust-audit", &["metadata", "audit"][..]),
    ] {
        let commands =
            dry_run(&probe, target, true, false).expect("inspect Make's selected command");
        for call in calls {
            assert!(
                commands.contains(&format!("probe-cargo {call}")),
                "make {target} must use the selected Cargo for {call}: {commands}"
            );
        }
        assert!(
            !has_bare_cargo(&commands),
            "make {target} must not use bare Cargo: {commands}"
        );
    }
}

/// Production routes must replace even hostile inherited development settings.
#[test]
fn release_and_package_select_llvm_without_development_flags() {
    let probe = CargoProbe::new().expect("create an isolated fake Cargo executable");
    for (target, expected_command) in [
        ("release", "build --release"),
        ("package", "package --locked"),
    ] {
        let output = Command::new("make")
            .arg(target)
            .arg(format!("CARGO={}", probe.path.join("probe-cargo")))
            .current_dir(Utf8Path::new(env!("CARGO_MANIFEST_DIR")))
            .env("CARGO_LOG", probe.path.join("cargo.log").as_str())
            .env("PRODUCTION_LOG", probe.path.join("production.log").as_str())
            .env("RUSTFLAGS", "-Zthreads=8 -C link-arg=-fuse-ld=mold")
            .env("CARGO_ENCODED_RUSTFLAGS", "-Zthreads=8")
            .env("CARGO_PROFILE_DEV_CODEGEN_BACKEND", "cranelift")
            .env("CARGO_PROFILE_RELEASE_CODEGEN_BACKEND", "cranelift")
            .env(
                "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
                "other-linker",
            )
            .output()
            .expect("execute a production Make target with fake Cargo");
        assert!(
            output.status.success(),
            "make {target} must reach fake Cargo: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let calls = probe
            .directory
            .read_to_string("cargo.log")
            .expect("read the selected Cargo command");
        assert!(
            calls.contains(expected_command),
            "make {target} must execute {expected_command}: {calls}"
        );
        let environment = probe
            .directory
            .read_to_string("production.log")
            .expect("read the production Cargo environment");
        for required in [
            "CARGO_PROFILE_DEV_CODEGEN_BACKEND=llvm",
            "CARGO_PROFILE_RELEASE_CODEGEN_BACKEND=llvm",
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=clang",
        ] {
            assert!(
                environment.lines().any(|line| line == required),
                "make {target} must select {required}: {environment}"
            );
        }
        let rustflags = environment
            .lines()
            .find(|line| line.starts_with("RUSTFLAGS="))
            .expect("production Cargo must receive explicit RUSTFLAGS");
        assert!(
            rustflags.contains("-D warnings")
                && !rustflags.contains("-Zthreads=8")
                && !rustflags.contains("-fuse-ld=mold"),
            "make {target} must replace inherited development flags: {rustflags}"
        );
        assert!(
            !environment.contains("CARGO_ENCODED_RUSTFLAGS="),
            "make {target} must remove the encoded flags override: {environment}"
        );
        probe
            .directory
            .remove_file("cargo.log")
            .expect("reset the Cargo command log");
    }
}

#[test]
fn nextest_fallback_doctests_and_act_are_independent_routes() {
    let probe = CargoProbe::new().expect("create an isolated fake Cargo executable");
    for (nextest, selected, omitted) in [
        (
            true,
            "probe-cargo nextest run",
            "probe-cargo test --all-targets",
        ),
        (
            false,
            "probe-cargo test --all-targets",
            "probe-cargo nextest run",
        ),
    ] {
        let commands = dry_run(&probe, "test", nextest, false).expect("inspect Make's test route");
        assert!(
            commands.contains(selected),
            "selected test runner must appear: {commands}"
        );
        assert!(
            !commands.contains(omitted),
            "unselected test runner must stay out: {commands}"
        );
        assert!(
            commands.contains("probe-cargo test --doc"),
            "doctests must run separately: {commands}"
        );
        assert!(
            !has_bare_cargo(&commands),
            "test route must use selected Cargo: {commands}"
        );
    }
    let ordinary = dry_run(&probe, "test", true, false).expect("inspect ordinary tests");
    let nested = dry_run(&probe, "test", true, true).expect("inspect Act-enabled tests");
    assert!(
        !ordinary.contains("--env ACT=true"),
        "ordinary tests must not start Act: {ordinary}"
    );
    assert!(
        nested.contains("--env ACT=true"),
        "WITH_ACT=1 must reach nested Act: {nested}"
    );
}

#[test]
fn pinned_tool_preflight_precedes_development_cargo() {
    let probe = CargoProbe::new().expect("create an isolated fake Cargo executable");
    for target in ["build", "test", "lint", "typecheck", "coverage"] {
        let output = probe
            .make(
                target,
                MakeOptions {
                    nextest_available: true,
                    dry_run: false,
                    with_act: false,
                    check: probe.path.join("preflight").as_str(),
                },
            )
            .expect("execute a Make target with fake tools");
        assert!(
            output.status.success(),
            "make {target} must accept the fake tools: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let log = probe
            .directory
            .read_to_string("cargo.log")
            .expect("read executed tool order");
        let calls: Vec<_> = log.lines().collect();
        assert_eq!(
            calls.first().copied(),
            Some("preflight"),
            "make {target} must check tools before compilation: {log}"
        );
        assert!(
            calls.len() > 1,
            "make {target} must reach Cargo after preflight: {log}"
        );
        probe
            .directory
            .remove_file("cargo.log")
            .expect("reset the command log");
    }
}

#[test]
fn failed_preflight_stops_cargo_execution() {
    let probe = CargoProbe::new().expect("create an isolated fake Cargo executable");
    let output = probe
        .make(
            "test",
            MakeOptions {
                nextest_available: true,
                dry_run: false,
                with_act: false,
                check: "false",
            },
        )
        .expect("run Make with an injected failing prerequisite");
    assert!(
        !output.status.success(),
        "a failed prerequisite must fail make test"
    );
    assert!(
        probe.directory.read_to_string("cargo.log").is_err(),
        "no Cargo compile or test command may run after the prerequisite fails"
    );
    assert!(
        has_bare_cargo("RUSTFLAGS=\"-D warnings\" cargo test"),
        "the substitution check must reject a hard-coded cargo command"
    );
}

#[test]
fn unsupported_make_target_override_fails_before_compilation() {
    let probe = CargoProbe::new().expect("create an isolated fake Cargo executable");
    let output = Command::new("make")
        .args([
            "test",
            "CHECK_BUILD_TOOLS=true",
            "TEST_FLAGS=--target=aarch64-unknown-linux-gnu",
        ])
        .arg(format!("CARGO={}", probe.path.join("probe-cargo")))
        .current_dir(Utf8Path::new(env!("CARGO_MANIFEST_DIR")))
        .env("CARGO_LOG", probe.path.join("cargo.log").as_str())
        .output()
        .expect("run Make with an unsupported cross target");
    assert!(
        !output.status.success(),
        "Make must reject the unsupported target"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("outside the supported native Make build"),
        "the target rejection must identify the native-only boundary"
    );
    assert!(
        probe.directory.read_to_string("cargo.log").is_err(),
        "unsupported target must stop compile and test commands"
    );
}
