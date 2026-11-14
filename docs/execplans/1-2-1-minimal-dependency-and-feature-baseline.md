# 1.2.1 Establish the minimal dependency and feature baseline

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan` must
be kept up to date as work proceeds.

Status: DRAFT (expert review applied; awaiting user approval)

Branch: `1-2-1-minimal-dependency-and-feature-baseline`

Roadmap item: `1.2.1` in `docs/roadmap.md`.

## Purpose / big picture

Peregrine Web is an experimental, resource-oriented HTTP framework written in
Rust. Today the crate's only library content is the experimental specimen under
`src/spike/`, which roadmap task 1.1.3 compiled to show that a resource can own
its policy and dependencies across asynchronous middleware. That spike added
its dependencies with caret requirements and recorded a one-off licence,
minimum-compiler, and feature review in
`docs/resource-policy-ownership-spike.md` under "Dependency review". The spike
ledger deliberately freezes that review as historical evidence and states:
"task 1.2.1 still owns the maintained dependency and feature baseline".

A caret requirement is Cargo's default version requirement: `bytes = "1.12.1"`
(equivalently `^1.12.1`) accepts any SemVer-compatible release at or above
1.12.1 and below 2.0.0. The lowest version a caret requirement admits is its
floor. A floor is honest when the code still compiles against exactly that
version.

Roadmap task 1.2.1 asks whether a contributor can build and verify the selected
contract through repository entrypoints, so that feature work can rely on the
baseline. Its success criterion is: "compatible caret requirements and licence
review are recorded; `make check-fmt`, `make lint`, and `make test` pass on the
supported baseline."

Today nothing re-checks the frozen review. A contributor can add a dependency,
widen its features, or write a non-caret requirement without any test failing;
Dependabot updates the lockfile daily without a licence check on its pull
requests; nothing proves that a floor such as `proptest = "1.9.0"` (resolved to
1.11.0 in `Cargo.lock`) still compiles; and the term "supported baseline" is
defined nowhere in the repository.

After this change a contributor can observe four things:

1. Every direct dependency in `Cargo.toml` carries a short record beside it in
   `[package.metadata.dependency-baseline]`: why it is needed, which repository
   paths use it, and why any non-default features are set. The
   `tests/dependency_baseline.rs` contract fails, naming the crate and the
   broken rule, when a dependency lacks a record, a record names no dependency,
   a requirement is not a full caret version, a declaration uses an unreviewed
   form (`optional`, `git`, `path`, a platform-specific table, `[patch]`), or
   the crate gains a Cargo feature without the feature-matrix work required by
   design §11.
2. `make deny` checks the licence of every resolved package, for every
   platform and including development dependencies, against the allow-list in
   `deny.toml`. It rejects sources outside crates.io and wildcard requirements,
   and runs on every pull request, Dependabot's included.
3. `make check-dependency-floors` resolves each direct dependency at exactly
   its floor in a scratch copy of the working tree, fails if any floor is
   silently lifted, and type-checks all targets there.
4. `docs/dependency-baseline.md` defines the supported baseline and records a
   dated licence review and the floor evidence; ADR 008 records the policy; and
   `make check-fmt`, `make lint`, and `make test` pass on that baseline.

Existing behaviour is preserved: no public Rust API changes, the spike module
keeps its surface, and no runtime dependency is added, removed, or re-featured.
Development-only default features may be narrowed where the gates prove them
unused.

## Constraints

- Implementation begins only after the user explicitly approves this plan.
- No file under `src/` changes. `peregrine_web::spike` and
  `peregrine_web::greet` keep their signatures.
- No runtime (`[dependencies]`) addition, removal, requirement change, or
  feature change. A runtime floor found dishonest in `EP-M3` is an escalation,
  not an automatic raise.
- The only authorized new direct dependency is the development dependency
  `semver = "1.0.28"` (licence `MIT OR Apache-2.0`). It is already resolved
  transitively through `rstest-bdd-macros`, so it adds no package to
  `Cargo.lock`. Any other new crate, including `insta`, requires escalation.
- The only authorized new host tool is `cargo-deny` 0.20.2 (licence
  `MIT OR Apache-2.0`, published 2026-07-09, `rust-version` 1.88.0), with its
  version recorded once in `tools/cargo-deny/VERSION`. It is already installed
  on the development host.
- The crate must not gain a `[features]` table, a `build.rs`, a
  `[build-dependencies]` table, a `rust-version` key, or a `publish` key in
  this task (decisions D2 and D7; `publish` is open question Q-PUBLISH).
- `make lint` keeps its current two-phase contract (`lint-clippy` then
  `lint-whitaker`), and `make all` keeps its current composition. Both are
  pinned by `tests/make_cargo_routes.rs`, `tests/make_gate_order.rs`, and
  `tests/build_standard_contract.rs`.
- Advisory auditing remains with `make audit` (`cargo audit`) and keeps its
  Dependabot exemption. `deny.toml` must not enable the advisories check.
- Historical records stay untouched: the "Dependency review" section of
  `docs/resource-policy-ownership-spike.md` and every completed ExecPlan.
- Repository rules from `AGENTS.md` apply: no code file over 400 lines; each
  module opens with a `//!` comment; every `assert!`-family call has a message;
  `expect` only in tests; no `unwrap`; lint suppressions only as a tightly
  scoped last resort with a reason; en-GB-oxendict spelling; Markdown wrapped
  at 80 columns and code blocks at 120.
- Tests read repository files relative to `env!("CARGO_MANIFEST_DIR")` and
  never call `std::env::var` (disallowed in `clippy.toml`).
- Rust builds go through the host's compile admission (`RUSTC_WRAPPER`); never
  bypass it and never create an isolated Cargo cache.
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
net lines (additions minus deletions), are enforced per category, and spare
capacity in one category never offsets another.

Classification is by whole file:

- Code: `tests/**`, `scripts/**`, and `Makefile`.
- Configuration: `Cargo.toml`, `Cargo.lock`, `deny.toml`, `tools/**`,
  `.config/**`, `.github/**`, `typos.local.toml`, and `typos.toml` (generated by
  `make spelling`; counted, not excluded).
- Documentation: `docs/**/*.md` except ExecPlans, plus `AGENTS.md` and
  `README.md`.

Budget survey. The comparable plans,
`docs/execplans/1-1-3-compile-resource-policy-and-async-ownership-spike.md` and
`docs/execplans/1-1-resolve-decisions-to-mitigate-api-rework.md`, set no
numeric budgets and raised none; their only size pressure was the 400-line file
rule, met by splitting files. There is no history of repeated increases. The
estimates come from the planned artefacts and the contract tests the expert
review found affected:

- Code, about 870 lines: the contract test binary and its three support files
  (about 450 including the rstest tables, the property, and the BDD bindings),
  the feature file (about 40), updates to six existing workflow and Make
  contract files (about 160), the `deny` and floors Make recipes (about 30),
  and the floors script (about 120). The limit allows about 25% contingency.
- Configuration, about 230 lines: metadata records for 25 dependencies (about
  125), `deny.toml` (about 55), the floors workflow (about 45), CI additions
  (about 10), and the version file.
- Documentation, about 520 lines: the baseline record (about 230), ADR 008
  (about 110), and updates to seven live documents (about 180).

Limits:

- Files: stop and escalate if more than 40 non-ExecPlan files change (about
  32 are planned).
- Code: stop and escalate above 1,100 net lines.
- Configuration: stop and escalate above 300 net lines.
- Documentation: stop and escalate above 650 net lines.
- Interface: any change under `src/` requires escalation.
- Dependencies: any direct dependency beyond `semver` (dev), or any runtime
  dependency change, requires escalation.
- Floors: if more than three development floors must be raised, or any runtime
  floor, stop and escalate; that many dishonest floors calls for a wider review.
- Licences: if a resolved package needs a licence outside the pre-approved
  set in decision D4, stop and escalate rather than widening the set.
- Architecture: if `cargo deny` cannot run offline once dependencies are
  fetched, or if `make lint` or `make all` would have to change shape, stop and
  escalate.
- Iterations: if a gate still fails after three fix attempts for the same
  cause, stop and escalate.
- Time: if a milestone takes more than four hours of active work, stop and
  escalate.
- Ambiguity: if the `insta` deviation (D11) or an open question below is
  contested during implementation, stop and present the options.

## Risks

- R1: A caret floor may be dishonest because code uses an API added after the
  floor. Severity: medium. Likelihood: medium. Mitigation: `EP-M3` raises a
  development floor only to the lowest passing version and records each raise;
  a runtime floor escalates.
- R2: The floors check depends on the live crates.io index, because
  `-Zdirect-minimal-versions` resolves transitive dependencies at their newest
  releases. A new upstream release or a yanked floor can fail it without a
  repository change. Severity: medium. Likelihood: medium over months.
  Mitigation: the script prints the generated lockfile's direct versions and
  the package that forced any lift; the floors workflow is not a required check;
  `docs/developers-guide.md` names the maintainer (`leynos`) as triage owner
  and gives the runbook.
- R3: `serde_yaml` 0.9.34 is published as `+deprecated` and is used by 15 test
  files. Severity: medium for maintenance, low for this task (development
  only). Likelihood: certain. Mitigation: record it in its metadata and the
  baseline record; propose replacement as a separate follow-up (D13).
- R4: Adding CI steps and Make goals breaks closed-world contract tests that
  allow only known steps and goals. Affected owners:
  `tests/support/act_workflow_policy.rs` (step scripts, `if:`, and `env`),
  `tests/support/act_workflow_contract.rs` (exactly 19 hosted steps, in order),
  `tests/support/act_workflow_command.rs` (step-to-target map),
  `tests/support/act_workflow_smoke.rs` (allowed `make:` and `binstall`
  commands), `tests/support/build_tool_workflows.rs` (known Make goals across
  workflows), and `tests/make_cargo_routes.rs` (Cargo subcommands per target).
  Severity: medium. Likelihood: certain. Mitigation: `EP-M2` and `EP-M3` update
  each owner in the same commit as the workflow or Makefile change, and run
  them in the red step.
- R5: Narrowing a development dependency's default features could break an
  invocation such as doctests. Severity: low. Likelihood: low. Mitigation: keep
  each narrowing only after `make lint` and `make test` pass; revert otherwise.
- R6: A transitive licence outside the allow-list could block a grouped
  Dependabot pull request and the security fixes inside it. Severity: medium.
  Likelihood: low with the pre-approved set in D4. Mitigation: the runbook in
  `docs/developers-guide.md`: amend `deny.toml` on `main` through review, then
  comment `@dependabot rebase`, rather than editing the Dependabot branch.
- R7: Tool drift: a nightly bump changes `cargo metadata` output, or a crate
  adopts an SPDX identifier the pinned `cargo-deny` does not know. Severity:
  medium. Likelihood: low. Mitigation: one version source
  (`tools/cargo-deny/VERSION`), a local-version check in `make deny`, and an
  ADR 008 rule that bumping the tool is its own reviewed change.
- R8: The RustSec database cannot be fetched on the development host (the
  planning probe failed in a credential helper), so advisory evidence comes
  from CI. Severity: low. Likelihood: certain on this host. Mitigation:
  advisories stay with `make audit` in CI (D3).

## Progress

- [x] (2026-11-14 11:20Z) Branch renamed; Lody session titled.
- [x] (2026-11-14 11:30Z) Wyvern reconnaissance of manifests, gates, CI,
  design documents, and historical plans.
- [x] (2026-11-14 11:45Z) Planning probes: `cargo-deny` scratch configuration,
  direct-minimal-versions resolution, advisory fetch, base `make check-fmt`.
- [x] (2026-11-14 12:10Z) Initial draft written and committed.
- [x] (2026-11-14 13:05Z) Community-of-experts review (structure and
  contracts; failure modes and cost; alternatives and viability) applied; see
  revision note.
- [ ] Plan approved by the user.
- [ ] `EP-M0` Baseline gates recorded on the unchanged base.
- [ ] `EP-M1` Manifest contract and in-manifest register.
- [ ] `EP-M2` Licence, source, and ban gate.
- [ ] `EP-M3` Caret-floor compatibility check.
- [ ] `EP-M4` Development-dependency feature narrowing.
- [ ] `EP-M5` Documentation closure and roadmap update.

## Surprises & discoveries

- Observation: the spike's dependency review exists but is frozen by design.
  Evidence: `docs/resource-policy-ownership-spike.md` "Dependency review",
  closing paragraph. Impact: this task writes a new, living record.
- Observation: `docs/developers-guide.md` "The experimental spike module"
  lists the spike's development dependencies but omits `camino`, `cap-std`,
  `mockable`, `serde`, `serde_yaml`, and `toml`, which serve the contract
  tests. Impact: `EP-M5` replaces that list with a pointer to the register.
- Observation: the RustSec database at
  `~/.cargo/advisory-dbs/advisory-db-3157b0e258782691` was last updated
  2026-10-14 and cannot be refreshed on this host. Evidence:
  `/tmp/deny-advisories-peregrine-web-1-2-1.out`. Impact: supports D3.
- Observation: `--lockfile-path` is not available on the pinned Cargo
  (`cargo 1.101.0-nightly (3c747ea3e 2026-11-03)`). Impact: the floors check
  works on a scratch copy (D8).
- Observation: the duplicate `toml` 0.5.11 and `thiserror` 1.0.69 enter through
  `rstest-bdd` (via `i18n-embed`), and `foldhash` (the only `Zlib` package)
  enters only through `rstest-bdd`'s `hashbrown`. Evidence: `cargo tree -i`.
  Impact: duplicates warn rather than deny (D4); `Zlib` is a development-only
  licence, which makes it the right negative control for `include-dev`.
- Observation: a host-only `cargo fetch` caches 205 of the 226 packages; the 21
  others (`windows-sys`, `windows_*`, `r-efi`, `wasi`, and similar) are
  platform-specific, and `cargo deny` reads metadata for all platforms.
  Evidence: `cargo metadata --filter-platform x86_64-unknown-linux-gnu` versus
  unfiltered (expert review). Impact: `make deny` fetches every platform first
  (D3).
- Observation: `semver` parses `"^1.*"` as a caret comparator with no minor
  version, and accepts `"1"`, `"0.1"`, and `"1.2.3-alpha"` as caret. Evidence:
  `semver-1.0.28/src/parse.rs`, `comparator` (expert review). Impact:
  `INV-CARET` also requires explicit minor and patch numbers and no pre-release
  (D5).
- Observation: the only Dependabot merge in this repository (`bb12b23`, the
  `cap-std` 3 to 4 bump) edited both `Cargo.toml` and `Cargo.lock`; the draft's
  claim that Dependabot moves only the lockfile was unverified. Impact: the
  floors rationale now rests on code adopting newer APIs, which happens
  whatever Dependabot's strategy.
- Observation: an earlier repository memory recorded `make check-fmt` failing
  on `main`; at `0155d04` it passes ("33 files left unchanged"). Impact: no
  pre-existing formatting failure needs separating from regressions.

## Decision log

- D1. Decision: scope 1.2.1 as converting the spike's frozen review into a
  maintained baseline: a policy (ADR 008), a register kept in the manifest, a
  dated baseline record, three mechanical guards (manifest contract,
  `cargo-deny` gate, floors check), and narrowing of unused development
  features. Framework dependency selection (`matchit`, `tracing`, `metrics`,
  runtime `serde`) stays with the slices that need them. Rationale: a review
  nothing re-checks stops being true at the next dependency change; the
  rejected alternative, re-recording the tables once, would meet the letter of
  "recorded" but not "reproducible" (roadmap goal G4: "reproducible checks").
  Date/Author: 2026-11-14, planning agent.
- D2. Decision: the supported baseline is the toolchain pinned in
  `rust-toolchain.toml` (`nightly-2026-11-06`, rustc 1.101.0-nightly
  `249c1e4e3`) on the `x86_64-unknown-linux-gnu` host configured in
  `.cargo/config.toml`, with the committed `Cargo.lock`, building all targets
  with all features. Do not declare `rust-version`. Rationale: terms of
  reference Q5 and ADR 007 leave the compiler floor open;
  `rust-version = "1.101"` would invite an untested stable 1.101 that cannot
  honour `.cargo/config.toml`'s unstable keys. This definition is a statement
  of what the gates verify, not a support commitment. Date/Author: 2026-11-14,
  planning agent.
- D3. Decision: add a separate `make deny` target that fetches every platform's
  packages (`$(CARGO) fetch --locked`, no `--target`), checks the local
  `cargo-deny` version against `tools/cargo-deny/VERSION`, and runs
  `$(CARGO) deny --offline --locked check licenses bans sources`. CI runs it in
  `build-test` with no Dependabot exemption, after "Audit dependencies" and
  before "Lint". Advisories stay with `make audit`. Rationale: the licence
  result depends only on the lockfile, so it cannot deadlock Dependabot, and
  Dependabot pull requests are where new transitive packages arrive. Folding it
  into `make lint` or `make all` would change contracts pinned by three test
  files and require `cargo-deny` for every lint run although it compiles
  nothing. Rejected alternatives: `cargo-deny` advisories (duplicates
  `cargo audit`, needs network) and a handwritten SPDX evaluator in `make test`
  (re-implements a maintained tool). Date/Author: 2026-11-14, planning agent;
  revised after expert review.
- D4. Decision: `deny.toml` pre-approves a documented set of permissive,
  ISC-compatible licences: `0BSD`, `Apache-2.0`,
  `Apache-2.0 WITH LLVM-exception`, `BSD-2-Clause`, `BSD-3-Clause`, `BSL-1.0`,
  `CC0-1.0`, `ISC`, `MIT`, `MIT-0`, `Unicode-3.0`, `Unicode-DFS-2016`, and
  `Zlib`. Copyleft (GPL, LGPL, AGPL, MPL, EUPL), non-OSI, and unknown licences
  stay denied. It sets `include-dev = true`, `confidence-threshold = 0.93`,
  `unused-allowed-license = "allow"`, no `[graph] targets` restriction (every
  platform is reviewed), unknown registries and Git sources denied,
  `wildcards = "deny"`, and `multiple-versions = "warn"`. No `mockable`
  clarification is added: `cargo-deny` identifies its `LICENSE` file as MIT
  (planning probe), and the detection confidence is recorded in the licence
  review instead. Rationale: the expert review showed an exactly-observed
  allow-list would fail grouped Dependabot pull requests on routine transitive
  changes; a pre-approved permissive set keeps the reviewable boundary
  (copyleft and unknown licences) while the dated licence review records what
  is actually present. Date/Author: 2026-11-14, planning agent; revised after
  expert review.
- D5. Decision: classify requirements with `semver::VersionReq::parse`,
  accepting only exactly one comparator with operator `Op::Caret`, explicit
  minor and patch numbers, and an empty pre-release. Classification is a
  separate function from manifest extraction, so each diagnostic names one
  rule. Rationale: Cargo parses requirements with the same crate, so the check
  shares Cargo's grammar; the extra field checks close the `"^1.*"`, `"1"`, and
  pre-release gaps found in review, and match `AGENTS.md`'s
  `some-crate = "1.2.3"` form, which every current requirement already uses.
  Date/Author: 2026-11-14, planning agent; revised after expert review.
- D6. Decision: keep the register in the manifest, as
  `[package.metadata.dependency-baseline.<table>.<crate>]` records with
  `purpose`, `used-by`, and, where the declaration sets `features` or
  `default-features = false`, `feature-rationale`. The contract test checks a
  one-to-one match between declarations and records by (table, crate), that
  `used-by` paths exist, and that `feature-rationale` is present exactly when
  features are customized. It does not check version numbers or licences.
  Rationale: this was the review panel's strongest alternative to a Markdown
  table parsed by a test. It puts the justification in the same diff as the
  declaration, Cargo ignores `package.metadata`, Dependabot edits only version
  strings, and no Markdown formatting can break a Rust test. Each fact has one
  primary mechanical owner (floors: the floors check; licences and sources:
  `cargo-deny`; records and requirement form: the contract test); wildcard and
  non-registry sources are deliberately checked by both `cargo-deny` and the
  contract test as defence in depth. Date/Author: 2026-11-14, planning agent;
  revised after expert review.
- D7. Decision: the baseline declares no Cargo features. The contract test
  fails on a `[features]` table or an `optional = true` declaration (which
  creates an implicit feature), citing design §11's matrix rule. The spike
  stays ungated. Rationale: every runtime dependency serves only the spike,
  which is the library's only content; gating it would create a
  minimal/each/all build matrix with no consumer. Roadmap 2.2.3 decides the
  spike's future. Date/Author: 2026-11-14, planning agent.
- D8. Decision: implement the floors check as
  `scripts/check-dependency-floors.sh`, called by
  `make check-dependency-floors`. It copies the working tree's tracked and
  untracked-but-not-ignored files (`git ls-files -co --exclude-standard`),
  excluding `.cargo/`, into a fresh `target/dependency-floors/tree/`, so that
  only the worktree's own Cargo configuration applies; runs
  `cargo generate-lockfile -Zdirect-minimal-versions` without `--offline`;
  fails unless every direct dependency resolves to exactly its floor, naming
  the package that forced any lift (`cargo tree -i`); then runs
  `cargo check --all-targets --all-features --locked` with
  `CARGO_TARGET_DIR=target/dependency-floors/build`, which survives between
  runs. CI runs it from `.github/workflows/dependency-floors.yml` on pull
  requests touching `src/**`, `tests/**`, `examples/**`, `Cargo.toml`,
  `rust-toolchain.toml`, `.cargo/**`, the script, or the workflow (not
  `Cargo.lock`, which the check discards), weekly, and on dispatch, with
  `concurrency` cancelling superseded runs. It is never a required check.
  Rationale: `--lockfile-path` is unavailable and rewriting the real lockfile
  would be destructive; copying the working tree lets a floor raise be
  re-checked before it is committed; the floor-equality assertion makes the
  check fail rather than pass vacuously when a transitive requirement or a yank
  lifts a floor; `-Zdirect-minimal-versions` rather than `-Zminimal-versions`
  because the Cargo book warns that transitive floors are often undeclared;
  type-checking suffices because a dishonest floor shows up as a missing item
  or signature mismatch. Path filters exclude the lockfile so routine
  Dependabot lockfile updates do not spend 5–10 runner-minutes re-testing
  unchanged inputs. Date/Author: 2026-11-14, planning agent; revised after
  expert review.
- D9. Decision: narrow development dependencies' default features where
  unused, starting with `proptest` (defaults `std`, `fork`, `timeout`,
  `bit-set`; the repository uses only `ProptestConfig { cases, .. }`) and then
  `rstest-bdd` (default `diagnostics`; check `docs/rstest-bdd-users-guide.md`
  for what it provides before removing it). Keep each change only if the gates
  pass. Runtime defaults (`std` for `bytes`, `http`, `thiserror`) stay, since
  the crate is a `std` library. Date/Author: 2026-11-14, planning agent.
- D10. Decision: no Verus proof and no Kani harness in this task. See
  "Formal verification decision". Date/Author: 2026-11-14, planning agent.
- D11. Decision: no `insta` snapshots; this is a recorded deviation from
  `AGENTS.md`'s snapshot guidance, for the user to accept or reject at
  approval. The contract's diagnostics are one plain-text format whose exact
  wording is not a contract; what matters is which crate and which rule fail,
  which `googletest` matchers assert directly. Adding `insta` would admit a new
  crate and its transitive packages under the policy this task creates, for one
  assertion. If the user prefers a snapshot, `EP-M1` adds `insta` through the
  admission procedure and snapshots one combined report alongside the semantic
  assertions. Date/Author: 2026-11-14, planning agent; reworded after expert
  review.
- D12. Decision: ADR 008, "Maintain the dependency and licence baseline", with
  status "Accepted as the repository's dependency-admission policy. It does not
  answer terms of reference Q5 or make a support commitment.", following ADR
  006's scoped wording; the deciding authority is `leynos`, the owner named in
  terms of reference Q6. The ADR records the policy (caret requirements, honest
  floors, licence allow-list and its boundary, in-manifest register,
  supported-baseline definition, tool-bump rule, and a non-registry source
  exception route); mechanics (script, target names, runbooks) live in
  `docs/developers-guide.md`. Date/Author: 2026-11-14, planning agent; revised
  after expert review.
- D13. Decision: keep `serde_yaml` 0.9.34 for now, record it as deprecated
  upstream, and propose its replacement as a separate follow-up. Rationale:
  replacing it touches 15 test files that parse workflows and Make output, an
  unrelated change that would obscure this task's diff. Date/Author:
  2026-11-14, planning agent.
- D14. Decision: put the contract in one test binary,
  `tests/dependency_baseline.rs`, whose support files are child modules loaded
  with `#[path]`, and bind the BDD scenarios in the same binary. Rationale: the
  repository's pattern (`tests/make_cargo_routes.rs`,
  `tests/support/make_fetch_contract_tests.rs`) avoids `dead_code` and
  `unused_imports` warnings that shared support modules raise across several
  binaries. Date/Author: 2026-11-14, planning agent; added after expert review.
- D15. Decision: pin the new CI steps and goals in the existing closed-world
  contract owners listed in R4, rather than adding a separate workflow test;
  floors-workflow assertions check shape (steps and the Make goal), never
  action revisions. Rationale: one owner per contract fact; action pins are
  owned by Dependabot (`docs/developers-guide.md` "Workflow pins and
  Dependabot"). Date/Author: 2026-11-14, planning agent; added after expert
  review.

Open questions for the user (not blocking implementation):

- Q-PUBLISH: should `Cargo.toml` set `publish = false` until the release
  boundary is decided? It is a one-line guard, but a release-policy decision
  outside this task.
- Q-YAML: should the `serde_yaml` replacement become a roadmap task or a
  GitHub issue?
- Q-INSTA: accept D11's deviation, or add `insta` for one snapshot?

## Outcomes & retrospective

Not yet started. Complete at each milestone and at the end, and reconcile
discoveries with the artefacts in `Conformance basis` before setting the status
to `COMPLETE`.

## Context and orientation

Inspected base revision: `0155d04` ("Compile resource-policy and async
ownership spike (1.1.3)"). Line numbers refer to that revision; relocate
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

Development dependencies (18 today; 19 with `semver`), all plain caret
requirements:

- Contract-test infrastructure: `camino`, `cap-std`
  (`features = ["fs_utf8"]`), `mockable` (`default-features = false`, used only
  by `tests/clippy_lint_contract.rs`), `rstest` (`default-features = false`),
  `proptest`, `serde` (`features = ["derive"]`, used by
  `tests/support/act_workflow_model.rs`), `serde_yaml` (15 test files), and
  `toml`.
- Spike witnesses: `googletest`, `pretty_assertions`, `hyper`
  (`features = ["http1", "server"]`) and `hyper-util` (`features = ["tokio"]`)
  in `tests/support/spike/network.rs` only, `serde_json` in
  `tests/resource_policy_example_executable.rs` only, `trybuild` in
  `tests/ownership_contract.rs` only, `tokio` (seven features), `rstest-bdd`,
  and `rstest-bdd-macros`. `rstest-bdd` has no direct `use`; the macros expand
  to paths in it, so it stays.

`Cargo.lock` resolves 225 non-root packages. Versions above floors include
`camino` 1.2.6, `proptest` 1.11.0, `serde` 1.0.229, and `toml` 1.1.6. Duplicate
names (`getrandom`, `io-lifetimes`, `r-efi`, `syn`, `thiserror`,
`thiserror-impl`, `toml`, `windows-sys`) are all transitive.

### Gates and their contract tests

`make check-fmt` runs `cargo fmt --all -- --check` and `mdtablefix --check`.
`make lint` recursively runs `lint-clippy` (`cargo doc --no-deps`, then
`cargo clippy` with `CARGO_FLAGS ?= --all-targets --all-features`) and
`lint-whitaker`. `make test` (`Makefile:94-101`) runs `check-build-tools`, the
nextest version gate,
`$(CARGO) fetch --locked --target "$${CARGO_BUILD_TARGET:-host-tuple}"`,
`cargo nextest run`, and `cargo test --doc`. `make audit` runs `cargo audit`,
honouring `CARGO_AUDIT_IGNORES`. `make markdownlint`, `make spelling`, and
`make nixie` validate documentation. Every Make recipe calls Cargo through
`$(CARGO)`; `tests/support/make_cargo_probe.rs` (`has_bare_cargo`) rejects bare
`cargo`.

The gates are themselves under test. Besides the R4 owners,
`tests/make_gate_order.rs` checks ordering and failure propagation through a
probe repository, `tests/build_toolchain_contract.rs` ties `tools/mold/VERSION`
and `tools/mold/SHA256SUMS` to the installer, and `.config/nextest.toml`
defines a `nested-cargo-builds` group (one thread) for tests that start a child
Cargo.

CI (`.github/workflows/ci.yml`, job `build-test`, `ubuntu-latest`, 19 steps)
installs build tools, checks formatting, Markdown, and spelling, installs
`cargo-nextest` and (except for Dependabot) `cargo-audit` with
`env: RUSTFLAGS: ""`, installs Whitaker, runs `make audit` (except for
Dependabot), runs `make lint`, then coverage. `audit.yml` runs `make audit`
weekly. `dependabot.yml` updates `cargo`, `github-actions`, and
`rust-toolchain` daily with grouped minor and patch updates.

### Host tooling

Installed on the development host: `cargo-deny` 0.20.2, `cargo-audit` 0.22.2,
`cargo-about` 0.9.2, `cargo-nextest`. Not installed: `cargo-vet`,
`cargo-machete`, `cargo-udeps`, Verus, Kani.

### Planning probes (evidence, not implementation)

A scratch `deny.toml` in `/tmp/peregrine-1-2-1-probe/` allowing `MIT`,
`Apache-2.0`, `ISC`, `Unicode-3.0`, `Zlib`, `Apache-2.0 WITH LLVM-exception`,
`BSD-3-Clause`, and `BSD-2-Clause`, with `include-dev = true`, gave on this
host (whose cache already held every platform's packages):

```plaintext
$ cargo deny --offline --locked --config /tmp/peregrine-1-2-1-probe/deny.toml check licenses bans sources
warning[no-license-field]: license expression was not specified in manifest for crate 'mockable = 3.0.0'
warning[license-not-encountered]: license was not encountered   (BSD-3-Clause)
bans ok, licenses ok, sources ok
```

A `git archive` copy under `target/` resolved with
`cargo generate-lockfile -Zdirect-minimal-versions` to `camino` 1.2.0,
`proptest` 1.9.0, `rstest` 0.27.0, `serde` 1.0.228, and `toml` 1.1.3 online;
offline it picked `camino` 1.2.1, a cached version above the floor. Every other
direct dependency stayed at its lockfile version. The copy was deleted; nothing
was compiled.

### Documentation to read

- `docs/roadmap.md` §1.2 for the task and its successor 1.2.2 (Verus).
- `docs/peregrine-design.md` §1.2, §2 (crate-extraction rule), §11 (evidence
  and the feature-matrix rule), and §12 (the paragraph this task updates).
- `docs/terms-of-reference.md` §8.1, §8.3, Q5, and Q6.
- `docs/repository-layout.md`, which must list the new files.
- `docs/resource-policy-ownership-spike.md` "Dependency review" (historical).
- `docs/adrs/adr-0006-extension-interface-stabilization.md` (scoped
  acceptance wording), `adr-0002-compiler-ownership-experiment.md`, and
  `adr-0007-resource-policy-and-body-erasure-boundary.md` (open compiler floor).
- `docs/developers-guide.md` "Local Workflow", "Tooling", "Security audit
  ignores", "Workflow pins and Dependabot", "Fetching dependencies before
  tests", "The experimental spike module", "Maintain the spike fixtures"
  (module-scoped `dead_code` guidance), and "Toolchain updates".
- `docs/documentation-style-guide.md` for the ADR template (status, date,
  summary) and Markdown rules; `docs/scripting-standards.md` for the floors
  script.
- `docs/rust-testing-with-rstest-fixtures.md`,
  `docs/rstest-bdd-users-guide.md`,
  `docs/reliable-testing-in-rust-via-dependency-injection.md`, and
  `docs/rust-doctest-dry-guide.md` for test idioms.
- `docs/complexity-antipatterns-and-refactoring-strategies.md` for keeping the
  support functions small.
- `docs/hexagonal-application-case-study.md`,
  `docs/pachislot-extension-design.md`, and
  `docs/polonius-ownership-experiment.md` only to confirm that this task
  decides none of their future dependencies (none is decided).

### Skills to load

`rust-router` first, then `rust-unit-testing` (fixtures and matchers),
`proptest`, `arch-supply-chain` (`cargo-deny` policy), `arch-decision-records`
(ADR 008), `execplans` (keeping this plan current), `en-gb-oxendict-style`
(prose), and `commit-message`. Delegate full gate runs to the `scrutineer`
agent.

## Conformance basis

Upstream artefacts at revision `0155d04`:

- Terms of reference `docs/terms-of-reference.md`: §8.1 (passing applicable
  quality gates); §8.3 (dependence on "compatible Rust ecosystem libraries, a
  maintained compiler baseline"); Q5 (compiler posture, open); Q6 (`leynos`
  holds decision authority).
- Roadmap `docs/roadmap.md`: goal G4's evidence, "reproducible checks" (goal
  table), and item 1.2.1's success criterion.
- Technical design `docs/peregrine-design.md` v0.6 (2026-10-13): §11
  (feature-matrix rule), §12 (caret requirements; licence and feature-default
  review).
- ADRs: ADR 002 (compiler posture not fixed); ADR 006 (scoped acceptance
  precedent); ADR 007 (open compiler floor). No existing ADR governs dependency
  or licence policy; ADR 008 is new.
- Governing standard: `AGENTS.md` "Dependency management" (caret requirements
  mandatory; no wildcard or open inequality) and "Change quality & committing".

Derived requirements:

- `REQ-CARET`: every direct requirement is a single caret comparator with
  explicit major, minor, and patch numbers and no pre-release.
- `REQ-FORM`: every declaration uses only reviewed forms (registry source;
  keys `version`, `features`, `default-features`); no platform-specific
  dependency table, `[patch]`, or `[replace]` exists.
- `REQ-FLOOR`: every caret floor is honest: resolving it exactly type-checks
  all targets.
- `REQ-LICENCE`: every resolved package's licence, on every platform and
  including development dependencies, is satisfiable by the ISC-compatible
  allow-list, and a dated review records what is present.
- `REQ-REGISTER`: every direct dependency carries a record of purpose, users,
  and feature rationale that cannot silently drift.
- `REQ-FEATURES`: the crate's feature surface is explicit, design §11's matrix
  rule activates when a feature first appears, and dependency features are the
  smallest set the gates need.
- `REQ-BASELINE`: the supported baseline is defined, and `make check-fmt`,
  `make lint`, and `make test` pass on it.

Observed gaps: `GAP-FROZEN` (the only review is frozen spike history),
`GAP-UNDEFINED` ("supported baseline" is undefined), `GAP-DRIFT` (nothing
catches a new dependency, a non-caret requirement, a new licence, or a
dishonest floor), `GAP-DOCS` (the developers' guide's dependency list is
incomplete).

Trace links:

```plaintext
AGENTS-deps, design-§12 -> REQ-CARET, REQ-FORM -> EP-M1 -> dependency_baseline::contract::requirements_are_full_caret
roadmap-G4, design-§12 -> REQ-FLOOR -> EP-M3 -> make check-dependency-floors (CI: dependency-floors.yml)
design-§12, roadmap-1.2.1 -> REQ-LICENCE -> EP-M2, EP-M5 -> make deny; docs/dependency-baseline.md "Licence review"
roadmap-1.2.1, GAP-FROZEN, GAP-DRIFT -> REQ-REGISTER -> EP-M1 -> dependency_baseline::contract::records_match_declarations
design-§11 -> REQ-FEATURES -> EP-M1, EP-M4 -> dependency_baseline::contract::crate_declares_no_features
TOR-§8.3, GAP-UNDEFINED, GAP-DOCS -> REQ-BASELINE -> EP-M0, EP-M5 -> gate logs; docs/dependency-baseline.md "Supported baseline"
```

## Verification plan

This task introduces repository-owned checking logic, not framework behaviour.
Its obligations concern the relationship between the manifest's declarations,
the manifest's records, the resolved graph, and the floors.

### Abstract model

Let `D` be the set of direct dependency declarations in `Cargo.toml`. Each
declaration `d` has a table `t(d)` in
`{dependencies, dev-dependencies, build-dependencies}`, a crate name `n(d)`, a
requirement string `req(d)`, a set of keys `keys(d)`, a boolean
`default_features(d)`, and a set `features(d)`. A declaration written as a
string (`name = "1.2.3"`) has `keys = {version}`, default features on, and no
features. Let `K` be the set of records in
`[package.metadata.dependency-baseline]`, each keyed by (table, crate) with
fields `purpose`, `used-by`, and optional `feature-rationale`. Let `G` be the
resolved graph, `lic(p)` the licence expression of package `p`, and `A` the
allow-list.

Precondition for all checks: `Cargo.toml` parses as TOML. A parse failure, a
missing or mistyped field, or an unknown key is a test failure whose diagnostic
names the file, the table, the crate, and the rule; never a panic without
context.

### Invariants and lemmas

- `INV-CARET` (`REQ-CARET`): for every `d` in `D`, `req(d)` parses as a
  `semver::VersionReq` with exactly one comparator, operator `Op::Caret`,
  `minor` and `patch` present, and an empty pre-release.
- `INV-FORM` (`REQ-FORM`): for every `d`, `keys(d)` is a subset of
  `{version, features, default-features}` and contains `version`; the manifest
  has no `target` table containing dependency tables, no `[patch]`, and no
  `[replace]`.
- `INV-REGISTER` (`REQ-REGISTER`): the keys of `D` and `K` are equal as sets;
  every record has a non-empty `purpose` and at least one `used-by` path that
  exists relative to the manifest directory; a record has `feature-rationale`
  exactly when its declaration sets `features` or `default-features = false`.
  The diagnostic lists unrecorded declarations, orphan records, and field
  violations separately.
- `INV-NO-FEATURES` (`REQ-FEATURES`): the manifest has no `[features]` table
  and no declaration with `optional`.
- `INV-LICENCE` (`REQ-LICENCE`): for every `p` in `G`, on every platform and
  including development and build dependencies, `lic(p)` is satisfiable using
  only licences in `A`, and `p` comes from crates.io.
- `INV-FLOOR` (`REQ-FLOOR`): resolving with `-Zdirect-minimal-versions`
  places every `d` at exactly the version `req(d)` names, and
  `cargo check --all-targets --all-features --locked` then succeeds.
- `L-CLASSIFY`: the requirement classifier's verdict depends only on the
  requirement's shape (operator, number of comparators, which of minor and
  patch are present, pre-release presence, wildcard positions), not on the
  numeric values or the crate name. With `INV-CARET` checked on the real
  manifest, this lemma is what lets a finite table of shapes stand for all
  requirements.
- `L-EXTRACT`: extraction maps every accepted declaration form to exactly the
  fields above and every rejected form to a typed error naming the crate and
  the rule. `INV-CARET`, `INV-FORM`, `INV-REGISTER`, and `INV-NO-FEATURES` on
  the real manifest follow from `L-EXTRACT`, `L-CLASSIFY`, and the record
  comparison, because the comparisons consume only extracted values.

### Axioms (trusted, not verified here)

- `AXIOM-SEMVER`: `semver::VersionReq::parse` implements Cargo's requirement
  grammar, including treating a bare version as caret. Source: the `semver`
  crate documentation; Cargo depends on it. Boundary: `INV-CARET`.
- `AXIOM-TOML`: the `toml` crate parses `Cargo.toml` as Cargo does for the
  keys used here. Boundary: `L-EXTRACT`.
- `AXIOM-DENY`: `cargo-deny` 0.20.2 evaluates SPDX expressions, licence
  files, and package sources correctly from `cargo metadata`, offline once
  every package is fetched. Boundary: `INV-LICENCE`.
- `AXIOM-METADATA`: crates' declared `license` fields are truthful. Boundary:
  `INV-LICENCE`.
- `AXIOM-MINVER`: `-Zdirect-minimal-versions` selects the lowest admitted
  version for direct dependencies unless another requirement or a yank forces a
  higher one. Source: the Cargo book, unstable features. Boundary: `INV-FLOOR`;
  the floor-equality assertion detects the "unless" case.

### Methods per obligation

`L-EXTRACT` and `INV-CARET` shapes:

- Method: exhaustive `rstest` tables over the finite partition.
- Rationale: the classification depends on finitely many shape choices, which
  the execplans guidance assigns to parameterized tests.
- Domain: requirement shapes `"1.2.3"`, `"^1.2.3"`, `"~1.2.3"`, `"=1.2.3"`,
  `">=1.2.3"`, `"<2"`, `"1"`, `"0.1"`, `"1.*"`, `"^1.*"`, `"*"`,
  `"1.2.3-alpha"`, `">=1.2, <1.5"`; declaration forms string and table; keys
  `optional`, `git`, `path`, `package`, `workspace`, `registry`,
  `default_features` (the obsolete spelling), and an unknown key; tables
  `[target.'cfg(unix)'.dependencies]`, `[patch.crates-io]`, `[replace]`, and
  `[features]`.
- Artefact: `tests/dependency_baseline/requirement.rs` and
  `tests/dependency_baseline/manifest.rs` (child modules of
  `tests/dependency_baseline.rs`).
- Evidence: `cargo nextest run --test dependency_baseline`; red fails with
  E0583 (module file not found) before the support files exist.
- Non-vacuity: every row names its expected verdict and, for rejections, the
  rule; the table includes at least one acceptance per form so that a
  classifier rejecting everything fails.

`L-CLASSIFY`:

- Method: `proptest` metamorphic property.
- Rationale: the property relates outputs for different inputs, so its oracle
  is the classifier's own verdict on a canonical representative, not a
  re-encoding of the rules.
- Domain: every shape from the table above, with each numeric component
  replaced by values drawn from `0..=10_000` and crate names drawn from
  `[a-z][a-z0-9_-]{0,15}`; 256 cases.
- Property: the verdict for the generated requirement equals the verdict for
  the same shape with numbers `1.2.3`.
- Artefact: `tests/dependency_baseline/property.rs`.
- Non-vacuity: a companion test asserts the shape list contains both accepted
  and rejected shapes. Seeded fault: temporarily make the classifier reject
  major version 0; the property must fail and shrink to a `0.x.y` case. Revert
  it and delete any `proptest-regressions/` file the fault created.

`INV-CARET`, `INV-FORM`, `INV-REGISTER`, `INV-NO-FEATURES` on the real
repository:

- Method: contract tests over the real `Cargo.toml`, `rstest` negative cases
  over inline fixture manifests, and `rstest-bdd` scenarios for the
  contributor-facing outcomes.
- Artefact: `tests/dependency_baseline.rs`, child module
  `tests/dependency_baseline/records.rs`, and
  `tests/features/dependency_baseline.feature`.
- Evidence: red, before any record exists, `records_match_declarations` fails
  listing all 25 declarations as unrecorded; green after the records are
  written.
- Non-vacuity: fixture cases must reject an unrecorded declaration, an orphan
  record, an empty `purpose`, a `used-by` path that does not exist, a missing
  `feature-rationale` for `default-features = false`, a superfluous one for a
  default declaration, a crate present in both `dependencies` and
  `dev-dependencies` with only one record, and an `optional` declaration; each
  diagnostic names the crate and rule.

`INV-LICENCE`:

- Method: `make deny` over the real graph, end to end.
- Rationale: SPDX evaluation is the tool's contract (`AXIOM-DENY`); the
  repository owns the policy file, the fetch, and the wiring.
- Artefact: `deny.toml`, `tools/cargo-deny/VERSION`, `Makefile` target `deny`,
  CI steps "Install cargo-deny" and "Check dependency licences and sources",
  pinned in the R4 contract owners.
- Evidence: `make deny` prints `bans ok, licenses ok, sources ok`; the first CI
  run on the pull request proves the cold-cache fetch on a fresh runner.
- Non-vacuity: with a scratch copy of `deny.toml` passed through
  `--config`, (a) omitting `Zlib`, a licence carried only by the development
  dependency chain `rstest-bdd` to `hashbrown` to `foldhash`, must fail on
  `foldhash`, proving development dependencies are checked; (b) omitting `MIT`
  must fail on a runtime crate such as `bytes`. Run these before `EP-M4`,
  because narrowing `rstest-bdd` may remove `foldhash`; if it is already gone,
  pick another development-only licence from
  `cargo deny --offline --locked list` and record the substitution. Discard the
  scratch files.

`INV-FLOOR`:

- Method: `make check-dependency-floors`, end to end.
- Rationale: honesty of a floor is a compiler fact; only compiling at the
  floor establishes it.
- Artefact: `scripts/check-dependency-floors.sh`, Make target,
  `.github/workflows/dependency-floors.yml`.
- Evidence: exit 0, with the printed table showing every direct dependency at
  its floor (including `camino` 1.2.0 and `proptest` 1.9.0, which differ from
  `Cargo.lock`).
- Non-vacuity: in a scratch edit of the working tree, never committed:
  (a) lower a floor below a version some other package requires (candidate:
  `serde = "1.0.100"`, which `serde_yaml` 0.9.34 is expected to lift; confirm
  by reading its manifest, and if it is not lifted try `serde_json`); the
  script must fail naming the lifted crate and the package that forced it. (b)
  Lower a floor below a version whose API the code uses, choosing the crate and
  version from that crate's changelog in the Cargo registry cache; the type
  check must fail with a compile error (E0425, E0432, E0433, or E0599), not a
  resolution error. Record both transcripts, then restore `Cargo.toml` with
  `git checkout -- Cargo.toml`.

### Formal verification decision

No Verus proof and no Kani harness (D10). The obligations are set equality
between declarations and records, a classification delegated to `semver`, and
properties of external tools' outputs. None is contractual business logic or a
lemma about Peregrine's semantics; a Verus proof of set equality would restate
the code, and the finite shape partition is covered exhaustively by `rstest`
with `L-CLASSIFY` justifying that the partition is complete. The roadmap's
first Verus obligation, the lifecycle stop/dispatch rule I1, belongs to task
1.2.2, which will introduce the Verus toolchain and any `vstd` dependency
through the admission procedure this task establishes, using the non-registry
source exception route if `vstd` is not on crates.io.

### BDD feature

Path: `tests/features/dependency_baseline.feature`, bound in
`tests/dependency_baseline.rs` with `rstest-bdd` 0.6.0 (`scenarios!` or
`#[scenario]`, following `tests/resource_policy_spike_bdd.rs`). Scenarios use
inline fixture manifests, never the real file.

```gherkin
Feature: Dependency baseline contract
  Contributors learn at test time when a dependency change needs review.

  Scenario: A recorded caret dependency passes
    Given a manifest declaring dependency "bytes" with requirement "1.12.1"
    And a baseline record for dependency "bytes" used by "src"
    When the dependency baseline is checked
    Then the check passes

  Scenario: An unrecorded dependency is reported
    Given a manifest declaring dev-dependency "insta" with requirement "1.43.0"
    And no baseline record for dev-dependency "insta"
    When the dependency baseline is checked
    Then the check fails naming dev-dependency "insta" as unrecorded

  Scenario: A non-caret requirement is rejected
    Given a manifest declaring dependency "http" with requirement "~1.5.0"
    And a baseline record for dependency "http" used by "src"
    When the dependency baseline is checked
    Then the check fails naming dependency "http" with requirement "~1.5.0"

  Scenario: An optional dependency requires the feature matrix
    Given a manifest declaring dependency "bytes" as optional
    And a baseline record for dependency "bytes" used by "src"
    When the dependency baseline is checked
    Then the check fails citing the feature-matrix rule in design section 11
```

These trace to `REQ-REGISTER`, `REQ-CARET`, and `REQ-FEATURES`, and are
implemented red-first in `EP-M1`.

## Plan of work

### `EP-M0` record the baseline gates

Run the gate sequence in `Concrete steps` on the unchanged branch, through the
`scrutineer` agent, and record results and log paths in `Artefacts and notes`.
Any failure is pre-existing and must be recorded, with evidence, before
`EP-M1`. Only this plan changes.

### `EP-M1` manifest contract and in-manifest register

Assigned: `REQ-CARET`, `REQ-FORM`, `REQ-REGISTER`, `REQ-FEATURES`; D5–D7, D11,
D14.

Before writing helpers, confirm with
`rg -n 'dev-dependencies|dependencies' tests/support` that no existing support
module extracts manifest dependencies (reconnaissance found `toml` used only
for the toolchain, Cargo configuration, and Clippy contracts).

Red, in order, each run with `cargo nextest run --test dependency_baseline`:

1. Create `tests/dependency_baseline.rs` declaring child modules with
   `#[path = "dependency_baseline/requirement.rs"]` and so on, plus the
   requirement tables. It fails with E0583 because the files do not exist.
2. Add the module files with signatures only (returning a placeholder error)
   so the tables compile and fail on their assertions.
3. Add `requirements_are_full_caret`, `declarations_use_reviewed_forms`,
   `crate_declares_no_features`, and `records_match_declarations` over the real
   manifest, the negative fixture tables, the property, and the four BDD
   scenarios with their feature file.

Green: add `semver = "1.0.28"` to `[dev-dependencies]`; implement
`requirement.rs` (classification), `manifest.rs` (extraction and table checks),
and `records.rs` (record parsing and comparison); then write the 25 records
under `[package.metadata.dependency-baseline]` in `Cargo.toml`, after
`[dev-dependencies]`, one table per record, for example:

```toml
[package.metadata.dependency-baseline.dev-dependencies.proptest]
purpose = "Property tests for body adapters and Make parsers."
used-by = ["tests/body_adapter_property.rs", "tests/support/make_gate_order_records.rs"]
```

Use `googletest` matchers (`elements_are!`, `contains_substring`,
`unordered_elements_are!`) for diagnostics and `pretty_assertions::assert_eq`
for structured values; load the real manifest through an `rstest` fixture
returning `Result`. Keep every function within `clippy.toml`'s limits
(cognitive complexity 9, 70 lines, four arguments, nesting 4), give every item,
field, and variant a doc comment (`missing_docs_in_private_items` is denied),
and use a `CrateName` newtype and a `DependencyTable` enum for the record key.

Refactor, rerun the focused test, then run the full gates.

### `EP-M2` licence, source, and ban gate

Assigned: `REQ-LICENCE`; D3, D4, D15.

Red: update the R4 contract owners first so they expect the new state, and run
them
(`cargo nextest run --test make_cargo_routes --test act_workflow
--test build_tool_workflows`
plus any binaries that include the act support modules; find them with
`rg -l 'act_workflow_(policy|contract|command|smoke)' tests`). They must fail
because the workflow and Makefile lack the new steps. The expected state:

- `tests/make_cargo_routes.rs`: a route row `("deny", &["fetch", "deny"])`.
- `tests/support/act_workflow_policy.rs`: "Install cargo-deny" with
  `env: RUSTFLAGS: ""` and script
  `cargo binstall --no-confirm --locked "cargo-deny@$(cat tools/cargo-deny/VERSION)"`,
  and "Check dependency licences and sources" running `make deny`, neither
  with an `if:` condition.
- `tests/support/act_workflow_contract.rs`: 21 hosted steps, the two new
  steps after "Audit dependencies" and before "Lint".
- `tests/support/act_workflow_command.rs` and
  `tests/support/act_workflow_smoke.rs`: the `deny` goal and the `binstall`
  command allowed.
- `tests/support/build_tool_workflows.rs`: `deny` classified as a known goal.
- `tests/build_toolchain_contract.rs` (or the owner of `tools/mold/VERSION`
  checks): `tools/cargo-deny/VERSION` holds one SemVer version and matches the
  version the Makefile checks.

Green: write `tools/cargo-deny/VERSION` (`0.20.2`), `deny.toml` per D4, and the
Makefile target, declared `.PHONY` and listed by `make help`:

```makefile
CARGO_DENY_VERSION := $(shell cat tools/cargo-deny/VERSION)

deny: ## Check dependency licences, sources, and bans (fetches all platforms first)
	$(CARGO) fetch --locked
	@$(CARGO) deny --version | grep -qx "cargo-deny $(CARGO_DENY_VERSION)" || \
		{ echo "build-tools: cargo-deny $(CARGO_DENY_VERSION) is required" >&2; exit 1; }
	$(CARGO) deny --offline --locked check licenses bans sources
```

Adapt the sketch to the Makefile's existing idioms (for example, how the
nextest version check reports errors at `Makefile:97`), then run
`mbake validate Makefile`. Add the two CI steps. Run the non-vacuity controls
for `INV-LICENCE`.

Refactor, then run the full gates plus `make deny`.

### `EP-M3` caret-floor compatibility check

Assigned: `REQ-FLOOR`; D8, D15.

Red: classify `check-dependency-floors` in
`tests/support/build_tool_workflows.rs` and add a shape assertion for
`.github/workflows/dependency-floors.yml` in its existing owner
(`tests/build_tool_workflows.rs`): the workflow installs build tools the same
way `ci.yml` does (setup step with `rustflags: ''` and the build-tools
installation step that provides clang, lld, and `mold`), runs
`make check-dependency-floors`, triggers on the D8 paths without `Cargo.lock`,
and sets `concurrency` with `cancel-in-progress: true`. Do not assert action
revisions. Run it; it fails because the workflow is absent.

Green: write `scripts/check-dependency-floors.sh` following
`docs/scripting-standards.md` (strict mode, clear diagnostics), implementing
D8: recreate `target/dependency-floors/tree/` from
`git ls-files -co --exclude-standard -z` excluding `.cargo/`; run
`cargo -Zunstable-options config get build.rustflags` there and print it so the
effective configuration is in the log; resolve with
`cargo generate-lockfile -Zdirect-minimal-versions`, refusing to run if
`CARGO_NET_OFFLINE` is set; compare each direct dependency's resolved version
(from `cargo metadata --format-version 1 --locked`) with its requirement's
floor and fail, naming the forcing package from `cargo tree -i`, on any
difference; then type-check with
`CARGO_TARGET_DIR=target/dependency-floors/build`. Add the `.PHONY` Make target
and the workflow.

Run the target. If a development floor fails to type-check, that is the red
evidence: raise the floor in `Cargo.toml` to the lowest passing version (the
working-tree copy sees the uncommitted raise), rerun, and record the raise in
the crate's metadata `purpose` only if relevant, in the baseline record, and in
`Decision log`. A runtime floor failure escalates. Keep `Cargo.lock` unchanged
unless a raised floor exceeds the locked version, which cannot happen for an
in-range raise. Run the `INV-FLOOR` negative controls.

Then run the full gates.

### `EP-M4` development-dependency feature narrowing

Assigned: `REQ-FEATURES`; D9.

For each candidate in turn (`proptest`, then `rstest-bdd`, then any other
development dependency whose defaults `cargo tree -e features --offline` shows
unused), set `default-features = false` with only the needed features, add the
matching `feature-rationale` to its record (the contract test requires it), and
run `make lint` and `make test`. Keep the change only if both pass. Record the
package count before and after from
`cargo metadata --locked --offline --format-version 1` (all packages) in the
baseline record. Run `make deny` and `make check-dependency-floors` after the
last narrowing. Commit each kept narrowing separately.

### `EP-M5` documentation closure

Assigned: `REQ-BASELINE`, `REQ-LICENCE` (record), `GAP-UNDEFINED`, `GAP-DOCS`;
D12.

- Write `docs/dependency-baseline.md`: purpose and scope; "Supported
  baseline" (D2, stated as what the gates verify, not a support commitment);
  "Where the register lives" (the manifest metadata, and how to view it with
  `cargo metadata --no-deps --format-version 1`); "Licence review" dated with
  the `Cargo.lock` revision, generated from
  `cargo deny --offline --locked list --layout license` with `deny.toml`
  present, covering all 226 packages and recording `mockable`'s detected
  licence and confidence; "Caret floors" with the floors evidence and any
  raises; "Feature posture" with the `EP-M4` outcomes; "Known follow-ups"
  (`serde_yaml`).
- Write `docs/adrs/adr-0008-dependency-and-licence-baseline.md` per
  `docs/documentation-style-guide.md` (status, date, summary, context,
  decision, consequences), recording the D12 policy only.
- Update `docs/peregrine-design.md` §12: keep the candidate-family list and
  replace the spike-era narrative with a pointer to ADR 008 and the baseline
  record.
- Update `docs/developers-guide.md`: a "Dependency baseline" section with the
  admission procedure (write the record, run `make deny` and
  `make check-dependency-floors`, update the licence review), the non-registry
  source exception route, the Dependabot licence runbook (R6), the floors
  triage runbook (R2), the `cargo-deny` bump procedure (edit
  `tools/cargo-deny/VERSION` in its own change), and the support modules' reuse
  policy; replace the spike section's dependency list with a pointer.
- Update `docs/repository-layout.md` (`deny.toml`, `tools/cargo-deny/`, the
  script, the workflow, the baseline record), `docs/contents.md` (the record
  under "Engineering practice"; ADR 008 with the other ADRs), and `AGENTS.md`
  (list `make deny` and `make check-dependency-floors` with the gates).
- Update `docs/users-guide.md` "Build the Repository" with the supported
  baseline and the two targets.
- Mark roadmap 1.2.1 done in `docs/roadmap.md` and update its introduction's
  list of completed tasks.

Run `make fmt`, `make markdownlint`, `make spelling`, and `make nixie`, then
the three gates and `make deny`.

## Milestones and plateaus

- `EP-M0`: the base's gate status is known. Acceptance: logs recorded.
  Conformance check:
  - no repository change.
  - pre-existing failures, if any, recorded separately.
  Recovery: rerun. Remaining gaps: all. Compatibility: none.
- `EP-M1`: every direct dependency is recorded and checked. Requirements:
  `REQ-CARET`, `REQ-FORM`, `REQ-REGISTER`, `REQ-FEATURES`. Acceptance: the
  contract, table, property, and BDD tests pass; negative fixtures fail as
  specified; seeded fault rejected and reverted. Conformance check:
  - requirements satisfied by passing tests.
  - design §11 rule encoded in the tripwire.
  - no `src/` change; only `semver` added.
  - no trust-boundary or persisted-format change.
  - trace links current.
  Recovery: revert the milestone's commits. Remaining gaps: licence, floors,
  narrowing, prose. Compatibility: none.
- `EP-M2`: licence, source, and wildcard policy enforced on every pull
  request. Requirements: `REQ-LICENCE`. Acceptance: `make deny` passes locally
  and in CI; negative controls fail as specified; contract owners pass.
  Conformance check:
  - advisories untouched; `make lint` and `make all` unchanged.
  - only `cargo-deny` 0.20.2 added as a tool, with one version source.
  - trace links current.
  Recovery: revert; CI returns to 19 steps. Remaining gaps: floors, narrowing,
  prose. Compatibility: none.
- `EP-M3`: floors proven honest. Requirements: `REQ-FLOOR`. Acceptance:
  `make check-dependency-floors` exits 0 with every direct dependency at its
  floor; both negative controls fail as specified. Conformance check:
  - at most three development floors raised; no runtime floor raised.
  Recovery: revert; floors return to their previous values. Remaining gaps:
  narrowing, prose. Compatibility: none.
- `EP-M4`: development features minimal. Requirements: `REQ-FEATURES`.
  Acceptance: records and declarations agree; gates pass; package counts
  recorded. Conformance check:
  - runtime features unchanged.
  Recovery: revert individual narrowing commits. Remaining gaps: prose.
  Compatibility: none.
- `EP-M5`: documentation and roadmap closed. Requirements: `REQ-BASELINE`,
  `REQ-LICENCE`. Acceptance: documentation gates and the Rust gates pass;
  roadmap shows 1.2.1 done. Conformance check:
  - design §12 and ADR 008 agree with `deny.toml`, the records, and the
    baseline record.
  - no historical document edited.
  Recovery: revert. Remaining gaps: `serde_yaml` follow-up and the open
  questions. Compatibility: none.

No milestone introduces compatibility machinery: every surface touched is
private test support or repository configuration.

## Concrete steps

Working directory: the worktree root. Use `set -o pipefail` in every shell that
pipes through `tee`. Logs follow
`/tmp/$ACTION-peregrine-web-1-2-1-minimal-dependency-and-feature-baseline.out`.

Gate sequence, run sequentially and never in parallel (delegate to `scrutineer`
where practical):

```sh
set -o pipefail
make check-fmt 2>&1 | tee /tmp/check-fmt-peregrine-web-1-2-1-minimal-dependency-and-feature-baseline.out
make lint 2>&1 | tee /tmp/lint-peregrine-web-1-2-1-minimal-dependency-and-feature-baseline.out
make test 2>&1 | tee /tmp/test-peregrine-web-1-2-1-minimal-dependency-and-feature-baseline.out
make deny 2>&1 | tee /tmp/deny-peregrine-web-1-2-1-minimal-dependency-and-feature-baseline.out
```

`make deny` joins the sequence from `EP-M2`; `make check-dependency-floors`
runs at `EP-M3` and `EP-M4` closure, with its log at
`/tmp/floors-peregrine-web-1-2-1-minimal-dependency-and-feature-baseline.out`.

Documentation gates (`EP-M5`, and whenever Markdown changes):

```sh
set -o pipefail
make fmt
make markdownlint 2>&1 | tee /tmp/markdownlint-peregrine-web-1-2-1-minimal-dependency-and-feature-baseline.out
make nixie 2>&1 | tee /tmp/nixie-peregrine-web-1-2-1-minimal-dependency-and-feature-baseline.out
```

(`make markdownlint` also runs `make spelling`.)

Focused commands and expected results:

```plaintext
EP-M1 red:   cargo nextest run --test dependency_baseline -> E0583 file not found for module
EP-M1 red:   same, after stubs -> table and contract assertions fail; 25 declarations unrecorded
EP-M1 green: same -> all pass
EP-M2 red:   cargo nextest run -E 'binary(make_cargo_routes) + binary(build_tool_workflows)' (+ act binaries) -> fail
EP-M2 green: same -> pass; make deny -> "bans ok, licenses ok, sources ok"
EP-M3 red:   cargo nextest run --test build_tool_workflows -> floors workflow missing
EP-M3 green: make check-dependency-floors -> every direct dependency at its floor; check passes
```

## Validation and acceptance

Acceptance is behaviour a reviewer can reproduce:

- `make check-fmt`, `make lint`, and `make test` exit 0 on the pinned nightly;
  the nextest summary includes `dependency_baseline`.
- `make deny` prints `bans ok, licenses ok, sources ok`, and CI's
  "Check dependency licences and sources" step passes on the pull request's
  fresh runner.
- `make check-dependency-floors` exits 0 and prints every direct dependency at
  its floor, including `camino` 1.2.0 and `proptest` 1.9.0.
- In a scratch edit, adding `insta = "1.43.0"` to `[dev-dependencies]` makes
  `cargo nextest run --test dependency_baseline` fail naming dev-dependency
  `insta` as unrecorded; changing `http = "1.5.0"` to `http = "~1.5.0"` makes
  it fail naming `http` and `~1.5.0`; reverting restores green.

Quality criteria:

- Tests: the gate sequence passes.
- Verification: `L-CLASSIFY` property and its seeded fault recorded;
  `INV-LICENCE` and `INV-FLOOR` negative controls recorded.
- Lint: `make lint` (Clippy and Whitaker) clean.
- Security: `make deny` clean; `make audit` clean in CI.

## Idempotence and recovery

Every step can be rerun. The floors script recreates its tree on each run and
keeps its build directory for speed; delete `target/dependency-floors/` when
the work is complete to save disk and inotify watches. Negative controls use
scratch configuration files or uncommitted edits; restore with
`git checkout -- Cargo.toml deny.toml Cargo.lock` and confirm
`git status --short` before committing. Seeded faults are reverted before the
milestone's commit, and any `proptest-regressions/` file they created is
deleted.

## Artefacts and notes

Planning-time evidence (recorded before approval; not implementation):

```plaintext
$ cargo deny --version                     -> cargo-deny 0.20.2
$ cargo deny --offline --locked --config <scratch> check licenses bans sources
bans ok, licenses ok, sources ok           (exit 0; mockable no-license-field warning)
$ cargo generate-lockfile -Zdirect-minimal-versions   (scratch copy, online)
Adding camino v1.2.0, proptest v1.9.0, rstest v0.27.0, serde v1.0.228, toml v1.1.3+spec-1.1.0
$ cargo tree -i foldhash --offline -e normal,dev,build
foldhash v0.2.0 <- hashbrown v0.17.1 <- rstest-bdd v0.6.0 <- [dev-dependencies] peregrine-web
$ make check-fmt                           -> "33 files left unchanged.", exit 0
```

Implementation evidence is appended per milestone below.

## Interfaces and dependencies

New development dependency: `semver = "1.0.28"` (`MIT OR Apache-2.0`), already
resolved. New host tool: `cargo-deny` 0.20.2. No runtime change.

New test-support interfaces, as child modules of `tests/dependency_baseline.rs`
(so private to that binary):

```rust
/// Which manifest table declared a dependency.
pub enum DependencyTable { Dependencies, DevDependencies, BuildDependencies }

/// A crate name as written in the manifest.
pub struct CrateName(String);

/// One direct dependency as the baseline contract sees it.
pub struct DependencyDecl {
    /// Table that declared it.
    pub table: DependencyTable,
    /// Declared crate name.
    pub name: CrateName,
    /// Raw requirement string, classified separately.
    pub requirement: String,
    /// Whether default features stay enabled.
    pub default_features: bool,
    /// Explicitly enabled features.
    pub features: std::collections::BTreeSet<String>,
}

pub fn classify_requirement(raw: &str) -> Result<(), RequirementViolation>;
pub fn extract_declarations(manifest: &toml::Table) -> Result<Vec<DependencyDecl>, ManifestViolations>;
pub fn check_records(
    manifest: &toml::Table,
    decls: &[DependencyDecl],
    root: &camino::Utf8Path,
) -> RecordReport;
```

`RequirementViolation`, `ManifestViolations`, and `RecordReport` are
`thiserror` enums or report structs whose `Display` names the table, crate, and
rule. The exact shapes may change during red-green; the contract is that
classification, extraction, and record checking stay separate functions.

Reuse policy: these modules belong to the dependency contract test binary. They
are not shared support and must not be imported by other test binaries or
production code; a later contract needing manifest parsing extends them through
a documented move to `tests/support/`, recorded in `docs/developers-guide.md`.
No second manifest parser may be added.

New Make targets: `deny`, `check-dependency-floors`. New files: `deny.toml`,
`tools/cargo-deny/VERSION`, `scripts/check-dependency-floors.sh`,
`.github/workflows/dependency-floors.yml`, `tests/dependency_baseline.rs`,
`tests/dependency_baseline/{requirement,manifest,records,property}.rs`,
`tests/features/dependency_baseline.feature`, `docs/dependency-baseline.md`, and
`docs/adrs/adr-0008-dependency-and-licence-baseline.md`.

## Revision note

- 2026-11-14: initial draft from Wyvern reconnaissance and planning probes.
- 2026-11-14: revised after a three-perspective expert review (structure and
  contracts; failure modes and cost; alternatives and viability). Changes: the
  register moved from a parsed Markdown table into
  `[package.metadata.dependency-baseline]` (D6), removing the Markdown parser
  and its formatting coupling; the manifest contract now rejects `optional`,
  unknown keys, platform-specific tables, `[patch]`, and `[replace]`, and the
  caret rule requires full versions without pre-release (D5, `INV-FORM`);
  `make deny` fetches every platform and checks a single pinned tool version
  (D3, R7); the allow-list became a pre-approved permissive set to avoid
  Dependabot stalls (D4, R6), and the `mockable` clarification and its
  ineffective control were dropped; the floors check copies the working tree,
  excludes `.cargo/`, asserts exact floors, and runs on code paths rather than
  the lockfile (D8); new CI steps are pinned in the existing closed-world
  contract owners instead of a new workflow test, with R4 listing every owner
  (D15); the contract became one test binary (D14); the proptest moved from a
  restated oracle to the metamorphic lemma `L-CLASSIFY`, with exhaustive
  `rstest` tables for the finite partition; D11 is now an explicit deviation
  for user decision; ADR 008 uses ADR 006-style scoped acceptance (D12);
  budgets were re-derived (files 40, configuration 300, documentation 650); the
  declaration count is 25 after `semver`. Milestone order is unchanged.
