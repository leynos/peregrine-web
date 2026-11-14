# 1.2.1 Establish the minimal dependency and feature baseline

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan` must
be kept up to date as work proceeds.

Status: DRAFT (awaiting expert review and user approval)

Branch: `1-2-1-minimal-dependency-and-feature-baseline`

Roadmap item: `1.2.1` in `docs/roadmap.md`.

## Purpose / big picture

Peregrine Web is an experimental, resource-oriented HTTP framework written in
Rust. Today the crate's only library content is the experimental specimen under
`src/spike/`, which roadmap task 1.1.3 compiled to prove that a resource can
own its policy and dependencies across asynchronous middleware. That spike
added its dependencies with caret requirements and recorded a one-off licence,
minimum-compiler, and feature review in
`docs/resource-policy-ownership-spike.md` under "Dependency review". The spike
ledger deliberately freezes that review as historical evidence and states:
"task 1.2.1 still owns the maintained dependency and feature baseline".

A caret requirement is Cargo's default version requirement: `bytes = "1.12.1"`
(equivalently `^1.12.1`) accepts any SemVer-compatible release at or above
1.12.1 and below 2.0.0. The lowest version a caret requirement admits is its
floor.

Roadmap task 1.2.1 asks: can a contributor build and verify the selected
contract through repository entrypoints, so that feature work can rely on the
baseline? Its success criterion is: "compatible caret requirements and licence
review are recorded; `make check-fmt`, `make lint`, and `make test` pass on the
supported baseline."

Today a contributor who adds, removes, or re-features a dependency faces no
check. Dependabot raises versions daily without re-running a licence review;
caret floors drift below the versions the code actually needs because
Dependabot moves only the lockfile for in-range releases (for example,
`proptest = "1.9.0"` currently resolves to 1.11.0, and nothing proves the tests
still compile at 1.9.0); and the term "supported baseline" is defined nowhere
in the repository.

After this change a contributor can observe four things:

1. `docs/dependency-baseline.md` lists every direct dependency with its kind,
   purpose, consuming code, and feature posture, records the supported
   baseline, and records a dated licence review of the whole resolved graph. A
   contract test fails, naming the crate, if `Cargo.toml` and that register
   disagree, if any requirement is not a caret requirement, or if a Cargo
   feature appears without the feature-matrix work design §11 requires.
2. `make deny` checks every resolved package's licence against the allow-list
   in `deny.toml`, rejects non-crates.io sources and wildcard requirements, and
   runs offline on every pull request, including Dependabot's.
3. `make check-dependency-floors` resolves every direct dependency at its
   caret floor in a scratch copy and type-checks all targets, proving that the
   recorded floors are compatible rather than nominal.
4. `make check-fmt`, `make lint`, and `make test` pass on the pinned nightly
   toolchain with the committed lockfile, and the gates are recorded in the
   plan's evidence.

Existing behaviour is preserved: no public Rust API changes, the spike module
keeps its surface, and no runtime dependency is added or removed. Dev-only
default features may be narrowed where the gates prove a feature unused.

## Constraints

- Implementation begins only after the user explicitly approves this plan.
- No public API change in `src/`. The spike module `peregrine_web::spike` and
  `peregrine_web::greet` keep their signatures; this task touches no file under
  `src/`.
- No runtime (`[dependencies]`) addition, removal, or requirement change.
  Runtime default features stay as they are unless a runtime dependency's
  default is shown unused, which requires escalation first (see `Tolerances`).
- The only authorized new direct dependency is the development dependency
  `semver = "1.0.28"` (licence `MIT OR Apache-2.0`), which is already resolved
  transitively through `rstest-bdd-macros`, so it adds no package to
  `Cargo.lock`. Any other new crate requires escalation.
- The only authorized new host tool is `cargo-deny` 0.20.2 (licence
  `MIT OR Apache-2.0`, published 2026-07-09, `rust-version` 1.88.0). It is
  already installed on the development host; CI installs it with
  `cargo binstall --no-confirm --locked cargo-deny@0.20.2`.
- The crate must not gain a `[features]` table, a `build.rs`, a
  `[build-dependencies]` table, or a `rust-version` key in this task (see
  decisions D2 and D7).
- `make lint` keeps its current two-phase contract (`lint-clippy` then
  `lint-whitaker`). The tests in `tests/make_cargo_routes.rs`,
  `tests/make_gate_order.rs`, and `tests/build_standard_contract.rs` pin that
  shape; this task does not widen it (decision D3).
- Advisory auditing remains with `make audit` (`cargo audit`) and keeps its
  Dependabot exemption. `deny.toml` must not enable the advisories check.
- The historical ledgers stay untouched:
  `docs/resource-policy-ownership-spike.md` "Dependency review" tables and the
  completed ExecPlans under `docs/execplans/` other than this file.
- Repository rules from `AGENTS.md` apply: no code file over 400 lines; every
  module starts with a `//!` comment; every `assert!`-family call carries a
  message; `expect` only in tests; no `unwrap`; no lint suppression except as a
  tightly scoped last resort with a reason; en-GB-oxendict spelling in prose
  and comments; Markdown wrapped at 80 columns.
- Environment access in tests must follow `AGENTS.md` "Inject the environment
  instead": the new tests read files relative to `env!("CARGO_MANIFEST_DIR")`
  (a compile-time constant) and must not call `std::env::var`.
- Rust builds must go through the host's compile admission (`RUSTC_WRAPPER`);
  never bypass it, and never create an isolated Cargo cache.
- Scratch build trees live under the worktree's `target/` directory, never
  under `/tmp`. Logs go to `/tmp`.

If satisfying the objective requires violating a constraint, stop, record the
conflict in `Decision log`, and escalate.

## Tolerances (exception triggers)

Comparison base: the merge base with `origin/main` (currently `0155d04`). Count
with:

```sh
git diff --numstat "$(git merge-base HEAD origin/main)" -- . \
  ':(exclude)docs/execplans/**'
```

Every size count excludes ExecPlan files, including this one. Limits measure
net lines (additions minus deletions) per category, enforced independently;
spare capacity in one category never offsets another.

Classification is by whole file:

- Code: `tests/**/*.rs`, `tests/features/*.feature`, `scripts/**`, and the
  executable recipe lines of `Makefile`.
- Configuration: `Cargo.toml`, `Cargo.lock`, `deny.toml`, `.config/**`,
  `.github/**`, `typos.local.toml`, and generated `typos.toml` (generated by
  `make spelling`; counted, not excluded).
- Documentation: `docs/**/*.md` except ExecPlans, `AGENTS.md`, `README.md`.

Budget survey. The two comparable plans,
`docs/execplans/1-1-3-compile-resource-policy-and-async-ownership-spike.md` and
`docs/execplans/1-1-resolve-decisions-to-mitigate-api-rework.md`, set no
numeric size budgets and raised none; their only size pressure was the 400-line
file rule, met by splitting files. There is therefore no history of repeated
increases to learn from. The estimates below come from the planned artefacts: a
manifest-contract support module (about 250 lines across three files), its
integration test and property test (about 200), BDD steps and feature (about
160), a workflow contract test (about 100), contract-test adjustments where CI
step lists are pinned (about 60), Makefile recipes and a floors script (about
90). That is roughly 860 lines of code, so the code limit allows about 25%
contingency. Configuration is `deny.toml` (about 50), one `Cargo.toml` line, CI
additions (about 20), and a new floors workflow (about 50). Documentation is
the register (about 220), ADR 008 (about 130), and updates to five live guides
(about 120).

- Files: stop and escalate if more than 30 non-ExecPlan files change.
- Code: stop and escalate above 1,100 net lines.
- Configuration: stop and escalate above 180 net lines.
- Documentation: stop and escalate above 600 net lines.
- Interface: any change under `src/` requires escalation.
- Dependencies: any direct dependency beyond `semver` (dev), or any
  runtime-dependency feature change, requires escalation.
- Floors: if more than three direct caret floors must be raised to make
  `make check-dependency-floors` pass, stop and escalate; that many dishonest
  floors suggests a wider review rather than piecemeal bumps.
- Licences: if any resolved package's licence cannot be satisfied by the
  allow-list in decision D4 without adding a copyleft or non-OSI licence, stop
  and escalate rather than widening the allow-list.
- Architecture: if the licence check cannot run without network access on
  pull requests, or if `make lint` would have to change shape, stop and
  escalate.
- Iterations: if a gate still fails after three fix attempts for the same
  cause, stop and escalate.
- Time: if a milestone takes more than four hours of active work, stop and
  escalate.
- Ambiguity: if the review of `serde_yaml` (risk R3) or the ADR status
  question (decision D12) is contested during implementation, stop and present
  the options.

## Risks

- R1: A caret floor may be dishonest: the code may use an API added after the
  floor. Severity: medium. Likelihood: medium (the planning probe shows
  `camino`, `proptest`, `serde`, and `toml` resolve below the lockfile at their
  floors). Mitigation: milestone `EP-M3` runs the floors check, raises a floor
  only to the lowest version that passes, and records each raise.
- R2: `cargo-deny` may not identify `mockable` 3.0.0's licence, because that
  crate declares `license-file` rather than an SPDX `license`. Severity: low.
  Likelihood: low (the planning probe passed with a `no-license-field`
  warning). Mitigation: pin a `[[licenses.clarify]]` entry with the file hash
  so any change to that file forces review.
- R3: `serde_yaml` 0.9.34 is published as `+deprecated` and is used by 15 test
  files. Severity: medium for maintenance, low for this task (it is
  development-only and ships nowhere). Likelihood: certain. Mitigation: record
  it in the register with an explicit follow-up; do not replace it here
  (decision D13). Escalate if the expert review requires replacement in this
  task.
- R4: Adding a CI step may break workflow contract tests that pin step lists
  (`tests/act_workflow.rs`, `tests/support/act_workflow_command.rs`,
  `tests/whitaker_workflow.rs`). Severity: medium. Likelihood: medium.
  Mitigation: run those tests in the red step of `EP-M2`, update the expected
  step lists together with the workflow, and keep the new step after
  `make audit` and before `make lint`.
- R5: The direct-minimal-versions resolution needs crates absent from the local
  cache, so the floors check needs network access. Severity: low. Likelihood:
  certain offline (the probe resolved `camino` 1.2.1 offline but 1.2.0 online).
  Mitigation: the target documents its network need; CI runs it; the three
  standard gates do not depend on it.
- R6: Narrowing a development dependency's default features could break a test
  only under some invocation, such as doctests or `--all-features`. Severity:
  low. Likelihood: low. Mitigation: each narrowing is kept only after
  `make lint` and `make test` (which include doctests) pass, and is reverted
  otherwise.
- R7: The RustSec advisory database cannot be fetched on the development host
  (the planning probe failed with a credential-helper error), so advisory
  evidence for this plan comes from CI. Severity: low. Likelihood: certain on
  this host. Mitigation: advisories stay with `make audit` in CI (decision D3);
  the plan records CI run evidence rather than local evidence.
- R8: The register drift test parses a Markdown table, which a formatter could
  reflow. Severity: low. Likelihood: low (`mdtablefix` preserves cell text).
  Mitigation: the parser reads only rows inside explicit HTML-comment markers
  and matches crate names in backticks; `EP-M1` includes a negative control.

## Progress

- [x] (2026-11-14 11:20Z) Branch renamed; Lody session titled.
- [x] (2026-11-14 11:30Z) Wyvern reconnaissance of manifests, gates, CI, design
  documents, and historical plans.
- [x] (2026-11-14 11:45Z) Planning probes: `cargo-deny` scratch configuration,
  direct-minimal-versions resolution, advisory fetch, base `make check-fmt`.
- [x] (2026-11-14 12:10Z) Initial draft written.
- [ ] Community-of-experts review applied.
- [ ] Plan approved by the user.
- [ ] `EP-M0` Baseline gates recorded on the unchanged base.
- [ ] `EP-M1` Manifest contract and dependency register.
- [ ] `EP-M2` Licence, source, and ban gate.
- [ ] `EP-M3` Caret-floor compatibility check.
- [ ] `EP-M4` Development-dependency feature narrowing.
- [ ] `EP-M5` Documentation closure and roadmap update.

## Surprises & discoveries

- Observation: the spike's dependency review exists but is frozen by design.
  Evidence: `docs/resource-policy-ownership-spike.md` "Dependency review",
  closing paragraph. Impact: this task writes a new, living register rather
  than editing the spike tables.
- Observation: `docs/developers-guide.md` "The experimental spike module"
  lists the spike's development dependencies but omits `camino`, `cap-std`,
  `mockable`, `serde`, `serde_yaml`, and `toml`, which predate the spike and
  serve the contract tests. Evidence: Wyvern documentation report. Impact:
  `EP-M5` replaces that list with a pointer to the register.
- Observation: the RustSec advisory database at
  `~/.cargo/advisory-dbs/advisory-db-3157b0e258782691` was last updated
  2026-10-14 and cannot be refreshed on this host. Evidence:
  `/tmp/deny-advisories-peregrine-web-1-2-1.out`. Impact: supports decision D3.
- Observation: `cargo update --lockfile-path` is not available on the pinned
  Cargo (`cargo 1.101.0-nightly (3c747ea3e 2026-11-03)`). Evidence: the probe
  failed with "unexpected argument '--lockfile-path'". Impact: the floors check
  must work on a scratch copy of the tree (decision D8).
- Observation: the duplicate `toml` 0.5.11 and `thiserror` 1.0.69 enter through
  `rstest-bdd` (via `i18n-embed`), not through any direct requirement. Evidence:
  `cargo tree -i toml@0.5.11` and `cargo tree -i thiserror@1.0.69`. Impact:
  the bans check warns rather than denies on duplicates (decision D4).
- Observation: an earlier repository memory recorded `make check-fmt` failing on
  `main`; at `0155d04` it passes ("33 files left unchanged"). Evidence:
  `/tmp/check-fmt-peregrine-web-1-2-1-minimal-dependency-and-feature-baseline.out`.
  Impact: no pre-existing formatting failure needs separating from regressions.

## Decision log

- D1. Decision: scope 1.2.1 as converting the spike's frozen, one-off review
  into a maintained baseline: a written policy (ADR 008), a living register,
  and three mechanical guards (manifest contract test, `cargo-deny` licence
  gate, caret-floor check), plus narrowing of unused development features.
  Framework dependency selection (`matchit`, `tracing`, `metrics`, runtime
  `serde`) stays with the slices that need them. Rationale: the roadmap asks
  that feature work can rely on the baseline; a review that nothing re-checks
  stops being true at the next Dependabot merge. The rejected alternative,
  re-recording the spike tables once, would satisfy the letter of "recorded"
  but not "reproducible". Date/Author: 2026-11-14, planning agent.
- D2. Decision: define the supported baseline as the toolchain pinned in
  `rust-toolchain.toml` (`nightly-2026-11-06`, rustc 1.101.0-nightly
  `249c1e4e3`), on the `x86_64-unknown-linux-gnu` host configured in
  `.cargo/config.toml`, with the committed `Cargo.lock`, building all targets
  with all features. Do not declare `rust-version`. Rationale: terms of
  reference Q5 and ADR 007 leave the compiler floor open until phase 2; a
  `rust-version` of 1.101 would invite stable 1.101, which has never been
  tested and cannot honour `.cargo/config.toml`'s unstable keys. The register
  states the baseline explicitly so it is no longer undefined. Date/Author:
  2026-11-14, planning agent.
- D3. Decision: add a separate `make deny` target running `cargo deny` for
  licences, bans, and sources, offline and locked; run it in CI's `build-test`
  job without a Dependabot exemption; leave advisories to `make audit`.
  Rationale: the licence result depends only on the lockfile, so unlike
  advisories it cannot deadlock Dependabot, and Dependabot PRs are exactly
  where new transitive packages arrive. Folding it into `make lint` would
  change a contract pinned by three test files and require `cargo-deny` for
  every lint run although it compiles nothing. Enabling `cargo-deny` advisories
  would duplicate `cargo audit` and need network access. The rejected
  alternative, a hand-written SPDX evaluator inside `make test`, would
  re-implement a maintained tool and add an `spdx` dependency. Date/Author:
  2026-11-14, planning agent.
- D4. Decision: `deny.toml` allows exactly the licences the graph needs today:
  `MIT`, `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`, `ISC`, `Unicode-3.0`,
  and `Zlib`; checks development dependencies (`include-dev = true`); sets
  `confidence-threshold = 0.93`; warns on unused allowances; clarifies
  `mockable` 3.0.0 as `MIT` pinned to its `LICENSE` hash; denies unknown
  registries and Git sources; denies wildcard requirements; and warns on
  duplicate versions. Rationale: an allow-list that names only observed,
  ISC-compatible permissive licences makes any new licence a reviewable event.
  Warnings rather than errors for unused allowances and duplicates avoid
  failing Dependabot PRs on removals or transitive duplication that the project
  does not control; the register records both. Date/Author: 2026-11-14,
  planning agent.
- D5. Decision: classify requirements with the `semver` crate
  (`semver::VersionReq::parse`), accepting a requirement only if it has exactly
  one comparator whose operator is `semver::Op::Caret`. Rationale: Cargo itself
  parses requirements with `semver`, and a bare `"1.2.3"` parses as a caret
  comparator, so the check shares Cargo's grammar instead of hand-parsing
  operators. `semver` is already resolved, so the graph does not grow.
  Date/Author: 2026-11-14, planning agent.
- D6. Decision: the register drift test compares dependency names, kinds,
  `default-features` settings, and explicit feature lists, but not version
  numbers or licences. Rationale: Dependabot edits requirement numbers daily;
  pinning them in prose would fail every update. Floors are checked by
  `make check-dependency-floors` and licences by `make deny`, so each fact has
  exactly one mechanical owner. Date/Author: 2026-11-14, planning agent.
- D7. Decision: the baseline declares no Cargo features, and a tripwire test
  fails if `[features]` appears, pointing to design §11's matrix rule. The
  spike stays ungated. Rationale: every runtime dependency is used only by the
  spike, which is the library's only content; gating it behind a feature would
  create a minimal/each/all build matrix with no consumer. Phase 2 (roadmap
  2.2.3) decides the spike's future. Date/Author: 2026-11-14, planning agent.
- D8. Decision: implement the floors check as
  `make check-dependency-floors`, which exports `HEAD` with `git archive` into
  `target/dependency-floors/`, runs
  `cargo generate-lockfile -Zdirect-minimal-versions` there, then
  `cargo check --all-targets --all-features --locked`. CI runs it in a new
  workflow on pull requests that touch `Cargo.toml` or `Cargo.lock`, weekly,
  and on dispatch. Rationale: the pinned Cargo lacks `--lockfile-path`, and
  rewriting the real `Cargo.lock` in place would make the target destructive.
  `-Zdirect-minimal-versions` rather than `-Zminimal-versions` because the
  Cargo book warns that transitive floors are often undeclared. Type-checking
  suffices because a dishonest floor shows up as a missing item or signature
  mismatch; running the whole suite at floors would double CI cost for little
  extra evidence. Date/Author: 2026-11-14, planning agent.
- D9. Decision: narrow development dependencies' default features where
  unused, starting with `proptest` (defaults `std`, `fork`, `timeout`,
  `bit-set`) and `rstest-bdd` (default `diagnostics`), keeping each change only
  if the gates pass. Runtime defaults (`std` for `bytes`, `http`, and
  `thiserror`) stay, because the crate is a `std` library. Rationale: "minimal"
  applies to the feature surface as well as to the crate list; each removed
  feature is graph the licence and floors checks no longer have to carry.
  Date/Author: 2026-11-14, planning agent.
- D10. Decision: no Verus proof and no Kani harness in this task.
  Rationale: see `Verification plan`, "Formal verification decision".
  Date/Author: 2026-11-14, planning agent.
- D11. Decision: no `insta` snapshots. The new diagnostics have one output
  format and a handful of variants, which `rstest` cases with `googletest`
  matchers state more precisely; `insta` is also not a current dependency.
  Date/Author: 2026-11-14, planning agent.
- D12. Decision: record the policy as ADR 008, "Maintain the dependency and
  licence baseline", with status "Accepted" on merge, citing the user's
  approval of this plan as the deciding authority. Rationale: unlike ADRs
  001–005, this is a repository engineering policy inside the maintainer's
  authority, comparable to ADR 006's acceptance as validation policy.
  Date/Author: 2026-11-14, planning agent.
- D13. Decision: keep `serde_yaml` 0.9.34 for now, record it as deprecated
  upstream in the register, and propose its replacement as a separate
  follow-up. Rationale: replacing it touches 15 test files that parse workflows
  and Make output, an unrelated change that would obscure this task's diff.
  Date/Author: 2026-11-14, planning agent.

## Outcomes & retrospective

Not yet started. Complete at each milestone and at the end, and reconcile
discoveries with the artefacts in `Conformance basis` before setting the status
to `COMPLETE`.

## Context and orientation

Inspected base revision: `0155d04` ("Compile resource-policy and async
ownership spike (1.1.3)"). Line numbers below refer to that revision; relocate
symbols with `rg -n '<symbol>'`.

### The crate and its dependencies

`Cargo.toml` declares one package, `peregrine-web` 0.1.0, edition 2024,
`license = "ISC"`, no `rust-version`, no `[features]`, no `build.rs`, and no
workspace. `src/lib.rs` (18 lines) exports `pub mod spike` and
`pub const fn greet()`. The spike lives in `src/spike/` (`mod.rs`, `body.rs`,
`method.rs`, `request.rs`, `policy.rs`, `policy_enforcer.rs`, `resource.rs`,
`invoke.rs`), with an example at `examples/resource_policy_spike.rs`.

Runtime dependencies, all plain caret requirements and all used only by
`src/spike/`: `async-trait = "0.1.92"`, `bytes = "1.12.1"`, `http = "1.5.0"`,
`http-body = "1.1.0"`, `http-body-util = "0.1.5"`, `thiserror = "2.0.21"`.

Development dependencies, all plain caret requirements:

- Contract-test infrastructure (predating the spike): `camino`, `cap-std`
  (`features = ["fs_utf8"]`), `mockable` (`default-features = false`), `rstest`
  (`default-features = false`), `proptest`, `serde` (`features = ["derive"]`,
  used by `tests/support/act_workflow_model.rs`), `serde_yaml` (15 test files),
  and `toml`.
- Spike witnesses: `googletest`, `pretty_assertions`, `hyper`
  (`features = ["http1", "server"]`) and `hyper-util` (`features = ["tokio"]`)
  in `tests/support/spike/network.rs` only, `serde_json` in
  `tests/resource_policy_example_executable.rs` only, `trybuild` in
  `tests/ownership_contract.rs` only, `tokio` (seven features), `rstest-bdd`,
  and `rstest-bdd-macros`. `rstest-bdd` has no direct `use`; the macros expand
  to paths in it, so it stays.

`Cargo.lock` resolves 225 non-root packages. Resolved versions above floors
include `camino` 1.2.6, `proptest` 1.11.0, `serde` 1.0.229, and `toml` 1.1.6.
Duplicate names (`getrandom`, `io-lifetimes`, `r-efi`, `syn`, `thiserror`,
`thiserror-impl`, `toml`, `windows-sys`) are all transitive.

### Gates and their contract tests

The `Makefile` gates are summarized in `AGENTS.md`. `make check-fmt` runs
`cargo fmt --all -- --check` and `mdtablefix --check`. `make lint` recursively
runs `lint-clippy` (`cargo doc --no-deps`, then `cargo clippy` with
`CARGO_FLAGS ?= --all-targets --all-features`) and `lint-whitaker`. `make test`
runs `check-build-tools`, `cargo fetch --locked --target <host>`,
`cargo nextest run` (minimum 0.9.100), and `cargo test --doc`. `make audit` runs
`cargo audit`, honouring `CARGO_AUDIT_IGNORES`. `make markdownlint`,
`make spelling`, and `make nixie` validate documentation.

These gates are themselves under test. `tests/make_cargo_routes.rs` pins which
Cargo subcommands each target runs (line 25: `lint` runs `doc` and `clippy`).
`tests/make_gate_order.rs` checks ordering and failure propagation through a
probe repository. `tests/act_workflow.rs`, `tests/whitaker_workflow.rs`,
`tests/markdown_wiring.rs`, and `tests/support/act_workflow_command.rs` parse
`.github/workflows/ci.yml` and pin its step order. `.config/nextest.toml`
defines a `nested-cargo-builds` test group (one thread) for tests that start a
child Cargo.

CI (`.github/workflows/ci.yml`, job `build-test`, `ubuntu-latest`, no matrix)
installs build tools, checks formatting, Markdown, and spelling, installs
`cargo-nextest` and (except for Dependabot) `cargo-audit`, installs Whitaker,
runs `make audit` (except for Dependabot), runs `make lint`, then coverage.
`.github/workflows/audit.yml` runs `make audit` weekly.
`.github/dependabot.yml` updates `cargo`, `github-actions`, and
`rust-toolchain` daily with grouped minor and patch updates.

### Host tooling

Installed on the development host: `cargo-deny` 0.20.2, `cargo-audit` 0.22.2,
`cargo-about` 0.9.2, `cargo-nextest`. Not installed: `cargo-vet`,
`cargo-machete`, `cargo-udeps`. The repository configures none of these except
through the Make targets above. Verus and Kani are not installed and not needed
here.

### Planning probes (evidence, not implementation)

A scratch `deny.toml` in `/tmp/peregrine-1-2-1-probe/` with the D4 allow-list
plus `BSD-2-Clause` and `BSD-3-Clause` gave, for
`cargo deny --offline --locked --config <scratch> check licenses bans sources`:

```plaintext
warning[no-license-field]: license expression was not specified in manifest for crate 'mockable = 3.0.0'
warning[license-not-encountered]: license was not encountered
bans ok, licenses ok, sources ok
```

The unmatched allowance was `BSD-3-Clause`; the BSD licences are therefore
excluded from D4. A `git archive` copy under `target/` resolved with
`cargo generate-lockfile -Zdirect-minimal-versions` to `camino` 1.2.0,
`proptest` 1.9.0, `rstest` 0.27.0, `serde` 1.0.228, and `toml` 1.1.3, with
every other direct dependency at its lockfile version. The copy was deleted;
nothing was compiled.

### Documentation to read

- `docs/roadmap.md` §1.2 for the task and its successor 1.2.2.
- `docs/peregrine-design.md` §1.2 (ecosystem), §2 (ownership and the
  "extract another crate only when" rule), §11 (evidence and the feature matrix
  rule), and §12 (dependencies; the paragraph this task updates).
- `docs/terms-of-reference.md` §8.1, §8.3, and Q5.
- `docs/repository-layout.md`, which must list `deny.toml` and the new
  register.
- `docs/resource-policy-ownership-spike.md` "Dependency review" (historical).
- `docs/adrs/adr-0002-compiler-ownership-experiment.md` and
  `docs/adrs/adr-0007-resource-policy-and-body-erasure-boundary.md` for the
  open compiler-floor question.
- `docs/developers-guide.md` "Local Workflow", "Tooling", "Security audit
  ignores", "Workflow pins and Dependabot", "Fetching dependencies before
  tests", "The experimental spike module", "Toolchain updates".
- `docs/documentation-style-guide.md` for ADR format and Markdown rules.
- `docs/rust-testing-with-rstest-fixtures.md`,
  `docs/rstest-bdd-users-guide.md`,
  `docs/reliable-testing-in-rust-via-dependency-injection.md`, and
  `docs/rust-doctest-dry-guide.md` for test idioms.
- `docs/complexity-antipatterns-and-refactoring-strategies.md` for keeping the
  support module's functions small.
- `docs/hexagonal-application-case-study.md`,
  `docs/pachislot-extension-design.md`, and
  `docs/polonius-ownership-experiment.md` only to confirm that no planned
  dependency of theirs is pre-empted here (none is).

### Skills to load

`rust-router` first, then `rust-unit-testing` (fixtures and matchers),
`proptest`, `arch-supply-chain` (cargo-deny policy), `arch-decision-records`
(ADR 008), `execplans` (keeping this plan current), `en-gb-oxendict-style`
(prose), and `commit-message`.

## Conformance basis

Upstream artefacts at revision `0155d04`:

- Terms of reference `docs/terms-of-reference.md`: goal G4 (maintainable
  adoption: "reproducible checks"); §8.1 (passing applicable quality gates);
  §8.3 (dependence on "compatible Rust ecosystem libraries, a maintained
  compiler baseline"); Q5 (compiler posture, open).
- Technical design `docs/peregrine-design.md` v0.6 (2026-10-13): §2
  (crate-extraction rule), §11 (feature-matrix rule: "If optional features are
  introduced, exercise minimal, each supported feature, and all-feature
  builds"), §12 (caret requirements; licence and feature-default review).
- ADRs: ADR 002 (compiler posture not fixed); ADR 007 (open compiler floor
  question). No existing ADR governs dependency or licence policy; ADR 008 is
  new in this task.
- Roadmap `docs/roadmap.md` item 1.2.1 and its success criterion.
- Governing standards: `AGENTS.md` "Dependency management" (caret
  requirements mandatory; no wildcard or open inequality) and "Change quality &
  committing".

Derived requirements:

- `REQ-CARET`: every direct requirement in `Cargo.toml` is a single caret
  comparator (from `AGENTS.md` and design §12).
- `REQ-FLOOR`: every caret floor admits a version set that type-checks all
  targets (from "compatible caret requirements").
- `REQ-LICENCE`: every resolved package's licence is recorded and satisfiable
  by an ISC-compatible allow-list (from design §12 and the roadmap).
- `REQ-REGISTER`: every direct dependency is recorded with its kind, purpose,
  consumers, and feature posture, and the record cannot silently drift.
- `REQ-FEATURES`: the crate's own feature surface is explicit and design §11's
  matrix rule activates when a feature first appears; dependency features are
  the smallest set the gates need.
- `REQ-BASELINE`: the supported baseline is defined, and `make check-fmt`,
  `make lint`, and `make test` pass on it.

Observed gaps:

- `GAP-FROZEN`: the only licence review is frozen as spike history.
- `GAP-UNDEFINED`: "supported baseline" is not defined anywhere.
- `GAP-DRIFT`: no check catches a new dependency, a non-caret requirement, a
  new licence, or a floor that no longer compiles.
- `GAP-DOCS`: the developers' guide's dependency list is incomplete.

Trace links:

```plaintext
TOR-G4, AGENTS-deps -> REQ-CARET -> EP-M1 -> tests::dependency_baseline_contract::requirements_are_caret
TOR-G4, design-§12 -> REQ-FLOOR -> EP-M3 -> make check-dependency-floors (CI: dependency-floors.yml)
design-§12, roadmap-1.2.1 -> REQ-LICENCE -> EP-M2 -> make deny; docs/dependency-baseline.md licence review
roadmap-1.2.1, GAP-FROZEN, GAP-DOCS -> REQ-REGISTER -> EP-M1, EP-M5 -> tests::dependency_baseline_contract::register_matches_manifest
design-§11 -> REQ-FEATURES -> EP-M1, EP-M4 -> tests::dependency_baseline_contract::crate_declares_no_features
TOR-§8.3, GAP-UNDEFINED -> REQ-BASELINE -> EP-M0, EP-M5 -> gate logs; register "Supported baseline"
```

## Verification plan

This task introduces repository-owned checking logic, not framework behaviour.
Its obligations concern the relationship between three artefacts (the manifest,
the register, and the resolved graph) and the truthfulness of the recorded
floors.

### Abstract model

Let `M` be the set of direct dependency declarations parsed from `Cargo.toml`.
Each declaration `d` has a name, a kind in `{runtime, dev, build}`, a
requirement string `req(d)`, a source in `{registry, git, path}`, a boolean
`default_features(d)`, and a set `features(d)`. Let `R` be the set of rows in
the register's machine-checked table, with the same fields except the
requirement and source. Let `G` be the resolved package graph in `Cargo.lock`
and `lic(p)` the licence expression of package `p`. Let `A` be the allow-list.

A declaration may be written as a string (`name = "1.2.3"`), which means
registry source, default features on, no extra features; or as a table with
`version`, `default-features`, `features`, `git`, `path`, `package`, or
`workspace` keys. `package` (renaming) and `workspace = true` are rejected
because this single-crate repository does not use them and they change what the
name means. A malformed manifest or register is a test failure with a
diagnostic naming the file and the offending key, never a panic without context.

### Invariants and lemmas

- `INV-CARET` (`REQ-CARET`): for every `d` in `M`, `req(d)` parses as a
  `semver::VersionReq` with exactly one comparator whose operator is
  `Op::Caret`, and `d` has registry source.
- `INV-REGISTER` (`REQ-REGISTER`): the projection of `M` onto (name, kind,
  default-features, features) equals `R` as a set; the diagnostic lists
  missing, extra, and mismatched entries separately.
- `INV-NO-FEATURES` (`REQ-FEATURES`): `Cargo.toml` has no `[features]` table.
- `INV-LICENCE` (`REQ-LICENCE`): for every `p` in `G`, including development
  and build dependencies, `lic(p)` (or its pinned clarification) is satisfiable
  using only licences in `A`; every `p` comes from crates.io.
- `INV-FLOOR` (`REQ-FLOOR`): resolving each `d` at the lowest version admitted
  by `req(d)` (transitives at their maxima) yields a graph on which
  `cargo check --all-targets --all-features --locked` succeeds.
- `L-EXTRACT`: the extraction function maps every accepted string or table
  form to exactly the declaration fields above, and maps every rejected form
  (non-caret, git, path, package rename, workspace inheritance, non-string
  version) to a typed error naming the dependency. `INV-CARET` and
  `INV-REGISTER` on the real manifest follow from `L-EXTRACT` plus the two
  comparisons, because the comparisons operate only on extracted values.

### Axioms (trusted, not verified here)

- `AXIOM-SEMVER`: `semver::VersionReq::parse` implements Cargo's requirement
  grammar, including treating a bare version as caret. Source: the `semver`
  crate documentation; Cargo depends on the same crate. Boundary: `INV-CARET`.
- `AXIOM-DENY`: `cargo-deny` 0.20.2 evaluates SPDX expressions and package
  sources correctly from `cargo metadata`. Boundary: `INV-LICENCE`.
- `AXIOM-METADATA`: crates' declared `license` fields are truthful, except
  where a clarification pins a file hash. Boundary: `INV-LICENCE`.
- `AXIOM-MINVER`: `-Zdirect-minimal-versions` selects the lowest admitted
  version for direct dependencies. Source: the Cargo book's unstable-features
  page. Boundary: `INV-FLOOR`.
- `AXIOM-TOML`: the `toml` crate parses `Cargo.toml` as Cargo does for the
  keys this repository uses. Boundary: `L-EXTRACT`.

### Methods per obligation

`L-EXTRACT`:

- Method: `rstest` table of named cases plus a `proptest` property.
- Rationale: the input space is structured (string or table forms with
  optional keys) and small enough to generate fully; named cases document the
  rejected forms readers care about.
- Domain: generated declarations combining name (from a fixed alphabet),
  form (string or table), version (major, minor, patch drawn from 0–99 with
  optional omitted minor or patch), operator prefix (none, `^`, `~`, `=`, `>=`,
  `<`, `*` wildcard component, or two comma-joined comparators),
  `default-features` (absent, true, false), features (zero to three names), and
  source key (none, `git`, `path`).
- Artefact: `tests/dependency_baseline_property.rs` and
  `tests/support/dependency_baseline/manifest.rs`.
- Evidence: `cargo nextest run --test dependency_baseline_property`; before the
  green step the function is absent and compilation fails with E0425 or E0432.
- Oracle: the generator records the expected outcome as it constructs each
  value (accepted with known fields, or rejected with a known reason), so the
  oracle is the construction, not the code under test.
- Non-vacuity: use `prop_oneof!` with weighted branches and tally outcomes
  through a classification helper asserted in a companion `#[test]`-style
  sampling check: at least one accepted and one of each rejection reason must
  appear in 256 cases. Seeded fault: temporarily treat `Op::Tilde` as accepted;
  the property must fail and shrink to a single `~` case. Revert and delete any
  `proptest-regressions/` file the fault created.

`INV-CARET`, `INV-REGISTER`, `INV-NO-FEATURES` on the real repository:

- Method: integration tests over the real `Cargo.toml` and
  `docs/dependency-baseline.md`, plus `rstest` negative cases over inline
  fixture strings, plus `rstest-bdd` scenarios for the contributor-facing
  outcomes.
- Rationale: these are finite facts about two files; exhaustive generation
  adds nothing beyond `L-EXTRACT`.
- Artefact: `tests/dependency_baseline_contract.rs`,
  `tests/dependency_baseline_bdd.rs`,
  `tests/features/dependency_baseline.feature`.
- Evidence:
  `cargo nextest run --test dependency_baseline_contract --test dependency_baseline_bdd`.
  Red: before the register exists the contract test fails reading
  `docs/dependency-baseline.md`; with an empty register table it fails listing
  all 24 crates as missing.
- Non-vacuity: fixture cases must reject an unregistered crate, a registered
  crate absent from the manifest, a `default-features` mismatch, a feature-set
  mismatch, `"~1.2"`, `">=1"`, `"1.*"`, `{ git = "…" }`, and a `[features]`
  table, each with a diagnostic naming the crate or table. A register row
  formatted by `make fmt` must still parse (R8).

`INV-LICENCE`:

- Method: `make deny` using `cargo-deny` (end-to-end over the real graph).
- Rationale: SPDX evaluation is the tool's contract (`AXIOM-DENY`); the
  repository owns only the policy file.
- Artefact: `deny.toml`, `Makefile` target `deny`, CI step "Check dependency
  licences and sources", contract test `tests/dependency_policy_workflow.rs`
  asserting the step exists, runs `make deny`, has no Dependabot condition, and
  installs the pinned version.
- Evidence: `make deny` prints `bans ok, licenses ok, sources ok`.
- Non-vacuity: run once with a scratch copy of `deny.toml` whose allow-list
  omits `MIT`; it must fail naming at least one MIT-only crate (for example
  `bytes`). Run once with the `mockable` clarification hash altered; it must
  fail on `mockable`. Record both transcripts, then discard the scratch files.

`INV-FLOOR`:

- Method: `make check-dependency-floors` (end-to-end resolution and type
  check).
- Rationale: compatibility is a compiler fact; only compiling at the floor
  establishes it.
- Artefact: `scripts/check-dependency-floors.sh`, Makefile target,
  `.github/workflows/dependency-floors.yml`.
- Evidence: exit 0 with the generated lockfile showing at least `camino`
  1.2.0 and `proptest` 1.9.0, which differ from `Cargo.lock`, proving the check
  exercised floors rather than the committed resolution.
- Non-vacuity: in the scratch copy only, lower one floor to a version known
  to lack an API the code uses (candidate: `hyper-util = "0.1.0"`, before
  `TokioIo` existed; confirm during `EP-M3`) and observe a compile error. Never
  commit the lowered floor.

### Formal verification decision

No Verus proof and no Kani harness (decision D10). The obligations are set
equality between two parsed files, a classification delegated to `semver`, and
two properties of external tools' outputs. None is contractual business logic
or a lemma about Peregrine's semantics; a Verus proof of set equality would
restate the code. Kani would explore the extraction function, but the property
test already covers its whole structured domain with a constructive oracle. The
roadmap's first Verus obligation, the lifecycle stop/dispatch rule I1, belongs
to task 1.2.2, which will introduce the Verus toolchain and any `vstd`
dependency under the policy this task establishes.

### Suggested BDD features

Proposed path: `tests/features/dependency_baseline.feature`, bound by
`tests/dependency_baseline_bdd.rs` with `rstest-bdd` 0.6.0. Scenarios operate
on fixture manifests and registers, not on the real files.

```gherkin
Feature: Dependency baseline contract
  Contributors learn at test time when a dependency change needs review.

  Scenario: The recorded baseline matches the manifest
    Given a manifest declaring runtime dependency "bytes" with requirement "1.12.1"
    And a register recording runtime dependency "bytes" with default features
    When the dependency baseline is checked
    Then the check passes

  Scenario: An unrecorded dependency is reported
    Given a manifest declaring dev dependency "insta" with requirement "1.43.0"
    And a register with no entry for "insta"
    When the dependency baseline is checked
    Then the check fails naming "insta" as unrecorded

  Scenario: A non-caret requirement is rejected
    Given a manifest declaring runtime dependency "http" with requirement "~1.5"
    And a register recording runtime dependency "http" with default features
    When the dependency baseline is checked
    Then the check fails naming "http" with requirement "~1.5"

  Scenario: Introducing a crate feature requires the feature matrix
    Given a manifest that declares a features table with feature "spike"
    When the dependency baseline is checked
    Then the check fails citing the feature-matrix rule in design section 11
```

These scenarios trace to `REQ-REGISTER`, `REQ-CARET`, and `REQ-FEATURES`, and
are implemented in `EP-M1`.

## Plan of work

### `EP-M0` record the baseline gates

Run the full gate sequence on the unchanged branch and record pass or fail with
log paths in `Artefacts and notes`. Any failure here is pre-existing and must
be recorded separately from regressions, with its evidence, before `EP-M1`
begins. No files change except this plan.

### `EP-M1` manifest contract and dependency register

Assigned: `REQ-CARET`, `REQ-REGISTER`, `REQ-FEATURES`, decisions D5–D7.

Add `semver = "1.0.28"` to `[dev-dependencies]` in alphabetical position.

Create `tests/support/dependency_baseline/` with three files:

- `mod.rs`: module documentation and re-exports.
- `manifest.rs`: `DependencyKind`, `DependencyDecl`, `ManifestError`,
  `extract_dependencies`, and `has_features_table` (signatures in
  `Interfaces and dependencies`).
- `register.rs`: `RegisterEntry`, `RegisterError`,
  `fn parse_register(markdown: &str) -> Result<Vec<RegisterEntry>, RegisterError>`,
  and
  `fn compare(manifest: &[DependencyDecl], register: &[RegisterEntry]) -> BaselineDiff`
  where `BaselineDiff` holds `missing`, `extra`, and `mismatched` lists and
  implements `Display` for the failure message.

Errors use `thiserror` (already resolved; add it to dev-dependencies only if
the integration test crate cannot reach the runtime one, which it can, since
integration tests may use the package's dependencies). Each function stays
under the Clippy limits in `clippy.toml` (cognitive complexity 9, 70 lines,
four arguments).

Red: write `tests/dependency_baseline_contract.rs` with these tests, and run
`cargo nextest run --test dependency_baseline_contract`; it fails to compile
because the support module does not exist yet:

- `requirements_are_caret` over the real manifest.
- `register_matches_manifest` over the real manifest and register.
- `crate_declares_no_features` over the real manifest.
- An `rstest` table `rejects_invalid_declarations` with the fixture cases in
  `Verification plan`.

Green: implement the support module, then create `docs/dependency-baseline.md`
with the register table between `<!-- dependency-register:start -->` and
`<!-- dependency-register:end -->`. Columns: crate (in backticks), kind,
default features (`on` or `off`), extra features (comma-separated backticked
names or `none`), purpose, consumers. The register test passes only when all 24
rows are correct.

Then add the property test (`tests/dependency_baseline_property.rs`) and the
BDD feature and bindings, each red before its supporting step exists. Use
`googletest` matchers (`elements_are!`, `contains_substring`) and
`pretty_assertions::assert_eq` for structured comparisons, and `rstest`
fixtures for loading the real files.

Register the real-file tests with no special nextest group; they start no child
Cargo.

Refactor, then run the full gates.

### `EP-M2` licence, source, and ban gate

Assigned: `REQ-LICENCE`, decisions D3–D4.

Red: write `tests/dependency_policy_workflow.rs` asserting that
`.github/workflows/ci.yml` job `build-test` has a step named "Install
cargo-deny" running `cargo binstall --no-confirm --locked cargo-deny@0.20.2`, a
later step "Check dependency licences and sources" running `make deny`, no
`if:` condition on either, and that the `deny` step precedes "Lint". Also
assert that the Makefile has a `deny` target whose dry run (`make -n deny`)
invokes `cargo deny` with `--offline --locked` and
`check licenses bans sources`. Run it and the existing workflow contract tests;
expect the new test to fail and record which existing tests need their step
lists updated.

Green: write `deny.toml` per D4. Obtain the `mockable` clarification hash from
`cargo deny` output with the clarification omitted, then pin it. Add to the
`Makefile`:

```makefile
CARGO_DENY ?= cargo deny

deny: ## Check dependency licences, sources, and bans offline
	$(CARGO) fetch --locked --target "$$($(RUSTC) -vV | sed -n 's/^host: //p')"
	$(CARGO_DENY) --offline --locked check licenses bans sources
```

(match the existing `fetch` recipe in the `test` target rather than this sketch
if they differ). Add the two CI steps after "Audit dependencies" and before
"Lint". Update the step lists pinned in the workflow contract tests. Add `deny`
to `make help` output by its `##` comment. Run the non-vacuity controls from
`Verification plan`.

Refactor, then run the full gates plus `make deny`.

### `EP-M3` caret-floor compatibility check

Assigned: `REQ-FLOOR`, decision D8.

Write `scripts/check-dependency-floors.sh` following
`docs/scripting-standards.md` (strict mode, `build-tools:`-style diagnostics,
no network assumptions beyond Cargo's). It removes and recreates
`target/dependency-floors/`, runs `git archive HEAD | tar -x`, runs
`cargo generate-lockfile -Zdirect-minimal-versions`, prints the direct
dependencies' floor versions, and runs
`cargo check --all-targets --all-features --locked` there with
`CARGO_TARGET_DIR=target/dependency-floors/target`. Add Make target
`check-dependency-floors` calling it. Add
`.github/workflows/dependency-floors.yml` (pull requests touching `Cargo.toml`
or `Cargo.lock`, a weekly schedule, and dispatch) that installs build tools and
runs the target; pin actions by SHA as `ci.yml` does.

Red-Green here is evidential rather than test-first: run the target; if a floor
fails, the failure is the red evidence; raise that floor in `Cargo.toml` to the
lowest passing version, update `Cargo.lock` with
`cargo update -p <crate> --precise <locked version>` only if needed to keep the
lockfile unchanged, and rerun. Record each raise in the register and in
`Decision log`. Add a contract assertion to
`tests/dependency_policy_workflow.rs` that the new workflow runs
`make check-dependency-floors`. Run the negative control from
`Verification plan`.

Then run the full gates.

### `EP-M4` development-dependency feature narrowing

Assigned: `REQ-FEATURES`, decision D9.

For each candidate in turn (`proptest`, then `rstest-bdd`, then any other
development dependency whose defaults `cargo tree -e features` shows as
unused), set `default-features = false` with only the needed features, run
`make lint` and `make test`, and keep the change only if both pass. Record the
package count before and after from
`cargo metadata --locked --offline --format-version 1` and the decision per
crate. Update the register; the `EP-M1` contract test enforces the update. Run
`make deny`.

Commit each kept narrowing separately.

### `EP-M5` documentation closure

Assigned: `REQ-BASELINE`, `REQ-REGISTER`, `GAP-UNDEFINED`, `GAP-DOCS`.

- Complete `docs/dependency-baseline.md`: purpose; "Supported baseline"
  (decision D2); "Admission procedure" for adding or changing a dependency; the
  register; "Licence review" dated with the lockfile revision, the allow-list
  rationale, the `mockable` clarification, and the summary from
  `cargo deny --offline --locked list --layout license`; "Caret floors" with
  the floors evidence and any raises; "Known follow-ups" (`serde_yaml`).
- Add `docs/adrs/adr-0008-dependency-and-licence-baseline.md` per
  `docs/documentation-style-guide.md`, recording D2–D8 as one policy.
- Update `docs/peregrine-design.md` §12: replace "The first build spike
  selects mutually compatible caret requirements and records the compiler
  floor" narrative with a pointer to the register and ADR 008, keeping the
  candidate-family list.
- Update `docs/developers-guide.md`: a "Dependency baseline" section (when
  to run `make deny` and `make check-dependency-floors`, the admission
  procedure, the support module's reuse policy: owned by the contract tests,
  callable only from `tests/`, not a production helper); replace the spike
  section's dependency list with a pointer to the register.
- Update `docs/repository-layout.md` (`deny.toml`, the register, the new
  script and workflow), `docs/contents.md` (register under "Engineering
  practice"; ADR 008 alongside the other ADRs), and `AGENTS.md` (list
  `make deny` and `make check-dependency-floors` where the gates are described).
- Update `docs/users-guide.md` "Build the Repository" with the supported
  baseline and the two new targets.
- Mark roadmap 1.2.1 done in `docs/roadmap.md`, and update its introductory
  "Only decision/specification tasks … are done" sentence.

Run `make fmt`, `make markdownlint`, `make spelling`, `make nixie`, then the
three gates.

## Milestones and plateaus

- `EP-M0`: the base's gate status is known. Requirements: `REQ-BASELINE`
  (evidence). Acceptance: logs exist. Conformance check:
  - no change to the repository.
  - baseline failures, if any, recorded separately.
  Recovery: rerun. Remaining gaps: all. Compatibility: none.
- `EP-M1`: every direct dependency is recorded and checked against the
  manifest. Requirements: `REQ-CARET`, `REQ-REGISTER`, `REQ-FEATURES`.
  Acceptance: the four contract tests, the property test, and four BDD
  scenarios pass; fixture negative cases fail as specified. Conformance check:
  - requirements satisfied by passing tests.
  - design §11 rule encoded in the tripwire.
  - no `src/` change; only `semver` added.
  - no trust-boundary or persisted-format change.
  - trace links current.
  Recovery: revert the milestone's commits. Remaining gaps: licence, floors,
  narrowing, prose. Compatibility: none.
- `EP-M2`: licence, source, and wildcard policy enforced on every PR.
  Requirements: `REQ-LICENCE`. Acceptance: `make deny` passes; negative
  controls fail as specified; workflow contract tests pass. Conformance check:
  - advisories untouched; `make lint` shape unchanged.
  - only `cargo-deny` 0.20.2 added as a tool.
  - trace links current.
  Recovery: revert; CI returns to its prior step list. Remaining gaps: floors,
  narrowing, prose. Compatibility: none.
- `EP-M3`: floors proven compatible. Requirements: `REQ-FLOOR`. Acceptance:
  `make check-dependency-floors` exits 0 with floors differing from the lock;
  negative control fails. Conformance check:
  - at most three floors raised.
  - no runtime requirement changed without escalation.
  Recovery: revert; floors return to their previous values. Remaining gaps:
  narrowing, prose. Compatibility: none.
- `EP-M4`: development features minimal. Requirements: `REQ-FEATURES`.
  Acceptance: register and manifest agree; gates pass; package count recorded.
  Conformance check:
  - runtime features unchanged.
  Recovery: revert individual narrowing commits. Remaining gaps: prose.
  Compatibility: none.
- `EP-M5`: documentation and roadmap closed. Requirements: `REQ-BASELINE`,
  `REQ-REGISTER`. Acceptance: documentation gates and the three Rust gates
  pass; roadmap shows 1.2.1 done. Conformance check:
  - design §12 and ADR 008 agree with `deny.toml` and the register.
  - no historical document edited.
  Recovery: revert. Remaining gaps: `serde_yaml` follow-up. Compatibility: none.

No milestone introduces compatibility machinery: every surface touched here is
private test support or repository configuration.

## Concrete steps

Working directory: the worktree root. Use `set -o pipefail` in every shell that
pipes through `tee`. Logs follow
`/tmp/$ACTION-peregrine-web-1-2-1-minimal-dependency-and-feature-baseline.out`.

Gate sequence (run sequentially, never in parallel):

```sh
set -o pipefail
make check-fmt 2>&1 | tee /tmp/check-fmt-peregrine-web-1-2-1-minimal-dependency-and-feature-baseline.out
make lint 2>&1 | tee /tmp/lint-peregrine-web-1-2-1-minimal-dependency-and-feature-baseline.out
make test 2>&1 | tee /tmp/test-peregrine-web-1-2-1-minimal-dependency-and-feature-baseline.out
make deny 2>&1 | tee /tmp/deny-peregrine-web-1-2-1-minimal-dependency-and-feature-baseline.out   # from EP-M2
```

Documentation gates (`EP-M5`, and whenever Markdown changes):

```sh
make fmt
make markdownlint 2>&1 | tee /tmp/markdownlint-peregrine-web-1-2-1-minimal-dependency-and-feature-baseline.out
make spelling 2>&1 | tee /tmp/spelling-peregrine-web-1-2-1-minimal-dependency-and-feature-baseline.out
make nixie 2>&1 | tee /tmp/nixie-peregrine-web-1-2-1-minimal-dependency-and-feature-baseline.out
```

Focused commands:

```plaintext
EP-M1 red:    cargo nextest run --test dependency_baseline_contract -> E0432 unresolved import `support::dependency_baseline`
EP-M1 green:  cargo nextest run -E 'binary(/^dependency_baseline/)' -> all pass
EP-M2 red:    cargo nextest run --test dependency_policy_workflow -> fails: no step "Check dependency licences and sources"
EP-M2 green:  same -> passes; make deny -> "bans ok, licenses ok, sources ok"
EP-M3:        make check-dependency-floors -> exit 0; floors printed include camino 1.2.0 and proptest 1.9.0
```

Gates are delegated to the `scrutineer` agent where practical; when it reports
a failure, read its cited log rather than re-running the gate.

## Validation and acceptance

Acceptance is behaviour a reviewer can reproduce:

- `make check-fmt`, `make lint`, and `make test` exit 0 on the pinned nightly;
  the nextest summary includes the new tests.
- `make deny` prints `bans ok, licenses ok, sources ok`.
- `make check-dependency-floors` exits 0 and prints floor versions that differ
  from `Cargo.lock` for at least two crates.
- Adding `insta = "1"` to `[dev-dependencies]` in a scratch edit makes
  `cargo nextest run --test dependency_baseline_contract` fail naming `insta`
  as unrecorded; reverting restores green.
- Changing `http = "1.5.0"` to `http = "~1.5"` in a scratch edit makes the same
  test fail naming `http` and `~1.5`.
- CI's `build-test` job shows the "Check dependency licences and sources" step
  passing, including on a Dependabot PR once one arrives.

Quality criteria:

- Tests: the gate sequence above passes.
- Verification: `L-EXTRACT` property and seeded fault recorded;
  `INV-LICENCE` and `INV-FLOOR` negative controls recorded.
- Lint: `make lint` (Clippy and Whitaker) clean.
- Security: `make deny` clean; `make audit` in CI clean.

## Idempotence and recovery

Every step can be rerun. `make check-dependency-floors` deletes and recreates
its scratch tree. Negative controls run on scratch copies; any accidental edit
to `Cargo.toml`, `deny.toml`, or `Cargo.lock` is reverted with
`git checkout -- <file>`, and `git status --short` must be clean of them before
committing. Seeded faults in the support module are reverted before the
milestone's commit, and any `proptest-regressions/` file they created is
deleted. Remove `target/dependency-floors/` when the work is complete to save
disk and inotify watches.

## Artefacts and notes

Planning-time evidence (recorded before approval; not implementation):

```plaintext
$ cargo deny --version            -> cargo-deny 0.20.2
$ cargo deny --offline --locked --config /tmp/peregrine-1-2-1-probe/deny.toml check licenses bans sources
warning[no-license-field]: ... 'mockable = 3.0.0'
bans ok, licenses ok, sources ok   (exit 0)
$ cargo generate-lockfile -Zdirect-minimal-versions   (scratch copy, online)
Adding camino v1.2.0, proptest v1.9.0, rstest v0.27.0, serde v1.0.228, toml v1.1.3+spec-1.1.0
$ make check-fmt                  -> "33 files left unchanged.", exit 0
```

Implementation evidence is appended per milestone below.

## Interfaces and dependencies

New development dependency: `semver = "1.0.28"` (`MIT OR Apache-2.0`), already
resolved. New host tool: `cargo-deny` 0.20.2. No runtime change.

New test-support interfaces, in `tests/support/dependency_baseline/`:

```rust
/// Which manifest table declared a dependency.
pub enum DependencyKind { Runtime, Dev, Build }

/// One direct dependency as the baseline contract sees it.
pub struct DependencyDecl {
    pub name: String,
    pub kind: DependencyKind,
    pub requirement: semver::VersionReq,
    pub default_features: bool,
    pub features: std::collections::BTreeSet<String>,
}

pub fn extract_dependencies(manifest: &toml::Table) -> Result<Vec<DependencyDecl>, ManifestError>;
pub fn has_features_table(manifest: &toml::Table) -> bool;
pub fn parse_register(markdown: &str) -> Result<Vec<RegisterEntry>, RegisterError>;
pub fn compare(manifest: &[DependencyDecl], register: &[RegisterEntry]) -> BaselineDiff;
```

Reuse policy: the module belongs to the dependency contract tests. Other
integration tests may call it, but production code must not, and no second
manifest or register parser may be added; extend this one. Before writing it,
confirm with `rg -n 'dependencies' tests/support` that no existing support
module already extracts manifest dependencies (the reconnaissance found `toml`
used only for toolchain, Cargo configuration, and Clippy contracts).

New Make targets: `deny`, `check-dependency-floors`. New files: `deny.toml`,
`scripts/check-dependency-floors.sh`, `.github/workflows/dependency-floors.yml`,
`docs/dependency-baseline.md`,
`docs/adrs/adr-0008-dependency-and-licence-baseline.md`.

## Revision note

- 2026-11-14: initial draft from Wyvern reconnaissance and planning probes.
