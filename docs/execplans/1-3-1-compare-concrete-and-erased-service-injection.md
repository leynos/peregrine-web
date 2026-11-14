# Compare concrete and erased service injection (1.3.1)

This ExecPlan (execution plan) is a living document. The sections `Constraints`,
`Tolerances`, `Risks`, `Progress`, `Surprises & discoveries`, `Decision log`,
`Outcomes & retrospective`, `Conformance basis`, and `Verification plan` must
be kept up to date as work proceeds.

Status: DRAFT (awaiting expert-panel review and user approval)

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
number of files changed when one dependency is added, compared with Actix and
Falcon patterns. Nothing in the repository has tested either claim yet.

This work builds experiment E1: a small, runnable task-viewing specimen in
which two differently wired resources share the existing spike router seam
(`peregrine_web::spike::ResourceRunner`). One resource is generic over a
concrete service. The other holds an `Arc<dyn …>` application port. Both use
explicit `Send` bounds on their futures. The work then adds one dependency to
each arm in separately recorded commits, counts forwarding layers and files
changed, compares those counts with written Actix and Falcon comparators, and
records which boundary to retain.

After the change, a contributor can run:

```plaintext
cargo run --example service_injection
```

and see both resources registered in one heterogeneous collection, admitted by
the same policy gate, and answering the same requests with identical outcomes.
They can run `cargo test --test service_injection_contract` to see the compiler
accept both storage options and reject a non-`Send` port future, a native-async
port used as a trait object, a non-thread-safe service, and a missing
constructor dependency. They can read `docs/service-injection-experiment.md` to
see the measured change surface, the comparator counts, and the retained
boundary with its reasons.

Observable outcome, in short: a reader can see which of the two service-storage
options Peregrine's documentation will recommend, why, and the commands that
reproduce the evidence. No framework API, router, or server is delivered.
Existing behaviour of `src/spike/` is preserved unchanged.

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
  the specimen's HTTP adapter or outbound modules (axiom A1 of the case study).
  Only the composition root (`examples/service_injection/main.rs`) may see
  every layer. Inbound adapter files must not import outbound adapter files.
- Application ports carry explicit `Send` bounds in their signatures. The
  erased port uses a hand-written boxed future type, not a macro that hides the
  bound. The generic arm's port uses a return-position `impl Future + Send`. No
  `?Send` relaxation, `Rc`, thread-local state, `Arc<Mutex<_>>` workaround, or
  `unsafe` code.
- Resources remain `Send + Sync + 'static` objects registered as
  `Arc<dyn Resource>`; the router's resource erasure must not force service
  erasure (ADR 004 proposed direction).
- Tenant and object authorization belong to the application use case, not to
  `PolicyEnforcer` (axiom A2). The spike policy gate demonstrates admission
  ordering only.
- E1 is not E2: no non-HTTP driver, no mutation, no unit of work, and no
  application-crate dependency isolation are delivered here; they belong to
  roadmap 1.3.2. Mutation would require a `POST` responder that the spike seam
  does not have.
- Actix and Falcon comparators are design comparators written as sketches in
  documentation. No `actix-web` or Python dependency is added. The design
  document states that no runnable Actix baseline is claimed.
- No runtime-performance or code-size claim is made. The terms of reference
  accept dynamic dispatch and future allocation as costs.
- Keep files under 400 lines; every module begins with a `//!` comment; every
  assertion carries a diagnostic message; no `expect`/`unwrap` outside tests;
  lint suppressions use `#[expect(…, reason = "…")]`, tightly scoped.
- Use the shared Cargo cache and compiler admission. Do not bypass
  `RUSTC_WRAPPER`, use `/tmp` as a build target, or run gates in parallel.
- Commit only gated plateaus. Measurement commits (see `EP-M3`) must each pass
  the gates on their own, because they are permanent rollback points.

## Tolerances (exception triggers)

All counts are against the merge base with `main`
(`git merge-base HEAD origin/main`), measured with
`git diff --numstat $(git merge-base HEAD origin/main) -- <paths>`. Net lines
are additions minus deletions. This ExecPlan file is excluded from every count.
Each category is enforced independently; spare capacity in one category never
offsets another.

- Code (Rust sources under `examples/`, `tests/` including `tests/ui/**`
  fixtures, `.stderr` expectations, `.feature` files, and `.snap` snapshots):
  stop if the change exceeds 3,200 net lines or 40 files.
- Configuration (`Cargo.toml`, `Cargo.lock`, `.config/nextest.toml`,
  `typos.local.toml`, regenerated `typos.toml`): stop if the change exceeds 450
  net lines or 6 files. `Cargo.lock` counts; most of this budget is reserved
  for the `insta` dependency graph.
- Documentation (`docs/**` except `docs/execplans/**`, `README.md`): stop if
  the change exceeds 1,200 net lines or 14 files.
- Interface: stop if any change to `src/` appears necessary, including a new
  `Resource` method, a `POST` responder, or a changed bound.
- Dependencies: only `insta` (development, with the `yaml` feature off unless
  required) is authorized beyond the existing manifest. `dynosaur`,
  `trait-variant`, `actix-web`, and any runtime dependency require escalation.
- Architecture: stop if E1 appears to need a shared framework-level port
  helper, a universal application-context trait, or service-locator storage.
- Experiment integrity: stop if the pre-registered counting rules or decision
  rule in `Verification plan` need to change after measurement starts.
- Iterations: stop if one focused test or fixture still fails after five
  substantive fix attempts.
- Ambiguity: stop and present options if the measurement produces a tie that
  the pre-registered rule does not resolve.

Budget rationale. The comparable plan is
`docs/execplans/1-1-3-compile-resource-policy-and-async-ownership-spike.md`.
Its implementation commit `0155d04` changed 52 code files (+6,402/−135), 4
configuration files (+1,528/−154, almost all `Cargo.lock`), and 9 documentation
files (+958/−19). That plan recorded no formal numeric budget and no budget
increases, so there is no revision history to learn from; its scope also
included a library module, two body adapters, and a network harness that this
item does not need. The estimate here is bottom-up: the example is about seven
files of 60–200 lines (about 900 lines); runtime tests and support about 900
lines; ten `trybuild` fixtures with expectations about 900 lines; one feature
file and one snapshot about 80 lines. That totals about 2,800 lines, so 3,200
leaves roughly 15% contingency. Documentation follows the 1.1.3 pattern: a new
evidence ledger (about 350 lines), a new ADR (about 140), targeted updates to
six existing documents (about 400), and index entries. Configuration is
dominated by the lockfile entries `insta` pulls in.

## Risks

- High impact, medium likelihood: the experiment is rigged by construction,
  for example by giving the generic arm a boxed port so that it can show no
  benefit. Mitigation: the generic arm uses its best case (native
  `impl Future + Send`, no allocation); the counting rules and decision rule
  are written in this plan before any measurement; the expert panel reviews
  them; measurement commits are separate and reproducible.
- High impact, medium likelihood: the Actix and Falcon comparators are unfair
  straw men. Mitigation: use resource-local Actix data (`web::resource()` with
  `app_data(Data::from(arc))`) as the fair baseline, not only Corbusier's global
  `ApiState` façade; use Falcon's documented constructor and `create_app`
  pattern; cite each source; mark sketches as uncompiled.
- Medium impact, medium likelihood: compile-fail fixtures pass for the wrong
  reason. Mitigation: every negative fixture has a positive sibling that
  differs in one named respect, and each `.stderr` is reviewed for its primary
  error code before acceptance.
- Medium impact, medium likelihood: `trybuild` expectations churn when the
  pinned nightly moves. Mitigation: keep fixtures minimal, assert the
  compiler's own error code, and follow the existing fixture maintenance rules
  in `docs/developers-guide.md` (never `TRYBUILD=overwrite` blindly).
- Medium impact, low likelihood: the generic arm's `impl Future + Send` port
  future is not accepted inside the `async_trait`-boxed `Resource::get` future,
  for example because a borrowed context is not `Sync`. Mitigation: `EP-M0`
  probes this before any other work; failure is recorded as E1 evidence, not
  worked around.
- Medium impact, low likelihood: loading a multi-file example into several
  test binaries with `#[path]` produces dead-code warnings. Mitigation: follow
  the existing `tests/resource_policy_example.rs` pattern with narrowly scoped
  `#[expect(dead_code, reason = …)]`, or expose a single `demonstrate()` per
  binary.
- Low impact, medium likelihood: `insta` adds licence or `rust-version`
  findings. Mitigation: record a dependency review in the evidence ledger as
  1.1.3 did, using `cargo metadata --locked --offline`.

## Progress

- [x] (2026-11-14) Renamed the branch to
  `1-3-1-compare-concrete-and-erased-service-injection`; loaded `codegraph-mcp`,
  `rust-router`, `execplans`, and `firecrawl-mcp`.
- [x] (2026-11-14) Two Wyvern agents surveyed the spike seam, test
  conventions, and every governing design document.
- [x] (2026-11-14) Researched async erasure options and Actix/Falcon injection
  patterns with Firecrawl.
- [x] (2026-11-14) Drafted this ExecPlan.
- [ ] Expert-panel design review applied.
- [ ] User approval received.
- [ ] `EP-M0` feasibility probe.
- [ ] `EP-M1` compile contract for both storage options.
- [ ] `EP-M2` runnable specimen and behavioural evidence.
- [ ] `EP-M3` change-surface measurement.
- [ ] `EP-M4` decision, documentation, and roadmap closure.

## Surprises & discoveries

- 2026-11-14: The CodeGraph MCP server failed to connect for this session.
  Planning used file reads, scoped searches, and Wyvern agents. Retry CodeGraph
  before implementation's abstraction sweep.
- 2026-11-14: `docs/peregrine-design.md` §9 already says "The pinned Actix
  design is a design comparator; no runnable Actix baseline is claimed." This
  settles the comparator form without a new decision.
- 2026-11-14: The terms of reference record the assumption "Dynamic dispatch
  and future allocation are acceptable costs." Static dispatch is therefore not
  a benefit the project currently values on its own; the decision rule reflects
  that.
- 2026-11-14: The spike `Resource` trait has only a `get` responder. A
  mutation use case cannot be expressed without changing `src/`, so E1 uses a
  protected read (view one task). Mutation and unit-of-work semantics stay with
  1.3.2.
- 2026-11-14: Native `async fn` in traits is still not dyn-compatible on
  stable Rust. `async-trait` and `dynosaur` (0.3.1, MIT or Apache-2.0) remain
  the common erasure routes; a hand-written boxed future is the dependency-free
  option.

## Decision log

- 2026-11-14, proposed: use a task-viewing specimen shaped after the case
  study's `TasksResource`, not Mornington's forum ports. The case study names a
  task/annotation specimen for E1, and Mornington's pinned design is not in
  this repository; 1.3.2 owns the Mornington direct-domain oracle.
- 2026-11-14, proposed: compare two arms. Arm G is a generic resource
  `TasksResource<S>` over a native-async port (`impl Future + Send`), holding a
  concrete service. Arm E is a non-generic `ErasedTasksResource` holding
  `Arc<dyn DynTaskQueries>`, a dyn-compatible port with an explicit boxed
  `Send` future. Arm G gets its best case (no allocation, static dispatch) so
  that "no benefit" cannot be an artefact of the experiment.
- 2026-11-14, proposed: erase the port with a hand-written
  `Pin<Box<dyn Future<Output = T> + Send + 'a>>` alias owned by the
  application. Rejected alternatives: `async-trait` on the port (hides the
  `Send` bound in a macro, and the roadmap asks for explicit bounds);
  `dynosaur` (new dependency, generates the same bridge); the unstable native
  dyn-async feature (incomplete, nightly-only, and would make the result
  unportable).
- 2026-11-14, proposed: the application owns one bridge,
  `impl<T: TaskQueries> DynTaskQueries for T`, so one concrete service serves
  both arms. The bridge is counted as one forwarding layer of the erased arm.
- 2026-11-14, proposed: add `insta` as a development dependency for one
  cross-arm outcome snapshot, paired with semantic assertions, because the
  roadmap asks for snapshot tests where multivariant output matters and the
  outcome matrix is exactly that. Compiler-diagnostic snapshots remain
  `trybuild` `.stderr` files.
- 2026-11-14, proposed: no Verus or Kani artefact. E1 introduces no
  contractual business logic or non-trivial pure kernel; its obligations are
  compile-time contracts, delegation, and arm equivalence. See
  `Formal verification decision`.
- 2026-11-14, proposed: record the retained boundary in a new ADR 008, because
  it selects the convention later phases and documentation will follow, and
  reference it from ADR 004 and design §4.2.
- 2026-11-14, proposed: Red-Green-Refactor is partly unavailable because E1
  changes no production code and its compile fixtures depend only on the
  existing seam. Red evidence is obtained by writing runtime tests against
  specimen stubs before implementing responders, and by seeded faults and
  negative controls for compile fixtures, as the 1.1.3 plan did.

## Outcomes & retrospective

Not started. Record, at completion: the measured counts per arm and per
comparator, the retained boundary, whether H1 survived its rejection
conditions, residual gaps for 1.3.2 and 2.1.2, and lessons for the plan format.

## Context and orientation

Base revision inspected: `0155d0448a014aa169b734c3701825791a4c4306` on `main`
("Compile resource-policy and async ownership spike (1.1.3)").

The crate `peregrine-web` (`Cargo.toml`, edition 2024) currently ships only an
experimental module, `src/spike/`, re-exported unconditionally from
`src/lib.rs` as `peregrine_web::spike`. The relevant items are:

- `peregrine_web::spike::Resource` (`src/spike/resource.rs`), an
  `#[async_trait]` trait with supertraits `Send + Sync`, methods
  `fn policy(&self, method: Method) -> &Policy`,
  `fn observation(&self) -> ResourceObservation`, and
  `async fn get(&self, view: RequestView<'_>, body: &mut ErasedBody)
  -> Result<ResponseDraft, ResourceError>`.
  Its future is boxed and `Send` through `async-trait`'s default.
- `peregrine_web::spike::ResourceRunner` (`src/spike/invoke.rs`), with
  `new()`, `with_enforcer(Arc<dyn PolicyEnforcer>)`,
  `register(&self, Arc<dyn Resource>, Method)
  -> Result<RegisteredResource, RegistrationError>`,
  and
  `async fn invoke(&self, &RegisteredResource, RequestView<'_>,
  &mut ErasedBody) -> InvocationOutcome`.
  Registration freezes the method and a cloned policy; invocation checks
  method, admission, then calls `get` once.
- `PolicyEnforcer` (`src/spike/policy_enforcer.rs`), `Policy`
  (`Public`, `Authenticated`, `Scopes`), `Identity`, `RequestView`,
  `ResponseDraft`, `ResourceError`, `AllocationId::of(&Arc<T: ?Sized>)`, and
  `ResourceObservation { resource, dependency, label }`.

"Heterogeneous" means differently typed resources stored in one
`Vec<Arc<dyn Resource>>`. "Erased" means a value is used through a trait object
so its concrete type is hidden. "Dyn-compatible" (formerly "object safe") means
a trait can be used as `dyn Trait`; a trait method returning `impl Future` is
not dyn-compatible. A "port" is an application-owned trait that an adapter
calls (driving port) or that the application calls (driven port). A "forwarding
layer" is defined precisely in `Verification plan`.

The only runnable example is `examples/resource_policy_spike.rs`. Its resource
holds a concrete `Arc<CatalogueStore>`; no example uses a trait-object port.
`tests/resource_policy_example.rs` loads the example with
`#[path = "../examples/resource_policy_spike.rs"]` and runs `demonstrate()`
under Tokio. `tests/ownership_contract.rs` runs `trybuild` over
`tests/ui/ownership/{pass,fail}/*.rs`; the closest existing fixture is
`tests/ui/ownership/pass/heterogeneous_collection.rs`, which registers two
unit-state resources. Behavioural tests use `rstest-bdd` 0.6 with
`tests/features/resource_policy_spike.feature` and
`tests/resource_policy_spike_bdd.rs`. `googletest` 0.14, `pretty_assertions`
1.4, `proptest` 1.9, `rstest` 0.27, `trybuild` 1.0, and `tokio` 1.54 are
development dependencies. `insta` is absent. `.config/nextest.toml` places
child-Cargo tests, including `ownership_contract`, in the serial
`nested-cargo-builds` group.

Gates are Make targets: `make check-fmt`, `make lint` (rustdoc, Clippy with
`--all-targets --all-features`, then Whitaker), `make test` (nextest then
doctests), `make markdownlint`, `make spelling`, and `make nixie`. Examples are
linted and built as targets. The toolchain is pinned to `nightly-2026-11-06`;
the 1.1.3 work established the borrow-checker posture as `polonius-alpha`,
which does not change `Send`, `Sync`, or dyn-compatibility rules. There is no
Verus or Kani wiring in the repository.

Signposts. Start with `docs/contents.md` and `docs/repository-layout.md`.
Governing design: `docs/hexagonal-application-case-study.md` §§2–6 (E1 in Table
4), `docs/adrs/adr-0004-application-port-boundaries.md`,
`docs/peregrine-design.md` §4.2 and §9,
`docs/adrs/adr-0007-resource-policy-and-body-erasure-boundary.md` (resource
erasure and its open question on generic authoring),
`docs/pachislot-extension-design.md` (generic authoring plus boxed-future
adapter as the baseline), `docs/polonius-ownership-experiment.md` (same
boundary in both compiler variants), `docs/terms-of-reference.md` §§6–9, and
`docs/resource-policy-ownership-spike.md`. Implementation guides:
`docs/developers-guide.md` (spike module and fixture maintenance),
`docs/reliable-testing-in-rust-via-dependency-injection.md`,
`docs/rust-testing-with-rstest-fixtures.md`, `docs/rstest-bdd-users-guide.md`,
`docs/rust-doctest-dry-guide.md`,
`docs/complexity-antipatterns-and-refactoring-strategies.md`, and
`docs/documentation-style-guide.md`. Skills: `rust-router` then
`rust-types-and-apis` (trait objects, generics) and `rust-async-and-concurrency`
(`Send` futures); `rust-unit-testing`, `proptest`, and
`hexagonal-architecture`; `arch-decision-records` for ADR 008; `mapsplice` for
the roadmap; `commit-message` and `pr-creation` for delivery; `codegraph-mcp`
and `leta` for navigation. Read each skill's `SKILL.md` before use.

Budget survey. Existing spike source is about 1,160 lines across eight files;
spike tests and support about 3,800 lines. The 1.1.3 commit is the only
comparable history and is summarized under `Tolerances`.

## Conformance basis

Upstream revisions at base `0155d04`: terms of reference draft v0.4
(2026-10-13); technical design proposed v0.6 (2026-10-13); hexagonal
application case study dated 2026-09-20; ADRs 001–005 and 007 proposed, ADR 006
accepted as policy. No formal requirement identifiers exist upstream beyond
goals G1–G4, questions Q1–Q6, axioms A1–A4, assumptions S1–S4, and hypotheses
H1–H5; this plan derives the following.

- `REQ-heterogeneous`: resources holding a concrete service and resources
  holding a dyn-compatible application port register and run through one
  `ResourceRunner` and one `Vec<Arc<dyn Resource>>`. (Roadmap 1.3.1; ADR 004.)
- `REQ-explicit-bounds`: every port future carries a visible `Send` bound, and
  every resource future is `Send`. (Roadmap 1.3.1; S1.)
- `REQ-visible-deps`: each dependency is a constructor argument; omitting one
  is a compile error, never a request-time failure. (A4; ADR 004.)
- `REQ-port-independence`: the application module imports no Peregrine or
  HTTP type. (A1.)
- `REQ-arm-equivalence`: both arms produce the same observable outcome for the
  same request and store. (Case study §5 Table 2: storage choice is independent
  of the lifecycle.)
- `REQ-measure`: forwarding layers and files changed when adding one
  dependency are measured for both arms and compared with Actix and Falcon
  comparators under pre-registered rules. (Roadmap 1.3.1; H1/E1.)
- `REQ-retain`: the clearer boundary is retained and recorded; generic
  resource wiring is kept only if it shows a benefit. (Roadmap 1.3.1.)
- `GAP-no-port-specimen`: no repository code exercises a trait-object
  application port inside a resource.
- `GAP-no-h1-evidence`: H1 is asserted in design §4.2 but unmeasured.

Trace links:

```plaintext
G1, G4 -> ADR-004 -> REQ-heterogeneous -> EP-M1 -> tests/ui/service_injection/pass/*
G4 -> S1 -> REQ-explicit-bounds -> EP-M1 -> tests/ui/service_injection/fail/non_send_port_future.rs
A4 -> REQ-visible-deps -> EP-M1 -> tests/ui/service_injection/fail/missing_dependency_*.rs
A1 -> REQ-port-independence -> EP-M2 -> rg import check in Concrete steps
Case-study Table 2 -> REQ-arm-equivalence -> EP-M2 -> tests/service_injection_equivalence.rs
H1/E1 -> REQ-measure -> EP-M3 -> docs/service-injection-experiment.md §Measurements
Roadmap 1.3.1 -> REQ-retain -> EP-M4 -> docs/adrs/adr-0008-resource-service-storage.md
```

## Verification plan

### Abstract model

Let `D` be a finite store mapping task identifiers to tasks, each task having
an owning tenant. Let a request be `q = (identity?, path)`, where identity is
optional and, when present, names a tenant and user. Let `direct(ctx, id)` be
the application use case: it returns the task when `D[id]` exists and its
tenant equals `ctx.tenant`, and `NotFound` otherwise (cross-tenant reads are
indistinguishable from missing tasks). Let `project` map a use-case result to
an HTTP outcome: `Ok(task)` to `200` with a body naming the task; `NotFound` to
`404`. Let `admit(policy, q)` be the existing runner's admission: an anonymous
request to an `Authenticated` resource is `Denied(MissingIdentity)` before any
responder runs. Let `parse(path)` fail closed: a malformed path yields `400`
without calling the port.

For each arm `a` in `{G, E}`, `run_a(q)` is the `InvocationOutcome` from
`ResourceRunner::invoke` on that arm's registered resource. Preconditions: the
store is constructed before registration and not mutated during a request; the
resource is registered once.

### Invariants and lemmas

- `INV-equivalence` (`REQ-arm-equivalence`): for every store `D` and request
  `q`, `run_G(q) = run_E(q) = oracle(q)`, where `oracle(q)` is
  `Denied(MissingIdentity)` when unauthenticated, `400` when the path is
  malformed, and `project(direct(ctx(q), id(q)))` otherwise.
- `INV-deny-before-port` (`REQ-heterogeneous`): when admission denies, the
  injected port observes zero calls in both arms.
- `INV-injected-identity` (`REQ-visible-deps`): the dependency observed by a
  resource's `observation()` and the service that answers the request are the
  allocation passed to its constructor, before and after an awaited invocation,
  in both arms.
- `INV-tenant-isolation` (A2): no outcome exposes a task whose tenant differs
  from the requesting context, in either arm or the direct path.
- `L-send` (`REQ-explicit-bounds`): if every port future is `Send` and every
  value the responder holds across an await is `Send`, then each arm's
  `Resource::get` future is `Send`, and so is `ResourceRunner::invoke` on it.
- `L-dyn` (`REQ-heterogeneous`): a port whose method returns
  `impl Future` cannot be named as `dyn Port`; a port returning the boxed
  `Send` future can, and the bridge makes every native implementation usable
  through it.
- `L-constructor` (`REQ-visible-deps`): a resource cannot be built without each
  of its declared dependencies.

### Axioms (trusted, not verified here)

- `AXIOM-rustc`: the pinned compiler's auto-trait inference for `Send`/`Sync`,
  dyn-compatibility rules, and return-position `impl Trait` in traits behave as
  documented in the Rust Reference. Boundary: all compile fixtures.
- `AXIOM-async-trait`: `#[async_trait]` boxes `Resource::get` as
  `Pin<Box<dyn Future + Send + '_>>`. Boundary: both arms' `Resource` impls.
- `AXIOM-runner`: `ResourceRunner` behaves as the 1.1.3 evidence ledger
  records (admission before responder, one call on permit). Boundary:
  `INV-deny-before-port`; this plan re-observes but does not re-prove it.
- `AXIOM-tokio`: the multi-thread runtime requires `Send + 'static` futures for
  `tokio::spawn`. Boundary: the runtime witness of `L-send`.
- `AXIOM-comparators`: the Actix `web::Data` documentation (Arc wrapper,
  registration through `App::app_data` or `Resource::app_data`, `Data<dyn T>`
  via `Data::from(Arc<dyn T>)`, a missing value yields `500` at request time)
  and the Falcon 4.4 ASGI tutorial's constructor and `create_app` pattern are
  accurate descriptions of those frameworks. Checked with Firecrawl on
  2026-11-14. Boundary: comparator counts only.

### Methods per obligation

`INV-equivalence`.

- Method: property test (`proptest`) with an independent oracle, plus an
  `rstest` table of named cases, plus one `insta` snapshot of the cross-arm
  outcome matrix.
- Rationale: the claim spans a space of stores and requests; generated inputs
  find mapping disagreements that hand-picked rows miss. The oracle is written
  in the test from the abstract model (a `BTreeMap` lookup and tenant
  comparison), not by calling either arm or the application service, so
  agreement is not self-comparison.
- Domain: stores of 0–8 tasks over 3 tenants; identifiers drawn so that about
  half the requests hit existing tasks; identity absent, same-tenant, or
  other-tenant; paths well-formed or malformed (non-numeric, extra segment,
  empty).
- Artefact: `tests/service_injection_equivalence.rs`, with generators in
  `tests/support/service_injection/cases.rs`.
- Evidence: `cargo nextest run --test service_injection_equivalence`; 256
  cases by default; any disagreement shrinks to a minimal store and request.
- Non-vacuity: the test classifies each case as `denied`, `malformed`,
  `found`, `cross_tenant`, or `missing`, and fails if any class has fewer than
  5% of cases. Seeded faults, each reverted after observation: make arm E map
  `NotFound` to `403`; make the specimen use case skip the tenant comparison;
  make arm G call a second, different service instance. Each must be rejected
  (the first two by the property, the third by `INV-injected-identity`).
  Regression files under `proptest-regressions/` are deleted after seeded
  faults.

`INV-deny-before-port`.

- Method: `rstest` cases with a counting fake port injected into each arm.
- Rationale: a finite partition (public, authenticated with and without
  identity, enforcer deny) with one meaningful case each.
- Artefact: `tests/service_injection.rs`.
- Evidence: `cargo nextest run --test service_injection`.
- Non-vacuity: a permitted sibling case observes exactly one call, so a fake
  that never counts fails; seeded fault: register the arm with `Policy::Public`
  and confirm the denied-case assertion fails.

`INV-injected-identity`.

- Method: named `rstest` test on each arm, comparing `AllocationId::of` the
  constructor's `Arc` with `observation().dependency` before registration and
  after an awaited `invoke`, plus a fake that records `AllocationId::of(self)`
  inside the call.
- Non-vacuity: two instances of the same concrete service type with different
  stores are registered side by side, so type identity cannot masquerade as
  object identity; seeded fault as listed above.

`INV-tenant-isolation`.

- Method: covered by the `cross_tenant` class of the equivalence property and
  one named `rstest` case per arm.
- Rationale: the rule is a single comparison in a specimen use case; the
  property already quantifies over stores and tenants.

`L-send`, `L-dyn`, `L-constructor`.

- Method: compile-pass and compile-fail fixtures under `trybuild`, each fail
  paired with a pass sibling, plus a runtime witness that spawns each arm's
  invocation on a multi-thread Tokio runtime.
- Artefact: `tests/service_injection_contract.rs` driving
  `tests/ui/service_injection/pass/*.rs` and
  `tests/ui/service_injection/fail/*.rs`.
- Fixtures (pass): `heterogeneous_service_storage.rs` (both arms in one
  `Vec<Arc<dyn Resource>>`, registered and invoked, with
  `fn require_send<F: Send>(_: &F)` applied to both `get` futures);
  `bridged_native_service.rs` (one native service used through both arms via
  the bridge); `erased_port_test_double.rs` (a fake implementing only the
  erased port).
- Fixtures (fail), each with the named sibling: `non_send_port_future.rs`
  (erased port alias without `+ Send`; sibling `heterogeneous_service_storage`;
  expected primary diagnostic "future cannot be sent between threads safely");
  `native_port_as_dyn.rs` (`Arc<dyn TaskQueries>` with the native port; sibling
  `erased_port_test_double`; expected `E0038`); `rc_backed_service.rs` (service
  holding `Rc`; expected `Send`/`Sync` bound failure on `Arc<dyn Resource>`
  coercion; sibling `heterogeneous_service_storage`);
  `missing_dependency_generic.rs` and `missing_dependency_erased.rs` (each
  constructor called without its second port after `EP-M3`; expected `E0061`;
  their stderr is also the evidence for H1's "constructor errors worsen"
  condition).
- Evidence: `cargo nextest run --test service_injection_contract`.
- Non-vacuity: each fail fixture's `.stderr` is inspected for the intended
  primary error code before it is committed; a pass sibling differing only in
  the named respect must compile. Runtime witness:
  `tests/service_injection.rs::invocations_run_on_worker_threads` spawns both
  arms with `tokio::spawn` on a multi-thread runtime.

`REQ-port-independence` (structural, not behavioural).

- Method: a scripted search at each milestone boundary, recorded in the
  evidence ledger. E5 in 1.3.2 replaces it with fixtures.
- Command (must print nothing):

  ```plaintext
  rg -n 'peregrine_web|\bhttp(_body)?::|\bbytes::|hyper|http_adapter|outbound' \
    examples/service_injection/application
  ```

- Non-vacuity: run the same command against
  `examples/service_injection/http_adapter` and confirm it does print matches.

`REQ-measure` (measurement, not a correctness obligation).

- Counting rules, pre-registered:
  - Files changed: `git show --numstat --format= <commit>` over the commits
    tagged for that arm and scenario, counting distinct files under
    `examples/service_injection/` and excluding tests, fixtures, and docs.
    Shared application commits count towards every arm that depends on them.
  - Forwarding layer: a function or trait-impl method whose body only calls
    one other function with the same semantic arguments, adding at most
    boxing, `Arc` cloning, or type conversion, and no decision or projection.
    Count distinct layers on the call path from the resource responder to the
    concrete service's use-case body, and separately the number of such
    functions an author must write when adding the dependency.
  - Type parameters touched: the number of generic parameter lists that change
    when the dependency is added.
  - Constructor diagnostic: from the `missing_dependency_*.stderr` files, the
    primary error code, whether the first line names the missing argument's
    type, and the number of diagnostic lines.
- Scenarios: `S-driven` adds a driven dependency (a `Clock` port) to the task
  service; `S-driving` adds a second driving port (`TaskAnnotations`) to the
  resource, so its response includes an annotation count.
- Comparators: the same two scenarios written as sketches for (a) an Actix
  application-wide façade in Corbusier's shape, (b) Actix resource-local
  `app_data(Data::from(arc))` with `Data<dyn T>` extraction, and (c) a Falcon
  ASGI resource with constructor injection and `create_app`, assuming one file
  per role (port, service, adapter, composition root) to match the specimen.
- Decision rule, pre-registered: retain generic resource wiring (arm G) as the
  documented default only if, in both scenarios, it changes no more files and
  needs no more authored forwarding functions than arm E, and its
  missing-dependency diagnostic names the missing type at least as clearly.
  Otherwise document arm E (`Arc<dyn Port>`) as the default, with concrete
  storage permitted where an application needs it. Independently, record H1 as
  revised if neither arm changes fewer files or forwarding layers than the
  resource-local Actix comparator, as the case study's rejection condition
  requires.

### Formal verification decision

No Verus or Kani artefact is warranted. E1 adds no repository-owned algorithm
or business rule: the tenant comparison is one equality in a specimen use case
that the property test quantifies over directly, and the remaining obligations
are compile-time contracts (`L-send`, `L-dyn`, `L-constructor`) that the
compiler itself decides, or wiring equivalences that a differential property
with an independent oracle tests well. A Verus proof of the tenant filter would
restate its one-line definition; a proof of `L-send` would re-derive the
compiler's auto-trait rules. Neither would add assurance. The lifecycle
transition proof remains owned by roadmap 1.2.2. If implementation exposes a
non-trivial kernel (for example, a path parser with real grammar), stop and
revise this section before implementing it. No `assume`, `admit`, or trusted
proof body is introduced.

### Suggested BDD features

Proposed file: `tests/features/service_injection.feature`, run by
`tests/service_injection_bdd.rs` with `rstest-bdd` 0.6
(`cargo nextest run --test service_injection_bdd`). Scenarios trace to
`REQ-heterogeneous`, `REQ-arm-equivalence`, and `INV-deny-before-port`
(`EP-M2`).

```gherkin
Feature: Resources own concrete or erased application services

  Scenario: Both storage options answer the same protected request
    Given a task store where task 7 belongs to tenant "acme"
    And a concrete-service resource and an erased-port resource registered together
    When user "ada" of tenant "acme" requests "/tasks/7" from each resource
    Then both resources respond with status 200 naming task 7

  Scenario: Another tenant cannot see the task
    Given a task store where task 7 belongs to tenant "acme"
    And a concrete-service resource and an erased-port resource registered together
    When user "bob" of tenant "globex" requests "/tasks/7" from each resource
    Then both resources respond with status 404

  Scenario: An anonymous request never reaches the service
    Given a task store where task 7 belongs to tenant "acme"
    And a concrete-service resource and an erased-port resource registered together
    When an anonymous caller requests "/tasks/7" from each resource
    Then both invocations are denied for missing identity
    And neither injected service has been called

  Scenario: A malformed path is rejected by the adapter
    Given a task store where task 7 belongs to tenant "acme"
    And a concrete-service resource and an erased-port resource registered together
    When user "ada" of tenant "acme" requests "/tasks/seven" from each resource
    Then both resources respond with status 400
    And neither injected service has been called
```

## Plan of work

### `EP-M0` feasibility probe

Question: does a generic `Resource` implementation whose `get` awaits a native
`impl Future<Output = …> + Send` port future, borrowing an application context,
compile under `#[async_trait]` on the pinned nightly? Write the smallest
version of `tests/ui/service_injection/pass/heterogeneous_service_storage.rs`
with one arm and run `cargo nextest run --test service_injection_contract`
(creating the runner first). Go: it compiles; promote the fixture into `EP-M1`.
No-go: record the diagnostic under `Surprises & discoveries`, set status
`BLOCKED`, and propose alternatives (for example an owned context) to the user.
The probe is retained as the first `EP-M1` fixture, not deleted.

### `EP-M1` compile contract for both storage options

Create `tests/service_injection_contract.rs` mirroring
`tests/ownership_contract.rs`: one `#[test]` calling `trybuild::TestCases` with
`pass("tests/ui/service_injection/pass/*.rs")` and
`compile_fail("tests/ui/service_injection/fail/*.rs")`. Add a nextest override
in `.config/nextest.toml` putting `binary(=service_injection_contract)` in the
`nested-cargo-builds` group with `slow-timeout = { period = "60s" }`, matching
the `ownership_contract` override and its comment.

Write the pass fixtures listed under `L-send`/`L-dyn`/`L-constructor`, each
self-contained (trybuild fixtures cannot import from `examples/`), declaring
minimal `TaskQueries`, `DynTaskQueries`, `PortFuture`, a service, and both
resource shapes. Then write each fail fixture as a one-respect mutation of its
sibling, generate its `.stderr` once with `TRYBUILD=overwrite` scoped to that
new file, and review that the primary error is the intended one. The
`missing_dependency_*` fixtures are added in `EP-M3`, after the second port
exists, so that their diagnostics measure the real constructor.

Red evidence: before the `.stderr` files exist, the runner fails each new fail
fixture with trybuild's "mismatch"/"wip" report; record that output. Green: all
fixtures pass. Run `make check-fmt`, `make lint`, `make test` and commit.

### `EP-M2` runnable specimen and behavioural evidence

Create the multi-file example `examples/service_injection/` (Cargo discovers
`examples/<name>/main.rs` automatically):

- `application/mod.rs`, `application/context.rs`: `TenantId`, `UserId`, and
  `RequestContext { tenant, user }`, with no Peregrine or HTTP imports.
- `application/tasks.rs`: `TaskId`, `Task`, `TaskView`, `TaskError`
  (`thiserror`), the driven port `TaskRepository` (native
  `fn find(&self, id: TaskId) -> impl Future<Output = Option<Task>> + Send`),
  the driving port `TaskQueries` (native, `+ Send`), the erased port
  `DynTaskQueries` returning `PortFuture<'a, Result<TaskView, TaskError>>`, the
  alias `PortFuture`, the bridge `impl<T: TaskQueries> DynTaskQueries for T`,
  and the concrete `TaskService<R: TaskRepository>` implementing `TaskQueries`
  with the tenant check.
- `outbound/memory.rs`: `MemoryTasks`, an in-memory `TaskRepository` built
  from a `BTreeMap`, immutable after construction.
- `http_adapter/mod.rs`: shared adapter functions `application_context`
  (from `RequestView::identity`, principal format `tenant:user`, failing
  closed), `parse_task_path` (`/tasks/{u64}`), and `project` (result to
  `ResponseDraft` or `ResourceError`), plus `TASKS_POLICY`
  (`Policy::Authenticated`).
- `http_adapter/generic_tasks.rs`: `TasksResource<S: TaskQueries>` holding
  `Arc<S>`, `policy`, and implementing `Resource`.
- `http_adapter/erased_tasks.rs`: `ErasedTasksResource` holding
  `Arc<dyn DynTaskQueries>` and implementing `Resource`.
- `main.rs`: the composition root. `pub async fn demonstrate()` builds one
  store, one `TaskService`, both resources (the erased one from the same
  service through the bridge, plus a second, independently constructed service
  to witness identity), a fixture enforcer as in
  `examples/resource_policy_spike.rs`, registers all resources in one
  `Vec<Arc<dyn Resource>>`, invokes each with a permitted, a cross-tenant, and
  an anonymous request, and asserts identical outcomes. `main` runs it on a
  current-thread runtime.

Red first. Before writing responders, add `tests/service_injection.rs`
(`rstest` and `googletest` named cases, deny-before-port, injected identity,
worker threads), `tests/service_injection_equivalence.rs` (proptest property
and the `insta` snapshot), `tests/service_injection_bdd.rs` with the feature
file, and `tests/service_injection_example.rs` (loads `main.rs` by `#[path]`
and runs `demonstrate()`, like `tests/resource_policy_example.rs`). Shared
helpers live in `tests/support/service_injection/` (`fixtures.rs` for counting
fakes and enforcer, `cases.rs` for generators and the oracle). With the
resources' `get` returning
`Err(ResourceError::Failed { message: "not implemented" })`, run the focused
tests and record the expected failures. Green: implement the adapter functions
and responders. Refactor: extract duplication between the two responders into
`http_adapter/mod.rs` only where it is not the very forwarding being measured;
record any such choice.

Add `insta` to `[dev-dependencies]` with a caret requirement and minimal
features, refresh `Cargo.lock` through the repository's fetch route described in
`docs/developers-guide.md` ("Fetching dependencies before tests"), and record
the dependency review. The snapshot renders one row per named case and arm
(`case | arm | status | body`), with allocation identifiers excluded, and is
stored at `tests/snapshots/`.

Run the `REQ-port-independence` search. Gates, then commit.

### `EP-M3` change-surface measurement

Each step is its own gated commit whose message carries a trailer
`E1-Measurement: <scenario>/<arm-or-shared>`, so `git log --grep` finds it.

1. `S-driven/shared`: add `Clock` (driven port, native `+ Send`) and a
   `SystemClock`-like fixed clock in `outbound/`; `TaskService` gains a type
   parameter or field and `TaskView` gains an "as of" field. Update tests
   minimally.
2. `S-driven/G` and `S-driven/E`: whatever each arm's adapter and the
   composition root need, if anything.
3. `S-driving/shared`: add `TaskAnnotations` (native) with its bridge
   `DynTaskAnnotations`, a service, and an in-memory store.
4. `S-driving/G`: `TasksResource` gains the second dependency (as a second
   type parameter or a small bundle; choose the form an author would write, and
   record the choice).
5. `S-driving/E`: `ErasedTasksResource` gains `Arc<dyn DynTaskAnnotations>`.
6. Add `missing_dependency_generic.rs` and `missing_dependency_erased.rs`
   compile-fail fixtures and their reviewed `.stderr`.

After each step, run the focused tests and the full gates. Then compute the
counts with the commands under `Concrete steps`, write the comparator sketches
for both scenarios, and record everything in
`docs/service-injection-experiment.md` §Measurements. If a step forces a change
to the other arm's files, record it; that is evidence, not a defect.

### `EP-M4` decision, documentation, and roadmap closure

Apply the pre-registered decision rule. Write
`docs/adrs/adr-0008-resource-service-storage.md` (status Proposed, following
the ADR format in `docs/documentation-style-guide.md` and ADR 007's layout)
recording the retained boundary, the rejected option, and their evidence.
Update: `docs/peregrine-design.md` §4.2 (replace "hypothesis H1, not a measured
improvement" with the measured result and link ADR 008);
`docs/adrs/adr-0004-application-port-boundaries.md` (consequences: E1 result
and ADR 008 link); `docs/hexagonal-application-case-study.md` Table 4 (append
an E1 result note without rewriting its research record); `docs/users-guide.md`
(a section "Run the service-injection experiment" with the example command and
what it shows); `docs/developers-guide.md` (port erasure convention:
`PortFuture`, the bridge, where they live and that Peregrine does not export
them; maintenance of `tests/ui/service_injection/`; the measurement trailer
convention); `docs/contents.md` and `docs/repository-layout.md` (new example,
test directory, ledger, ADR). Mark roadmap 1.3.1 done with `mapsplice` if it
supports the edit, otherwise by a reviewed manual edit as recorded in the 1.1.3
plan. Run all gates including `make markdownlint`, `make spelling`, and
`make nixie`, then commit and set this plan's status to `COMPLETE` after
reconciling the conformance basis.

## Milestones and plateaus

- `EP-M1`: both storage options and four negative contracts compile or fail
  as specified. Requirements: `REQ-heterogeneous`, `REQ-explicit-bounds`.
  Evidence: `service_injection_contract` passes. Conformance: no `src/` change;
  no new dependency; no container. Recovery: fixtures are additive; revert the
  commit. Remaining: runtime evidence, measurement. Compatibility: none.
- `EP-M2`: the runnable specimen demonstrates both arms in one runner with
  equivalent outcomes. Requirements: `REQ-arm-equivalence`,
  `REQ-port-independence`, `INV-*`. Evidence: the four new test binaries and
  `cargo run --example service_injection` exit 0. Conformance: application
  module import search clean; `insta` is the only new dependency. Recovery:
  revert; `EP-M1` remains valid. Remaining: measurement. Compatibility: none.
- `EP-M3`: both scenarios applied to both arms with tagged commits; counts and
  comparators recorded. Requirements: `REQ-measure`, `REQ-visible-deps`.
  Evidence: the ledger tables and the `missing_dependency_*` fixtures.
  Conformance: counting and decision rules unchanged since approval. Recovery:
  each measurement commit is gated and revertible; re-run the count commands to
  regenerate tables. Remaining: decision. Compatibility: none.
- `EP-M4`: decision recorded and documentation reconciled. Requirements:
  `REQ-retain`. Evidence: ADR 008, updated design §4.2, roadmap checkbox.
  Conformance: upstream artefacts updated for every discovery. Recovery:
  documentation-only revert. Compatibility: none.

## Concrete steps

All commands run from the repository root. Pipe long output through `tee` with
`set -o pipefail`, logging to
`/tmp/<action>-peregrine-web-1-3-1-compare-concrete-and-erased-service-injection.out`.

```plaintext
EP-M0/M1 focused:
  cargo nextest run --test service_injection_contract
EP-M2 focused (red, then green):
  cargo nextest run --test service_injection --test service_injection_equivalence \
    --test service_injection_bdd --test service_injection_example
  cargo run --example service_injection
  rg -n 'peregrine_web|\bhttp(_body)?::|\bbytes::|hyper|http_adapter|outbound' \
    examples/service_injection/application        # expect no output
EP-M3 counting:
  git log --format='%h %(trailers:key=E1-Measurement,valueonly)' --grep 'E1-Measurement'
  git show --numstat --format= <commit> -- examples/service_injection
Gates (sequential, after each milestone):
  make check-fmt && make lint && make test
Documentation gates (EP-M4 and whenever docs change):
  make fmt && make markdownlint && make spelling && make nixie
```

Gates are delegated to the `scrutineer` agent, which runs them sequentially and
returns log paths; read the cited log on failure rather than re-running.

## Validation and acceptance

- `cargo run --example service_injection` exits 0 and prints, for each of the
  permitted, cross-tenant, anonymous, and malformed requests, one line per arm
  with identical outcomes (exact lines recorded at `EP-M2`).
- `cargo nextest run --test service_injection_contract` passes; removing
  `+ Send` from `PortFuture` in the pass sibling makes it fail with "future
  cannot be sent between threads safely".
- The equivalence property passes 256 cases with every class above 5%; each
  seeded fault is rejected as described.
- The BDD scenarios pass; each failed against the stub responders first.
- `docs/service-injection-experiment.md` contains, for each scenario, a table
  of files changed, authored forwarding functions, call-path forwarding layers,
  type parameters touched, and diagnostic quality for arms G and E and the
  three comparators, followed by the decision and its rule.
- Quality: `make check-fmt`, `make lint`, `make test`, `make markdownlint`,
  `make spelling`, and `make nixie` succeed. No Verus or Kani command applies.

## Idempotence and recovery

Every step is additive and committed only when green. Re-running focused tests
is safe. `TRYBUILD=overwrite` must only be used for a single new fixture and
its output reviewed. Seeded faults are applied in the working tree only, never
committed, and reverted with `git checkout -- <file>`; delete any
`proptest-regressions/` file they create. If a measurement commit needs
correction, add a new tagged commit rather than rewriting history, and note it
in the ledger.

## Artefacts and notes

Planning-time reconnaissance (2026-11-14), not implementation evidence:

```plaintext
Actix web::Data<T>: Arc wrapper; registered with App::app_data or Resource::app_data;
  Data<dyn T> via Data::from(Arc<dyn T>); missing data -> 500 at request time.
Falcon 4.4 ASGI tutorial: class Images: __init__(self, config, store);
  create_app() constructs Store and Images, then app.add_route(...).
dynosaur 0.3.1 (MIT OR Apache-2.0): generates a DynTrait for async/RPITIT traits.
```

## Interfaces and dependencies

No library interface changes. The specimen's application-owned signatures, in
`examples/service_injection/application/tasks.rs`, are:

```rust
pub type PortFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait TaskQueries: Send + Sync {
    fn view(
        &self,
        context: &RequestContext,
        id: TaskId,
    ) -> impl Future<Output = Result<TaskView, TaskError>> + Send;
}

pub trait DynTaskQueries: Send + Sync {
    fn view<'a>(
        &'a self,
        context: &'a RequestContext,
        id: TaskId,
    ) -> PortFuture<'a, Result<TaskView, TaskError>>;
}

impl<T: TaskQueries> DynTaskQueries for T {
    fn view<'a>(
        &'a self,
        context: &'a RequestContext,
        id: TaskId,
    ) -> PortFuture<'a, Result<TaskView, TaskError>> {
        Box::pin(TaskQueries::view(self, context, id))
    }
}
```

In `examples/service_injection/http_adapter/`:

```rust
pub struct TasksResource<S> { tasks: Arc<S> }
impl<S: TaskQueries + 'static> TasksResource<S> { pub fn new(tasks: Arc<S>) -> Self }

pub struct ErasedTasksResource { tasks: Arc<dyn DynTaskQueries> }
impl ErasedTasksResource { pub fn new(tasks: Arc<dyn DynTaskQueries>) -> Self }
```

Reuse policy: `PortFuture` and the bridge are application-owned conventions
demonstrated by the specimen. Peregrine does not export them, because axiom A1
forbids ports from depending on Peregrine. A sweep of `src/` and `tests/` found
no existing equivalent. The developers' guide records that each application
defines its own alias and bridge beside its ports, and that adapters may call
either trait but must not define bridges themselves.

Dependencies: add `insta = "1.<current>"` (caret, development only, minimal
features). Reuse existing `async-trait`, `thiserror`, `rstest`, `rstest-bdd`,
`googletest`, `pretty_assertions`, `proptest`, `trybuild`, and `tokio`. No
runtime dependency is added.

## Revision note

- 2026-11-14: initial draft for expert-panel review.
