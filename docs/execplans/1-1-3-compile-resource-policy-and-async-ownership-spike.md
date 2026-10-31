# Compile a resource-policy and async ownership spike (1.1.3)

Status: DRAFT — awaiting explicit approval before implementation.

This execution plan is a living document. Keep Constraints, Tolerances, Risks,
Progress, Surprises & discoveries, Decision log, Outcomes & retrospective,
Conformance basis, and Verification plan current throughout implementation.

## Purpose / big picture

Establish whether heterogeneous resource objects can expose typed policy and
retain their identity across awaited middleware and responder calls, using
synthetic bodies and real Hyper request bodies. A contributor should be able to
run positive and negative compiler fixtures and a small executable specimen
that demonstrates those contracts. This supplies evidence for roadmap 1.1.3 and
goals G1, G2, and G4; it does not deliver the framework or settle its API.

Approval authorizes only the bounded spike below. Q3 (public policy contract)
and Q5 (compiler support) remain open for maintainer review and phase 2. The
plan PR contains documentation only. Do not implement the spike, add its
manifest dependencies, or mark the roadmap item done before approval.

## Constraints

- Preserve the selected resource as the source of both policy and behaviour.
  No route-policy table, service locator, `dyn Any` policy discovery, hidden
  injection, or application-global request state is permitted.
- Resources are shared `Send + Sync` objects. Request bodies and suspended
  invocation futures are `Send`; bodies need not be `Sync`. A request borrow
  may cross a local await but cannot escape into an owned response body or a
  detached task. No unsafe code, leaked allocations, or locks to evade
  borrowing.
- Scope the code under an explicitly experimental spike module. It is not a
  stable `PeregrineApp`, production router, authorization provider, lifecycle
  engine, body collector, server, or WebSocket implementation.
- Preserve ADR 004's application-owned dependencies and authorization. The
  specimen's policy gate demonstrates admission ordering; it must not become
  the application's tenant/object authorization implementation.
- Keep `nightly-2026-08-27` as the candidate compiler baseline. Do not claim a
  stable minimum supported Rust version from a nightly build. Do not change the
  global checker/solver policy or accept ADRs 001–005 by implication.
- Do not implement the phase 2 comparator or task 1.2.2's transition kernel in
  this slice. No compatibility layer is needed for this pre-1.0 spike.
- Use shared compiler admission and Cargo caches. Never bypass wrappers, use
  `/tmp` as a build target, or interfere with another worktree's processes.
  Stop and report a full filesystem or unhealthy admission with its evidence.
- Commit only green plateaus. Run formatting, format checks, lint, and tests
  sequentially. Preserve custom assertion diagnostics and the 400-line limit.

## Tolerances (exception triggers)

The permitted implementation area is `src/spike/`, its export in `src/lib.rs`,
focused tests and fixtures, one example, Cargo manifests/lockfile, focused Make
entrypoints if required, and the documentation named below. No elapsed-time
limit applies. Stop and set status BLOCKED for a compiler change, a new runtime
family beyond the dependencies listed below, a full router or server, public
API stabilization, a changed trust boundary, or a design-contract weakening.

If the proposed borrowing API fails, retain the diagnostic and propose a
revision; do not add clones, `Arc<Mutex<_>>`, `?Send`, or `Sync` body bounds as
silent workarounds. Ordinary owned metadata and `Arc` resource storage are
allowed where ownership actually requires them. A newly discovered non-trivial
policy algorithm or transition kernel needs a revised specification and proof
milestone before implementation. Record alternatives, affected trace links, and
the required design/ADR change; obtain explicit approval for deviations.

## Risks

- High impact, medium likelihood: an erased async API accidentally requires
  the complete context to be `Sync`. Mitigate with separate immutable metadata
  and mutable body/response capabilities, and a deliberately non-`Sync` body.
- High impact, medium likelihood: compile-fail tests pass for the wrong reason.
  Give each rejection a positive sibling and review its primary diagnostic.
- High impact, medium likelihood: a toy policy gate is mistaken for secure
  framework authorization. Keep decisions injected in the specimen, label the
  boundary, and defer complete enforcement to phase 4.
- Medium impact, medium likelihood: dated compiler defaults and Whitaker's
  separate compiler disagree. Record both environments and run all gates;
  neither dependency requirements nor unstable diagnostics may be guessed.
- Medium impact, medium likelihood: socket or cancellation tests hang. Use
  readiness signals, ephemeral loopback ports, bounded test deadlines, and
  joined tasks; use explicit polling barriers instead of sleeps.

## Progress

- [x] (2026-10-20) Inspected branch ownership, prerequisite decisions, and
  repository guidance; renamed the branch to the requested task name.
- [x] (2026-10-20) Two Wyvern agents inspected contracts and verification seams;
  shared source excerpts through context pack `pk_wsn54ndp`.
- [x] (2026-10-20) Loaded CodeGraph, Rust Router, async/concurrency, ExecPlans,
  Firecrawl, and community-of-experts review guidance; researched body erasure,
  async trait dispatch, compiler fixtures, and assertion tooling.
- [x] (2026-10-20) Completed six-perspective review and incorporated policy
  stability, exact frame oracles, owned/borrowed cancellation, and fixture
  cleanup corrections.
- [x] (2026-10-21) Validated the plan for draft PR publication: formatting,
  lint, tests, Markdown/spelling, and Mermaid gates passed sequentially.
  Nextest reported 295 passed and one existing ignored Act/Docker fixture; the
  doctest stage passed with zero tests. Publication is recorded in PR metadata
  rather than treated as implementation approval.
- [ ] Obtain explicit approval of this plan.
- [ ] M1: compile the resource and ownership boundary.
- [ ] M2: exercise identity, policy admission, and body adapters.
- [ ] M3: reconcile evidence and documentation; mark 1.1.3 done.

## Surprises & discoveries

The prerequisite is already satisfied: `docs/roadmap.md` marks 1.1.1 and 1.1.2
done. The current `src/lib.rs` is a generated greeting stub and `Cargo.toml`
has no runtime dependencies. The compiler examples in the design are evidence,
not an existing framework seam to extend.

CodeGraph MCP tools were unavailable during planning. Known-file inspection and
scoped file searches supplied the evidence; no semantic index result is
claimed. CodeGraph should be retried before implementation's abstraction sweep.

The dated nightly, Cargo configuration, and Makefile contain no explicit
checker selection. The current posture is therefore `unknown`, pending the
canary in M1; the historical four-way probe results do not establish today's
effective wrapper/configuration posture.

## Decision log

- 2026-10-20, proposed: use the current pinned nightly as the spike's tested
  floor, with exact compiler and flags recorded. A floor here means the one
  tested candidate, not a supported-platform or stable-release promise.
- 2026-10-20, proposed: use `async-trait` for the initial heterogeneous boundary
  and `UnsyncBoxBody` for body erasure. This follows the existing baseline and
  tests the asymmetric sharing bounds directly. Native async `dyn` dispatch is
  not assumed; generic authoring comparisons remain phase 2.
- 2026-10-20, proposed: keep a minimal injectable policy-decision fixture and
  one direct invocation path. Full lifecycle transitions and scope-set
  authorization would enlarge this spike and its proof obligations.
- 2026-10-20, proposed: use conventional compiler/integration/property tests
  here. Retain the upstream lifecycle proof sketch and its later owner without
  manufacturing speculative Verus code for adapter plumbing.

## Outcomes & retrospective

The plan has completed community review and repository validation. No
implementation, passing spike, compiler canary, API acceptance, or proof
completion is claimed. Final implementation evidence must distinguish compiler
acceptance, executable behaviour, and decisions still reserved for phase 2. A
negative spike result remains useful evidence but does not satisfy the
roadmap's success criterion or justify marking it done.

## Context and orientation

`src/lib.rs` is the only current library source. Existing `tests/*.rs` and
`tests/support/` primarily validate build/workflow contracts. `Makefile`
provides the required gates; `.cargo/config.toml` selects Cranelift and the
native linker wrapper, while `rust-toolchain.toml` pins the nightly.
`Cargo.toml` already includes `rstest` and `proptest` as development
dependencies.

Heterogeneous means differently typed resource values share one collection of
trait objects. Type erasure hides their concrete types without losing the
actual selected object. `Send` permits a value or future to move between
threads; `Sync` permits sharing references between threads. A frame is a body
item containing bytes or HTTP trailers. A phase view grants only the data a
hook needs, rather than borrowing a complete context with unsuitable bounds.

Start with [documentation contents](../contents.md) and
[repository layout](../repository-layout.md). Read the design references in
Conformance basis, then these implementation guides:

- [Developer guide](../developers-guide.md).
- [Dependency-injection testing guide](../reliable-testing-in-rust-via-dependency-injection.md).
- [`rstest` guide](../rust-testing-with-rstest-fixtures.md).
- [`rstest-bdd` guide](../rstest-bdd-users-guide.md).
- [Doctest guide](../rust-doctest-dry-guide.md).
- [Complexity guidance](../complexity-antipatterns-and-refactoring-strategies.md).

The [documentation style guide](../documentation-style-guide.md) governs prose,
ADRs, and executable user-guide examples.

Use `codegraph-mcp` for structural discovery and `rust-router` to establish
posture and route to `rust-async-and-concurrency`; use `leta` if available for
symbol-level navigation. Use `firecrawl-mcp` for remote documentation gaps. Use
`rust-unit-testing`, `proptest`, or `rust-verification` when implementing their
respective evidence. Use `verus` only if an approved revised milestone
introduces the substantive kernel described below. Use `mapsplice` when marking
the roadmap, `commit-message` for commits, and `pr-creation` for publication.
These are skill names; read their `SKILL.md` files before using them.

## Conformance basis

The baseline is commit `e358e4641b4f5cf909ef9752cd270d003aff6a83`.
[Terms of reference](../terms-of-reference.md) is draft v0.4 dated 2026-10-13;
[technical design](../peregrine-design.md) is proposed v0.6 dated 2026-10-13.
Use that revision of the following documents for traceability; recheck changes
before starting implementation and record any changed requirements.

- Roadmap 1.1.3, ToR G1/G2/G4 and Q3/Q5, design §§3–4 and I6/V6 map to M1's
  positive/negative ownership fixtures and M2's identity/body evidence.
- Design §4 and I3/V3 map to M2's public/protected examples and denial counts.
  This advances Q3 evidence without closing the question or completing phase 4.
- Design §§11–12 and ADR 002 map to M1's compiler ledger and M3's phase 2
  handoff. Adoption remains task 2.2.3.
- [ADR 001](../adr-001-resource-aware-lifecycle.md) and
  [ADR 003](../adr-003-http-integration-boundaries.md) constrain the resource
  and invocation seam. Full lifecycle I1/I2 and HTTP finalization I5 remain
  tasks 1.2.2, 2.1.3, and phase 3 rather than spike acceptance claims.
- [ADR 004](../adr-004-application-port-boundaries.md) and the
  [hexagonal case study](../hexagonal-application-case-study.md) constrain
  dependency ownership. E1–E5 comparisons remain tasks 1.3 and later phases.
- [ADR 005](../adr-005-extension-and-upgrade-boundaries.md),
  [ADR 006](../adr-006-extension-interface-stabilization.md), and the
  [Pachislot design](../pachislot-extension-design.md) constrain owned handoff.
  Borrow-escape fixtures give early evidence, not upgrade or stabilization
  evidence. ADRs 001–005 remain proposed; ADR 006's stabilization policy
  applies.
- The [ownership experiment](../polonius-ownership-experiment.md) and
  [compiler probe archive](../compiler-probe-evidence.md) supply comparison
  context. Reproducing all four configurations and assessing ergonomics remain
  phase 2; this spike does not claim a solver-specific advantage.

## Contracts and proof sketch before implementation

Define the abstract specimen independently of Rust storage: a selected resource
identity `r`, a frozen method `m`, immutable policy view `p(r,m)`, a decision
supplied by the configured enforcer, and an invocation observation. The
selection is fixed before policy observation. The enforcer may permit or deny;
it is not an oracle for correctness of real authentication. If denied,
invocation count is zero. If permitted and execution completes normally, the
responder called is `r` and its count is one. Cancellation makes no completion
guarantee.

Represent `r` by one retained resource allocation, not a copied route label.
The same retained handle is passed to the awaited hook and responder. Use two
instances of the same concrete type with different state as well as two
concrete types, so a type name cannot masquerade as object identity. Immutable
method/policy views prevent substitution during the borrow. Store `Policy`
without interior-mutability fields on each resource; its accessor returns a
shared reference. Registration and admission must observe equal policy for the
frozen method. Stable policy after registration is an explicit resource
implementor contract, not a consequence of `Send + Sync`: a malicious accessor
could choose different values. Capture the policy reference once for an
invocation and pass that same view through the hook; never re-query after await
or maintain an independent policy registry. Test repeated
registration/admission observations and pre/post-await equality; these
witnesses do not prove arbitrary third-party resource implementations honest.
Test observations must come from each resource's own state and injected
dependency, not solely from labels emitted by the runner.

For bodies, let an input be an ordered sequence of byte frames, optional
trailers, and a terminal success or error. Adaptation preserves that sequence
and terminal result; it does not collect the body. Ownership transfers once
into the erased body; no borrow of the request context survives an owned
response or task handoff. The correspondence is between upstream `Body` polls
and wrapper polls, not between arbitrary network chunk boundaries: HTTP may
repartition bytes. Socket tests compare bytes and trailers semantically.

Candidate semantic arguments are identity preservation through one retained
handle, denial preventing the sole invocation, and frame/error preservation by
an adapter delegating polls. These are hypotheses to pressure-test API shape,
not theorem declarations. The spike adds no general authorization algorithm or
recursive lifecycle engine. Rust's borrow checker checks aliasing and lifetime
obligations; conventional tests check this straightforward delegation and
finite control flow. Verus is not warranted for those boundaries.

The upstream I1 proof is warranted for the later pure transition kernel: start
active with zero dispatches; a dispatch transition requires eligibility and
consumes it before invoking user code; stop and interruption transitions cannot
restore eligibility. Induction over permitted transitions should derive count
at most one and no dispatch after stop. Preconditions must define legal initial
states and transitions, never assume the desired safety property. A future
runtime must refine those transitions, with generated trace tests connecting
the proof to execution. That proof belongs to 1.2.2; it is not an acceptance
condition fulfilled by this spike.

If implementation reveals a substantive policy/transition kernel, stop and
revise the plan first: specify its input domain, representation relation,
pre/postconditions, satisfiable witnesses, and semantic lemma hypotheses. Add a
Verus milestone with the repository-planned command
`prover-tools verus run --proof-file verus/lifecycle.rs` through a proposed
`make verus` target, alongside every ordinary gate. For a different approved
kernel, specify its actual path and pinned tool in that revised plan. No
`assume`, `admit`, trusted/external bodies, or equivalent shortcuts are
permitted. Defer solver plumbing until actual verification conditions exist.

## Verification plan

External assumptions are Rust's compiler ownership rules, `async-trait`'s boxed
future contract, `http-body` frame/poll semantics, Hyper's HTTP/1.1 body
interface, and Tokio's documented task bounds. Do not prove those libraries.
Exercise the repository-owned adapters against their real interfaces. Local
socket availability is an integration-test prerequisite; a disabled socket must
report failure, not silently skip acceptance. Runtime scheduling does not
guarantee prompt completion; test deadlines bound failure detection only.

### V-OWN: resource sharing and request-local ownership

Use `tests/ownership_contract.rs` to run `trybuild` pass/fail files under
`tests/ui/ownership/pass/` and `tests/ui/ownership/fail/`. Positive files must
use the actual exported spike module. They compile a heterogeneous collection,
shared resource state, a `Send` future that borrows disjoint context fields
across a real suspension, and a `Send` but non-`Sync` synthetic body. Add an
owned-data counterpart for each rejected escape. Avoid requiring every local
future to be `'static`; only the owned task/response boundary requires that.

Negative fixtures reject an `Rc`-backed resource, non-`Send` state retained
across await, simultaneous overlapping mutable loans, a request borrow captured
by an owned response body, and a request borrow passed to a spawned `'static`
task. Also reject a resource implementation omitting required policy. Each
fixture must fail for its named bound/borrow/trait requirement after imports
and types resolve. A positive sibling that ends the lexical scope containing
`Rc` before await prevents the suite from forbidding safe local non-`Send`
values accidentally.

Run `make test TEST_FLAGS='--test ownership_contract'`. Success requires all
positive cases and all reviewed negative diagnostics to match. First run red
before adding the missing API, then establish that the final negative fixtures
fail for their intended ownership errors. `trybuild` snapshots full normalized
diagnostics in `.stderr`; review them on the pinned compiler, rather than
claiming error-code-only matching. Never blindly run `TRYBUILD=overwrite`.
Compiler changes require deliberate baseline review and phase 2 attribution.

### V-IDENTITY: typed policy and the selected object

Use `rstest` in `tests/resource_policy_spike.rs` with public, authenticated,
and non-empty scope metadata, absent/present identity, and permit/deny
decisions. Test a required policy method, empty-scope construction rejection,
and a protected-resource registration attempt without an enforcer. The specimen
uses explicit enforcer presence and injected decisions; no hidden allow
default. The fixture truth table is explicit: public without an enforcer
permits; protected without one fails registration; absent identity for
protected policy produces denial; present identity delegates to the configured
fixture decision. For scopes, permit/deny is supplied explicitly rather than
computing a scope algorithm. The fixture enforcer implements that truth table.
The runner trusts its decision and does not claim to detect a dishonest
enforcer. For public resources, test the documented no-enforcer success path.

A hook must suspend once before the responder. Observe the exact same resource
allocation, frozen method, and resource-owned dependency before and after that
suspension. A denial returns a typed specimen outcome and zero responder calls;
an allowed normal invocation calls the selected resource once. Check that an
unselected resource remains untouched. Use `googletest` matchers for semantic
outcomes and `pretty_assertions` for ordered observation differences, with
helpful diagnostics. Prefer result-returning `verify_that!` with `rstest`
fixtures; non-fatal expectations require the documented GoogleTest harness.

Run `make test TEST_FLAGS='--test resource_policy_spike'`. Include distinct
resource instances and a denial witness. A temporary resource-substitution
fault and an unconditional invocation fault must make the relevant test fail;
remove each fault and rerun before committing. Do not interpret this finite
specimen as a proof of the whole lifecycle or application authorization.

Bind `tests/features/resource_policy_spike.feature` to
`tests/resource_policy_spike_bdd.rs` using `rstest-bdd`. The specification is:

```gherkin
Feature: Resource identity survives async admission
  Scenario: Public resource permits direct invocation
    Given a public resource with an injected dependency and no enforcer
    When the selected resource crosses an awaited hook
    Then that resource's responder runs once
    And its dependency supplies the response

  Scenario: Protected policy denies before invocation
    Given a protected resource and an enforcer returning denial
    When its frozen policy and method cross an awaited hook
    Then the selected resource is unchanged
    And no responder runs

  Scenario: Protected registration requires enforcement
    Given a protected resource without an enforcer
    When the specimen validates registration
    Then registration fails with a typed missing-enforcer error
```

Add the permitted protected case and missing-identity denial to the same
feature. The BDD runner invokes the actual spike seam, not a parallel model. Run
`make test TEST_FLAGS='--test resource_policy_spike_bdd'` and expect every
scenario to run and pass. These outcomes are experimental and do not yet
promise HTTP 401/403 rendering or response-hook unwinding.

### V-BODY: adapter fidelity and ownership

Use `tests/body_adapter_spike.rs` with an explicitly non-`Sync` synthetic body
containing `Cell` state while remaining `Send`. Check empty input, several data
frames, trailers, pending-then-ready polling, terminal error, and drop of a
partially consumed body. Use a drop observer to verify release exactly once; do
not imply asynchronous cleanup or rollback from a destructor.

Use lightweight `proptest` cases for byte preservation under generated frame
partitions: total payload 0–4096 bytes and 0–32 frames, including empty frames,
with and without trailers. Zero frames implies an empty payload. Generate frame
vectors whose total size fits the budget, or partition generated bytes using a
nonzero frame count. Construct valid cases directly, without rejection filters.
Compare synthetic output frame-by-frame with an independent input sequence,
including empty-frame positions and exact trailer values; also compare its
concatenated bytes. An oracle that compares only concatenated bytes cannot
detect dropped empty frames or changed synthetic frame boundaries. Record
classifications for empty/non-empty and trailers/no-trailers, add named
witnesses for each class, and reject a temporary adapter mutation that drops a
frame. These bounds support sampled evidence; they are not a universal proof or
a framework memory-budget guarantee.

`tests/network_body_spike.rs` runs a bounded loopback HTTP/1.1 fixture using
Hyper's actual `Incoming`. Both a synthetic request and that network request
use the same adaptation and resource invocation functions. Assert body bytes,
trailers where legal, and selected-resource observations. Use a chunked
HTTP/1.1 request for trailers. Add a truncated body case with nonzero
`Content-Length`, fewer transmitted bytes, and an explicit client write-half
close; consume `Incoming` until its typed error, not successful EOF. Do not
snapshot Hyper error wording. Bind to port zero and signal readiness. Bound
fixture input and accumulated output to 4096 bytes and trailer count to four;
stop on excess output rather than allocating until timeout. Synthetic fixtures
additionally cap poll observations at 64. Socket frame counts are not
assertions because transport partitioning may vary.

All success, error, timeout, and assertion-failure paths close owned client
handles, signal fixture shutdown, and join tasks before propagating results. If
bounded shutdown fails, abort and reap only fixture-owned tasks; never drop a
pending join handle into detached execution. Use generous deadlines as failure
bounds, not timing assertions. No production listener API or server supervision
is added. This is end-to-end evidence for the adapter integration only;
backpressure, HTTP finalization, and graceful shutdown remain later tasks.

Run the focused `body_adapter_spike` and `network_body_spike` test targets. The
final `make test` must discover both. A generic function accepting `Incoming`
without obtaining one from Hyper is insufficient network evidence.

### V-CANCEL: owned lifetime after suspension

Within `tests/resource_policy_spike.rs`, poll invocations to a controlled
pending point, then drop them. For a future owning the body and request-local
guard, assert both are released once. For a future borrowing caller-owned
arguments, assert the loans end and the caller can access them again; their
drop counts stay zero until the caller explicitly drops them, then become one.
In both cases the responder has not run if suspension precedes dispatch. A
further witness completes normally. Do not infer response-hook completion,
durable rollback, or cancellation of independent application work. Positive
owned-stream/task fixtures and negative borrowed escapes complement this
runtime test. Use a supervised multi-thread Tokio test to demonstrate that a
fully owned invocation can be spawned and joined; use a compile-time `Send`
assertion for locally borrowed futures.

Snapshots are not required for this slice: it introduces no stable multivariant
renderer or wire error envelope. `trybuild` supplies compiler baselines, while
semantic assertions cover the small observation sequences. If a stable
multivariant output becomes a requirement, revise scope and add focused `insta`
snapshots with nondeterministic fields normalized. Kani and Verus add no useful
assurance to the delegated body poll and compiler-enforced loans here; later
transition/business kernels need their own explicit review.

## Proposed interfaces and dependencies

Before adding helpers, repeat the repository equivalence sweep with CodeGraph
or a documented fallback. Record scope and permitted callers in the design and
developer guide. Proposed files are `src/spike/mod.rs`, `resource.rs`,
`body.rs`, and `invoke.rs` under that directory; split by coherent feature if
needed to stay below 400 lines. `src/lib.rs` exports an explicitly experimental
`spike` module so external compile fixtures test the real library surface.

`Resource: Send + Sync` has a required synchronous typed policy accessor and a
minimal GET responder using `async-trait`'s default `Send` future. The policy
accessor returns a reference to the immutable value described above. Keep
policy on the object. The specimen selects the resource explicitly and freezes
its method; it does not add route parsing or all method dispatch. Use policy
variants `Public`, `Authenticated`, and `Scopes` with a domain scope wrapper
and validated non-empty required scopes. Validation is straightforward
construction, not a general policy engine. Exact names remain provisional.

A `RequestView<'a>` borrows immutable HTTP metadata only; mutable body and
response draft access are separate arguments. Do not put the non-`Sync` body
inside an immutable view held across await. A retained `Arc<dyn Resource>` may
own the selected object when a task requires ownership; local invocation can
borrow it. The invocation future may borrow its arguments until completion. The
specimen runner accepts a configured enforcer that returns a typed permit/deny
decision after inspecting the actual resource and frozen method. It performs
one conditional invocation; it does not model the full lifecycle.

Adapt a body once to `UnsyncBoxBody<Bytes, BodyError>`, mapping upstream errors
to a focused semantic error while retaining a source when useful. Synthetic and
Hyper bodies use the same generic boundary with
`Body<Data = Bytes> + Send + 'static` and an appropriate error conversion. The
`'static` bound is for body ownership, not the locally borrowing invocation
future. Outgoing owned bodies cannot retain request-local references. Delegated
frames retain trailers and errors without implicit collection.

Proposed implementation dependencies are the smallest compatible caret versions
of `async-trait`, `bytes`, `http`, `http-body`, `http-body-util`, and
`thiserror`. Hyper 1, `hyper-util`, and Tokio 1 belong in development
dependencies for this socket specimen unless a real library signature needs
Hyper. Test additions are `trybuild`, `rstest-bdd` and its required macros,
`googletest`, and `pretty_assertions`; reuse `rstest` and `proptest`. Enable
only the server, HTTP/1, I/O, runtime, synchronization, and time features
actually exercised. Check exact release compatibility, minimum compiler
declarations, licences, and lockfile resolution during M1. Do not add
`matchit`, a database, an identity provider, a serialization framework, or
global tracing/metrics subscribers. Task 1.2.1 still owns the maintained
dependency and feature baseline.

An explicit boxed-future trait could satisfy the same bounds but adds manual
lifetime/boxing machinery without improving this baseline experiment. Generic
resource authoring with an erased endpoint adapter remains a phase 2
alternative. `UnsyncBoxBody` is selected over `BoxBody` because the latter adds
sharing requirements the request owner does not need; this is a bound choice,
not a performance claim.

## Milestones and plateaus

### M1: compile the resource and ownership boundary

After approval, record `rustc -Vv`, Cargo/toolchain versions, effective flags,
and wrapper configuration without printing credentials. Add the Rust Router
canary as an example target and run it through Cargo with the project wrapper,
then with the explicit NLL control. Preserve output under `/tmp`; update the
posture from evidence. Remove the temporary canary target before the milestone
commit, retaining its source/commands/results in the evidence document. Do not
directly invoke `rustc` to compile it or bypass admission.

Write V-OWN positive tests first and observe failure due to the absent spike
API. Add the smallest experimental resource, policy, and body interfaces plus
dependencies. Then add the negative fixtures and their positive siblings and
review the resulting diagnostics. Record tested compiler/bounds in
`docs/resource-policy-ownership-spike.md`, a proposed evidence ledger linked
from the documentation contents. Do not copy fixture APIs into the UI tests.

Acceptance M1-A is a passing ownership suite, documented effective posture, and
all ordinary gates below. Recheck I6/V6 and the scope of each exported item; no
compiler adoption follows. Commit this coherent compile-only boundary. Recovery
is to fix the diagnostic or revert this milestone's commit while preserving
logs. Runtime identity and network observations remain M2 work.

### M2: exercise policy identity and both body adapters

Add the failing V-IDENTITY, V-BODY, and V-CANCEL tests and BDD scenarios before
implementing their seam. Complete the single invocation runner, body error
mapping, and minimal synthetic/network fixtures. Each red failure must identify
a missing behaviour rather than infrastructure failure; then implement the
smallest correction and rerun the focused target. Add
`examples/resource_policy_spike.rs` as a bounded in-process demonstration with
public and protected resources using injected dependencies. Check outcomes
programmatically and return errors rather than printing ad hoc diagnostics.

Acceptance M2-A is identity equality across an actual suspension, denied count
zero, permitted count one, protected registration failure without enforcement,
and public registration success. M2-B is generated and named body cases plus
real `Incoming` success and failure through the same seam. M2-C is the joined
multi-thread invocation, cancellation/drop observations, and rejected escapes.
Run the example and all ordinary gates; record negative-control results after
restoring the correct implementation. Commit the validated behaviour. Recover
by fixing or reverting this milestone alone; M1 remains a compile evidence
plateau. Full HTTP serving, production policy, and lifecycle proofs remain out
of scope.

### M3: reconcile evidence and hand off the experiment

Update `docs/peregrine-design.md` §§3–4 and 11–12 with the tested provisional
signatures, body/error boundary, ownership limitations, and proof/test
relationships. Record substantive decisions in a new focused ADR following
`docs/documentation-style-guide.md`; keep unaccepted choices proposed and link
the ADR from the design and contents. Do not rewrite historical completed plans
or mark existing proposed ADRs accepted.

Update `docs/users-guide.md` for the experimental consumer surface and runnable
example, explicitly distinguishing it from a usable framework. Follow the
repository's tested-example markers and load published examples in tests rather
than copying them. Update `docs/developers-guide.md` with fixture maintenance,
compiler evidence, and internal conventions; use the design for internal
interface ownership and `docs/repository-layout.md` for new paths. Link the
ledger and ADR from `docs/contents.md`.

Acceptance M3-A is a ledger mapping every V-OWN/IDENTITY/BODY/CANCEL witness to
its path, command, result, compiler, limitations, and relevant design
identifier. Reconcile all assumptions against the conformance basis. Run every
ordinary gate after documentation changes. Only then mark roadmap 1.1.3 checked
and explicitly `done` using `mapsplice`; update its opening status paragraph to
include this completed spike without implying a delivered framework. Recheck
documentation gates after that final edit and commit. Leave Q3/Q5 acceptance,
1.2.1, 1.2.2, phase 2, and extension work open. Recovery is a focused
correction or revert; do not retain a false completion marker if acceptance is
lost.

At each milestone, verify assigned contracts, design conformance, assumptions,
new dependencies/public surfaces, trust boundaries, and evidence links before
committing. No milestone needs a compatibility shim. Record revisions in the
living sections immediately, including partial progress when blocked.

## Concrete steps and acceptance commands

Run from the repository root. Start with `git branch --show-current` and
`git status --short`; preserve unrelated changes. The implementation branch is
`1-1-3-compile-resource-policy-and-async-ownership-spike`. Inspect current
instructions again, and obtain explicit plan approval before M1.

Use this sequential gate pattern after each major implementation milestone. The
log directory is scratch only; Cargo keeps its normal target and cache. A
non-zero gate stops the sequence and blocks the commit.

```bash
set -o pipefail
spike_logs=$(mktemp -d /tmp/peregrine-113-gates.XXXXXX)
make fmt 2>&1 | tee "$spike_logs/fmt.log" &&
make check-fmt 2>&1 | tee "$spike_logs/check-fmt.log" &&
make lint 2>&1 | tee "$spike_logs/lint.log" &&
make test 2>&1 | tee "$spike_logs/test.log" &&
make markdownlint 2>&1 | tee "$spike_logs/markdownlint.log" &&
make nixie 2>&1 | tee "$spike_logs/nixie.log"
```

`make markdownlint` includes spelling. If generated `typos.toml` changes,
inspect it and follow repository policy; never hand-edit it. `make lint`
includes Rustdoc, Clippy, and Whitaker. `make test` includes doctests. All must
exit zero; capture complete logs rather than relying on truncated terminal
output. Have the `scrutineer` agent own full sequential gate runs when an agent
team is used. Do not overlap another gate in the same candidate worktree.

Focused red/green commands, used separately and sequentially, are:

```bash
make test TEST_FLAGS='--test ownership_contract'
make test TEST_FLAGS='--test resource_policy_spike'
make test TEST_FLAGS='--test resource_policy_spike_bdd'
make test TEST_FLAGS='--test body_adapter_spike'
make test TEST_FLAGS='--test network_body_spike'
cargo run --example resource_policy_spike
```

Log each with `tee` and `pipefail` as above. No Make example target currently
exists, so the direct Cargo command preserves the repository defaults. A
focused target must actually execute its named cases, not pass with zero tests.
Red output shows the intended absent API or wrong observation; green output
reports passed cases, and the example exits zero after checking its outcomes.
Do not hard-code a guessed test count; record observed counts in the ledger.

For the temporary canary in M1, use the exact source in Rust Router's
`references/polonius-alpha.md` and these commands:

```bash
cargo rustc --example polonius_canary
cargo rustc --example polonius_canary -- -Zpolonius=off
```

The canary's positive/control distinction identifies posture, not performance
or compiler support. Check effective flag precedence before interpreting it; if
flags already select a checker, use the documented additive target override and
retain build/linker/admission flags. Control rejection is expected evidence,
not a failing ordinary repository gate. No canary runs during plan drafting.

## Idempotence, recovery, and delivery

Re-running tests must not mutate process-wide environment, fixed ports, or
external services. Inject environment/clock dependencies when needed; child
process environment configuration is permitted. Review compiler baselines and
lockfile changes explicitly. Never rewrite another worktree's branch or discard
unrelated changes. Revert only the affected green milestone if needed, keeping
its evidence and explaining the new status.

This plan is delivered in a draft PR titled
`Plan: Compile resource-policy and async ownership spike (1.1.3)` with a
branch-wise plan link and a terminal References section containing the Lody
session URL. Push over authenticated SSH as `leynos`, refresh the named remote
tracking ref after explicit-URL pushes, and verify HEAD parity and upstream.
The current session is
<https://lody.ai/leynos/sessions/6b629083-304d-4a28-9b4b-51cfa8f2e65d>. Plan
publication is not implementation approval. After approved implementation,
update the PR to describe delivered behaviour and evidence, preserving the
required task reference and session link; do not merge without authorization.

## Research references

Firecrawl retrieved these primary API references on 2026-10-20. Recheck exact
versions and compatibility during implementation; documentation alone is not
compile evidence.

- [UnsyncBoxBody](https://docs.rs/http-body-util/latest/http_body_util/combinators/struct.UnsyncBoxBody.html):
  its constructor requires a `Send + 'static` body, without a `Sync` bound.
- [async-trait](https://docs.rs/async-trait/latest/async_trait/): boxed `Send`
  futures support heterogeneous dispatch; `?Send` would weaken this contract.
- [trybuild](https://docs.rs/trybuild/latest/trybuild/): compile-pass/fail
  fixtures and reviewed `.stderr` baselines.
- [GoogleTest for Rust](https://docs.rs/googletest/latest/googletest/):
  result-returning assertions and harness requirements for non-fatal assertions.

## Revision notes

2026-10-20: initial plan drafted from the prerequisite decisions and two Wyvern
reconnaissance reports. Scoped proof obligations before selecting the API;
retained compiler and ownership acceptance for phase 2. Implementation remains
unapproved.

2026-10-20: Pandalump and Wafflecat clarified policy stability and
alternatives; Telefono and Dinolump refined ownership witnesses; Buzzy Bee and
Doggylump separated owned/borrowed cancellation and bounded network failure
cleanup. Exact synthetic frame observations complement semantic socket
comparisons. The review trades a small amount of fixture machinery for decisive
evidence, without expanding the spike into production enforcement or a server.

2026-10-21: recorded passing sequential gates for the plan-only candidate.
Shared Cargo cache and compiler admission caused long waits; all runs completed
without bypassing admission. No implementation or compiler-posture acceptance
was inferred from the existing repository test results.
