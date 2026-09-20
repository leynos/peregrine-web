# Peregrine Web potential roadmap

This proposed roadmap turns the [terms of reference](terms-of-reference.md) and
[technical design](peregrine-design.md)
into a candidate delivery sequence. It is not a release commitment and promises
no dates. Tasks remain unchecked because compiler probes are evidence for the
proposal, not completion of the planned framework prototypes.
[ADR 001](adr-001-resource-aware-lifecycle.md) and
[ADR 002](adr-002-compiler-ownership-experiment.md) remain proposed. The key
compiler experiment in phase 2 gates the implementation direction.

The Goals, Ideas, Steps, Tasks (GIST) model links delivery to evidence. Goals
state the outcomes, phases carry testable ideas, steps answer delivery
questions, and tasks define review-sized execution units. Reject or revise an
idea when its evidence fails; do not treat task completion as proof of product
value.

| Goal from terms of reference §6 | Evidence sought                                                                              | Principal phases |
| ------------------------------- | -------------------------------------------------------------------------------------------- | ---------------- |
| G1: Endpoint cohesion           | A resource supplies behaviour and policy without a second route-policy registry.             | 1, 2, 3, 4       |
| G2: Explainable processing      | Observed lifecycle traces agree with the specified ordering and failure rules.               | 1, 2, 3, 4, 6    |
| G3: Controlled resource use     | Body, concurrency, cancellation, and shutdown bounds are demonstrated.                       | 5, 6             |
| G4: Maintainable adoption       | Readable examples, reproducible checks, and measured performance support adoption decisions. | 1–7              |

_Table 1: Goals and their delivery evidence._

Unit and behavioural tests accompany implementation tasks. Property checks,
proofs, public documentation, and semantic diagnostics belong to the contracts
they validate. Dedicated end-to-end and interaction suites appear where they
establish evidence beyond an individual component. Numbered dependencies are
prerequisites; adjacent tasks need not be serial unless stated.

## 1. Establish testable contracts

Idea: if a small compiled contract can retain resource identity across async
middleware without hidden application machinery, the proposed architecture is
worth extending. This phase serves G1, G2, and G4.

### 1.1. Resolve the decisions that would otherwise force API rework

Can the proposal identify one useful initial deployment and a coherent public
contract? The answers determine which later slices are relevant. See
[terms of reference §9](terms-of-reference.md#9-open-questions-and-decision-authority)
and technical design §§1, 4, 9, and 12.

- [ ] 1.1.1. Record the pilot use case, decision owner, and initial protocol
      scope.
  - See terms-of-reference.md Q1, Q2, and Q6; peregrine-design.md §§1 and 12.
  - Success: a named review scenario and owner confirm or revise HTTP/1.1,
    external secure ingress, and the proposed release boundary.
- [ ] 1.1.2. Specify lifecycle, error precedence, and operational configuration.
  - Requires 1.1.1.
  - See peregrine-design.md §§3 and 5–9; adr-001-resource-aware-lifecycle.md.
  - Success: contract tables cover every ordinary exit and cancellation
    boundary;
    limits have units, defaults, validation rules, and explicit opt-outs.
- [ ] 1.1.3. Compile a resource-policy and async ownership spike.
  - Requires 1.1.2.
  - See peregrine-design.md §§3–4 and 11–12; terms-of-reference.md Q3 and Q5.
  - Success: heterogeneous resource objects, typed policy, synthetic and network
    body adapters, and `Send` futures compile on the selected compiler floor;
    negative `trybuild` cases reject invalid ownership. Defer final ownership
    and compiler-posture acceptance to phase 2.

### 1.2. Make the smallest contract reproducible

Can a contributor build and verify the selected contract through repository
entrypoints? This determines whether feature work can rely on the baseline. See
technical design §§2, 11, and 12.

- [ ] 1.2.1. Establish the minimal dependency and feature baseline.
  - Requires 1.1.3.
  - See peregrine-design.md §§2 and 12; repository-layout.md.
  - Success: compatible caret requirements and licence review are recorded;
    `make check-fmt`, `make lint`, and `make test` pass on the supported baseline.
- [ ] 1.2.2. Define the lifecycle transition model and trace vocabulary.
  - Requires 1.1.2 and 1.2.1.
  - See peregrine-design.md §§6–7 and 11, invariants I1 and I2.
  - Success: an exhaustive Verus proof derives stop/dispatch safety from the
    transitions; normal and early-exit trace examples provide a runtime oracle.

## 2. Test a pure Polonius and new-solver implementation

Idea: if Peregrine can design exclusively for Polonius and the new trait
solver, borrowing helpers and middleware composition may become easier to
maintain without weakening entity identity or lifecycle guarantees. This is a
key experiment for G1, G2, and G4, not a compiler-adoption decision.

### 2.1. Separate compiler benefits from ownership-design benefits

Does exclusive use of the two compiler mechanisms buy anything beyond an
old-checker-compatible implementation with the same phase views? Results decide
which implementation later slices should extend. See peregrine-design.md §3.1
and polonius-ownership-experiment.md §§1–4.

- [ ] 2.1.1. Preserve and reproduce the four-way compiler evidence matrix.
  - Requires steps 1.1–1.2.
  - See polonius-ownership-experiment.md §§2–3.
  - Success: the pinned compiler separately toggles Polonius and the solver;
    positive and negative examples record diagnostics, compiler identity, and
    whether benefits are checker-specific, solver-specific, or available to both.
- [ ] 2.1.2. Build comparable entity-first ownership prototypes.
  - Requires 2.1.1.
  - See polonius-ownership-experiment.md §§4–5; peregrine-design.md §§4–8.
  - Success: both prototypes run the same protected resource, fallible cache,
    and application-owned middleware sequence with the same typed phase views,
    within a pre-recorded effort budget and minimum specimen; compare against
    standard-library entry APIs rather than compulsory cloning baselines. Compile
    the candidate lifetime-indexed middleware, associated-type, and returned-
    future `Send` bounds under both solvers; reduce differences or record none.
- [ ] 2.1.3. Exercise lifecycle and ownership boundaries end to end in process.
  - Requires 2.1.2.
  - See polonius-ownership-experiment.md §§4–5; peregrine-design.md §11.
  - Success: traces match for authorization, 404, `OPTIONS *`, pre-routing
    completion, hook failure,
    and cancellation; streaming escapes and detached borrowed work fail to
    compile, while supported request-scoped awaits remain usable. Preserve the
    multithreaded invocation contract and reject non-`Send` responder futures.

### 2.2. Decide whether the exclusive compiler contract earns its cost

Can a downstream application build, diagnose, and maintain the prototype, and
are the measured benefits worth that requirement? The result selects one
production direction before the following delivery phases. See
polonius-ownership-experiment.md §§5–6 and
adr-002-compiler-ownership-experiment.md.

- [ ] 2.2.1. Measure ergonomics, runtime cost, and compilation cost separately.
  - Requires step 2.1.
  - See polonius-ownership-experiment.md §5.
  - Success: equivalent workloads report helper complexity, allocations,
    retained memory, latency, clean/incremental build time, and binary size;
    independent reviewers repeat the same resource and middleware edits against
    pre-recorded workloads, numeric regression budgets, and a review rubric.
- [ ] 2.2.2. Validate the compiler contract outside the repository.
  - Requires 2.1.2.
  - See polonius-ownership-experiment.md §6; developers-guide.md.
  - Success: a clean downstream build, documentation, editor, lint, and test
    matrix use explicit compatible flags; package builds do not rely on this
    repository's Cargo configuration propagating to consumers. Verify distinct
    rustc/rustdoc/editor/analyser identities and configuration; test omitted flags
    against effective defaults and explicit incompatible flags with recovery.
- [ ] 2.2.3. Record an adoption, revision, or rejection decision.
  - Requires 2.2.1 and 2.2.2.
  - See polonius-ownership-experiment.md §§5–6;
    adr-002-compiler-ownership-experiment.md.
  - Success: evidence attributes each benefit, addresses all review conditions,
    and chooses one maintained implementation; absent solver-specific benefit
    is recorded explicitly rather than reported as a compiler necessity. Justify
    policy-only solver costs against Polonius-only support; inconclusive or
    exhausted-budget results select the compatible design or a bounded extension.

## 3. Serve and explain a small resource API

Idea: if one resource can serve a request through both the in-process API and a
real socket with the same observable lifecycle, the custom engine delivers
value without a second execution path. This phase serves G1, G2, and G4.

### 3.1. Deliver a resource through the complete request lifecycle

Can a minimal application preserve routing identity, method dispatch, and error
behaviour end to end? This establishes the path later policy and body features
must reuse. See technical design §§2–7.

- [ ] 3.1.1. Implement immutable resource registration and context ownership.
  - Requires phase 2.
  - See peregrine-design.md §§2–5 and §3.1.
  - Success: route conflicts fail at build time; effective target freezing,
    the selected parameter representation and request-local extensions satisfy
    the ownership model chosen by task 2.2.3.
- [ ] 3.1.2. Implement in-process GET dispatch and response finalization.
  - Requires 3.1.1.
  - See peregrine-design.md §§3, 5, and 7.
  - Success: a synthetic request reaches the selected resource once; misses
    return 404; unsupported methods return 405 with the declared `Allow` set.
- [ ] 3.1.3. Implement the three middleware phases and semantic failure path.
  - Requires 3.1.2 and 1.2.2.
  - See peregrine-design.md §§6–7 and 11.
  - Success: generated traces match the model for every short-circuit position;
    response failures preserve diagnostics and cannot skip remaining hooks.
- [ ] 3.1.4. Complete method semantics and final HTTP response constraints.
  - Requires 3.1.3.
  - See peregrine-design.md §§4–5 and 7, invariant I5 in §11.
  - Success: explicit responders, automatic HEAD/OPTIONS, `OPTIONS *`, unknown
    methods, declaration mismatches, and no-content statuses match the contract.

### 3.2. Expose the same operation over HTTP/1.1

Does the transport preserve application semantics? This identifies differences
that in-process tests cannot expose before the example becomes a template for
other services. See technical design §§3, 5, 9, and 11.

- [ ] 3.2.1. Add the minimal Hyper adapter and runnable resource example.
  - Requires step 3.1.
  - See peregrine-design.md §§3 and 9; users-guide.md.
  - Success: an application-owned runtime and listener serve the same resource;
    the adapter uses validated connection/header limits and reports typed errors.
- [ ] 3.2.2. Add socket-level parity and protocol cases.
  - Requires 3.2.1.
  - See peregrine-design.md §§5 and 11.
  - Success: in-process and network outcomes agree for success, rejection, HEAD,
    OPTIONS, 404, 405, and 501; framing tests verify absent forbidden content.

If this slice needs a separate network routing path or obscures resource
identity, revise the architecture before adding authorization.

## 4. Apply policy from the selected resource

Idea: if a protected resource supplies its policy directly to middleware,
service authors can add authorization without synchronizing a separate policy
registry. This phase serves G1, G2, and G4.

### 4.1. Deliver a protected read/write resource

Can explicit resource policy govern method and tenant decisions without leaking
protected work into early phases? The result validates or revises the policy
API. See technical design §§4, 6, 7, and 10.

- [ ] 4.1.1. Implement typed policy declaration and enforcement registration.
  - Requires phase 3 and 1.1.3.
  - See peregrine-design.md §4; terms-of-reference.md Q3.
  - Success: protected resources without enforcement fail to build; invalid
    scope declarations fail; explicit public resources need no identity provider.
- [ ] 4.1.2. Deliver identity injection and protected-resource middleware.
  - Requires 4.1.1.
  - See peregrine-design.md §§4, 7, and 10.
  - Success: missing identity yields 401 with a challenge, denied scope yields
    403, and allowed requests invoke the responder once; dependencies are injected.
- [ ] 4.1.3. Publish a method-sensitive and tenant-aware example.
  - Requires 4.1.2.
  - See peregrine-design.md §§4–5 and 10; users-guide.md.
  - Success: read/write policy belongs to one resource, encoded identifiers are
    interpreted consistently, and no route-name policy lookup is required.

### 4.2. Verify that lifecycle interactions preserve policy

Do automatic methods, failures, and early responses respect the documented
trust boundary? This informs whether the protected example is suitable for
pilot use. See technical design §§5–7 and 10–11.

- [ ] 4.2.1. Add the end-to-end policy interaction matrix.
  - Requires step 4.1.
  - See peregrine-design.md §11, invariants I2 and I3.
  - Success: identity, method, route outcome, and failure phase combinations
    include denied HEAD/OPTIONS and absent setup state during response hooks;
    trusted early-response bypasses are explicit and documented.

## 5. Transfer bodies under explicit bounds

Idea: if body convenience and streaming share one ownership contract, typical
JSON services and large transfers can coexist without hidden whole-body
buffering. This phase serves G3 and G4.

### 5.1. Deliver bounded JSON request/response handling

Can the context expose convenient data access without ambiguous replay or
unbounded collection? The answer establishes body-state semantics before adding
long-lived transfers. See technical design §§3, 7, and 8.

- [ ] 5.1.1. Implement bounded body collection and terminal consumption states.
  - Requires step 3.1 and 1.1.2.
  - See peregrine-design.md §8 and invariant I4 in §11.
  - Success: property checks cover arbitrary frame partitions, exact limits,
    over-limit frames, repeat reads, transfer conflicts, and failed consumption.
- [ ] 5.1.2. Add fallible JSON helpers and a bounded echo example.
  - Requires 5.1.1.
  - See peregrine-design.md §§7–8; users-guide.md.
  - Success: malformed input, unsupported media type, excessive bytes, timeout,
    and serialization failure produce the specified sanitized response contracts.

### 5.2. Deliver streaming upload and download

Can a slow consumer exert backpressure without the framework accumulating the
payload? This determines whether the body abstraction is ready for realistic
transfer workloads. See technical design §§8–9.

- [ ] 5.2.1. Implement single-owner request and response streams.
  - Requires 5.1.1 and 3.2.1.
  - See peregrine-design.md §§3 and 8.
  - Success: streams preserve trailers, observe byte and idle policies, and
    release owned resources on drop; hooks cannot trigger implicit collection.
- [ ] 5.2.2. Add transfer failure and connection-reuse behaviour.
  - Requires 5.2.1.
  - See peregrine-design.md §§7–9.
  - Success: unread rejected bodies close HTTP/1.1 connections safely; failure
    after commitment ends the stream without attempting another response.
- [ ] 5.2.3. Add a streaming end-to-end backpressure suite and example.
  - Requires 5.2.2 and 5.1.2.
  - See peregrine-design.md §§8 and 11; users-guide.md.
  - Success: slow upload/download, chunked oversize, disconnect, and HEAD over
    a streamed representation demonstrate bounded framework buffering and cleanup.

## 6. Establish operational and adoption evidence

Idea: if the same resource model remains understandable under overload,
cancellation, and realistic workloads, it is ready for a bounded pilot release.
This phase serves G2, G3, and G4; completing it does not establish general
market adoption.

### 6.1. Bound execution and make shutdown observable

Can the service stop accepting work and release connections within its stated
limits? Results determine the deployment guidance and release configuration.
See technical design §§9–11.

- [ ] 6.1.1. Add request admission and supervised execution deadlines.
  - Requires steps 4.1 and 5.2.
  - See peregrine-design.md §9.
  - Success: request permits cover their documented lifetime; timeouts before
    commitment produce 504 without claiming cancelled response hooks ran.
- [ ] 6.1.2. Implement tracked graceful shutdown and bounded accept backoff.
  - Requires 6.1.1.
  - See peregrine-design.md §9.
  - Success: injected shutdown stops acceptance, drains within the configured
    deadline, cancels remaining work, and reports typed listener failures.
- [ ] 6.1.3. Add request and transfer instrumentation.
  - Requires 6.1.2.
  - See peregrine-design.md §10.
  - Success: counters, gauges, histograms, and traces distinguish finalization
    from transfer completion; labels stay bounded and sensitive data is excluded.
- [ ] 6.1.4. Add overload, disconnect, and shutdown interaction tests.
  - Requires 6.1.3 and 4.2.1.
  - See peregrine-design.md §11.
  - Success: slow streams, protected requests, and response failures under drain
    preserve documented bounds; pairwise and mandatory higher-order cases pass.

### 6.2. Decide whether the experiment merits a release

Does representative evidence justify the API and its runtime costs? A negative
result revises scope or design rather than triggering speculative optimization.
See terms of reference §§7–9 and technical design §§11–12.

- [ ] 6.2.1. Publish a reproducible workload and allocation benchmark harness.
  - Requires step 6.1.
  - See peregrine-design.md §11; terms-of-reference.md Q4.
  - Success: direct-Hyper comparisons report payload, concurrency, middleware
    depth, dependency latency, memory, allocations, and latency distributions.
- [ ] 6.2.2. Record the pilot review and compatibility decision.
  - Requires 6.2.1 and 4.1.3.
  - See terms-of-reference.md Q1–Q6; peregrine-design.md §12.
  - Success: reviewers assess endpoint/policy cohesion; maintainers record
    release budgets, supported platforms, compiler floor, and API stability policy.
- [ ] 6.2.3. Prepare the pilot release documentation and package evidence.
  - Requires 6.2.2.
  - See peregrine-design.md §§1–13; users-guide.md and developers-guide.md.
  - Success: examples build from the packaged crate; documented limitations,
    dependency choices, design status, and accepted ADRs match delivered behaviour;
    applicable repository gates pass.

## 7. Evaluate deferred extensions

Idea: after the core promise is supported by pilot evidence, extensions can be
selected by demonstrated user value without obscuring lifecycle contracts. This
phase serves G4. Tasks below are conditional investigations with go/no-go
evidence, not commitments to ship every extension.

### 7.1. Evaluate broader transport and ecosystem integration

Which missing boundary prevents a confirmed adopter from using the core? The
answer determines whether an extension deserves a separately scoped design. See
technical design §§1 and 12.

- [ ] 7.1.1. Evaluate HTTP/2, native TLS, and WebSocket adoption requirements.
  - Requires phase 6.
  - See peregrine-design.md §§1, 9, and 12; terms-of-reference.md Q2.
  - Success: each candidate has a user requirement, trust and cancellation
    analysis, and an explicit accept/defer decision before implementation planning.
- [ ] 7.1.2. Prototype an outer Tower compatibility adapter.
  - Requires phase 6.
  - See peregrine-design.md §§1–2 and 12; adr-001-resource-aware-lifecycle.md.
  - Success: readiness and backpressure are defined; a representative layer
    composes without changing inner ordering or losing resource visibility.

### 7.2. Evaluate evidence-led ergonomic and performance changes

Which measured limitation justifies extra abstraction? The answer gates changes
to public resource, policy, and body contracts. See technical design §§4 and 12.

- [ ] 7.2.1. Record decisions on optional API and execution extensions.
  - Requires 6.2.2.
  - See peregrine-design.md §§4 and 12.
  - Success: arbitrary methods, audit/rate-limit metadata, local-thread
    execution, hot route reload, and registration helpers each receive a
    documented user need or deferral.
- [ ] 7.2.2. Evaluate parameter and dispatch allocation changes.
  - Requires 6.2.1.
  - See peregrine-design.md §§3, 11, and 12.
  - Success: a measured bottleneck justifies a bounded prototype; retain it only
    if repeatable results improve without weakening lifetime or lifecycle contracts.
