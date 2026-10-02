//! Contract tests for the Rust build standard.
//!
//! The standard makes the parallel `rustc` frontend the default for every
//! development build and mold the default linker for `x86_64` GNU Linux. Cargo reads both
//! from `.cargo/config.toml`, but it applies a single `rustflags` source rather
//! than merging them, and an assigned `RUSTFLAGS` replaces every source. So the
//! flags must be repeated in each configuration source, restated wherever the
//! Makefile assigns `RUSTFLAGS` for a development target, and kept out of the
//! coverage, release, and packaging recipes, which measure or ship and so stay on the
//! default flags.
//!
//! The Makefile clauses run `make -n` and read the commands it would run,
//! rather than the Makefile's text, so a flag lost through a variable or a
//! recipe edit fails here. Each assigned value is expanded by the shell, with
//! and without an inherited `RUSTFLAGS`, exactly as the recipe would expand
//! it. The clauses run once as a Linux host and once as a macOS host, because
//! mold is added on Linux alone. File access goes through a
//! `cap_std` directory handle rooted at the crate manifest directory.

#[path = "support/build_standard_contract.rs"]
mod support;

use std::process::Command;

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};
use support::{
    ASSIGNING_TARGETS,
    HELD_OUT_TARGETS,
    Host,
    INHERITED,
    LINUX_TARGET,
    MOLD_FLAG,
    THREADS_FLAG,
    check_development_targets,
    make_rustflags,
    make_whitaker_recipe,
    sources,
};

#[test]
fn every_rustflags_source_carries_the_parallel_frontend() {
    let found = sources().expect("read the configuration sources");
    assert!(
        found.iter().any(|(key, _)| key == "build"),
        "no [build] rustflags for non-Linux hosts"
    );
    let missing: Vec<&str> = found
        .iter()
        .filter(|(_, flags)| !flags.names(THREADS_FLAG))
        .map(|(key, _)| key.as_str())
        .collect();
    assert!(
        missing.is_empty(),
        "{THREADS_FLAG} missing from {missing:?}"
    );
}

#[test]
fn mold_is_confined_to_the_supported_linux_target() {
    let found = sources().expect("read the configuration sources");
    let linux: Vec<_> = found
        .iter()
        .filter(|(key, _)| key.as_str() == LINUX_TARGET)
        .collect();
    assert!(!linux.is_empty(), "no Linux target table carries rustflags");
    assert!(
        linux.iter().all(|(_, flags)| flags.names(MOLD_FLAG)),
        "a Linux table lost mold"
    );
    let wider: Vec<&str> = found
        .iter()
        .filter(|(key, flags)| key.as_str() != LINUX_TARGET && flags.names(MOLD_FLAG))
        .map(|(key, _)| key.as_str())
        .collect();
    assert!(wider.is_empty(), "mold named beyond Linux in {wider:?}");
}

/// Bare Cargo must reach the native wrapper that controls Clang's linker search.
#[test]
fn native_cargo_target_selects_the_pinned_clang_wrapper() {
    let root = Dir::open_ambient_dir(
        Utf8Path::new(env!("CARGO_MANIFEST_DIR")),
        ambient_authority(),
    )
    .expect("open the repository root through a capability");
    let source = root
        .read_to_string(".cargo/config.toml")
        .expect("read the Cargo build defaults");
    let config: toml::Value = toml::from_str(&source).expect("parse Cargo's native target table");
    let linker = config
        .get("target")
        .and_then(|targets| targets.get(LINUX_TARGET))
        .and_then(|target| target.get("linker"))
        .and_then(toml::Value::as_str);
    assert_eq!(
        linker,
        Some("scripts/native-clang-linker.sh"),
        "bare Cargo must invoke the wrapper that puts pinned mold first in Clang's search"
    );
}

#[test]
fn sources_differ_only_by_the_linker() {
    let mut stripped: Vec<Vec<String>> = sources()
        .expect("read the configuration sources")
        .into_iter()
        .map(|(_, flags)| flags.without_mold())
        .collect();
    stripped.dedup();
    assert_eq!(
        stripped.len(),
        1,
        "rustflags sources disagree: {stripped:?}"
    );
}

#[test]
fn development_targets_restate_both_flags_on_linux() {
    let problems = check_development_targets(Host::Linux, None).expect("read `make -n` output");
    assert!(problems.is_empty(), "{problems:#?}");
    for target in ASSIGNING_TARGETS {
        let assigned = make_rustflags(target, Host::Linux, None)
            .expect("read `make -n` output")
            .into_iter()
            .flatten()
            .count();
        assert!(assigned > 0, "`make {target}` assigns no RUSTFLAGS");
    }
}

#[test]
fn development_targets_keep_the_standard_under_inherited_rustflags() {
    for host in [Host::Linux, Host::Darwin] {
        let problems =
            check_development_targets(host, Some(INHERITED)).expect("read `make -n` output");
        assert!(
            problems.is_empty(),
            "`make` development routes on {host:?} must retain both inherited and standard flags: \
             {problems:#?}"
        );
    }
}

#[test]
fn development_targets_keep_the_frontend_but_not_mold_elsewhere() {
    let problems = check_development_targets(Host::Darwin, None).expect("read `make -n` output");
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn whitaker_clears_driver_overrides_and_checks_with_repository_defaults() {
    let recipe = make_whitaker_recipe(Host::Linux).expect("read `make -n lint-whitaker` output");
    let check = recipe
        .lines()
        .find(|line| line.contains("check-build-tools.sh"))
        .expect("Whitaker must preflight the development build tools");
    let whitaker = recipe
        .lines()
        .find(|line| line.contains(" --all -- "))
        .expect("Make must invoke the Whitaker workspace check");

    for setting in [
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_PROFILE_DEV_CODEGEN_BACKEND",
        "CARGO_BUILD_TARGET",
        "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
        "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS",
        "CFLAGS",
        "LDFLAGS",
    ] {
        assert!(
            whitaker.contains(&format!("-u {setting}")),
            "Whitaker must remove inherited {setting}: {whitaker}"
        );
        assert!(
            check.contains(&format!("-u {setting}")),
            "Whitaker's preflight must inspect the same clean route for {setting}: {check}"
        );
    }
    assert!(
        !whitaker.contains(" RUSTFLAGS=\"") && !whitaker.contains("=llvm"),
        "Whitaker must not replace Cargo defaults with empty flags or LLVM: {whitaker}"
    );
    assert!(
        whitaker.contains("DYLINT_RUSTFLAGS=\"-D warnings"),
        "warnings-as-errors must reach the repository check through Dylint's supported input: \
         {whitaker}"
    );
    assert!(
        whitaker.ends_with("--all -- --all-targets --all-features"),
        "the repository check must retain all targets and features: {whitaker}"
    );
}

#[test]
fn make_does_not_inject_mold_for_a_musl_host() {
    let output = Command::new("make")
        .args([
            "-n",
            "-B",
            "build",
            "BUILD_HOST_OS=Linux",
            "BUILD_HOST_ARCH=x86_64",
            "BUILD_HOST_TRIPLE=x86_64-unknown-linux-musl",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("inspect a musl-host Make build");
    assert!(
        output.status.success(),
        "the musl route must be inspectable"
    );
    let recipe = String::from_utf8_lossy(&output.stdout);
    assert!(
        !recipe.contains("-fuse-ld=mold"),
        "Make must not inject the GNU-only mold route on musl: {recipe}"
    );
}

/// Coverage measures and release/packaging ship, so they stay on production flags.
/// Every command must assign `RUSTFLAGS`, since only an assignment displaces
/// the configuration's sources.
#[test]
fn coverage_release_and_package_take_neither_flag() {
    for target in HELD_OUT_TARGETS {
        for assigned in make_rustflags(target, Host::Linux, None).expect("read `make -n` output") {
            let flags = assigned.unwrap_or_else(|| {
                panic!("`make {target}` runs a command that takes the configuration's flags")
            });
            assert!(
                !flags.names(THREADS_FLAG),
                "`make {target}` takes {THREADS_FLAG}"
            );
            assert!(!flags.names(MOLD_FLAG), "`make {target}` takes {MOLD_FLAG}");
        }
    }
}
