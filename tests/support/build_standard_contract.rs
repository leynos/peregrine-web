//! Probe Cargo configuration and Make recipes for the Rust build contract.

use std::{error::Error, process::Command};

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};

/// The parallel-frontend flag every `rustflags` source must carry.
pub(super) const THREADS_FLAG: &str = "-Zthreads=8";

/// The linker flag the Linux source must add, normalized to one token.
pub(super) const MOLD_FLAG: &str = "-Clink-arg=-fuse-ld=mold";

/// The one supported Linux target table; broad predicates would select cross-targets.
pub(super) const LINUX_TARGET: &str = "x86_64-unknown-linux-gnu";

/// Makefile targets that build for development. A command in one either
/// assigns `RUSTFLAGS` with the standard flags or assigns none and so takes
/// the configuration's.
const DEVELOPMENT_TARGETS: [&str; 5] = ["test", "typecheck", "lint", "build", "act-contract-smoke"];

/// Makefile targets that measure or ship, so every command assigns
/// `RUSTFLAGS` and none carries a standard flag.
pub(super) const HELD_OUT_TARGETS: [&str; 3] = ["coverage", "release", "package"];

/// Development targets that must assign `RUSTFLAGS` in at least one command,
/// so the restatement checks above cannot pass by finding nothing to check.
pub(super) const ASSIGNING_TARGETS: [&str; 5] =
    ["test", "typecheck", "lint", "build", "act-contract-smoke"];

/// A caller's own flags, distinct from anything a recipe adds, to prove a
/// recipe composes an exported `RUSTFLAGS` with the standard flags rather than
/// replacing either.
pub(super) const INHERITED: &str = "--cfg inherited_from_caller";

/// The result of a reader, which the tests unwrap.
pub(super) type Read<T> = Result<T, Box<dyn Error>>;

/// A host used to read Make's platform-specific development recipe.
#[derive(Clone, Copy, Debug)]
pub(super) enum Host {
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
pub(super) struct Flags(Vec<String>);

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

    pub(super) fn names(&self, flag: &str) -> bool {
        self.0.iter().any(|candidate| candidate == flag)
    }

    fn carries_run(&self, caller: &str) -> bool {
        let wanted: Vec<&str> = caller.split_whitespace().collect();
        self.0
            .windows(wanted.len())
            .any(|run| run.iter().map(String::as_str).eq(wanted.iter().copied()))
    }

    pub(super) fn without_mold(self) -> Vec<String> {
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
pub(super) fn sources() -> Read<Vec<(String, Flags)>> {
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
    let mut remaining = line;
    let mut value = None;
    while let Some((prefix, rest)) = remaining.split_once("RUSTFLAGS=\"") {
        if prefix.chars().last().is_none_or(char::is_whitespace) {
            value = Some(rest);
            break;
        }
        remaining = rest;
    }
    let Some(rest) = value else {
        return if line.contains("RUSTFLAGS=") {
            Err(format!("unreadable RUSTFLAGS assignment in `{line}`").into())
        } else {
            Ok(None)
        };
    };
    let (assigned_flags, _) = rest
        .split_once('"')
        .ok_or_else(|| format!("unterminated RUSTFLAGS in `{line}`"))?;
    expanded(assigned_flags, inherited).map(Some)
}

/// Returns each Cargo command's assigned flags, or `None` for Cargo's config.
/// Whitaker is probed separately because Dylint builds its driver away from
/// the repository but checks repository crates from the Make working directory.
pub(super) fn make_rustflags(
    target: &str,
    host: Host,
    inherited: Option<&str>,
) -> Read<Vec<Option<Flags>>> {
    let commands = dry_run(target, host, inherited)?
        .lines()
        .filter(|line| {
            line.split_whitespace()
                .any(|word| word == "cargo" || word.ends_with("/cargo"))
        })
        .map(|line| assignment(line, inherited))
        .collect::<Read<Vec<_>>>()?;
    if commands.is_empty() {
        return Err(format!("`make -n {target}` runs no cargo command").into());
    }
    Ok(commands)
}

/// Returns the evaluated Whitaker recipes so its separate Cargo route is tested directly.
pub(super) fn make_whitaker_recipe(host: Host) -> Read<String> {
    dry_run("lint-whitaker", host, None)
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
        problems.push(format!(
            "`make {target}` on {host:?} drops {THREADS_FLAG}: {flags:?}"
        ));
    }
    if flags.names(MOLD_FLAG) != host.expects_mold() {
        problems.push(format!(
            "`make {target}` on {host:?} gets mold wrong: {flags:?}"
        ));
    }
    if inherited.is_some_and(|caller| !flags.carries_run(caller)) {
        problems.push(format!(
            "`make {target}` drops the caller's RUSTFLAGS: {flags:?}"
        ));
    }
    problems
}

/// Checks every supported development target, including the direct build.
pub(super) fn check_development_targets(host: Host, inherited: Option<&str>) -> Read<Vec<String>> {
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
