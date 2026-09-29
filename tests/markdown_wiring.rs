//! Contract tests that the Markdown formatting wiring meets the estate baseline.
//!
//! `make check-fmt` must run `mdtablefix --check` over the Git-selected
//! Markdown set with its exit status reaching Make; the CI job that runs
//! `make check-fmt` must install mdtablefix in an earlier step; and every
//! markdownlint-cli2-action step must lint `**/*.md`. Each clause is exercised
//! against weakened and legitimate fixtures as well as the repository's own
//! files.

use std::collections::BTreeMap;

use serde_yaml::Value;

const MAKEFILE: &str = include_str!("../Makefile");
const CI_WORKFLOW: &str = include_str!("../.github/workflows/ci.yml");

/// Flags `check-fmt` must pass to mdtablefix, in any order.
const SELECT_FLAGS: [&str; 3] = ["--check", "--git", "--include-untracked"];

/// The shared action that installs mdtablefix.
const INSTALL_ACTION: &str = "leynos/shared-actions/.github/actions/install-mdtablefix@";

/// The upstream Markdown lint action.
const LINT_ACTION: &str = "DavidAnson/markdownlint-cli2-action@";

/// The estate variables, as the repository's Makefile spells them.
const VARIABLES: &str = concat!(
    "MDTABLEFIX ?= mdtablefix\n",
    "MDTABLEFIX_SELECT = --git --include-untracked\n",
    "MDTABLEFIX_RULES = --wrap --renumber --breaks --ellipsis --fences\n",
);

/// Returns the name and value of a top-level variable assignment line.
fn assignment(line: &str) -> Option<(&str, &str)> {
    if line.starts_with(['\t', ' ', '#']) {
        return None;
    }
    let (target, value) = line.split_once('=')?;
    let name = target.trim_end_matches(['?', ':', '+']).trim();
    let is_name = !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_');
    is_name.then_some((name, value.trim()))
}

/// The text of a Makefile, read line by line.
#[derive(Clone, Copy)]
struct Makefile<'a>(&'a str);

/// Returns each variable assigned exactly once, so a reference to anything
/// else stays unexpanded.
fn variables(makefile: Makefile<'_>) -> BTreeMap<&str, &str> {
    let mut seen: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (name, value) in makefile.0.lines().filter_map(assignment) {
        seen.entry(name).or_default().push(value);
    }
    seen.into_iter()
        .filter_map(|(name, values)| match values.as_slice() {
            [only] => Some((name, *only)),
            _ => None,
        })
        .collect()
}

/// Expands `$(NAME)` references, three passes deep.
fn expand(text: &str, known: &BTreeMap<&str, &str>) -> String {
    let mut expanded = text.to_owned();
    for _ in 0..3 {
        for (name, value) in known {
            expanded = expanded.replace(&format!("$({name})"), value);
        }
    }
    expanded
}

/// Returns the recipe lines of the `check-fmt` rule.
fn check_fmt_recipes(makefile: Makefile<'_>) -> Vec<&str> {
    makefile
        .0
        .lines()
        .skip_while(|line| !line.starts_with("check-fmt:"))
        .skip(1)
        .take_while(|line| line.starts_with('\t'))
        .map(|line| line.trim_start_matches('\t'))
        .collect()
}

/// Returns whether one `&&` segment runs mdtablefix with every select flag.
fn is_check_invocation(segment: &str) -> bool {
    let mut words = segment.split_whitespace();
    let is_mdtablefix = words
        .next()
        .is_some_and(|program| program.rsplit('/').next() == Some("mdtablefix"));
    let arguments: Vec<&str> = words.collect();
    is_mdtablefix && SELECT_FLAGS.iter().all(|flag| arguments.contains(flag))
}

/// Returns whether a recipe line's exit status reaches Make and it runs the
/// check. The `-` prefix ignores the status; `@` and `+` do not.
fn recipe_runs_check(recipe: &str, known: &BTreeMap<&str, &str>) -> bool {
    let prefix: String = recipe.chars().take_while(|c| "@+- ".contains(*c)).collect();
    let text = expand(recipe.trim_start_matches(['@', '+', '-', ' ']), known);
    let masks_status = prefix.contains('-') || text.contains('|') || text.contains(';');
    !masks_status && text.split("&&").any(is_check_invocation)
}

/// Returns whether the Makefile's `check-fmt` rule runs the mdtablefix check.
fn runs_mdtablefix_check(makefile: Makefile<'_>) -> bool {
    let known = variables(makefile);
    check_fmt_recipes(makefile)
        .into_iter()
        .any(|recipe| recipe_runs_check(recipe, &known))
}

/// A decoded workflow: each job's name with its steps, in file order.
struct Workflow {
    /// The jobs, as `(name, steps)`.
    jobs: Vec<(String, Vec<Value>)>,
}

impl Workflow {
    /// Decodes workflow YAML, keeping only what the contract reads.
    fn parse(text: &str) -> Result<Self, serde_yaml::Error> {
        let parsed: Value = serde_yaml::from_str(text)?;
        let mapping = parsed.get("jobs").and_then(Value::as_mapping).cloned();
        let jobs = mapping
            .unwrap_or_default()
            .into_iter()
            .map(|(name, job)| {
                let steps = job.get("steps").and_then(Value::as_sequence).cloned();
                (
                    name.as_str().unwrap_or_default().to_owned(),
                    steps.unwrap_or_default(),
                )
            })
            .collect();
        Ok(Self { jobs })
    }
}

/// Returns a step's `uses:` reference, or the empty string.
fn uses(step: &Value) -> &str { step.get("uses").and_then(Value::as_str).unwrap_or_default() }

/// Returns a step's `run:` script, or the empty string.
fn run(step: &Value) -> &str { step.get("run").and_then(Value::as_str).unwrap_or_default() }

/// Returns whether a step's `run:` has a line that is exactly `make check-fmt`.
fn runs_check_fmt(step: &Value) -> bool {
    run(step)
        .lines()
        .any(|line| line.trim() == "make check-fmt")
}

/// Returns each job that runs `make check-fmt` before installing mdtablefix.
fn uninstalled_check_fmt(workflow: &Workflow) -> Vec<String> {
    let mut missing = Vec::new();
    for (name, steps) in &workflow.jobs {
        let installed_before = |index: usize| {
            steps
                .iter()
                .take(index)
                .any(|step| uses(step).starts_with(INSTALL_ACTION))
        };
        let uninstalled = steps
            .iter()
            .enumerate()
            .any(|(index, step)| runs_check_fmt(step) && !installed_before(index));
        if uninstalled {
            missing.push(name.clone());
        }
    }
    missing
}

/// Returns `(action steps, steps not linting **/*.md)` in a workflow.
fn lint_action_globs(workflow: &Workflow) -> (usize, usize) {
    let steps: Vec<&Value> = workflow
        .jobs
        .iter()
        .flat_map(|(_, steps)| steps)
        .filter(|step| uses(step).starts_with(LINT_ACTION))
        .collect();
    let narrowed = steps
        .iter()
        .filter(|step| {
            let globs = step.get("with").and_then(|with| with.get("globs"));
            globs.and_then(Value::as_str) != Some("**/*.md")
        })
        .count();
    (steps.len(), narrowed)
}

/// Returns a minimal Makefile whose `check-fmt` runs one recipe line.
fn makefile(recipe: &str, variables: &str) -> String {
    format!("{variables}\ncheck-fmt: ## Verify formatting\n\t{recipe}\n")
}

/// The repository's `check-fmt` runs the mdtablefix check with its status.
#[test]
fn the_repository_makefile_runs_the_check() {
    assert!(
        runs_mdtablefix_check(Makefile(MAKEFILE)),
        "`make check-fmt` does not run `mdtablefix --check` with its exit status"
    );
}

/// CI installs mdtablefix before `make check-fmt` and lints all Markdown.
#[test]
fn the_repository_workflows_install_and_lint() {
    let workflow = Workflow::parse(CI_WORKFLOW).expect("the CI workflow parses");
    let uninstalled = uninstalled_check_fmt(&workflow);
    assert_eq!(
        uninstalled,
        Vec::<String>::new(),
        "a job runs `make check-fmt` without installing mdtablefix first"
    );
    let (steps, narrowed) = lint_action_globs(&workflow);
    assert!(steps > 0, "no step runs the markdownlint-cli2-action");
    assert_eq!(
        narrowed, 0,
        "a markdownlint-cli2-action step lints less than **/*.md"
    );
}

/// A check that cannot fail the target, or does not check, is refused.
#[test]
fn a_weakened_check_is_refused() {
    let weakened = [
        "$(MDTABLEFIX) $(MDTABLEFIX_SELECT) $(MDTABLEFIX_RULES)",
        "$(MDTABLEFIX) --check $(MDTABLEFIX_RULES)",
        "-$(MDTABLEFIX) --check $(MDTABLEFIX_SELECT) $(MDTABLEFIX_RULES)",
        "$(MDTABLEFIX) --check $(MDTABLEFIX_SELECT) || true",
        "$(MDTABLEFIX) --check $(MDTABLEFIX_SELECT); true",
        "echo $(MDTABLEFIX) --check $(MDTABLEFIX_SELECT)",
    ];
    for recipe in weakened {
        assert!(
            !runs_mdtablefix_check(Makefile(&makefile(recipe, VARIABLES))),
            "accepted: {recipe}"
        );
    }
}

/// Equivalent spellings of the check are accepted, so the contract is narrow.
#[test]
fn a_legitimate_check_is_accepted() {
    let legitimate = [
        (
            "$(MDTABLEFIX) --check $(MDTABLEFIX_SELECT) $(MDTABLEFIX_RULES)",
            VARIABLES,
        ),
        ("mdtablefix --include-untracked --check --git --wrap", ""),
        ("@$(MDTABLEFIX) --check $(MDTABLEFIX_SELECT)", VARIABLES),
        (
            "cargo fmt --check && mdtablefix --check --git --include-untracked",
            "",
        ),
        (
            "/usr/local/bin/mdtablefix --check --git --include-untracked",
            "",
        ),
    ];
    for (recipe, variables) in legitimate {
        assert!(
            runs_mdtablefix_check(Makefile(&makefile(recipe, variables))),
            "refused: {recipe}"
        );
    }
}

/// An install step after `make check-fmt` does not provision it.
#[test]
fn an_install_after_check_fmt_is_refused() {
    let text = concat!(
        "jobs:\n  build-test:\n    steps:\n",
        "      - run: make check-fmt\n",
        "      - uses: leynos/shared-actions/.github/actions/install-mdtablefix@abc\n",
    );
    let workflow = Workflow::parse(text).expect("the fixture parses");
    let uninstalled = uninstalled_check_fmt(&workflow);
    assert_eq!(
        uninstalled,
        vec!["build-test".to_owned()],
        "an install step after `make check-fmt` must not count"
    );
}

/// A lint step over a narrower glob is counted as narrowed.
#[test]
fn narrowed_lint_globs_are_refused() {
    let text = concat!(
        "jobs:\n  lint:\n    steps:\n",
        "      - uses: DavidAnson/markdownlint-cli2-action@abc\n",
        "        with:\n          globs: 'docs/**/*.md'\n",
    );
    let workflow = Workflow::parse(text).expect("the fixture parses");
    let counts = lint_action_globs(&workflow);
    assert_eq!(
        counts,
        (1, 1),
        "one lint step with a narrowed glob is one step, one narrowed"
    );
}
