# Resolve decisions to mitigate API rework (1.1)

Status: BLOCKED at M1 — execution approved; pilot acceptance is missing.

This ExecPlan is a living document. Keep Constraints, Tolerances, Risks,
Progress, Surprises & discoveries, Decision log, Outcomes & retrospective,
Conformance basis, and Verification plan current at every milestone boundary.

## Purpose / big picture

Resolve roadmap tasks **1.1.1 and 1.1.2** so the next ownership spike has an
agreed deployment scenario and testable contracts. A reviewer should be able to
name the decision authority, explain which hooks run for any documented exit,
choose the winning failure, and determine exactly which configured bound
applies. Those answers reduce public API rework before a framework exists.

The deliverable after approval is a consistent decision and specification set,
not a running HTTP framework. This plan records proof obligations and future
test oracles before implementation structure is selected. Task 1.1.3's compiled
spike, task 1.2.2's executable lifecycle model and Verus proof, and subsequent
runtime, transport, and extension delivery remain separate roadmap work.

Approval of this plan authorizes its documentation milestones. It does not
supply the missing pilot-owner acceptance, accept every proposed ADR, commit to
a release, or authorize the downstream runtime work described for handoff.

## Constraints

- Preserve resource identity as the source of endpoint behaviour and policy;
  no duplicate route-policy registry, hidden service container, or extraction
  scheduler.
- This slice edits documentation only. Do not change `src/`, `tests/`, Cargo
  manifests, toolchain, Makefile, or workflows to make a proposed contract look
  implemented. Do not add speculative proof source or passing proof stubs.
- Preserve ordinary-result versus cancellation/panic distinctions. Response
  hooks do not guarantee cleanup, audit delivery, rollback, or durable commit.
- Keep proposed and accepted decisions visibly distinct. Only a named human
  authority can close Q1, Q2, and Q6. Do not infer that authority from a GitHub
  login, this planning request, or a case-study author's name.
- Preserve ADR 006's separate-consumer and independent-generality conditions.
  Earlier HTTP delivery must leave extension interfaces experimental or absent.
- Preserve the compiler experiment in ADR 002. No compiler adoption or API
  compatibility layer follows from this documentation work.
- Run the required gates sequentially, log exit codes, and commit only green
  milestones. Do not bypass shared compiler admission or use `/tmp` for builds.
- No `assume`, `admit`, external/trusted proof bodies, or equivalent shortcuts
  may be introduced to make a speculative theorem compile.

## Tolerances (exception triggers)

Stop the affected milestone and record a proposed deviation if it needs runtime
code, a new dependency, a compiler change, an additional protocol, native TLS,
a new trust boundary, or changed lifecycle guarantees. Set status to BLOCKED
until the user accepts the deviation. Also stop if a required authority cannot
be identified, a normative table retains contradictory rows, or a limit cannot
be enforced at its stated boundary using the selected upstream interface.

The expected edit set is this plan, `docs/terms-of-reference.md`,
`docs/peregrine-design.md`, ADRs 001 and 003, `docs/users-guide.md`,
`docs/developers-guide.md`, `docs/contents.md`, `docs/repository-layout.md`, and
`docs/roadmap.md`. Targeted cross-reference corrections to ADRs 004–006 or the
case studies are allowed when they preserve existing scope. Substantive changes
to those decisions require review before proceeding. Do not rewrite completed
historical ExecPlans. No elapsed-time limit applies; report dependency or gate
blockers with evidence rather than lowering acceptance criteria.

## Risks

- **Unconfirmed pilot (high impact, high likelihood):** the sources name no
  individual authority or committed deployment. Make M1 an evidence gate; a
  plausible specimen must not become a fictional customer commitment.
- **Arbitrary defaults (high impact, medium likelihood):** defaults can imply
  deployment guarantees. Record their workload rationale, enforcement point,
  and owner acceptance; keep performance targets under Q4 separately open.
- **False cleanup guarantees (high impact, medium likelihood):** an unwind
  table can hide future cancellation. Give every await boundary a separate
  cancellation row and define what the supervisor can still observe.
- **Specification drift (high impact, medium likelihood):** proving a model
  alone cannot establish async behaviour. Require a representation relationship
  and generated runtime trace comparisons in the later implementation slice.
- **Unbounded custom code (high impact, medium likelihood):** checking renderer
  output after allocation does not bound its allocations or execution time.
  Document trusted application code and use an independent static timeout path.
- **Baseline tooling failure (medium impact, medium likelihood):** document
  the failing gate and log; repair only within authorized scope. A failed or
  missing check must not be reported as passed.

## Progress

- [x] (2026-10-12) Inspected branch ownership, repository guidance, roadmap,
  design, ADRs, and testing references; renamed the local branch as requested.
- [x] (2026-10-12) Two Wyvern agents gathered pilot and contract evidence;
  CodeGraph confirmed the library remains a stub.
- [x] (2026-10-12) Researched protocol and tooling boundaries with Firecrawl.
- [x] (2026-10-12) Completed six-perspective expert review and incorporated
  its ownership, streaming, verification, and evidence corrections.
- [x] (2026-10-12) Passed formatting, Markdown/spelling, Mermaid, lint, and
  test gates sequentially; checked every local document reference.
- [x] (2026-10-12) Published planning commit `6414fda` as draft
  [PR #26](https://github.com/leynos/peregrine-web/pull/26).
- [x] (2026-10-12) The requesting user explicitly approved execution by
  invoking `/sdlc-implementation` with this plan slug in
  [the implementation session](https://lody.ai/leynos/sessions/724e71be-1ca2-49c9-a6c9-3aff69b0aeac).
  This approves execution only; no individual pilot authority is inferred.
- [x] (2026-10-12) Inspected the clean checkout, matching PR head/base,
  ToR §9, and PR comments/reviews. No Q1/Q2/Q6 acceptance evidence was found.
  Requested the named authority, reviewer/operator, workload/comparator,
  protocol scope, proxy policy, and upgrade requirements from the user.
- [ ] M1: accept the pilot, authority, and protocol/release boundary.
- [ ] M2: agree complete lifecycle, precedence, and configuration contracts.
- [ ] M3: reconcile documentation, acceptance evidence, and roadmap completion.

## Surprises & discoveries

Execution approval and pilot acceptance are separate evidence. The user has
approved this plan, but ToR §9 still assigns only proposed roles. PR #26 has no
human acceptance of Q1/Q2/Q6. M1 therefore remains blocked under its stated
tolerance; M2 and M3 cannot proceed on an inferred pilot or operator.

A repeated execution-intake test gate blocked in an inherited Lody Git shim
inside a build-contract fixture. The gate owner verified and terminated only
that validation run's process chain; the gate exited 143 and is not a pass.
Final validation uses the clean Git environment required by the supplied agent
instructions, preserving compiler admission and shared caches. No daemon or
unrelated process was signalled. The subsequent clean-environment test run
failed with a child-process spawn error (`Resource temporarily unavailable`)
and two build-contract assertions. Preserve that failed log separately; use
`NEXTEST_TEST_THREADS=2` for the next complete gate run to reduce process
demand without excluding tests or changing their expectations.

The source design already specifies independent reverse response hooks and
error precedence, but deliberately leaves all operational defaults to 1.1.2.
The work is to close decisions and make a complete, auditable contract; it is
not permission to invent an alternative lifecycle.

`src/lib.rs` only exports `greet()`. `Cargo.toml` has no runtime dependencies;
its test dependencies include `rstest` and `proptest`. There is no Verus target
in `Makefile`. Existing green tests establish repository tooling health, not
framework conformance.

The toolchain is `nightly-2026-08-27`. `.cargo/config.toml` uses Cranelift and
parallel frontend flags; `Makefile` leaves `POLONIUS_FLAGS` empty by default.
The ambient borrow-checker posture is **unknown** until a selected-compiler
canary establishes it. This plan makes no borrow-sensitive implementation
recommendation; the later spike must use the Rust Router canary through Cargo,
preserving wrapper admission, before attributing acceptance to Polonius.

## Decision log

- (2026-10-12, execution) Record the user's explicit implementation request
  as plan approval. Stop at M1 because A1/A2 lack named authority and explicit
  pilot/deployment acceptance. The user can accept the proposed synthetic
  scenario with named roles and scope, or supply a real workload and
  comparator. A synthetic review cannot establish deployment demand. No
  contract default, ADR acceptance, or roadmap completion follows from
  execution approval.

- (2026-10-12, execution) Remove inherited Lody Git routing from the
  final gate environment after observing a blocked Git-backed fixture. Retain
  the interrupted gate log and validate the final candidate again; do not
  change product contracts or count the interrupted run as successful.
- (2026-10-12, planning) Limit this plan to 1.1.1 and 1.1.2. The roadmap assigns
  compilation to 1.1.3 and exhaustive transition proof to 1.2.2. Advancing
  those here would bypass their dependencies and the requested approval gate.
- (2026-10-12, planning) Propose a protected task/annotation service as the
  review specimen because the hexagonal case study already motivates it. The
  pilot deployment, reviewer, operator, and decision authority remain
  unconfirmed; M1 must obtain evidence rather than claim acceptance.
- (2026-10-12, planning) Specify the abstract lifecycle and candidate lemmas
  before API signatures. Retain proof sketches in prose until the pure kernel
  exists; defer solver triggers and auxiliary algebra to the proof milestone.
- (2026-10-12, planning) Use conventional review for this documentation slice.
  Assign executable tests and Verus obligations to their owning roadmap tasks
  explicitly; adding tests that merely search prose would not validate
  semantics.

## Outcomes & retrospective

Execution intake verified PR #26 against local head `6414fda` and base
`de6b900`. Only the execution record has changed; A1–A7 remain incomplete. No
upstream requirement, runtime interface, dependency, or trust boundary has
changed. M1 needs the user's separate acceptance evidence before substantive
documentation delivery can continue. Execution-intake formatting, Markdown and
spelling, Mermaid, lint, and test gates passed sequentially with exit code
zero. Tests reported 295 passed and 1 skipped; doctests had no cases. Logs are
under `/tmp/` with the worktree UUID and branch in their names and the suffix
`-execution-intake.out`. The planning results below remain historical evidence
for their original candidate.

The six-perspective review found no blocking design flaw. Its concerns led to
exact downstream task assignments, body-owner transfer accounting, explicit
streaming-limit exits, observable transient buffer bounds, versioned upstream
evidence, and explicit formatter restaging. The scope remains decision closure
with later proof delivery. No runtime feature, formal proof, pilot approval, or
roadmap completion is claimed. Record review changes, measured gate results,
and the draft PR here before handoff. At implementation completion, compare
M1–M3 evidence to the purpose above and reconcile every affected upstream
assumption before changing status to COMPLETE.

## Context and orientation

Start with [documentation contents](../contents.md) and
[repository layout](../repository-layout.md). The terms of reference explain
why the project exists; the technical design proposes its contracts. A resource
is the shared application object containing operations, dependencies, and
policy. A responder is its HTTP-method operation. A response draft can still
change; commitment is the final response handoff to Hyper. An ordinary exit
means the future is driven to completion through success or a returned error,
excluding cancellation and panic.

Read these sources before M1 and revisit their affected sections after M2:

- [Terms of reference](../terms-of-reference.md), especially goals G1/G2/G4,
  success criteria, and Q1/Q2/Q6 in §9.
- [Technical design](../peregrine-design.md), §§1, 3–9, 11, and 12.
- [Hexagonal application case study](../hexagonal-application-case-study.md),
  especially E1/E2 and the small consumer specimen in §6.
- [Pachislot extension design](../pachislot-extension-design.md), including
  correlation, HTTP/session ownership, and compatibility experiments X1–X5.
- [Compiler experiment](../polonius-ownership-experiment.md) and
  [ADR 002](../adr-002-compiler-ownership-experiment.md): experimental compiler
  benefits remain a separate decision.
- [ADR 001](../adr-001-resource-aware-lifecycle.md): lifecycle ownership.
- [ADR 003](../adr-003-http-integration-boundaries.md): HTTP integration.
- [ADR 004](../adr-004-application-port-boundaries.md): application ports.
- [ADR 005](../adr-005-extension-and-upgrade-boundaries.md): session ownership.
- [ADR 006](../adr-006-extension-interface-stabilization.md): stabilization
  gates.
- [Developer guide](../developers-guide.md) and
  [documentation style guide](../documentation-style-guide.md).
- [Dependency-injection testing guide](../reliable-testing-in-rust-via-dependency-injection.md):
  inject environment, clock, randomness, and outbound dependencies.
- [Fixture guide](../rust-testing-with-rstest-fixtures.md) and
  [BDD guide](../rstest-bdd-users-guide.md): finite cases and reusable fixtures.
- [Doctest guide](../rust-doctest-dry-guide.md): executable public examples,
  with private edge cases in ordinary tests.
- [Complexity and refactoring guide](../complexity-antipatterns-and-refactoring-strategies.md):
  coherent functions, command/query separation, and existing-helper discovery.

Use the `execplans`, `codegraph-mcp`, `rust-router`, and `firecrawl-mcp`
skills. For verification selection, consult `rust-verification`, then `verus`
when implementing the proof. Use `logisphere-experts` for design review and
`pr-creation` and `commit-message` for delivery. Skills are procedural aids;
this plan contains the contract and must remain usable without agent memory.

## Conformance basis

The baseline is commit `de6b900d08b2a7ffc798dca47e40ea997dcbc267`: terms of
reference draft v0.3 and technical design proposed v0.5, both dated 2026-09-21,
plus ADRs 001–006 at that commit. These ADRs remain proposed. Normative HTTP
semantics come from RFC 9110; the design deliberately defines only a subset.
Retrieved tooling documentation is research evidence, not a locked or
compatibility-tested dependency set.

| Source requirement or gap                                     | Milestone | Acceptance evidence                                                   |
| ------------------------------------------------------------- | --------- | --------------------------------------------------------------------- |
| G1/G4; Q1; 1.1.1                                              | M1        | A1: named pilot and comparative review scenario                       |
| Q2/Q6; design §§1/12; ADR 006                                 | M1        | A2: named authority, operator requirements, explicit scope acceptance |
| G2; design §§3/5–8; ADR 001; 1.1.2                            | M2        | A3: exhaustive ordinary-exit and cancellation tables                  |
| Design §7; ADR 003                                            | M2        | A4: error precedence, safe presentation, and commitment matrix        |
| Design §9; ADRs 003–005                                       | M2        | A5: complete validated configuration and ownership tables             |
| Design §11 I1–I6; G4                                          | M2/M3     | A6: proof sketch and downstream verification assignments              |
| User/developer documentation; requested completion discipline | M3        | A7: reconciled sources, green gates, accepted task completion         |

_Table 1: Selective traceability from requirements to evidence._

Change a traced decision only after checking its upstream assumptions and later
consumers. In particular, transport changes affect Q2, ADRs 003/005/006, and
roadmap 1.4/7.3; lifecycle changes affect I1/I2 and roadmap 1.2.2/2.1.3.

## Verification plan

### Abstract model before implementation

Specify a request independently of Rust storage as a phase, a fixed ordered
middleware sequence, an optional frozen resource identity and effective method,
a monotonic forward-stop reason, a dispatch count, a response-hook cursor,
private primary failure, selected public failure, and a commitment flag. Keep
ordinary execution and external cancellation as distinct transitions.

Initially, dispatch count is zero, no response hooks have run, commitment is
false, and routing is unset. Initialization can establish a pending failure.
Request hooks may change the effective URI until routing freezes the target.
Only a successfully matched, authorized, active request can dispatch. Record
dispatch before invoking the responder so a responder failure cannot re-enter
it. Completion and failure disable every remaining forward action. Every
ordinary path enters reverse response processing, then selects/render errors
and finalizes once. Cancellation enters an interrupted terminal state and does
not synthesize response-hook completion. A supervisor may separately construct
a pre-commit timeout response.

The representation relationship for later code must map its phase and private
state to this model, its hook indices to positions in the fixed sequence, and
its trace events to model transitions. The same resource identity must appear
at policy and dispatch. Each awaited hook may produce an ordinary successor or
an interruption; it must never grant access to reset the stop state. A proof of
the abstract model alone does not establish that this relationship holds.

### Obligations, methods, and witnesses

| ID      | Contract and proposed semantic lemma                                                                                                                                                                                                                  | Method and artefact owner                                                                                                                                                             | Non-vacuity and discharge                                                                                                                                                                 |
| ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| V1 / I1 | From a valid initial state, every permitted transition preserves dispatch count at most one; after stop, no later transition dispatches. Candidate lemmas: stop is preserved, dispatch requires active state, and dispatch consumes that eligibility. | Unbounded Verus proof of the later pure transition kernel, roadmap 1.2.2; generated runtime traces in 2.1.3 and lifecycle implementation.                                             | Witness zero hooks with one successful dispatch and a stopped request with zero dispatches. Deliberately permit a second dispatch or clear stop: verification must fail.                  |
| V2 / I2 | Ordinary completion visits exactly the reverse middleware sequence once, including hooks without forward setup; hook failure does not truncate it.                                                                                                    | `proptest` over lengths and stop/failure positions, plus finite `rstest` examples; roadmap 1.2.2 and 3.1.3.                                                                           | Include zero/one/three hooks, every failure phase, and failure in first/last reverse hook. A skip or duplicate mutation must fail. Cancellation is excluded explicitly.                   |
| V3 / I3 | Denied policy never invokes the responder, and policy and dispatch use the same frozen resource.                                                                                                                                                      | Finite policy/identity/method `rstest` cases and `rstest-bdd`; roadmap 1.1.3, 4.1.1–4.1.3, and 4.2.1.                                                                                 | Accepted identity reaches responder once; missing/wrong-tenant identity reaches it zero times. Swapping the resource or bypassing denial must fail.                                       |
| V4 / I4 | Checked collection cannot exceed its byte cap; failed or transferred consumption cannot become cached success.                                                                                                                                        | Frame-partition and state-sequence `proptest`; roadmap 5.1.1 and 5.2.1–5.2.3.                                                                                                         | Empty frame, exact cap, cap plus one, lying length, partial failure, repeated read. Check before append; a post-append check mutation must fail.                                          |
| V5 / I5 | Finalization applies the selected failure and mandatory HTTP constraints once, without changing application effects or a committed status.                                                                                                            | Finite protocol/error tables, `insta` presentation snapshots, and socket tests; roadmap 3.1.3–3.1.4, 3.2.2, and 5.2.2.                                                                | 409 then hook failure yields sanitized 500; HEAD errors have no content; post-commit body failure cannot emit another response. Status restoration and stale framing mutations must fail. |
| V6 / I6 | Public async values meet the selected `Send` and ownership bounds; request borrows cannot escape into streams or detached tasks.                                                                                                                      | `trybuild` pass/fail pairs in 1.1.3 and phase 2.                                                                                                                                      | Positive owned escape and negative borrowed escape; record compiler diagnostics and reject unrelated compile failures.                                                                    |
| V7      | Correlation is engine-owned and consistent across configured echo/presentation; private causes never enter safe details.                                                                                                                              | Finite correlation/profile cases, byte-boundary properties, redaction sentinels, and normalized snapshots; roadmap 1.4.1, 3.1.3, 6.1.1, and 7.4.1.                                    | Exercise duplicates, invalid bytes, untrusted input, generator failure, echo off, fallback, and later hook failure. Inject a secret sentinel and reject its appearance.                   |
| V8      | Validated limits are finite unless a named opt-out permits otherwise; permits remain accounted for until their documented release boundary.                                                                                                           | `rstest` invalid/boundary configuration cases, generated numeric boundaries, deterministic cancellation and real transport tests; roadmap 3.2.1, 5.1.1, 5.2.1–5.2.3, and 6.1.1–6.1.4. | Minimum/maximum, zero, overflow, stalled peer, dropped stream, and shutdown expiry. Premature permit release must be observable.                                                          |

_Table 2: Verification obligations and their later executable evidence._

For V4, inspect the bound after every collection transition and observe peak
retained buffer length or use an injected bounded sink. A test observing only
the returned error cannot detect append-then-discard. The measured quantity is
the accumulator, excluding upstream frame allocation.

The lemma statements are hypotheses about a useful proof structure, not fixed
proof decomposition. M2 must pressure-test them against every contract row and
record counterexamples before selecting public API shapes. V1 warrants Verus
because it claims safety for every finite execution length. Do not formalize
JSON formatting, ordinary adapter wiring, or documentation synchronization.
Kani is not additionally required: bounded exploration would not replace V1's
unbounded guarantee; cheap properties and explicit protocol cases suit the
other obligations. Reassess if implementation exposes a different invariant.

For this slice, A3–A6 are review evidence: tables, reachable trace examples,
preconditions, and proposed lemmas. They do **not** discharge V1–V8 for a
runtime. M3 must leave that distinction explicit in the design.

### External premises and residual gaps

Assume ordinary futures are polled to completion and their hooks terminate;
reverse-hook completion is conditional on that premise. Dropping futures and
runtime termination can prevent progress. A deadline is a cooperative runtime
boundary, not pre-emption of synchronous application code. Assume upstream body
frames and HTTP parsing follow the chosen version's documented interface; frame
allocation before framework receipt is outside its collection bound. Assume
Rust's ownership checker and the Verus verifier/compiler toolchain are sound
within their documented trusted bases. Do not attempt to prove Tokio, Hyper,
serializers, or application authorization internals.

External ingress authentication and application-owned effects require their own
operational evidence. A safe public error does not prove private logging is
secret-free. A fixed renderer-output cap bounds accepted encoded bytes, not
arbitrary renderer allocation or running time. Enforce framework-owned budget
logic against actual adapters when they exist, retaining these residual gaps.

### Downstream test and proof handoff

When each later implementation is approved, first add the smallest failing
unit/behavioural case, record the expected red failure, implement the minimum
change, then refactor and run the full gates. Use `rstest` fixtures and case
tables, `googletest` matchers for semantic assertions, and `pretty_assertions`
for structured trace diffs, with custom diagnostic messages. Both assertion
libraries are authorized dependencies; introduce compatible caret versions in
the owning implementation, not in this documentation slice. Upstream documents
`rstest` integration; if using non-fatal assertions, place `#[gtest]` before
`#[rstest]` and verify each generated case runs once.[^googletest]

Proposed future paths are `tests/lifecycle_contract.rs`,
`tests/features/lifecycle_contract.feature`, `tests/http_finalization.rs`,
`tests/configuration_contract.rs`, and `verus/lifecycle.rs`. They do not exist
as a consequence of this plan. Keep BDD bindings in ordinary Rust integration
tests so `make test` executes them. The following feature specifies
representative oracles; expand it from A3/A4 when implementing the lifecycle.

```gherkin
Feature: Resource-aware lifecycle outcomes
  Scenario: Completion before routing
    Given middleware A, B, and C in registration order
    When B completes the request before routing
    Then the trace is A.request, B.request, C.response, B.response, A.response
    And no responder is invoked

  Scenario: Response failure overrides a domain conflict
    Given a responder returns a private domain conflict mapped to 409
    When C's response hook fails and B's response hook also fails
    Then all response hooks run once in reverse order
    And the selected public failure is a sanitized 500
    And private diagnostic causes do not appear in the response

  Scenario: Supervised timeout before commitment
    Given a HEAD request is suspended in a responder
    When the request execution deadline expires before commitment
    Then the supervisor produces a minimal 504 with no response content
    And no remaining response hooks are promised

  Scenario: Failure after commitment
    Given response headers have been handed to the transport
    When the response body fails
    Then the transfer terminates without a replacement status
    And the transfer failure is recorded separately from application effects
```

Use `insta` for default/custom/fallback error representations and stable
traces, normalizing clocks and generated IDs while also asserting semantic
status, header, and secrecy rules. Use end-to-end socket tests for parser
rejection, HEAD/framing, streaming, timeouts, disconnect, and shutdown.
In-process traces alone cannot establish wire behaviour. Inject clocks,
environment, identity, generators, and application ports; only child processes
may receive mutated environments. Keep public doctests executable once the API
exists.

Roadmap 1.2.2 must add the proof entrypoint and pinned tool metadata, then run
`prover-tools verus run --proof-file verus/lifecycle.rs` through a proposed
`make verus` target. That command is a future acceptance requirement, not an
existing gate. Record verifier version, solver/toolchain identity, successful
verification with zero errors and no admitted obligations, and the negative
control failure. Also run `make check-fmt`, `make lint`, and `make test` at
that proof milestone. A skipped proof cannot satisfy it. Generated trace tests
must continue to connect the pure kernel to async execution; the proof cannot
replace integration tests. Defer auxiliary induction lemmas and solver tuning
until the actual verification conditions exist.[^verus]

## Plan of work

### M1 — Confirm the deployment and decision authority

Read Q1/Q2/Q6 with the maintainer and prospective operator. Propose the named
review scenario **protected task/annotation service**: a resource offers read
and mutation operations, middleware checks resource policy before dispatch, and
the application owns tenant authorization and a per-operation unit of work. Use
the case study's HTTP and non-HTTP invocation comparison as a later
architecture oracle. Do not implement the specimen or migrate an application.

In `docs/terms-of-reference.md` §9 record the actual named service/workload,
individual decision authority, pilot reviewer and operator (one person may hold
multiple roles), date, and durable approval evidence. Record its current
framework/comparator and how reviewers will compare locating policy, adding an
operation, and explaining rejection without duplicating policy registration. A
synthetic fixture is acceptable for architecture review only if the owner
explicitly accepts that limitation; it is not evidence of deployment demand.

Confirm or revise: one `peregrine-web` library, in-process execution and
HTTP/1.1 serving, external secure ingress, buffered and streaming core, with
HTTP/2/native TLS deferred and Pachislot delivery separate. Specify trusted
proxy configuration and whether the pilot needs upgrades now or later. Reconcile
`docs/peregrine-design.md` §§1/12 with the accepted boundary and ADR
006. If this changes scope materially, record the deviation and seek approval.
Keep Q3/Q4/Q5 open for their actual API, measurement, and compiler evidence.

**Acceptance A1/A2:** a named reviewer can describe the use case and
comparison; a named authority explicitly accepts the deployment and maintenance
scope. No placeholder owners or inferred sign-off remain. Run the sequential
gates, record conformance and evidence, then commit the coherent decision set.
If acceptance is unavailable, remain at M1; M2 cannot complete against an
invented pilot. Recovery is to revise the proposed decision without claiming
closure.

### M2 — Specify lifecycle, errors, and operational configuration

After M1, edit `docs/peregrine-design.md` §§3 and 5–9 with stable contract-row
identifiers. Link the lifecycle table from ADR 001 and put ownership and budget
decisions in ADR 003. Keep a single normative definition per contract, using
cross-references elsewhere. Record alternatives and rationale for substantive
changes. Do not promote ADR 001 to Accepted before its separate spike and trace
prerequisites are satisfied.

For A3, each row must identify entry phase, route/resource availability,
remaining forward work, responder count, response-hook sequence, selected
outcome, commitment state, and cleanup/diagnostic guarantees. Cover all of:

- Initialization/correlation rejection; request completion or failure; success;
  route miss; resource completion or policy failure; responder failure.
- Recognized unsupported methods (405 and `Allow`), unknown methods (501),
  automatic and explicit OPTIONS, `OPTIONS *`, explicit HEAD and GET fallback,
  HEAD short circuits/errors, and forbidden informational final responses.
- Body limit (413), unsupported media (415), malformed JSON (400), body-read
  deadline (408), body transport errors, and invalid/repeated consumption.
- Request-stream idle expiry and total-byte exhaustion before and after
  response commitment; response-stream idle expiry after commitment.
  Distinguish ordinary error unwinding, execution cancellation, and transfer
  termination without status replacement, including request ingestion owned by
  a response stream.
- Rejection with an unread request body: close after a safe response, or close
  immediately if framing prevents it; do not drain unbounded input.
- Response-hook failures at every reverse position, multiple failures, error
  presentation invalidation, renderer error, and output-budget failure.
- Future drop at initialization, every awaited forward/response hook and
  responder, pre-finalization, and response-body transfer; supervised execution
  expiry (504 before commitment), disconnect, panic, runtime shutdown, and
  graceful-drain expiry. State which boundaries cannot produce an HTTP response.

Cross the ordinary response cases with HEAD/204/304 constraints. Check 205's
no-content requirement against RFC 9110 as a targeted design gap; record its
resolution rather than copying the existing subset blindly.[^http] Do not
promise arbitrary 1xx outcomes or upgrade semantics in the ordinary finalizer.

For A4, separately tabulate first forward failure, completion followed by
failure, first response-hook failure, subsequent hook failures, renderer
failure, timeout, and committed body failure. Preserve private first cause; the
first response-hook failure selects sanitized 500, later failures do not stop
unwind, and subsequent failures invalidate presentation overrides. Clear
partial-success bodies and stale representation headers. Derive required
headers from the selected typed failure, document the safe-header allowlist,
and retain correlation according to configured echo policy. An original 401's
challenge or 405's `Allow` must not leak into a replacement 500 by accident.

Name the deterministic tie rule for simultaneous timeout/completion readiness,
the timeout clock's start and end, and the commitment check. Specify bounded
secondary-error recording when capacity is exhausted. Define a
generator-failure outcome without claiming a correlation guarantee that cannot
be met. Choose the static fallback's exact safe envelope and how it fits the
minimum output cap. These are decisions to close with evidence, not unspecified
implementation choices. A synchronous renderer cannot be interrupted
cooperatively; state its trusted-code contract and keep it off the supervisor's
timeout response path.

For A5, build a configuration table with these mandatory columns: name, typed
unit, default, allowed range, cross-field validation, scope, enforcement point,
exceedance outcome, and explicit opt-out. Every row must have a concrete agreed
default before M2 completes. Missing values and zero must never mean unlimited.
Choose values with the pilot operator and record a workload rationale; do not
present upstream defaults as measured Peregrine release budgets.

| Required configuration family                         | Required distinctions and review decisions                                                                                                                                            |
| ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Accepted connections and concurrent request execution | Positive counts; before-accept admission versus request permit; waiting/backlog policy; permit release versus live transfer accounting                                                |
| Header parsing                                        | Header count, aggregate bytes including start line, parser buffer bytes, read deadline in milliseconds; adapter-only rejection before context; timer dependency and backend minimum   |
| Buffered request collection                           | Bytes and total read deadline in milliseconds; known and unknown lengths; validate before append and mark failed consumption                                                          |
| Streamed request and response bodies                  | Request total bytes versus long-lived response transfer; idle deadline in milliseconds and what resets it; named total-byte opt-out without disabling idle/admission/drain accounting |
| Whole request execution                               | Milliseconds from the defined admission/initialization boundary to commitment; queued-time treatment; supervised 504 versus ordinary body-read 408                                    |
| Shutdown and accept retries                           | Drain milliseconds; retry count and minimum/maximum backoff milliseconds; terminal outcome and tracked-task accounting                                                                |
| Correlation                                           | Header name, accepted syntax, byte limit, duplicate handling, peer trust, generator and failure policy, validation profile, echo default; immutable facts and propagation ownership   |
| Safe error details                                    | Maximum item count, per-item and total encoded bytes, truncation/omission rules, and exclusion of submitted secrets                                                                   |
| Error rendering and secondary diagnostics             | Maximum encoded representation bytes, bounded writer/check boundary, static fallback minimum, secondary failure count/bytes, overflow logging policy                                  |
| Observability                                         | Bounded label vocabulary, unknown-method classification, no raw path/ID/secret labels, and distinct request/transfer/application outcomes                                             |

_Table 3: Configuration inventory that M2 must resolve without placeholders._

Define bytes as octets and document duration conversion/overflow validation.
Reject invalid configuration before serving or application execution. Express
opt-outs as named variants in the contract rather than sentinel integers; which
concrete Rust types represent them belongs to the spike. Document which
settings are enforced by in-process execution and which require the transport.
Hyper's header-size limit, connection buffer, and header count are different
controls; validate their selected-version semantics and timer requirements
rather than treating one as a substitute for another.[^hyper]

For A5, record the exact upstream version reviewed, version-specific method
links, supported enforcement boundary, and residual gaps beside the table. The
`latest` research link is not sufficient implementation evidence.

In ADR 003 record a responsibility table: the engine owns immutable facts,
phase transitions, failure selection, renderer invocation, required headers,
and HTTP finalization; the application supplies policy, safe mappings,
renderer/generator configuration, identity, durable effects, and lifespan
cleanup; the body owner records transfer outcomes even in process. The
transport owns parser/connection enforcement and integrates body outcomes into
connection accounting. Preserve ADR 004's HTTP-independent ports and ADR 005's
separation of request and session correlation. Document the limits of the
renderer cap, logging cap, and cancellation guarantees explicitly.

**Acceptance A3–A6:** walk every table row with concrete A/B/C traces and
zero-hook cases, complete every configuration cell, and reconcile the abstract
model and V1–V8 assignments. Record unresolved counterexamples as blockers. A
second reviewer should derive the selected result without consulting author
intent. Run all gates sequentially and commit. Recovery is a focused document
revision; never weaken a guarantee solely to remove a review finding.

### M3 — Reconcile guides and deliver the decision set

Update `docs/users-guide.md` with a clearly labelled proposed-contract section
or link: explain public behaviour, cancellation limitations, and configuration
scope without advertising unavailable APIs. Put internal engine/application
interfaces and proof-influenced invariants in `docs/peregrine-design.md` and
ADR 003. Update `docs/developers-guide.md` with contract-row/test traceability,
injected dependencies, and the later proof/test relationship. Update
`docs/contents.md` for new documents and `docs/repository-layout.md` for the
ExecPlan directory; no separate architecture document is needed because the
technical design already owns these interfaces.

Review all six expert perspectives: architecture, alternatives, performance and
observability, contracts, reliability, and maintainability. Reconcile
conflicting advice against G1/G2/G4 and record accepted/rejected findings with
reasons. Do not substitute expert review for pilot-owner acceptance.

**Acceptance A7:** verify A1–A6 are present and accepted as required, all
guides agree with the design, and every downstream obligation has an owner
task. Run all gates. Only after implementation of M1–M3 is complete, mark
roadmap **1.1.1 and 1.1.2** checked and explicitly `done`, retaining their
acceptance text. Do not mark all of 1.1 done: 1.1.3 remains outstanding. Leave
every roadmap item unchecked in this planning PR. Use the `mapsplice` skill
when making that later structural roadmap edit. Record approval and evidence,
mark the executed plan COMPLETE, and commit the final coherent documentation.

At each milestone record whether assigned requirements are satisfied, the
approved design is followed, assumptions still hold, there is no unapproved
API/dependency/trust/format change, and trace links remain current. No source
compatibility machinery is needed: there is no implemented framework API.
Recovery is to revert the specific milestone commit if necessary, preserving
later independent work; do not reset a shared branch.

## Concrete steps

Run from the repository root on `1-1-resolve-decisions-to-mitigate-api-rework`.
Before execution, read this plan, `AGENTS.md`, `git status --short`, and
`git branch --show-current`; confirm approval evidence and the exact baseline.
Keep unrelated work intact.

For the plan PR and after every major implementation milestone, stage only
intended documentation so the spelling gate includes the new files. Delegate
full gates to the scrutineer agent and run these sequentially with shell
pipeline failure propagation. Run formatting before checks; inspect its diff.

```bash
set -o pipefail
make fmt 2>&1 | tee /tmp/fmt-peregrine-web-1-1.out
make check-fmt 2>&1 | tee /tmp/check-fmt-peregrine-web-1-1.out
make markdownlint 2>&1 | tee /tmp/markdownlint-peregrine-web-1-1.out
make nixie 2>&1 | tee /tmp/nixie-peregrine-web-1-1.out
make lint 2>&1 | tee /tmp/lint-peregrine-web-1-1.out
make test 2>&1 | tee /tmp/test-peregrine-web-1-1.out
```

Run each command separately and stop on failure; do not continue merely because
`tee` succeeded. Each must exit zero. `make markdownlint` includes spelling;
`make test` includes doctests. Hosted `build-test` remains the authoritative
repository gate; local results do not claim hosted success. Capture exact
counts and logs after execution rather than inventing expected pass counts.
Read a failing log before changing anything; rerun only after a relevant fix.

For each green atomic milestone, inspect `git diff --check` and the staged
diff, restage intended formatter changes, and verify staged and working copies
agree. Then write an imperative commit message to a temporary file, and use
`git commit -F` with that file. Preserve rollback points. No failed gate may be
hidden by committing the documentation anyway.

For this plan's delivery, use the standard GitHub client authenticated as
`leynos`, a clean command environment as specified in `AGENTS.md`, and verified
SSH identity. Push to the requested branch, fetch its explicit tracking ref,
set upstream to `origin/1-1-resolve-decisions-to-mitigate-api-rework`, and
compare local/remote SHAs. Create a **draft** PR titled
`Plan: Resolve decisions to mitigate API rework (1.1)` with a branch-wise plan
link and a terminal References section containing this Lody session:
<https://lody.ai/leynos/sessions/a4599fc6-f75b-46a1-9fdc-3116d6c5774c>. Read
back title, body, draft state, and branch head. Publication is authorized;
implementation waits for explicit user approval.

## Validation and acceptance

This slice has no executable feature to drive Red-Green-Refactor. The nearest
honest red evidence is the baseline's unassigned Q1/Q2/Q6 owners and incomplete
contract/configuration tables. Green means A1–A7 are complete with explicit
acceptance evidence and all repository gates passing. Review must reject a
missing exit row, implicit unlimited setting, unsupported cleanup promise, or
unassigned authority. That negative check validates the specification process;
it does not prove the future framework.

Runtime tests use the red/green procedure and BDD oracles above when their
owning roadmap work starts. Do not create fake framework tests to make the plan
appear executable. The plan PR is complete when reviewed, validated, committed,
pushed, and available as a draft for approval. Roadmap task delivery is
complete only after the approved M1–M3 work meets A1–A7.

## Idempotence and recovery

Documentation edits and gates can be repeated. Retain stable row identifiers
when refining text and record superseded choices rather than silently losing
acceptance history. Use one gate process at a time; wait for compile admission
instead of launching a competing build. If disk capacity is exhausted, stop and
report it. Do not kill unrelated processes, change the Lody daemon, or create
isolated Cargo caches. Inspect broad formatter output and retain only necessary
changes within scope, never discarding someone else's work.

A partial push or PR failure does not require rewriting commits: inspect the
remote head and existing PR before retrying. Refresh the remote-tracking ref
after explicit-URL pushes. Approval remains pending regardless of publication.

## Interfaces and dependencies

The documentation interface consists of the decision record in ToR §9,
canonical contract/configuration tables in the design, and the responsibility
and budget decisions in ADR 003. Their consumers are the ownership spike,
lifecycle model, body/finalization implementation, transport adapter, and
extension experiments. Every new abstraction later needs an equivalent-helper
sweep and documented ownership, permitted callers, and composition rules.

Do not freeze Rust signatures before M1/M2 or add runtime/test dependencies
here. Later tests use the requested `rstest`, `rstest-bdd`, `googletest`,
`pretty_assertions`, `insta`, and `proptest` where their evidence applies, plus
`trybuild` for ownership. Verus is a separate verifier, not a Cargo dependency.
Pin compatible tool versions and record them at the owning implementation
milestone; the present research does not establish a tested dependency set.

## Artefacts and notes

Planning validation on 2026-10-12 passed `make fmt`, `make check-fmt`,
`make markdownlint` (including spelling), `make nixie`, `make lint`, and
`make test`, each with exit code zero. The scrutineer ran checks sequentially.
Logs use `/tmp/<gate>-a4599fc6-f75b-46a1-9fdc-3116d6c5774c-` followed by
`1-1-resolve-decisions-to-mitigate-api-rework.out`; the formatter log is
`/tmp/fmt-peregrine-web-1-1.out`. Initial process-slot exhaustion delayed
execution; no shared process or admission control was changed. Rust builds
reported the managed cache unavailable and succeeded uncached. Hosted checks
are separate evidence and are not claimed here.

Planning evidence was gathered through CodeGraph and two Wyvern agents using
context packs `pk_poizqppi` and `pk_e7ma7ccy`. The repository paths and
contracts above are the durable evidence; executing this plan does not require
those packs to remain available. External references were checked with
Firecrawl on 2026-10-12; structured extraction was cross-checked against source
prose where it omitted distinctions or produced suspect section references.

[^http]: [RFC 9110](https://www.rfc-editor.org/rfc/rfc9110.html), especially
    §§8.6, 9.1, 9.3.2, 15.3.5–15.3.6, 15.4.5, and 15.5.6. HEAD and the
    specified no-content statuses require explicit finalization rules.

[^hyper]: [Hyper HTTP/1 builder](https://docs.rs/hyper/latest/hyper/server/conn/http1/struct.Builder.html),
    methods `max_headers`, `max_header_size`, `max_buf_size`,
    `header_read_timeout`, and `timer`. Recheck against the selected version;
    do not confuse an upstream connection buffer bound with a header contract.

[^googletest]: [GoogleTest Rust integration guidance](https://github.com/google/googletest-rust)
    and [assertion documentation](https://docs.rs/googletest/latest/googletest/).

[^verus]: [Verus guide](https://verus-lang.github.io/verus/guide/) describes
    static functional verification and its limits. The `verus` skill provides
    the repository integration pattern using `prover-tools`.

Revision note (2026-10-12): expert review clarified downstream ownership,
streaming exits, transfer accounting, non-vacuity checks, and versioned limit
evidence. These refinements preserve the decision-only scope and approval gate.

Revision note (2026-10-12, execution intake): record explicit execution
approval and the missing M1 acceptance evidence. Preserve all contract and
verification requirements; M2/M3 remain dependent on accepted A1/A2. Record the
interrupted Git-backed gate separately and use the prescribed clean Git
environment for final validation.
