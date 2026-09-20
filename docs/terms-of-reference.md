# Peregrine Web – terms of reference

- Status: draft v0.2, reconstructed from the supplied project papers.
- Date: 2026-09-20.
- Audience: project maintainers, prospective library users, and design
  reviewers.
- Companions: [technical design](peregrine-design.md),
  [potential roadmap](roadmap.md), and
  [proposed architecture decision](adr-001-resource-aware-lifecycle.md).

## 1. Background and evidence

Peregrine Web explores whether the resource-oriented programming model of
Python's Falcon can serve developers building HTTP services in Rust. The
project brief identifies the central need: middleware must be able to inspect
the destination resource before its handler executes, while endpoint behaviour
and policy remain together.[^1] The accompanying architectural paper proposes
an implementation direction.[^2]

These are design inputs, not evidence of adoption or measured performance. No
customer interviews, commercial commitments, staffing plan, or performance
results accompany them. The repository currently provides a generated library
stub; the framework described here is proposed work.

This document distinguishes three evidence states:

- **Known:** explicitly stated in the supplied papers or current repository.
- **Assumed:** a working interpretation requiring validation.
- **Open:** an unanswered question, with a resolution path in §9.

## 2. Domain and terminology

The domain is application programming interface (API) service development.
Service authors need to connect HTTP requests to application behaviour, apply
policy before protected work occurs, and explain failures to callers and
operators. Application-specific data storage and identity systems remain the
service author's responsibility.

The following vocabulary applies across the companion documents. These entries
are candidates for a separate `context.md` if the vocabulary grows.

| Term                      | Meaning                                                                                                                         |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Resource                  | An application object grouping endpoint operations, dependencies, and declared policy. It is not necessarily a database record. |
| Responder                 | The resource operation selected for an HTTP method.                                                                             |
| Resource-aware middleware | A lifecycle participant that can inspect the matched resource before a responder executes.                                      |
| Context                   | State belonging to one request and its developing response.                                                                     |
| Short circuit             | An intentional early response that skips remaining forward processing.                                                          |
| Finalization              | The boundary that converts accumulated response state into an HTTP response.                                                    |
| Streaming                 | Incremental body production or consumption without mandatory whole-body buffering.                                              |

_Table 1: Shared project vocabulary._

**Open:** neither paper identifies regulatory obligations, deployment-specific
contracts, or a business model. Their absence is not evidence that none will
apply to applications using the framework.

## 3. Alternatives and the intended gap

**Known:** the papers name Falcon as architectural prior art and Axum, Actix
Web, and Tower-based composition as alternatives. Remaining on Falcon is also
an alternative for teams whose existing services meet their needs. Peregrine
does not require a migration merely to preserve a preferred programming style.

The proposed gap is ergonomic: endpoint identity and policy should be visible
at a defined lifecycle point without maintaining a second route-policy table.
The papers do not establish that competing frameworks cannot implement this.
The experiment must compare the clarity and maintenance cost of representative
applications, rather than claim an exclusive capability.

**Assumed:** enough service authors prefer this organization to justify another
framework and its maintenance burden. This assumption gates adoption work, not
the initial architectural experiment.

## 4. Users and stakeholders

| Group                                                     | Evidence and working context                                                 | Desired outcome and likely objection                                                                    |
| --------------------------------------------------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| Primary: Rust API service authors                         | Known target; exact team sizes and experience levels are open.               | Keep related operations and policy together; avoid compulsory framework-specific metaprogramming.       |
| Primary candidate: Falcon-experienced teams adopting Rust | Suggested by the architectural paper; actual migration demand is unverified. | Preserve a familiar lifecycle while accepting Rust ownership; avoid promises of source compatibility.   |
| Secondary: service operators and reviewers                | Assumed participants in maintaining deployed services.                       | Diagnose rejection, latency, and shutdown behaviour; avoid hidden runtime ownership.                    |
| Stakeholders: framework maintainers                       | Required by the repository, but individual decision owners are unspecified.  | Maintain a bounded API and evidence for its guarantees; avoid an open-ended platform remit.             |
| Non-target: teams seeking a complete application platform | Consistent with the brief's explicit non-goals.                              | Use a broader platform for bundled persistence, administrative interfaces, and application scaffolding. |

_Table 2: Users and stakeholders; assumptions require validation._

## 5. Jobs to be done

When adding an endpoint with several operations, a service author wants to keep
its behaviour, dependencies, and policy together, so a later change can be
reviewed without reconstructing the endpoint across unrelated registrations.

When applying authorization or tenant restrictions, a service author wants to
inspect the resolved destination before protected work executes, so policy is
based on the resource actually handling the request.

When diagnosing a rejected or failed request, a maintainer wants to follow a
defined sequence of processing and response handling, so the cause and skipped
work can be established from tests and diagnostics.

When handling a large or slow body, a service author wants incremental transfer
and explicit limits, so payload size does not silently become memory usage.

These jobs are reconstructed from the papers. Their frequency and relative
importance to prospective users remain open.

## 6. Scope

### 6.1. Goals

- **G1 – Endpoint cohesion:** support resources that group method behaviour and
  dependencies, with middleware inspecting the resolved resource before
  dispatch.
- **G2 – Explainable processing:** give normal completion, rejection, and
  recoverable failures a deterministic lifecycle with response processing.
- **G3 – Controlled resource use:** offer bounded body convenience operations
  alongside streaming, and make deployment limits and shutdown explicit.
- **G4 – Maintainable adoption:** keep a minimal service readable, test the
  lifecycle without a listener, and measure runtime costs before optimization.

G1–G4 express requirements in the brief. Their proposed acceptance evidence is
in §7; the implementation mechanisms belong in the technical design.

### 6.2. Non-goals

- Python source compatibility or a complete reproduction of Falcon's APIs.
  Existing Falcon applications require deliberate adaptation.
- A universal framework that replaces every handler or middleware model.
  Applications satisfied by an existing framework can retain it.
- A bundled database, identity provider, dependency-injection container, or
  administrative application. Integrations remain application responsibilities.
- Benchmark leadership or unconditional claims of negligible overhead.
  Performance conclusions require measured workloads.
- A macro language that owns application structure or circumvents ownership.
  Small, explicit helpers are compatible with the brief.
- Immediate support for every HTTP extension or deployment arrangement.
  The proposed initial protocol boundary is subject to Q2 below.

## 7. Success criteria

The following are proposed acceptance criteria, not reported results.

| Goal | Evidence required                                                                                                   | Acceptance signal                                                                                                                 |
| ---- | ------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| G1   | A representative protected resource with read and write operations.                                                 | Policy comes from the matched resource; denied requests never invoke its responder; no duplicate route-policy registry is needed. |
| G2   | Recorded lifecycle traces across success, early completion, missing routes, unsupported methods, and hook failures. | Every trace agrees with the design's transition rules and error precedence.                                                       |
| G3   | Slow-consumer, oversized-body, disconnect, and shutdown experiments.                                                | Configured bounds hold; streaming does not collect the complete payload; shutdown respects its documented deadline.               |
| G4   | Runnable minimal and protected-service examples, plus in-process tests.                                             | A reviewer can locate resource behaviour and policy and reproduce request outcomes without hidden setup.                          |
| G4   | Reproducible workload and allocation measurements.                                                                  | Results disclose workload, hardware, concurrency, and latency distributions; regression budgets are agreed before release.        |

_Table 3: Proposed success evidence, linked to project goals._

User-facing adoption evidence should include reviews by service authors against
their current approach. Participant count and task-completion targets are open.
Operational latency and memory budgets also remain open; invented thresholds
would not establish success. Strategic success initially means establishing
whether this architectural experiment merits continued maintenance. Revenue and
adoption targets require a sponsor decision.

## 8. Constraints, assumptions, and dependencies

### 8.1. Established constraints

The project targets Rust and retains endpoint identity, explicit lifecycle
phases, and request-scoped mutation as central requirements. Its source papers
select Hyper and Tokio as the implementation foundation; the technical design
records those choices without turning them into user outcomes.

Repository engineering rules require documented interfaces, semantic errors,
injected external dependencies in tests, and passing applicable quality gates.
The current package uses Rust edition 2024. The architectural paper's phrase
'Rust 2025' is not adopted as an edition or compiler requirement.

### 8.2. Assumptions and consequences

- Application resources can be shared safely across concurrent requests. If
  important integrations require thread-local state, the proposed runtime model
  needs revision or a separately scoped execution mode.
- Resource policy can be represented through explicit typed interfaces. If
  applications need arbitrary runtime trait discovery, the proposed contract
  will not satisfy them without additional design.
- The compiler/ownership experiment can compare one exclusively Polonius and
  new-solver implementation with a compatible control without committing to two
  supported products. If exclusivity adds no material benefit, retain the
  compatible ownership design. See
  [ADR 002](adr-002-compiler-ownership-experiment.md).
- Dynamic dispatch and future allocation are acceptable costs. If measurements
  contradict this, optimize the implementation or reconsider the API before
  claiming readiness.
- An HTTP/1.1 service behind an application-managed secure ingress is a useful
  first release. If target adopters require native encrypted or multiplexed
  serving, the release boundary must change.

### 8.3. Dependencies

Delivery depends on compatible Rust ecosystem libraries, a maintained compiler
baseline, protocol test tooling, and representative application reviewers. The
project owns its lifecycle semantics; upstream libraries cannot establish that
resource authorization or response ordering is correct. No external team's
committed deliverable is identified in the inputs.

## 9. Open questions and decision authority

Owners below are proposed roles; no individual has been assigned or consulted.

| ID  | Question and consequence                                                                             | Closure evidence                                                                            | Proposed owner                       |
| --- | ---------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | ------------------------------------ |
| Q1  | Which service authors and workload should validate the architectural benefit? Gates adoption claims. | A named pilot scenario and comparative review against its current framework.                | Project maintainer with pilot users. |
| Q2  | Is HTTP/1.1 with external secure ingress an acceptable initial boundary? Gates serving scope.        | Confirmed deployment requirements and an accepted protocol-scope decision.                  | Project maintainer with an operator. |
| Q3  | Which resource-policy interface and enforcement default should be public? Gates API stability.       | A working public/protected example and review of missing-policy behaviour.                  | API maintainer.                      |
| Q4  | What latency, allocation, memory, and connection budgets define release readiness?                   | Reproducible measurements and agreed thresholds for the pilot workload.                     | Maintainer with an operator.         |
| Q5  | What compiler posture, supported platforms, and compatibility policy can be maintained?              | The phase 2 compiler experiment, a downstream build matrix, and a published support policy. | Release maintainer.                  |
| Q6  | Who can accept the design and allocate ongoing maintenance effort?                                   | Named decision authority and explicit scope acceptance.                                     | Project sponsor or maintainer.       |

_Table 4: Open decisions and evidence required to close them._

The evidence is sufficient for a draft design and potential roadmap. API
stabilization depends on Q3 and Q5; release commitments depend on Q1, Q2, Q4,
and Q6. The [proposed ADR](adr-001-resource-aware-lifecycle.md) records the
principal architectural choice for review, without implying acceptance.

## 10. Source register

The documents are synthesized from the supplied local files, read on
2026-09-19. Source section references remain useful even if the files move. The
repository documents restate the requirements needed for implementation; access
to the Downloads directory is not required to follow the design.

[^1]: `project_peregrine.md`, supplied at
    `/mnt/d/Downloads/project_peregrine.md`. Especially §§1–5, 9–10, 13–21, and
    22: proposition, lifecycle, bodies, policy, testability, and design rules.

[^2]: _Project Peregrine: Architectural Specification for a Trait-Based,
    Middleware-Centric Web Framework in Rust_, supplied at
    `/mnt/d/Downloads/Rust Falcon Web Framework Design.md`. Especially §§2–6:
    application ownership, resources, middleware, routing, and transport.
