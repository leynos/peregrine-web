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

use std::{error::Error, process::Command};

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};

/// The parallel-frontend flag every `rustflags` source must carry.
const THREADS_FLAG: &str = "-Zthreads=8";

/// The linker flag the Linux source must add, normalized to one token.
const MOLD_FLAG: &str = "-Clink-arg=-fuse-ld=mold";

/// The one supported Linux target table; broad predicates would select cross-targets.
const LINUX_TARGET: &str = "x86_64-unknown-linux-gnu";

/// Makefile targets that build for development. A command in one either
/// assigns `RUSTFLAGS` with the standard flags or assigns none and so takes
/// the configuration's.
const DEVELOPMENT_TARGETS: [&str; 4] = ["test", "typecheck", "lint", "build"];

/// Makefile targets that measure or ship, so every command assigns
/// `RUSTFLAGS` and none carries a standard flag.
const HELD_OUT_TARGETS: [&str; 3] = ["coverage", "release", "package"];

/// Development targets that must assign `RUSTFLAGS` in at least one command,
/// so the restatement checks above cannot pass by finding nothing to check.
const ASSIGNING_TARGETS: [&str; 4] = ["test", "typecheck", "lint", "build"];

/// A caller's own flags, distinct from anything a recipe adds, to prove a
/// recipe composes an exported `RUSTFLAGS` with the standard flags rather than
/// replacing either.
const INHERITED: &str = "--cfg inherited_from_caller";

/// The result of a reader, which the tests unwrap.
type Read<T> = Result<T, Box<dyn Error>>;

/// A host used to read Make's platform-specific development recipe.
#[derive(Clone, Copy, Debug)]
enum Host {
    Linux,
    Darwin,
}

impl Host {
    const fn uname(self) -> &'static str {
        match self {
            Self::Linux => "Linux",
            Self::Darwin => "Darwin",
        }
    }

    const fn expects_mold(self) -> bool { matches!(self, Self::Linux) }
}

/// Normalized `rustflags`; `-C value` and `-Cvalue` compare equally.
#[derive(Debug)]
struct Flags(Vec<String>);

impl Flags {
    fn from_words<S: AsRef<str>>(words: &[S]) -> Self {
        let mut joined: Vec<String> = Vec::new();
        for flag in words.iter().map(AsRef::as_ref) {
            match joined.last_mut() {
                Some(last) if last == "-C" => *last = format!("-C{flag}"),
                _ => joined.push(flag.to_owned()),
            }
        }
        Self(joined)
    }

    fn names(&self, flag: &str) -> bool { self.0.iter().any(|candidate| candidate == flag) }

    fn carries_run(&self, caller: &str) -> bool {
        let wanted: Vec<&str> = caller.split_whitespace().collect();
        self.0
            .windows(wanted.len())
            .any(|run| run.iter().map(String::as_str).eq(wanted.iter().copied()))
    }

    fn without_mold(self) -> Vec<String> {
        self.0
            .into_iter()
            .filter(|flag| flag != MOLD_FLAG)
            .collect()
    }
}

/// Reads one table's `rustflags` as a list of strings, if it has one.
fn table_flags(table: &toml::Value) -> Option<Flags> {
    let flags = table.get("rustflags")?.as_array()?;
    let words: Vec<&str> = flags.iter().filter_map(toml::Value::as_str).collect();
    Some(Flags::from_words(&words))
}

/// Returns every `rustflags` source in the configuration, by table name.
fn sources() -> Read<Vec<(String, Flags)>> {
    let root = Dir::open_ambient_dir(
        Utf8Path::new(env!("CARGO_MANIFEST_DIR")),
        ambient_authority(),
    )?;
    let text = root.read_to_string(".cargo/config.toml")?;
    let config: toml::Value = toml::from_str(&text)?;
    let mut found = Vec::new();
    if let Some(flags) = config.get("build").and_then(table_flags) {
        found.push(("build".to_owned(), flags));
    }
    if let Some(targets) = config.get("target").and_then(toml::Value::as_table) {
        for (key, table) in targets {
            if let Some(flags) = table_flags(table) {
                found.push((key.clone(), flags));
            }
        }
    }
    Ok(found)
}

/// Expands an assigned value as the recipe's shell would, with or without an
/// inherited `RUSTFLAGS`, and splits it into normalized flags.
fn expanded(value: &str, inherited: Option<&str>) -> Read<Flags> {
    let mut shell = Command::new("bash");
    shell
        .args(["-c", &format!("printf '%s' \"{value}\"")])
        .env_remove("RUSTFLAGS");
    if let Some(flags) = inherited {
        shell.env("RUSTFLAGS", flags);
    }
    let output = shell.output()?;
    if !output.status.success() {
        return Err(format!("the shell could not expand `{value}`").into());
    }
    let words: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    Ok(Flags::from_words(&words))
}

/// Returns Make's commands, joining backslash-continued recipe lines.
fn dry_run(target: &str, host: Host, inherited: Option<&str>) -> Read<String> {
    let mut make = Command::new("make");
    make.args([
        "-n",
        "-B",
        &format!("BUILD_HOST_OS={}", host.uname()),
        target,
    ])
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .env_remove("RUSTFLAGS");
    if let Some(flags) = inherited {
        make.env("RUSTFLAGS", flags);
    }
    let output = make.output()?;
    if !output.status.success() {
        return Err(format!("`make -n {target}` failed").into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).replace("\\\n", " "))
}

/// Reads one command's assignment, rejecting unrecognized spellings.
fn assignment(line: &str, inherited: Option<&str>) -> Read<Option<Flags>> {
    let Some((_, rest)) = line.split_once("RUSTFLAGS=\"") else {
        return if line.contains("RUSTFLAGS=") {
            Err(format!("unreadable RUSTFLAGS assignment in `{line}`").into())
        } else {
            Ok(None)
        };
    };
    let (value, _) = rest
        .split_once('"')
        .ok_or_else(|| format!("unterminated RUSTFLAGS in `{line}`"))?;
    expanded(value, inherited).map(Some)
}

/// Returns each Cargo command's assigned flags, or `None` for Cargo's config.
/// Whitaker deliberately runs with empty flags and LLVM outside this route.
fn make_rustflags(target: &str, host: Host, inherited: Option<&str>) -> Read<Vec<Option<Flags>>> {
    let commands = dry_run(target, host, inherited)?
        .lines()
        .filter(|line| line.contains("cargo") && !line.contains("whitaker"))
        .map(|line| assignment(line, inherited))
        .collect::<Read<Vec<_>>>()?;
    if commands.is_empty() {
        return Err(format!("`make -n {target}` runs no cargo command").into());
    }
    Ok(commands)
}

/// Describes missing standard flags in one assigned development command.
fn assigned_flag_problems(
    target: &str,
    host: Host,
    inherited: Option<&str>,
    flags: &Flags,
) -> Vec<String> {
    let mut problems = Vec::new();
    if !flags.names(THREADS_FLAG) {
        problems.push(format!("`make {target}` on {host:?} drops {THREADS_FLAG}: {flags:?}"));
    }
    if flags.names(MOLD_FLAG) != host.expects_mold() {
        problems.push(format!("`make {target}` on {host:?} gets mold wrong: {flags:?}"));
    }
    if inherited.is_some_and(|caller| !flags.carries_run(caller)) {
        problems.push(format!("`make {target}` drops the caller's RUSTFLAGS: {flags:?}"));
    }
    problems
}

/// Checks every supported development target, including the direct build.
fn check_development_targets(host: Host, inherited: Option<&str>) -> Read<Vec<String>> {
    let mut problems = Vec::new();
    for target in DEVELOPMENT_TARGETS {
        let commands = make_rustflags(target, host, inherited)?;
        if inherited.is_some() && commands.iter().any(Option::is_none) {
            problems.push(format!(
                "`make {target}` runs a command that takes only the caller's RUSTFLAGS"
            ));
        }
        // An empty assignment is `Some(Flags(vec![]))` and is checked like any other.
        for flags in commands.into_iter().flatten() {
            problems.extend(assigned_flag_problems(target, host, inherited, &flags));
        }
    }
    Ok(problems)
}

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
