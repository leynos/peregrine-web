//! Contract tests for build-tool provisioning across reachable workflow suites.

use std::{error::Error, process::Command};

use camino::Utf8Path;
use cap_std::{ambient_authority, fs_utf8::Dir};
use serde_yaml::Value;

pub(super) type Read<T> = Result<T, Box<dyn Error>>;

/// One shell line with its tokens retained for conservative suite detection.
struct WorkflowCommand<'a> {
    source: &'a str,
    words: Vec<&'a str>,
}

/// Traverse optional YAML mappings without panicking on changed workflow shape.
fn path<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().try_fold(value, |parent, key| parent.get(*key))
}

fn string<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    path(value, keys).and_then(Value::as_str)
}

/// Parse YAML twice so duplicate mapping keys cannot silently override a job.
pub(super) fn workflow(source: &str) -> Result<Value, serde_yaml::Error> {
    serde_yaml::from_str::<serde_yaml::Mapping>(source)?;
    serde_yaml::from_str(source)
}

/// Discover committed workflow documents instead of fixing today's job names.
pub(super) fn workflows() -> Read<Vec<(String, Value)>> {
    let root = Dir::open_ambient_dir(
        Utf8Path::new(env!("CARGO_MANIFEST_DIR")),
        ambient_authority(),
    )?;
    let mut documents = Vec::new();
    for listing in root.read_dir(".github/workflows")? {
        let entry = listing?;
        let name = entry.file_name()?;
        if Utf8Path::new(&name).extension().is_some_and(|extension| {
            extension.eq_ignore_ascii_case("yml") || extension.eq_ignore_ascii_case("yaml")
        }) {
            let source = root.read_to_string(format!(".github/workflows/{name}"))?;
            documents.push((name, workflow(&source)?));
        }
    }
    documents.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(documents)
}

/// Classify runner labels, including matrix values in scalar or mapped labels.
fn runner_label(value: &Value, job: &Value) -> Option<bool> {
    match value {
        Value::String(label) => classify_runner_name(label, job),
        Value::Sequence(labels) => {
            let classes: Vec<_> = labels
                .iter()
                .filter_map(|label| runner_label(label, job))
                .collect();
            classes
                .contains(&true)
                .then_some(true)
                .or_else(|| classes.contains(&false).then_some(false))
        }
        Value::Mapping(mapping) => mapping
            .get("labels")
            .and_then(|labels| runner_label(labels, job)),
        _ => None,
    }
}

/// Resolve one runner label before combining sequence and mapping cases.
fn classify_runner_name(label: &str, job: &Value) -> Option<bool> {
    if label.contains("matrix.") {
        return matrix_runner(label, job);
    }
    if ["ubuntu", "linux"].iter().any(|name| label.contains(*name)) {
        return Some(true);
    }
    ["windows", "macos"]
        .iter()
        .any(|name| label.contains(*name))
        .then_some(false)
}

fn matrix_runner(expression: &str, job: &Value) -> Option<bool> {
    let key = expression
        .split("matrix.")
        .nth(1)?
        .trim_end_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_');
    let matrix = path(job, &["strategy", "matrix"])?;
    let mut cases: Vec<&Value> = matrix
        .get(key)
        .and_then(Value::as_sequence)
        .map(|values| values.iter().collect())
        .unwrap_or_default();
    if let Some(include) = matrix.get("include").and_then(Value::as_sequence) {
        cases.extend(include.iter().filter_map(|case| case.get(key)));
    }
    if cases.is_empty()
        || cases
            .iter()
            .any(|case| case.as_str().is_some_and(|label| label.contains("matrix.")))
    {
        return None;
    }
    let classes: Vec<_> = cases
        .iter()
        .filter_map(|case| runner_label(case, job))
        .collect();
    (classes.len() == cases.len()).then_some(classes.contains(&true))
}

/// Resolve a supported matrix runner expression against its declared values.
fn runner_is_linux(job: &Value) -> Option<bool> { runner_label(job.get("runs-on")?, job) }

/// Detect a test suite in a dry-run of a Make goal, including prerequisites.
fn make_goal_runs_suite(goal: &str) -> Read<bool> {
    let output = Command::new("make")
        .args([
            "-n",
            "-B",
            "CARGO=probe-cargo",
            "CHECK_BUILD_TOOLS=true",
            goal,
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()?;
    if !output.status.success() {
        return Err(format!("cannot inspect Make goal {goal}").into());
    }
    let commands = String::from_utf8_lossy(&output.stdout);
    Ok(commands.contains("probe-cargo test")
        || commands.contains("probe-cargo nextest run")
        || commands.contains("probe-cargo llvm-cov"))
}

/// Resolve one Make command, failing closed on options or dynamic goals.
fn make_line_reaches_suite(command: &WorkflowCommand<'_>) -> Read<bool> {
    for (index, word) in command.words.iter().enumerate() {
        if !matches!(*word, "make" | "$(MAKE)") {
            continue;
        }
        let goal = command
            .words
            .get(index + 1)
            .map(|value| value.trim_end_matches(';'))
            .ok_or_else(|| format!("Make invocation has no goal: {}", command.source))?;
        if goal.starts_with('-') || goal.contains('$') {
            return Err(format!("unresolved Make goal in workflow: {}", command.source).into());
        }
        if make_goal_runs_suite(goal)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Resolve one direct Cargo command without mistaking binstall for a suite.
fn cargo_line_reaches_suite(command: &WorkflowCommand<'_>) -> Read<bool> {
    let Some(index) = command.words.iter().position(|word| *word == "cargo") else {
        return Ok(false);
    };
    let arguments = command
        .words
        .get(index + 1..)
        .ok_or("Cargo command has no arguments")?;
    if cargo_arguments_reach_suite(arguments) {
        return Ok(true);
    }
    if arguments.iter().any(|argument| argument.contains('$')) {
        return Err(format!("unresolved Cargo command in workflow: {}", command.source).into());
    }
    Ok(false)
}

/// Recognize suite subcommands even after a toolchain or Cargo option.
fn cargo_arguments_reach_suite(arguments: &[&str]) -> bool {
    arguments.contains(&"test")
        || arguments.contains(&"llvm-cov")
        || arguments.windows(2).any(|pair| pair == ["nextest", "run"])
}

/// Recognize directly invoked test tools and Make goals in a workflow script.
fn run_reaches_suite(script: &str) -> Read<bool> {
    for line in script
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
    {
        let command = WorkflowCommand {
            source: line,
            words: line.split_whitespace().collect(),
        };
        if command.words.first().is_some_and(|word| {
            matches!(*word, "bash" | "sh" | "source" | "eval") || word.starts_with("./")
        }) {
            return Err(format!("unresolved shell command in workflow: {line}").into());
        }
        if cargo_line_reaches_suite(&command)? || make_line_reaches_suite(&command)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Recognize a suite action or an executable suite command.
fn suite_step(step: &Value) -> Read<bool> {
    if string(step, &["uses"]).is_some_and(|action| action.contains("/generate-coverage@")) {
        return Ok(true);
    }
    string(step, &["run"])
        .map(run_reaches_suite)
        .transpose()
        .map(|result| result.unwrap_or(false))
}

/// Find an unguarded installer before the first suite, with host packages first.
fn installed_before(steps: &[Value], suite: usize, needs_lld: bool) -> bool {
    let install = steps.iter().position(is_effective_installer);
    let Some(install_index) = install.filter(|index| *index < suite) else {
        return false;
    };
    steps.get(..install_index).is_some_and(|earlier| {
        earlier
            .iter()
            .any(|step| installs_native_packages(step, needs_lld))
    })
}

/// Reject conditional or masked installation before treating a step as ready.
fn is_effective_installer(step: &Value) -> bool {
    string(step, &["run"]) == Some("make install-build-tools")
        && step.get("if").is_none()
        && step.get("continue-on-error").is_none()
}

/// Recognize the host packages needed by the native or measured suite.
fn installs_native_packages(step: &Value, needs_lld: bool) -> bool {
    let script = string(step, &["run"]).unwrap_or_default();
    script.contains("apt-get install")
        && script.contains("clang")
        && script.contains("mold")
        && (!needs_lld || script.contains("lld"))
        && step.get("continue-on-error").is_none()
        && string(step, &["if"]).is_none_or(|condition| condition == "runner.os == 'Linux'")
}

/// Check the caller commands passed into the pinned mutation reusable workflow.
fn mutation_setup_is_safe(job: &Value) -> bool {
    let Some(script) = string(job, &["with", "setup-commands"]) else {
        return false;
    };
    let commands: Vec<_> = script.lines().map(str::trim).collect();
    let install = commands
        .iter()
        .position(|line| *line == "make install-build-tools");
    let apt = commands
        .iter()
        .position(|line| line.contains("apt-get install"));
    let (Some(apt_index), Some(install_index)) = (apt, install) else {
        return false;
    };
    apt_index < install_index
        && mutation_packages_ready(&commands, apt_index)
        && mutation_suite_not_started(&commands, install_index)
        && !script.contains("|| true")
        && job.get("continue-on-error").is_none()
}

/// Check the reusable workflow's required native package command.
fn mutation_packages_ready(commands: &[&str], apt_index: usize) -> bool {
    commands
        .get(apt_index)
        .is_some_and(|line| line.contains("clang") && line.contains("mold") && line.contains("lld"))
}

/// Keep mutation execution after the pinned build-tool installer.
fn mutation_suite_not_started(commands: &[&str], install_index: usize) -> bool {
    !commands
        .get(..install_index)
        .unwrap_or_default()
        .iter()
        .any(|line| line.contains("cargo-mutants") || line.contains("cargo test"))
}

/// Return the first suite and whether its route needs lld.
fn first_suite(steps: &[Value]) -> Read<Option<(usize, bool)>> {
    let mut first = None;
    let mut needs_lld = false;
    for (index, step) in steps.iter().enumerate() {
        if suite_step(step)? {
            first.get_or_insert(index);
            needs_lld |= string(step, &["uses"])
                .is_some_and(|value| value.contains("/generate-coverage@"))
                || string(step, &["run"]).is_some_and(|value| value.contains("make coverage"));
        }
    }
    Ok(first.map(|index| (index, needs_lld)))
}

/// Report one job's suite status and any provisioning error.
fn job_violation(location: &str, job: &Value) -> Read<(bool, Option<String>)> {
    if let Some(reusable) = string(job, &["uses"]) {
        return Ok(reusable_job_violation(location, job, reusable));
    }
    let steps = path(job, &["steps"])
        .and_then(Value::as_sequence)
        .ok_or("job has no steps sequence")?;
    let Some((first, needs_lld)) = first_suite(steps)? else {
        return Ok((false, None));
    };
    let problem = match runner_is_linux(job) {
        Some(true) if !installed_before(steps, first, needs_lld) => {
            Some(format!("{location}: suite runs before pinned tools"))
        }
        None => Some(format!("{location}: suite runner is unplaced")),
        _ => None,
    };
    Ok((true, problem))
}

/// Classify only the known mutation reusable workflow as a suite route.
fn reusable_job_violation(location: &str, job: &Value, reusable: &str) -> (bool, Option<String>) {
    if !reusable.contains("/mutation-cargo.yml@") {
        return (
            false,
            Some(format!("{location}: reusable workflow path is unproven")),
        );
    }
    (
        true,
        (!mutation_setup_is_safe(job))
            .then(|| format!("{location}: mutation setup lacks ordered install")),
    )
}

/// Report every unprovisioned or unplaceable suite across all workflow jobs.
pub(super) fn workflow_violations(documents: &[(String, Value)]) -> Read<Vec<String>> {
    let mut problems = Vec::new();
    let mut suites = 0;
    for (file, document) in documents {
        let jobs = path(document, &["jobs"])
            .and_then(Value::as_mapping)
            .ok_or("workflow has no jobs mapping")?;
        for (name, job) in jobs {
            let location = format!("{file}/{}", name.as_str().unwrap_or("<unknown>"));
            let (has_suite, problem) = job_violation(&location, job)?;
            suites += usize::from(has_suite);
            problems.extend(problem);
        }
    }
    if suites == 0 {
        problems.push("no suite jobs discovered".to_owned());
    }
    Ok(problems)
}
