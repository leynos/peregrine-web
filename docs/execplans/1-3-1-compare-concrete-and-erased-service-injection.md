# Compare concrete and erased service injection (1.3.1)

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan` must
be kept up to date as work proceeds.

Status: DRAFT (expert-panel review applied; awaiting user approval)

Branch: `1-3-1-compare-concrete-and-erased-service-injection`

Roadmap item: `1.3.1` in `docs/roadmap.md`.

## Purpose / big picture

Peregrine proposes that an HTTP resource is a small inbound adapter that owns
the application services it needs, supplied by an application composition root.
The [hexagonal application case study](../hexagonal-application-case-study.md)
and [ADR 004](../adrs/adr-0004-application-port-boundaries.md) name two ways a
resource could hold such a service: as a concrete type, statically dispatched,
or as an application-owned trait object (an "erased" port). They also claim, as
hypothesis H1, that resource-local injection reduces forwarding code and the
number of files changed when one dependency is added, compared with Actix,
Wildside-style state bundles, and Falcon. Nothing in the repository has tested
either claim.

This work builds experiment E1: a small, runnable task-viewing specimen in
which three differently wired resource arms share the existing spike router seam
(`peregrine_web::spike::ResourceRunner`):

- Arm G: one generic resource type, `TasksResource<S: ?Sized + TaskQueries>`,
  holding a concrete service through a native-async port whose future is
  declared `+ Send`. No allocation and static dispatch: the generic arm's best
  case.
- Arm H: the same generic `TasksResource` instantiated with
  `dyn DynTaskQueries`, through an application-owned reverse bridge. This is
  the case study's Table 2 shape (`TasksResource<dyn TaskCommands>`) and the
  Pachislot design's "generic authoring plus a boxed-future adapter" baseline.
- Arm E: a non-generic `ErasedTasksResource` holding
  `Arc<dyn DynTaskQueries>`, a dyn-compatible port whose boxed future is
  declared `+ Send`.

The work then adds one driven and one driving dependency to each arm in
separately recorded commits, measures the change surface under rules fixed in
this plan, compares it with written Actix, Wildside, and Falcon comparators,
and records the retained boundary in a new ADR.

After the change, a contributor can run:

```plaintext
cargo run --example service_injection
```

and observe a silent exit with status 0: the example's `demonstrate()` builds
all three arms in one heterogeneous collection, invokes them with permitted,
cross-tenant, missing, malformed, badly identified, and anonymous requests, and
returns an error naming the first case where the arms disagree. Running
`cargo test --test service_injection_contract` shows the compiler accepting all
three storage options and rejecting a non-`Send` port future in each style, a
native-async port used as a trait object, a non-thread-safe service, a type
implementing both port styles, and a resource built without one of its
dependencies. Reading `docs/service-injection-experiment.md` shows the measured
change surface, the comparator verdicts, and the retained boundary with its
reasons; ADR 008 records the decision.

No framework API, router, or server is delivered. `src/` is unchanged.

## Constraints

- Do not add a service container, service locator, type-keyed registry,
  request-time lookup, `dyn Any` discovery, or implicit fixture fallback. Every
  dependency is a constructor argument supplied by the composition root (terms
  of reference non-goal; ADR 004; design §4.2).
- Do not modify `src/` (the `peregrine_web::spike` module and `src/lib.rs`).
  E1 must compile against the 1.1.3 seam as it stands. If the seam cannot
  express E1, stop and escalate (see `Tolerances`).
- The specimen's application module (`examples/service_injection/application/`)
  must not import `peregrine_web`, `http`, `http_body`, `bytes`, `hyper`, or
  the specimen's `http_adapter` or `outbound` modules (case study axiom A1).
  Only the composition module may see every layer. Inbound adapter files must
  not import outbound adapter files.
- Application ports carry explicit `Send` bounds in their signatures. The
  erased port uses a handwritten boxed future type, not a macro that hides the
  bound. The native port uses a return-position `impl Future + Send + 'a`. No
  `?Send` relaxation, `Rc`, thread-local state, `Arc<Mutex<_>>` workaround, or
  `unsafe` code in the specimen.
- Resources remain `Send + Sync + 'static` objects registered as
  `Arc<dyn Resource>`; the router's resource erasure must not force service
  erasure (ADR 004 proposed direction).
- Tenant and object authorization belong to the application use case, not to
  `PolicyEnforcer` (axiom A2). The specimen's enforcer permits every
  authenticated principal, so cross-tenant requests reach the use case.
- E1 is not E2: no non-HTTP driver, no mutation, no unit of work, and no
  separately built application crate are delivered here; they belong to roadmap
  1.3.2. Mutation would need a `POST` responder that the spike seam does not
  have. The second compiler variant (Polonius off and on, both solvers) belongs
  to 2.1.1 and 2.1.2.
- Actix, Wildside, and Falcon comparators are uncompiled sketches in
  documentation. No `actix-web` or Python dependency is added. This follows
  terms of reference §9.2, which records that the pinned Actix design is a
  design comparator and that no runnable Actix baseline is claimed.
- No runtime-performance or code-size claim is made. The terms of reference
  accept dynamic dispatch and future allocation as costs.
- The example prints nothing: `Cargo.toml` denies `print_stdout` and
  `print_stderr`, and `docs/developers-guide.md` requires `demonstrate` to be
  public and free of printing. `main` builds a runtime explicitly rather than
  using `#[tokio::main]`, which Whitaker rejects.
- Keep files under 400 lines; every module begins with a `//!` comment; every
  assertion carries a diagnostic message; no `expect`/`unwrap` outside tests;
  functions returning `Result` do not `assert!` (`panic_in_result_fn`); lint
  suppressions are tightly scoped with a `reason`.
- Use the shared Cargo cache and compiler admission. Do not bypass
  `RUSTC_WRAPPER`, use `/tmp` as a build target, or run gates in parallel.
- Commit only gated plateaus. Refactors are separate commits from the
  behaviour they tidy. Measurement commits (see `EP-M3`) must each pass the
  gates, because they are permanent evidence and rollback points.
- If `make spelling` regenerates `typos.toml` for reasons unrelated to this
  work, put that change in a separate base pull request and stack this branch
  on it, as `AGENTS.md` requires.

## Tolerances (exception triggers)

The comparison base is `base=$(git merge-base HEAD origin/main)`. Counts are
net lines (additions minus deletions) of committed changes; commit before
counting so new files are included. This ExecPlan file is excluded from every
count. Each category is enforced independently; spare capacity in one never
offsets another.

```plaintext
base=$(git merge-base HEAD origin/main)
# Code: Rust sources, fixtures, .stderr, .feature and .snap files.
git diff --numstat "$base"..HEAD -- examples tests \
  | awk '{n+=$1-$2; f++} END {print n" net lines, "f" files"}'
# Configuration.
git diff --numstat "$base"..HEAD -- Cargo.toml Cargo.lock .config \
  typos.toml typos.local.toml | awk '{n+=$1-$2; f++} END {print n" net lines, "f" files"}'
# Documentation, excluding ExecPlans.
git diff --numstat "$base"..HEAD -- docs README.md ':(exclude)docs/execplans' \
  | awk '{n+=$1-$2; f++} END {print n" net lines, "f" files"}'
```

- Code: stop if the change exceeds 4,000 net lines or 52 files.
- Configuration: stop if the change exceeds 450 net lines or 6 files.
  `Cargo.lock` counts; most of this budget is reserved for `insta`'s graph.
- Documentation: stop if the change exceeds 1,400 net lines or 16 files.
- Interface: stop if any change to `src/` appears necessary, including a new
  `Resource` method, a `POST` responder, or a changed bound.
- Dependencies: only `insta` (development, `default-features = false`) is
  authorized beyond the existing manifest. `dynosaur`, `trait-variant`,
  `actix-web`, and any runtime dependency require escalation.
- Architecture: stop if E1 appears to need a shared framework-level port
  helper, a universal application-context trait, or service-locator storage.
  Record such a need in the ledger as an H1 rejection condition as well.
- Experiment integrity: stop if the pre-registered shapes, counting rules, or
  decision rule in `Verification plan` need to change after `EP-M3` starts.
- Feasibility: stop if `EP-M0` shows that arm G or arm H cannot compile
  without one of the forbidden workarounds; present the diagnostics and options
  before continuing.
- Test-suite time: stop if the new `trybuild` binary's cold run, recorded in
  `EP-M1`, puts the whole suite at risk of nextest's 60-minute
  `global-timeout`; do not raise the timeout silently.
- Iterations: stop if one focused test or fixture still fails after five
  substantive fix attempts.
- Time: elapsed time is not a trigger (standing user instruction); a
  milestone whose focused tests make no progress across two sessions is.

Budget rationale. The only comparable plan is
`docs/execplans/1-1-3-compile-resource-policy-and-async-ownership-spike.md`.
Its implementation commit `0155d04` changed 52 code files (+6,402/−135), 4
configuration files (+1,528/−154, almost all `Cargo.lock`), and 9 documentation
files (+958/−19). That plan set no numeric budget and recorded no budget
increases, so there is no revision history to learn from; its scope also
included a library module, two body adapters, and a network harness that this
item does not need. The estimate here is bottom-up. The example has about 14
files (about 1,200 lines including the `EP-M3` additions). Six test binaries
plus three support files come to about 1,500 lines. Ten `trybuild` fixtures
with seven `.stderr` files come to about 1,000 lines; one feature file and one
snapshot about 100. That totals about 3,400 lines in about 45 files, so 4,000
lines and 52 files leave roughly 15% contingency. Documentation: a new evidence
ledger (about 450 lines), ADR 008 (about 160), targeted updates to ten existing
documents (about 500), and index entries; 1,400 lines and 16 files leave a
similar margin.

## Risks

- High impact, medium likelihood: the experiment is rigged by construction.
  Mitigation: arm G gets its best case (no allocation); arm E is also counted
  in its best case (a service implementing the erased port directly, no
  bridge); arm H covers the case study's own shape; shapes, counting rules, and
  the decision rule are fixed in this plan before measurement; measurement
  commits are separate and reproducible.
- High impact, medium likelihood: comparators are straw men. Mitigation:
  include resource-local Actix data
  (`web::resource().app_data(Data::from(arc))`) and Falcon's constructor
  pattern as the strong comparators, not only Corbusier's façade and Wildside's
  bundle; declare each comparator's role list before counting; cite sources;
  label sketches as uncompiled and, where pinned consumer sources are read,
  label those counts "observed, not normalized".
- Medium impact, medium likelihood: the reverse bridge for arm H is rejected
  by the compiler (for example, a hidden-type lifetime-capture error).
  Mitigation: `EP-M0` probes it with a single `'a` lifetime on the native port.
  If it fails, record the diagnostic as E1 evidence and stop for direction; the
  case study's shape failing is itself a finding.
- Medium impact, medium likelihood: compile-fail fixtures pass for the wrong
  reason. Mitigation: every negative has a positive sibling differing in one
  named respect, a paired control (the mutation compiles when undone), and a
  reviewed primary error code.
- Medium impact, medium likelihood: `trybuild` expectations churn when the
  pinned nightly or `async-trait` moves. Mitigation: place Send failures on an
  explicit `require_send` call rather than inside `async_trait` expansion where
  possible; follow `docs/developers-guide.md` fixture rules.
- Medium impact, medium likelihood: test hangs. Mitigation: never use the
  suspending `FixtureEnforcer` from `tests/support/spike/admission.rs`; use a
  non-suspending counting enforcer; bound spawned tasks with
  `tokio::time::timeout` as a failure detector only.
- Medium impact, low likelihood: per-binary dead code from loading the
  multi-file example. Mitigation: one wrapper module with an inner
  `#![allow(dead_code, reason = …)]`, following
  `tests/support/spike/components.rs`.
- Low impact, medium likelihood: `insta` adds licence or `rust-version`
  findings. Mitigation: record a dependency review in the ledger using
  `cargo metadata --format-version 1 --locked --offline`, as 1.1.3 did.

## Progress

- [x] (2026-11-14) Renamed the branch to
  `1-3-1-compare-concrete-and-erased-service-injection`; loaded `codegraph-mcp`,
  `rust-router`, `execplans`, and `firecrawl-mcp`.
- [x] (2026-11-14) Two Wyvern agents surveyed the spike seam, test
  conventions, and every governing design document.
- [x] (2026-11-14) Researched async erasure options and Actix/Falcon injection
  patterns with Firecrawl.
- [x] (2026-11-14) Drafted this ExecPlan (`b187197`).
- [x] (2026-11-14) Four-member expert panel (Rust API and async; experiment
  design and architecture; verification; process and documentation) reviewed
  the draft; findings applied in this revision.
- [ ] User approval received.
- [ ] `EP-M0` feasibility probe.
- [ ] `EP-M1` compile contract and ledger skeleton.
- [ ] `EP-M2` runnable specimen and behavioural evidence.
- [ ] `EP-M3` change-surface measurement.
- [ ] `EP-M4` decision, documentation, and roadmap closure.

## Surprises & discoveries

- 2026-11-14: The CodeGraph MCP server failed to connect for this session.
  Planning used file reads, scoped searches, and Wyvern agents. Retry CodeGraph
  before implementation's abstraction sweep; fall back to the `rg` sweep in
  `Interfaces and dependencies`.
- 2026-11-14: `docs/terms-of-reference.md` §9.2 (not the design document)
  says "The pinned Actix design is a design comparator; no runnable Actix
  baseline is claimed." That clause concerns Mornington's pinned design; E1's
  comparators are uncompiled sketches by analogy with it.
- 2026-11-14: The terms of reference record the assumption "Dynamic dispatch
  and future allocation are acceptable costs." Static dispatch is therefore not
  a benefit the project values on its own; the decision rule reflects that.
- 2026-11-14: The spike `Resource` trait has only a `get` responder, so E1
  uses a protected read (view one task). Mutation and unit-of-work semantics
  stay with 1.3.2.
- 2026-11-14: Native `async fn` in traits is still not dyn-compatible.
  `async-trait` and `dynosaur` (0.3.1, MIT or Apache-2.0) remain the common
  erasure routes; a handwritten boxed future is the dependency-free option.
- 2026-11-14 (panel): `async_trait` already boxes `Resource::get` as a `Send`
  future, so asserting `Send` on that future can never fail. `Send` evidence
  must target `ResourceRunner::invoke` and the port futures themselves.
- 2026-11-14 (panel): `AllocationId::of` takes `&Arc<T>`, so a service cannot
  observe its own allocation through it. Fakes record
  `std::ptr::from_ref(self).cast::<()>().addr()` instead.
- 2026-11-14 (panel): `u64::from_str` accepts `"+7"` and `"007"`, so the path
  parser must check its grammar explicitly.
- 2026-11-14 (panel): `trybuild` compiles each fixture at its original path,
  so a fixture can mount the real example modules with `#[path]`. `EP-M0`
  confirms this before the `missing_dependency_*` fixtures rely on it.
- 2026-11-14 (panel): `mapsplice` cannot mark 1.3.1 done, because 1.3.2 and
  2.1.2 declare it as a dependency; the 1.1.3 plan hit the same
  `dangling_dependency` refusal. A scratch roadmap reproduced it: `replace`
  fails even when the fragment is identical to the existing item. This is a
  `mapsplice` defect, reported as
  [leynos/mapsplice#228](https://github.com/leynos/mapsplice/issues/228); see
  `EP-M4` for the closure procedure.

## Decision log

- 2026-11-14, proposed: use a task-viewing specimen shaped after the case
  study's `TasksResource`, not Mornington's forum ports. The case study names a
  task/annotation specimen for E1, and Mornington's pinned design is not in
  this repository; 1.3.2 owns the Mornington direct-domain oracle.
- 2026-11-14, proposed (revised after panel): measure three arms, G, H, and E,
  as described in `Purpose`. The draft's two-arm comparison omitted the case
  study's own `TasksResource<S: ?Sized>` shape and the Pachislot baseline, so a
  loss for the generic resource could have been dismissed as testing the wrong
  generic. G and H share one resource type; H adds only the reverse bridge.
- 2026-11-14, proposed: erase ports with a handwritten
  `PortFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>` alias
  owned by the application. Rejected: `async-trait` on the port (hides the
  `Send` bound; the roadmap asks for explicit bounds); `dynosaur` (new
  dependency generating the same bridge); the unstable native dyn-async feature
  (incomplete, nightly-only).
- 2026-11-14, proposed: the application owns a forward bridge,
  `impl<T: TaskQueries> DynTaskQueries for T` (implicitly `T: Sized`), and a
  reverse bridge, `impl TaskQueries for dyn DynTaskQueries`. The implicit
  `Sized` bound must stay: removing it would overlap the compiler's built-in
  impl for the trait object, and the reverse bridge relies on that. Bridges are
  per-port erasure artefacts, reported separately from forwarding layers.
- 2026-11-14, proposed: pre-register the measured shapes so that no choice is
  made after seeing data. `TaskService<R>` becomes `TaskService<R, C>`;
  `TasksResource<S: ?Sized>` becomes `TasksResource<S: ?Sized, A: ?Sized>` with
  positional constructor arguments; `ErasedTasksResource` gains a positional
  `Arc<dyn DynTaskAnnotations>`. A bundle-struct variant may be reported as
  secondary evidence only.
- 2026-11-14, proposed (revised after panel): the decision rule breaks ties in
  favour of arm E, because the roadmap says to "retain the clearer boundary if
  generic resource wiring brings no benefit". A tie is no benefit. Arm E is
  declared the clearer boundary in advance because its constructor names the
  port trait and it has no type parameters.
- 2026-11-14, proposed: use `insta` for one snapshot of the example's rendered
  outcome rows across all arms and cases, paired with semantic assertions. The
  example output is the reader-facing, multivariant artefact; compiler
  diagnostics remain `trybuild` `.stderr` files.
- 2026-11-14, proposed: no Verus or Kani artefact. See
  `Formal verification decision`.
- 2026-11-14, proposed: record the retained boundary in a new ADR,
  `docs/adrs/adr-0008-resource-service-storage.md`, titled "Choose resource
  service storage" so the measured result cannot bias the name. It decides
  service storage inside a dyn `Resource` and explicitly does not decide ADR
  007's open question about `Resource` dyn-compatibility. ADR 004 receives a
  back-link only.
- 2026-11-14, proposed: Red-Green-Refactor is partly unavailable because E1
  changes no production code. Red evidence comes from runtime tests written
  against stub responders, from running each compile-fail mutation as a pass
  fixture first, and from seeded faults, as the 1.1.3 plan did.
- 2026-11-14, proposed: `docs/terms-of-reference.md` §9.2's statement that
  "Roadmap 1.3.1–1.3.2 owns the port-boundary specimen" stays accurate and
  needs no edit.
- 2026-11-14, proposed: the example is silent. `demonstrate()` returns the
  rendered outcome rows; tests and the snapshot read those rows, and an
  executable test checks the silent process contract.

## Outcomes & retrospective

Not started. Record, at completion: the measured values per arm and per
comparator, the retained boundary, whether H1 survived its rejection
conditions, residual gaps for 1.3.2 and 2.1.2, and lessons for the plan format.

## Context and orientation

Base revision inspected: `0155d0448a014aa169b734c3701825791a4c4306` on `main`
("Compile resource-policy and async ownership spike (1.1.3)"). Relocate symbols
with `rg -n '<symbol>' src tests examples`.

The crate `peregrine-web` (`Cargo.toml`, edition 2024) ships only an
experimental module, `src/spike/`, re-exported unconditionally from
`src/lib.rs` as `peregrine_web::spike`. The relevant items are:

- `Resource` (`src/spike/resource.rs`): an `#[async_trait]` trait with
  supertraits `Send + Sync`, methods
  `fn policy(&self, method: Method) -> &Policy`,
  `fn observation(&self) -> ResourceObservation`, and
  `async fn get(&self, view: RequestView<'_>, body: &mut ErasedBody)
  -> Result<ResponseDraft, ResourceError>`.
  Its future is boxed and `Send` through `async-trait`'s default.
- `ResourceRunner` (`src/spike/invoke.rs`): `new()`,
  `with_enforcer(Arc<dyn PolicyEnforcer>)`,
  `register(&self, Arc<dyn Resource>, Method)
  -> Result<RegisteredResource, RegistrationError>`,
  and
  `async fn invoke(&self, &RegisteredResource, RequestView<'_>,
  &mut ErasedBody) -> InvocationOutcome`.
  Registration freezes the method and a cloned policy; invocation checks the
  method, then admission, then calls `get` once. An anonymous request to an
  `Authenticated` resource is denied with `DenialReason::MissingIdentity`
  before the enforcer runs.
- `PolicyEnforcer` (`src/spike/policy_enforcer.rs`),
  `Policy::{Public, Authenticated, Scopes}`, `Identity::new`/`principal`,
  `RequestView::new(method, path, &headers, Option<&Identity>)`,
  `ResponseDraft::new(StatusCode, Bytes)`, `ResourceError`,
  `AllocationId::of(&Arc<T: ?Sized>)`,
  `ResourceObservation { resource, dependency, label }`, and `adapt_body`.
  `InvocationOutcome` does not implement `PartialEq`.

Terms. "Heterogeneous" means differently typed resources stored in one
`Vec<Arc<dyn Resource>>`. "Erased" means a value is used through a trait object
so its concrete type is hidden. "Dyn-compatible" (formerly "object safe") means
a trait can be used as `dyn Trait`; a trait method returning `impl Future` is
not dyn-compatible. A "port" is an application-owned trait that an adapter
calls (driving port) or that the application calls (driven port). A "bridge" is
a blanket or trait-object impl translating between the native and erased forms
of one port. "Forwarding layer" and the other measures are defined in
`Verification plan`.

Existing evidence and conventions. `examples/resource_policy_spike.rs` holds a
concrete `Arc<CatalogueStore>`; no example uses a trait-object port.
`tests/resource_policy_example.rs` loads it with `#[path]` and runs
`demonstrate()`; `tests/resource_policy_example_executable.rs` builds it with a
child `cargo build --offline`, runs the executable, and requires empty standard
output and standard error. `tests/ownership_contract.rs` runs `trybuild` over
`tests/ui/ownership/{pass,fail}/*.rs`; the closest fixture is
`tests/ui/ownership/pass/heterogeneous_collection.rs`.
`tests/support/spike/components.rs` shows the inner `#![allow(dead_code)]`
convention for modules shared across binaries;
`tests/support/spike/fixtures.rs` has the `verified` googletest helper and
`current_thread_runtime`. `tests/support/spike/admission.rs` has
`FixtureEnforcer`, which suspends on a rendezvous and must not be reused here.
Behavioural tests use `rstest-bdd` 0.6 (`tests/resource_policy_spike_bdd.rs`).
Development dependencies include `googletest` 0.14, `pretty_assertions` 1.4,
`proptest` 1.9, `rstest` 0.27, `rstest-bdd` 0.6, `trybuild` 1.0, `tokio` 1.54,
`cap-std` 4 (`fs_utf8`), `camino`, and `serde_json`; `insta` is absent.
`.config/nextest.toml` places child-Cargo tests (`ownership_contract`,
`resource_policy_example_executable`) in the serial `nested-cargo-builds` group.

Gates are Make targets: `make check-fmt`, `make lint` (rustdoc, Clippy with
`--all-targets --all-features`, then Whitaker; examples are linted),
`make test` (nextest then doctests), `make markdownlint`, `make spelling`, and
`make nixie`. Clippy denies, among others, `missing_const_for_fn`,
`must_use_candidate`, `needless_pass_by_value`, `print_stdout`,
`panic_in_result_fn`, `allow_attributes` (outer), and `expect_used`. The
toolchain is pinned to `nightly-2026-11-06`; the 1.1.3 work established the
borrow-checker posture as `polonius-alpha`, which does not change `Send`,
`Sync`, or dyn-compatibility rules. There is no Verus or Kani wiring in the
repository.

Gate execution. The repository's agent workflow delegates gates to a
`scrutineer` agent that runs them sequentially and returns log paths. Without
that agent, run them yourself, one after another:

```plaintext
set -o pipefail
make check-fmt 2>&1 | tee /tmp/check-fmt-peregrine-web-1-3-1.out
make lint 2>&1 | tee /tmp/lint-peregrine-web-1-3-1.out
make test 2>&1 | tee /tmp/test-peregrine-web-1-3-1.out
```

Signposts. Start with `docs/contents.md` and `docs/repository-layout.md`.
Governing design: `docs/hexagonal-application-case-study.md` §§2–6 (Table 2; E1
in Table 4), `docs/adrs/adr-0004-application-port-boundaries.md`,
`docs/peregrine-design.md` §4.2 and the E1–E5 paragraph in its verification
section, `docs/adrs/adr-0007-resource-policy-and-body-erasure-boundary.md`
(resource erasure; open question on generic authoring),
`docs/pachislot-extension-design.md` (search "boxed-future invocation adapter"),
`docs/polonius-ownership-experiment.md` (same boundary in both compiler
variants), `docs/terms-of-reference.md` §§6–9, and
`docs/resource-policy-ownership-spike.md`. Implementation guides:
`docs/developers-guide.md` ("The experimental spike module", "Maintain the
spike fixtures", "Fetching dependencies before tests"),
`docs/reliable-testing-in-rust-via-dependency-injection.md`,
`docs/rust-testing-with-rstest-fixtures.md`, `docs/rstest-bdd-users-guide.md`,
`docs/rust-doctest-dry-guide.md`,
`docs/complexity-antipatterns-and-refactoring-strategies.md`, and
`docs/documentation-style-guide.md`. Skills: `rust-router`, then
`rust-types-and-apis` (trait objects, generics, bridges) and
`rust-async-and-concurrency` (`Send` futures); `rust-unit-testing`, `proptest`,
`nextest`, and `rust-verification` (grounds the Verus/Kani decision);
`hexagonal-architecture`; `arch-decision-records` for ADR 008;
`en-gb-oxendict-style` for prose; `sdlc-implementation` for delivery;
`commit-message` and `pr-creation`; `codegraph-mcp` and `leta` for navigation.
Read each skill's `SKILL.md` before use.

Budget survey. Existing spike source is about 1,160 lines across eight files;
spike tests and support about 3,800 lines. The 1.1.3 commit is the only
comparable history and is summarized under `Tolerances`.

## Conformance basis

Upstream revisions at base `0155d04`: terms of reference draft v0.4
(2026-10-13); technical design proposed v0.6 (2026-10-13); hexagonal
application case study dated 2026-09-20; ADRs 001–005 and 007 proposed, ADR 006
accepted as policy. Upstream identifiers are goals G1–G4, questions Q1–Q6,
axioms A1–A4, assumptions S1–S4, and hypotheses H1–H5; no numbered requirements
exist upstream. This plan derives the following.

- `REQ-heterogeneous`: resources holding a concrete service and resources
  holding a dyn-compatible application port register and run through one
  `ResourceRunner` and one `Vec<Arc<dyn Resource>>`. (Roadmap 1.3.1; ADR 004.)
- `REQ-explicit-bounds`: every port future carries a visible `Send` bound, and
  every invocation future is `Send`. (Roadmap 1.3.1; S1.)
- `REQ-visible-deps`: each dependency is a constructor argument; omitting one
  is a compile error, never a request-time failure. (A4; ADR 004.)
- `REQ-port-independence`: the application module imports no Peregrine or
  HTTP type and no adapter module. (A1.)
- `REQ-arm-equivalence`: all arms produce the same observable outcome for the
  same request and store. (Case study §5 Table 2.)
- `REQ-measure`: forwarding layers and files changed when adding one
  dependency are measured for every arm and compared with Actix, Wildside, and
  Falcon comparators under pre-registered rules. (Roadmap 1.3.1; H1/E1.)
- `REQ-retain`: the clearer boundary is retained and recorded; generic
  resource wiring is kept only if it shows a benefit. (Roadmap 1.3.1.)
- `REQ-handoff`: the port shape, erasure strategy, and fixtures that 2.1.2
  must hold constant are fixed in writing. (Roadmap 2.1.2; ADR 004.)
- `GAP-no-port-specimen`: no repository code exercises a trait-object
  application port inside a resource.
- `GAP-no-h1-evidence`: H1 is asserted in design §4.2 but unmeasured.
- `GAP-second-compiler-variant`: ADR 004 asks for both options "on both
  compiler variants"; deferred to 2.1.2, which `REQ-handoff` prepares.

Trace links:

```plaintext
G1, G4 -> ADR-004 -> REQ-heterogeneous -> EP-M1 -> tests/ui/service_injection/pass/heterogeneous_service_storage.rs
G4 -> S1 -> REQ-explicit-bounds -> EP-M1 -> tests/ui/service_injection/fail/non_send_*_port_future.rs
A4 -> REQ-visible-deps -> EP-M3 -> tests/ui/service_injection/fail/missing_dependency_*.rs
A1 -> REQ-port-independence -> EP-M2 -> tests/service_injection_layering.rs
Case-study Table 2 -> REQ-arm-equivalence -> EP-M2 -> tests/service_injection_equivalence.rs
H1/E1 -> REQ-measure -> EP-M3 -> docs/service-injection-experiment.md (Measurements)
Roadmap 1.3.1 -> REQ-retain -> EP-M4 -> docs/adrs/adr-0008-resource-service-storage.md
Roadmap 2.1.2 -> REQ-handoff -> EP-M4 -> ADR 008 "Fixed for 2.1.2"
```

## Verification plan

### Abstract model

Let `D` be a finite store mapping task identifiers to tasks, each task owned by
one tenant. A request is `q = (identity?, path)`. A present identity's
principal either has the form `tenant:user` (both parts non-empty, no further
colon) or is badly formed. A path is well formed exactly when it is `/tasks/`
followed by a decimal identifier matching `[1-9][0-9]{0,18}`; every other path
(empty identifier, sign, leading zero, non-digit, extra segment, more than 19
digits) is malformed.

The use case `direct(ctx, id)` returns the task's view when `D[id]` exists and
its tenant equals `ctx.tenant`, and `NotFound` otherwise, so a cross-tenant
read is indistinguishable from a missing task. `project` maps `Ok(view)` to
status 200 with a body naming the task and `NotFound` to 404 with a fixed body
that names only the requested identifier.

The oracle, written in test code from this model alone:

- anonymous: `Denied(MissingIdentity)` (the runner's existing behaviour);
- badly formed principal: 403 with a fixed body, port not called;
- malformed path: 400 with a fixed body, port not called;
- otherwise: `project(direct(ctx(q), id(q)))`.

For each arm `a` in `{G, H, E}`, `run_a(q)` is the normalized outcome of
`ResourceRunner::invoke` on that arm's registered resource. Normalization maps
`InvocationOutcome` into a test-local `Normalized` enum (`Denied(DenialReason)`,
`Responded(u16, Vec<u8>)`, `Failed(String)`), because `InvocationOutcome` has
no `PartialEq`. Preconditions: the store is built before registration and is
immutable; each resource is registered once; the enforcer permits every
authenticated principal.

### Proof sketch for the compile-time lemmas

These lemmas are decided by the compiler, but stating why they should hold
shaped the interfaces and the `EP-M0` probe.

- `L-send`: `TasksResource<S>::get` holds `&RequestContext` and `Arc<S>`
  across the port await. `&RequestContext` is `Send` only if `RequestContext` is
  `Sync`; `&S` is `Send` only if `S: Sync`, which the port's `Send + Sync`
  supertraits supply, including for `S = dyn DynTaskQueries`. The port future is
  `Send` by declaration. Hence the responder future is `Send`, and so is
  `ResourceRunner::invoke`'s future over it. Design consequence:
  `RequestContext` must contain only `Sync` data, and `Send` appears in every
  port signature rather than being inferred.
- `L-dyn`: a method returning `impl Future` has no single vtable type, so
  `dyn TaskQueries` is ill-formed (`E0038`); a method returning `PortFuture`
  has one, so `dyn DynTaskQueries` is well formed. The forward bridge makes
  every sized native implementation usable as the erased port; the reverse
  bridge makes the erased object usable where the native port is required.
  Design consequence: both bridges are application-owned and the forward bridge
  stays `Sized`-bounded to avoid overlapping the built-in object impl.
- `L-constructor`: constructors are the only way to build a resource (private
  fields), and each dependency is a positional parameter, so omitting one is an
  arity error (`E0061`) at compile time.

### Invariants and lemmas

- `INV-equivalence` (`REQ-arm-equivalence`): for every store `D` and request
  `q` in the domain above, `run_G(q) = run_H(q) = run_E(q) = oracle(q)`.
- `INV-deny-before-port` (`REQ-heterogeneous`): when admission denies, or the
  adapter rejects a malformed path or principal, the injected port observes
  zero calls in every arm.
- `INV-injected-identity` (`REQ-visible-deps`): the service answering a
  request is the instance passed to the resource's constructor, before and
  after an awaited invocation, in every arm; `observation().dependency` is the
  `TaskQueries` allocation (after `EP-M3`, the annotations port is checked
  through its fake's recorded address, because `ResourceObservation` has one
  dependency slot).
- `INV-tenant-isolation` (A2): no outcome exposes a task whose tenant differs
  from the requesting context, and for the same identifier the cross-tenant
  response is byte-identical to the missing-task response.
- `L-send`, `L-dyn`, `L-constructor` as sketched above.

### Axioms (trusted, not verified here)

- `AXIOM-rustc`: the pinned compiler's auto-trait inference, dyn-compatibility
  rules, coherence, and return-position `impl Trait` in traits behave as the
  Rust Reference documents. Boundary: all compile fixtures.
- `AXIOM-async-trait`: `#[async_trait]` boxes `Resource::get` as
  `Pin<Box<dyn Future + Send + '_>>`. Boundary: every arm's `Resource` impl.
- `AXIOM-runner`: `ResourceRunner` behaves as the 1.1.3 evidence ledger
  records (admission before the responder; one call on permit). Boundary:
  `INV-deny-before-port`; re-observed here, not re-proved.
- `AXIOM-tokio`: `tokio::spawn` requires `Send + 'static` futures, and on a
  multi-thread runtime spawned tasks run on worker threads, never on the thread
  blocked in `block_on`. Boundary: the runtime witness of `L-send`.
- `AXIOM-comparators`: the Actix `web::Data` documentation (Arc wrapper;
  registration through `App::app_data` or `Resource::app_data`; `Data<dyn T>`
  through `Data::from(Arc<dyn T>)`; missing data yields 500 at request time)
  and the Falcon 4.4 ASGI tutorial (constructor injection, `create_app`
  construction, a missing constructor argument raising `TypeError` at startup)
  are accurate. Checked with Firecrawl on 2026-11-14; re-check the Falcon
  `TypeError` claim when writing the ledger. Boundary: comparator rows only.

### Methods per obligation

`INV-equivalence`.

- Method: `proptest` property with an independent oracle, an `rstest` table of
  named cases, and a deterministic generator-coverage test.
- Rationale: the claim spans stores and requests; generated inputs find
  mapping disagreements that hand-picked rows miss. The oracle is computed from
  the generated case's own tags and a `BTreeMap` lookup with a tenant
  comparison; it never calls an arm, the application service, or the path
  parser.
- Domain: generate the class first with `prop_oneof!` over
  `CaseClass::{Anonymous, BadPrincipal, Malformed, Found, CrossTenant, Missing}`,
  then build a store of 1–8 tasks over 3 tenants, an identity, and a path for
  that class. Malformed paths come from a tagged enum (`EmptyId`, `Signed`,
  `LeadingZero`, `NonDigit`, `ExtraSegment`, `TooLong`). This guarantees every
  class is reachable without filtering.
- Artefact: `tests/service_injection_equivalence.rs`; generators, the oracle,
  and `Normalized` in `tests/support/service_injection/cases.rs`.
- Evidence: `cargo nextest run --test service_injection_equivalence`; 256
  cases by default; any disagreement shrinks to a minimal store and request.
  Inside the property use `prop_assert_eq!` with a case description.
- Non-vacuity: `generator_covers_every_class` builds a `TestRunner` with
  `TestRng::deterministic_rng(RngAlgorithm::ChaCha)`, samples 1,000 cases via
  `new_tree(&mut runner)?.current()`, classifies them with the oracle into a
  local `BTreeMap<CaseClass, usize>`, and asserts every class is at least 5%
  and every malformed tag appears. No global state is used. Its own negative
  control: temporarily removing one `prop_oneof!` arm makes it fail. Seeded
  faults, each applied in the working tree and reverted: arm E maps `NotFound`
  to 403; the use case skips the tenant comparison; the path parser accepts
  leading zeros. Each must be rejected by the property.

`INV-deny-before-port`.

- Method: `rstest` cases with a counting fake port (a per-fake
  `Arc<AtomicUsize>`, no global state) injected into each arm.
- Artefact: `tests/service_injection.rs`.
- Evidence: `cargo nextest run --test service_injection`.
- Non-vacuity: a permitted sibling observes exactly one call. Red evidence
  comes from seeded faults, because the existing runner already denies before
  the responder: register with `Policy::Public` (the anonymous case must then
  observe a call and fail), and make a responder call the port before parsing
  the path (the malformed case must fail).

`INV-injected-identity`.

- Method: a named `rstest` test per arm. The fake records
  `std::ptr::from_ref(self).cast::<()>().addr()` on each call; the test
  compares it with `Arc::as_ptr(&arc).cast::<()>().addr()` for the `Arc` passed
  to the constructor, and compares `observation().dependency` with
  `AllocationId::of(&arc)` before registration and after an awaited invoke.
- Non-vacuity: two instances of the same concrete service type with different
  stores are registered side by side, so type identity cannot masquerade as
  object identity. Seeded fault: construct a resource with a second service
  instance while reporting the first in `observation()`; the test must fail.

`INV-tenant-isolation`.

- Method: the `CrossTenant` class of the property, plus one named `rstest`
  case per arm asserting byte-identity between the cross-tenant and missing
  responses for the same identifier.
- Rationale: the rule is one comparison in a specimen use case; the property
  already quantifies over stores and tenants.

`L-send`, `L-dyn`, `L-constructor`.

- Method: `trybuild` pass and fail fixtures, plus a runtime witness.
- Artefact: `tests/service_injection_contract.rs` driving
  `tests/ui/service_injection/pass/*.rs` and
  `tests/ui/service_injection/fail/*.rs`.
- Pass fixtures:
  - `heterogeneous_service_storage.rs`: arms G, H, and E in one
    `Vec<Arc<dyn Resource>>`, registered and invoked. `require_send` is
    applied to `runner.invoke(…)` and, inside a generic
    `fn witness<S: TaskQueries>(…)`, to `service.view(…)`.
  - `generic_only_native_port.rs`: arm G alone, with no bridges.
  - `erased_only_port.rs`: arm E's best case, a service implementing
    `DynTaskQueries` directly with no native trait or bridge; also the shape
    of a test double for the erased port.
- Fail fixtures, each a one-respect mutation of the named sibling:
  - `non_send_erased_port_future.rs` (sibling `erased_only_port`): drop
    `+ Send` from `PortFuture`; the error is placed on an explicit
    `require_send(port.view(…))`. Expected `E0277`, "cannot be sent between
    threads safely".
  - `non_send_native_port_future.rs` (sibling `generic_only_native_port`):
    drop `+ Send` from `TaskQueries::view`. Expected "future cannot be sent
    between threads safely" at `TasksResource::get`.
  - `native_port_as_dyn.rs` (sibling `erased_only_port`): name
    `Arc<dyn TaskQueries>`. Expected `E0038`.
  - `rc_backed_service.rs` (sibling `generic_only_native_port`): the service
    holds `Rc`. Expected `E0277` at `impl TaskQueries for RcService`, required
    by the `Send + Sync` supertraits.
  - `direct_erased_impl_conflicts_with_bridge.rs` (sibling
    `heterogeneous_service_storage`): one service implements both
    `TaskQueries` and `DynTaskQueries`. Expected `E0119`; this is the forward
    bridge's real cost and is reported in the ledger.
  - `missing_dependency_generic.rs` and `missing_dependency_erased.rs` (added
    in `EP-M3`; siblings are the real example constructors): each mounts the
    real example modules with `#[path]` and calls the constructor without its
    annotations argument, assigning to
    `let _resource: Arc<dyn Resource> = Arc::new(…)` so no secondary inference
    error appears. Expected `E0061`.
- Red evidence: for each fail fixture, first place the mutation in a pass
  fixture and observe the intended error code; then move it to `fail/` and
  record its `.stderr` with `TRYBUILD=overwrite` scoped to that file. Paired
  control: undoing the mutation in the fail fixture yields trybuild's "expected
  compile failure, but compiled successfully". Log both in the ledger.
- Runtime witness: `invocations_run_on_worker_threads` in
  `tests/service_injection.rs`, a
  `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`. It wraps the
  runner in `Arc`, and for each registered arm spawns a task that builds its own
  `HeaderMap`, `Identity`, `RequestView`, and body inside the task (a
  `RequestView` cannot cross `spawn`), returning
  `(std::thread::current().id(), outcome)`. Each `JoinHandle` is wrapped in
  `tokio::time::timeout` (failure bound only). It asserts each task ran on a
  thread other than the test thread and that outcomes match the oracle.
  Compile-time assertions that every real resource type is
  `Send + Sync + 'static` sit in the same file.

`REQ-port-independence` (structural).

- Method: `tests/service_injection_layering.rs` walks
  `examples/service_injection/application/` with `cap_std::fs_utf8`, and
  rejects any `use` or path root naming `peregrine_web`, `http`, `http_body`,
  `bytes`, `hyper`, or `super::http_adapter`/`super::outbound`/
  `crate::http_adapter`/`crate::outbound`. Comments are ignored.
- Non-vacuity: the same scanner run over
  `examples/service_injection/http_adapter/` must find matches. E5 in 1.3.2
  replaces this with forbidden-dependency fixtures; this test only guards the
  specimen.

End-to-end. `tests/service_injection_example_executable.rs`, modelled on
`tests/resource_policy_example_executable.rs`, builds the example with a child
`cargo build --offline --example service_injection --message-format=json`, runs
the produced executable, and requires exit status 0 and empty standard output
and standard error. It joins the `nested-cargo-builds` group.

Snapshot. `tests/service_injection.rs::example_outcomes_match_snapshot` calls
`demonstrate()`, renders its rows (`case | arm | status | body`, allocation
addresses excluded) to a `String`, and checks it with
`insta::assert_snapshot!`. It is paired with a semantic assertion that every
case's rows agree across arms. The first snapshot is written with
`INSTA_UPDATE=always` on that focused test and reviewed in `git diff` before
commit; afterwards snapshots change only for a confirmed contract change.

Assertion libraries. Use googletest `verify_that!` with `eq` and
`matches_pattern!` on `Normalized` values, through the existing `verified`
helper pattern in `tests/support/spike/fixtures.rs`; use
`pretty_assertions::assert_eq!` with messages for rows, address pairs, and
rendered tables. Every assertion has a diagnostic message. Fakes are
handwritten; `mockall` copes poorly with methods returning `impl Future`.

`REQ-measure` (measurement, not a correctness obligation).

Specimen layout frozen at the end of `EP-M2` and listed in the ledger. During
`EP-M3`, new files are created only at `application/clock.rs`,
`application/annotations.rs`, `outbound/clock.rs`, and
`outbound/annotations.rs`; erased ports and bridges for any port live in
`application/erased_ports.rs`.

Primary measures, per arm and scenario:

1. Files changed: distinct files under `examples/service_injection/` in the
   shared commit plus that arm's commit (`git show --numstat --format=`).
2. Sites changed: distinct items edited or added (type definitions, impl
   blocks, free functions, constructor call sites), counted from the same
   diffs. This measure does not depend on file layout.
3. Authored forwarding functions: application-authored functions or impl
   methods whose body only calls one other function with the same semantic
   arguments, adding at most boxing, `Arc` cloning, or a type conversion, and
   no decision or projection, on the call path from the adapter responder body
   to the use-case body. Framework dispatch and macro expansion are excluded.
4. Diagnostic names the dependency: yes when the text from the first
   `error[` line up to the next `error` line in that arm's
   `missing_dependency_*.stderr`, including `note:` and `help:` lines, contains
   the port trait identifier (`TaskAnnotations` or `DynTaskAnnotations`);
   otherwise no. (Arm H shares arm G's constructor and its result.)

Secondary measures (reported, not decisive): erasure artefacts (bridge or
erased-port methods per port; arm E's best case without the forward bridge is
reported here); type parameter lists touched; primary error code and diagnostic
line count; missing-dependency detection time (compile, startup, or request);
whether a universal façade, container, or application-context trait was needed
(yes/no with evidence).

Commit discipline: shared commits contain only items every arm uses (native
ports, services, outbound adapters, composition-root wiring of shared
services). Erased ports and both bridges go in the arm H or arm E commit that
first needs them. Each measurement commit carries a trailer
`E1-Measurement: <scenario>/<shared|G|H|E>`.

Scenarios, with shapes fixed now:

- `S-driven`: add a driven `Clock` port (native, `+ Send`) used by
  `TaskService`, which becomes `TaskService<R, C>`; `TaskView` gains an `as_of`
  field.
- `S-driving`: add a driving `TaskAnnotations` port (native, `+ Send`) so the
  response includes an annotation count. `TasksResource<S: ?Sized>` becomes
  `TasksResource<S: ?Sized, A: ?Sized>` with a positional `annotations: Arc<A>`
  constructor argument; `ErasedTasksResource` gains a positional
  `annotations: Arc<dyn DynTaskAnnotations>`.

Comparators, written as sketches for both scenarios with role lists declared in
the ledger before counting:

- (a) Actix application-wide façade in Corbusier's shape (`ApiState` holding
  `Arc<dyn …>` façades forwarding to services);
- (b) Actix resource-local data, `web::resource(…).app_data(Data::from(arc))`
  with `Data<dyn T>` extraction;
- (c) Falcon ASGI resource with constructor injection and `create_app`;
- (d) Wildside's `HttpState`/`HttpStatePorts` bundle with a builder.

Where pinned consumer sources from the case study (Corbusier `19e1daf…`,
Wildside `095178b…`, Episodic `0740f26…`) can be read with `gh api`, also
report the files and sites an equivalent port addition would touch, labelled
"observed, not normalized". If they cannot be read, record the gap.

Decision rule, pre-registered. Let "generic wiring" be the better of arms G and
H on each measure. Retain generic wiring as the documented default only if, in
both scenarios, it is no worse than arm E on every primary measure and strictly
better on at least one. Otherwise, including any full tie, document arm E
(`Arc<dyn Port>` in a non-generic resource) as the default, with concrete
storage permitted where an application needs it. Arm E is the clearer boundary
by declaration made before measurement, because its constructor names the port
trait and it has no type parameters.

H1 verdict rule, pre-registered. For each comparator, classify the retained
Peregrine arm as "saves" (strictly fewer files changed and no more authored
forwarding functions), "worse" (more files changed or more authored forwarding
functions), or "parity" (otherwise). H1 is supported only if the retained arm
saves against (a) and (d), reaches parity or better against (b) and (c), and
detects a missing dependency no later than any comparator. Otherwise record H1
as revised, using the per-comparator verdicts as the revised wording. If a
universal façade, container, or application-context trait was needed, H1 is
rejected regardless of counts.

### Formal verification decision

No Verus or Kani artefact is warranted, for these reasons.

- E1 introduces no contractual business logic that Peregrine ships. The tenant
  comparison is specimen code; real tenant and object authorization belong to
  each application (axiom A2). A Verus proof of a one-line equality would
  restate its definition.
- `L-send`, `L-dyn`, and `L-constructor` are decided by the compiler; a proof
  would re-derive Rust's auto-trait and dyn-compatibility rules. `trybuild`
  fixtures with paired controls observe the compiler's decision directly.
- The two small parsers (path and principal) are covered by tagged malformed
  classes in the property test. Kani is not proportionate for heap-allocated
  strings, trait-object futures, and async code, and the repository has no Kani
  or Verus wiring.
- The decision rule is applied by a person to recorded measurements, not
  implemented in code.

The lifecycle transition proof remains owned by roadmap 1.2.2. If
implementation exposes a non-trivial kernel (for example, a parser with a real
grammar), stop and revise this section with its specification and lemma
hypotheses before implementing it. No `assume`, `admit`, or trusted proof body
is introduced.

### Suggested BDD features

File: `tests/features/service_injection.feature`, run by
`tests/service_injection_bdd.rs` with `rstest-bdd` 0.6
(`cargo nextest run --test service_injection_bdd`). Scenarios trace to
`REQ-heterogeneous`, `REQ-arm-equivalence`, `INV-deny-before-port`, and
`INV-tenant-isolation` (`EP-M2`). The first, second, and fourth scenarios must
fail against stub responders; the third's no-call step gets red evidence from
the `Policy::Public` seeded fault.

```gherkin
Feature: Resources own concrete or erased application services

  Background:
    Given a task store where task 7 belongs to tenant "acme"
    And concrete, generic-over-erased, and erased resources registered together

  Scenario: Every storage option answers the same protected request
    When user "ada" of tenant "acme" requests "/tasks/7" from each resource
    Then every resource responds with status 200 naming task 7

  Scenario: Another tenant cannot tell the task exists
    When user "bob" of tenant "globex" requests "/tasks/7" from each resource
    Then every resource responds with status 404
    And the response matches the response for missing task 8

  Scenario: An anonymous request never reaches a service
    When an anonymous caller requests "/tasks/7" from each resource
    Then every invocation is denied for missing identity
    And no injected service has been called

  Scenario: A malformed path is rejected by the adapter
    When user "ada" of tenant "acme" requests "/tasks/07" from each resource
    Then every resource responds with status 400
    And no injected service has been called
```

## Plan of work

### `EP-M0` feasibility probe

Questions: (1) does `TasksResource<S: ?Sized + TaskQueries>`, whose `get`
awaits the native `impl Future + Send + 'a` port while borrowing a
`RequestContext`, compile under `#[async_trait]`? (2) Does the reverse bridge
`impl TaskQueries for dyn DynTaskQueries` compile alongside the `Sized` forward
bridge, so `TasksResource<dyn DynTaskQueries>` works? (3) Can a `trybuild`
fixture mount a module from `examples/` with `#[path]`?

Create `tests/service_injection_contract.rs` and the first version of
`tests/ui/service_injection/pass/heterogeneous_service_storage.rs`, plus a
throwaway pass fixture that mounts a one-line module from a scratch file under
`examples/service_injection/`. Run
`cargo nextest run --test service_injection_contract`. Go: all three compile;
keep the first fixture for `EP-M1` and delete the throwaway. No-go on (1) or
(2): record the diagnostic in `Surprises & discoveries`, set the status to
`BLOCKED`, and present options. No-go on (3): the `missing_dependency_*`
fixtures fall back to self-contained copies, and the ledger says so. Record the
transcript under `Artefacts and notes`.

### `EP-M1` compile contract and ledger skeleton

Mirror `tests/ownership_contract.rs` in `tests/service_injection_contract.rs`:
one `#[test]` calling `trybuild::TestCases::new()`, then
`pass("tests/ui/service_injection/pass/*.rs")` and
`compile_fail("tests/ui/service_injection/fail/*.rs")`. Add a
`.config/nextest.toml` override putting `binary(=service_injection_contract)`
in the `nested-cargo-builds` group with `slow-timeout = { period = "60s" }`,
with a comment matching the `ownership_contract` override.

Write the three pass fixtures and five fail fixtures listed under `L-send`,
`L-dyn`, and `L-constructor`, following the red-evidence and paired-control
procedure there. Fixtures are self-contained and declare minimal ports,
services, and resources. Record the binary's cold run time.

Create `docs/service-injection-experiment.md` as a skeleton: purpose, arms,
pre-registered measures and rules (copied from this plan), fixture evidence
table with captions, and empty measurement sections. Run the gates and commit.

### `EP-M2` runnable specimen and behavioural evidence

Create the multi-file example `examples/service_injection/` (Cargo discovers
`examples/<name>/main.rs`):

- `main.rs`: only `mod` declarations and a `main` that builds a current-thread
  runtime explicitly and returns `demonstrate()`'s error as
  `Result<(), Box<dyn Error>>`.
- `composition.rs`: the composition root.
  `pub async fn demonstrate() -> Result<Vec<OutcomeRow>, DemonstrationError>`
  builds one store, one `TaskService`, a second independently constructed
  `TaskService`, the three arms, and `PermitAuthenticated` (an enforcer
  permitting every authenticated principal, never suspending); registers all
  resources in one `Vec<Arc<dyn Resource>>`; invokes each with the named cases;
  and returns an error naming the first case where arms disagree. `OutcomeRow`
  has a `render()` method used by tests. No printing.
- `application/mod.rs`, `application/context.rs`: `TenantId`, `UserId`,
  `RequestContext { tenant, user }` (only `Sync` data).
- `application/tasks.rs`: `TaskId`, `Task`, `TaskView`, `TaskError`
  (`thiserror`), the driven port `TaskRepository`, the native driving port
  `TaskQueries`, and the concrete `TaskService<R: TaskRepository>` with the
  tenant check.
- `application/erased_ports.rs`: `PortFuture`, `DynTaskQueries`, the forward
  bridge, and the reverse bridge.
- `outbound/mod.rs`, `outbound/memory.rs`: `MemoryTasks`, an immutable
  `BTreeMap` repository whose `find` returns `std::future::ready(…)`.
- `http_adapter/mod.rs`: `application_context` (from the verified identity,
  principal grammar as in the model, failing closed), `parse_task_path`
  (explicit grammar, not `u64::from_str` alone), `project`, and the shared
  policy. Sibling imports use `super::`.
- `http_adapter/generic_tasks.rs`: `TasksResource<S: ?Sized + TaskQueries>`.
- `http_adapter/erased_tasks.rs`: `ErasedTasksResource`.

Test binaries reach the specimen through one wrapper,
`tests/support/service_injection/specimen.rs`, which mounts `application`,
`outbound`, `http_adapter`, and `composition` with `#[path]` at its root under
an inner `#![allow(dead_code, reason = …)]` whose reason says each integration
binary uses a different subset of the specimen, documented like
`tests/support/spike/components.rs`.
`tests/support/service_injection/fixtures.rs` holds counting fakes and a
non-suspending counting enforcer; `tests/support/service_injection/cases.rs`
holds generators, the oracle, and `Normalized`. The sweep (see
`Interfaces and dependencies`) justifies these new modules: the spike's
`FixtureEnforcer` suspends and its components model a different seam.

Sequence, each step its own commit after the gates pass:

1. Red: write the specimen with responders returning
   `Err(ResourceError::Failed { message: "not implemented".into() })`, then
   `tests/service_injection.rs` (named cases, identity, worker threads,
   compile-time bounds, snapshot) and `tests/service_injection_bdd.rs` with the
   feature. Observe the expected failures (200, 404, 400, 403 cases and
   scenarios 1, 2, and 4). This red state is not committed; record its output.
2. Green: implement the adapter functions and responders; commit with the
   named-case and BDD tests.
3. Green: add `tests/service_injection_equivalence.rs` with the property and
   coverage test; apply and revert each seeded fault, recording the rejections;
   commit.
4. Green: add `tests/service_injection_layering.rs` and
   `tests/service_injection_example_executable.rs`, with its nextest override;
   commit.
5. Add `insta` (below) and the snapshot; commit.
6. Refactor: tidy only what does not alter the forwarding being measured;
   commit separately, or record that none was needed.

Adding `insta`: add
`insta = { version = "1.<latest>", default-features = false }` under
`[dev-dependencies]` (caret requirement). Because `cargo fetch --locked`
rejects a changed manifest, run `cargo fetch` once without `--locked` in the
clean environment described in `docs/developers-guide.md`, confirm with
`git diff --stat Cargo.lock` that only `insta`'s graph changed, then return to
the locked workflow. Record the dependency review in the ledger.

Freeze and list the specimen file tree in the ledger at the end of this
milestone.

### `EP-M3` change-surface measurement

Each step is a separately gated commit with its `E1-Measurement` trailer:

1. `S-driven/shared`: `application/clock.rs` (`Clock`), `outbound/clock.rs`
   (a fixed clock), `TaskService<R, C>`, `TaskView::as_of`, composition-root
   wiring, and minimal test updates.
2. `S-driven/G`, `S-driven/H`, `S-driven/E`: whatever each arm's adapter
   needs, if anything; an empty arm commit is not made, and the ledger records
   zero.
3. `S-driving/shared`: `application/annotations.rs` (`TaskAnnotations`, its
   service), `outbound/annotations.rs` (in-memory store).
4. `S-driving/G`: `TasksResource<S, A>` and its constructor; composition
   wiring.
5. `S-driving/H`: `DynTaskAnnotations`-side reverse bridge in
   `application/erased_ports.rs` and composition wiring.
6. `S-driving/E`: `DynTaskAnnotations`, forward bridge,
   `ErasedTasksResource` field and constructor, composition wiring.
7. Add `missing_dependency_generic.rs` and `missing_dependency_erased.rs`
   with the red-evidence procedure.

Then compute the measures with the commands in `Concrete steps`, write the
comparator sketches and role lists, read the pinned consumer sources if
possible, and fill the ledger's measurement tables. If a step forces a change
to another arm's files, record it; that is evidence, not a defect.

### `EP-M4` decision, documentation, and roadmap closure

Apply the decision and H1 verdict rules. Write
`docs/adrs/adr-0008-resource-service-storage.md` ("Architectural decision
record (ADR) 008: Choose resource service storage", status Proposed, in ADR
007's layout and the style guide's ADR format), including: the decision, the
rejected options, the evidence links, a scope statement that it does not decide
ADR 007's open question on `Resource` dyn-compatibility, and a section "Fixed
for 2.1.2" naming the port trait shapes, `PortFuture`, the bridge directions,
where `Send` appears, the pass fixtures 2.1.2 must re-run unchanged under both
checkers, and the `rustc -vV` output and Polonius posture under which E1 was
measured.

Update these documents:

- `docs/peregrine-design.md` §4.2: replace "hypothesis H1, not a measured
  improvement" with the measured result; document the internal interface
  (`PortFuture`, both bridges, both constructors, the `Sync` requirement on
  `RequestContext`) as the component architecture, and link ADR 008. Add an E1
  evidence sentence to the E1–E5 paragraph in the verification section.
- `docs/adrs/adr-0004-application-port-boundaries.md`: a back-link to the
  E1 result and ADR 008 in "Consequences and verification" only.
- `docs/adrs/adr-0007-resource-policy-and-body-erasure-boundary.md`: under
  its outstanding decisions, state that E1 decided service storage, not
  `Resource` dyn-compatibility, and link ADR 008.
- `docs/hexagonal-application-case-study.md`: a dated note after Table 4
  pointing to the E1 result, without rewriting the research record.
- `docs/resource-policy-ownership-spike.md`: a cross-link to the new ledger.
- `docs/users-guide.md`: a section "Run the service-injection experiment"
  with a `<!-- tested-example: service-injection-example -->` marker before its
  fence, the command, and the silent-success contract.
- `docs/developers-guide.md`: the port-erasure convention and its re-use
  policy (each application owns its alias and bridges beside its ports;
  Peregrine exports none; adapters call either port but never define bridges),
  maintenance rules for `tests/ui/service_injection/`, the `E1-Measurement`
  trailer convention, and the specimen wrapper module.
- `docs/contents.md`: entries for the ledger, ADR 008, and this ExecPlan.
- `docs/repository-layout.md`: the example directory, the fixture directory,
  and the ledger; also correct its stale `docs/adr-*.md` path to `docs/adrs/`.
- `docs/roadmap.md`: change the 1.3.1 line to
  `- [x] 1.3.1. Compare concrete and erased service injection — done.` and
  update the opening paragraph's list of completed tasks. If
  [leynos/mapsplice#228](https://github.com/leynos/mapsplice/issues/228) is
  fixed in the installed release, use `mapsplice replace` with a preview diff
  first; otherwise edit the line manually and note the issue in the commit.

Search for stale claims with
`rg -n 'H1, not a measured|E1.*(planned|remain)' docs --glob '!docs/execplans/**'`
and reconcile each hit. Run every gate, including `make markdownlint`,
`make spelling`, and `make nixie`, then set this plan's status to `COMPLETE`
after reconciling the conformance basis.

## Milestones and plateaus

`EP-M1`: all three storage options compile, and five negative contracts fail as
specified.

- Requirements: `REQ-heterogeneous`, `REQ-explicit-bounds`.
- Evidence: `service_injection_contract` passes; the ledger skeleton holds the
  red and control transcripts.
- Conformance:
  - requirements satisfied for compile evidence;
  - design still followed (no `src/` change; no container);
  - upstream assumptions (S1) still valid, or the falsification is recorded;
  - no new public interface or dependency;
  - no trust-boundary or persisted-format change;
  - trace links current.
- Recovery: fixtures are additive; revert the commit.
- Remaining gaps: runtime evidence, measurement.
- Compatibility decision: none.

`EP-M2`: the runnable specimen shows all arms in one runner with equivalent
outcomes.

- Requirements: `REQ-arm-equivalence`, `REQ-port-independence`, and the
  `INV-*` obligations.
- Evidence: the five new runtime test binaries pass;
  `cargo run --example service_injection` exits 0 silently.
- Conformance: as for `EP-M1`, plus `insta` is the only new dependency and
  the layering test passes.
- Recovery: revert the milestone's commits; `EP-M1` remains valid.
- Remaining gaps: measurement.
- Compatibility decision: none.

`EP-M3`: both scenarios are applied to every arm in tagged commits; measures
and comparators are recorded.

- Requirements: `REQ-measure`, `REQ-visible-deps`.
- Evidence: ledger tables; the two `missing_dependency_*` fixtures.
- Conformance: as above, plus the pre-registered shapes and rules are
  unchanged since approval.
- Recovery: each commit is gated and revertible; re-run the count commands to
  regenerate tables; correct mistakes with a new tagged commit.
- Remaining gaps: decision and documentation.
- Compatibility decision: none.

`EP-M4`: the decision is recorded and documentation reconciled.

- Requirements: `REQ-retain`, `REQ-handoff`.
- Evidence: ADR 008; updated design §4.2; the roadmap checkbox.
- Conformance: every discovery is reflected upstream; ADR 007's question is
  untouched.
- Recovery: documentation-only revert.
- Compatibility decision: none.

## Concrete steps

All commands run from the repository root. Use `set -o pipefail` and log long
output with `tee` to
`/tmp/<action>-peregrine-web-1-3-1-compare-concrete-and-erased-service-injection.out`.

```plaintext
EP-M0/EP-M1 focused:
  cargo nextest run --test service_injection_contract
EP-M2 focused (red, then green):
  cargo nextest run --test service_injection --test service_injection_bdd
  cargo nextest run --test service_injection_equivalence
  cargo nextest run --test service_injection_layering \
    --test service_injection_example_executable
  cargo run --example service_injection; echo "exit=$?"   # expect exit=0, no output
EP-M3 counting:
  git log --format='%h %(trailers:key=E1-Measurement,valueonly)' --grep 'E1-Measurement'
  git show --numstat --format= <commit> -- examples/service_injection
  git show <commit> -- examples/service_injection   # count sites and forwarding by hand
Gates, sequential, after each milestone:
  make check-fmt; make lint; make test
Documentation gates, after documentation changes:
  make fmt; make markdownlint; make spelling; make nixie
Seeded-fault cleanup:
  git status --porcelain | grep proptest-regressions   # delete only files the fault created
```

## Validation and acceptance

- `cargo run --example service_injection` exits 0 and prints nothing on either
  stream; `tests/service_injection_example_executable.rs` enforces this.
- `cargo nextest run --test service_injection_contract` passes with three
  pass fixtures and seven fail fixtures. Each fail fixture's paired control
  compiles when its mutation is undone.
- The equivalence property passes 256 cases; `generator_covers_every_class`
  shows every class at 5% or more; each seeded fault is rejected as described.
- The BDD scenarios pass; scenarios 1, 2, and 4 failed against the stubs.
- The snapshot matches, and its rows agree across arms.
- `docs/service-injection-experiment.md` contains, per scenario, a captioned
  table of the four primary and the secondary measures for arms G, H, and E and
  comparators (a)–(d), then the decision and H1 verdict with the rules that
  produced them.
- Quality: `make check-fmt`, `make lint`, `make test`, `make markdownlint`,
  `make spelling`, and `make nixie` succeed. No Verus or Kani command applies
  (see `Formal verification decision`).

## Idempotence and recovery

Every step is additive and committed only when green; re-running focused tests
is safe. `TRYBUILD=overwrite` is used only for a single new fixture, and its
output is reviewed. Seeded faults are applied in the working tree only, never
committed, and reverted with `git checkout -- <file>`; afterwards check
`git status --porcelain` for `tests/*.proptest-regressions` files and delete
only those the fault created. If a measurement commit needs correction, add a
new tagged commit rather than rewriting history, and note it in the ledger.

## Artefacts and notes

Planning-time reconnaissance (2026-11-14), not implementation evidence:

```plaintext
Actix web::Data<T>: Arc wrapper; registered with App::app_data or Resource::app_data;
  Data<dyn T> via Data::from(Arc<dyn T>); missing data -> 500 at request time.
Falcon 4.4 ASGI tutorial: class Images: __init__(self, config, store);
  create_app() constructs Store and Images, then app.add_route(...).
dynosaur 0.3.1 (MIT OR Apache-2.0): generates a DynTrait for async/RPITIT traits.
```

`EP-M0` transcript: to be recorded.

## Interfaces and dependencies

No library interface changes. The specimen's application-owned signatures are:

```rust
// examples/service_injection/application/tasks.rs
pub trait TaskQueries: Send + Sync {
    fn view<'a>(
        &'a self,
        context: &'a RequestContext,
        id: TaskId,
    ) -> impl Future<Output = Result<TaskView, TaskError>> + Send + 'a;
}

// examples/service_injection/application/erased_ports.rs
pub type PortFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait DynTaskQueries: Send + Sync {
    fn view<'a>(
        &'a self,
        context: &'a RequestContext,
        id: TaskId,
    ) -> PortFuture<'a, Result<TaskView, TaskError>>;
}

// Forward bridge: keep the implicit `T: Sized`.
impl<T: TaskQueries> DynTaskQueries for T {
    fn view<'a>(
        &'a self,
        context: &'a RequestContext,
        id: TaskId,
    ) -> PortFuture<'a, Result<TaskView, TaskError>> {
        Box::pin(TaskQueries::view(self, context, id))
    }
}

// Reverse bridge, so `TasksResource<dyn DynTaskQueries>` (arm H) works.
impl TaskQueries for dyn DynTaskQueries {
    fn view<'a>(
        &'a self,
        context: &'a RequestContext,
        id: TaskId,
    ) -> impl Future<Output = Result<TaskView, TaskError>> + Send + 'a {
        DynTaskQueries::view(self, context, id)
    }
}
```

In `examples/service_injection/http_adapter/` (before `EP-M3`):

```rust
pub struct TasksResource<S: ?Sized> { tasks: Arc<S>, state: Arc<()>, policy: Policy }
impl<S: ?Sized + TaskQueries + 'static> TasksResource<S> {
    #[must_use]
    pub const fn new(tasks: Arc<S>, state: Arc<()>, policy: Policy) -> Self { /* … */ }
}

pub struct ErasedTasksResource { tasks: Arc<dyn DynTaskQueries>, state: Arc<()>, policy: Policy }
impl ErasedTasksResource {
    #[must_use]
    pub const fn new(tasks: Arc<dyn DynTaskQueries>, state: Arc<()>, policy: Policy) -> Self { /* … */ }
}
```

`state` gives `observation().resource` a per-resource allocation, as in the
1.1.3 fixtures. If `const fn` with these field types is rejected on the pinned
compiler, record it and drop `const`; Clippy will then not require it.

Abstraction sweep, required before adding `tests/support/service_injection/`
and the bridges (CodeGraph first if it reconnects, otherwise):

```plaintext
rg -n 'PortFuture|BoxFuture|Pin<Box<dyn Future' src tests examples
rg -n 'impl<T: .*> .* for T' src tests examples
rg -ln 'PolicyEnforcer for' tests examples
rg -n 'boxed-future' docs
```

Record the results in the ledger. Planning-time result: no existing port future
alias or bridge in `src/`, `tests/`, or `examples/`; enforcers exist in
`examples/resource_policy_spike.rs` and `tests/support/spike/admission.rs`,
neither suitable (one admits a single principal, the other suspends).

Reuse policy: `PortFuture` and the bridges are application-owned conventions
demonstrated by the specimen. Peregrine does not export them, because axiom A1
forbids ports from depending on Peregrine. The developers' guide records that
each application defines its own alias and bridges beside its ports, and that
adapters may call either port but must not define bridges.

Dependencies: add
`insta = { version = "1.<latest>", default-features = false }` as a development
dependency. Reuse `async-trait`, `thiserror`, `rstest`, `rstest-bdd`,
`rstest-bdd-macros`, `googletest`, `pretty_assertions`, `proptest`, `trybuild`,
`tokio`, `cap-std`, `camino`, `http`, `bytes`, `http-body-util`, and
`serde_json`. No runtime dependency is added.

## Revision note

- 2026-11-14: initial draft for expert-panel review.
- 2026-11-14: applied the four-member panel review. Added arm H (the case
  study's generic-over-dyn shape) and arm E's best case; pre-registered
  measured shapes, layout-independent measures, an operational diagnostic rule,
  a tie-safe decision rule favouring the declared clearer boundary, and an H1
  verdict rule over four comparators including Wildside's bundle; corrected
  expected diagnostics (`rc_backed_service` is `E0277` at the service impl),
  removed a vacuous `Send` assertion, and added the generic-arm `Send` negative
  and the bridge-conflict negative; replaced the trybuild "wip" red with
  mutation-first evidence and paired controls; made the example silent with an
  executable test; replaced the manual `rg` import check with a layering test;
  specified class-first generation with a deterministic coverage test, a fixed
  path grammar, `Normalized` outcomes, and a permissive non-suspending
  enforcer; corrected the terms-of-reference citation; added ADR 007, design,
  contents, layout, and roadmap updates, the 2.1.2 hand-off, exact counting
  commands, and raised the code and documentation budgets to cover the third
  arm. Milestone order is unchanged; scope grows by one arm and two fixtures.
- 2026-11-14: reported the `mapsplice` refusal to edit depended-upon tasks as
  leynos/mapsplice#228, and made `EP-M4` use `mapsplice` once the fix is
  released. No scope, obligation, or milestone change.
