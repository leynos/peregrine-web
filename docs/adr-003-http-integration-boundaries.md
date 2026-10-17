# Architectural decision record (ADR) 003: Bound HTTP integration responsibilities

## Status

Proposed. This record refines the unimplemented framework design; it does not
adopt actix-v2a, change compiler requirements, or publish runtime APIs.

## Date

2026-09-20.

## Context and problem statement

The [actix-v2a case study](actix-v2a-middleware-case-study.md) compares
existing error, key, pagination, SSE, and schema helpers with proposed shared
mutation contracts. It finds an advantage in resource-aware policy and
consistent HTTP presentation, but no justification for moving durable mutations
or wire algorithms into middleware.

The existing design already has phase views, private failure state,
finalization, and owned body streams. The implementation sweep finds only the
library stub. The question is how to refine those boundaries without replacing
compact helpers with another scheduling or dependency-injection system.

## Decision drivers

- Keep resource identity and policy visible before dispatch and replay.
- Preserve explicit endpoint inputs and semantic errors.
- Render the final failure without collecting arbitrary response bodies.
- Distinguish durable operation outcomes from HTTP and transfer outcomes.
- Retain independent response hooks without pretending they always had setup.

## Options considered

| Option                                                                      | Advantage                                                       | Cost or failure mode                                                                              |
| --------------------------------------------------------------------------- | --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| Put every extension in lifecycle hooks                                      | One apparent integration mechanism.                             | Hidden inputs, ordering dependencies, body consumption, and false transaction/cleanup guarantees. |
| Leave all integration to each responder                                     | Visible ordinary control flow.                                  | Repeated correlation/error wiring and inconsistent coverage of framework-generated failures.      |
| Keep typed helpers and application services, with engine-owned finalization | Explicit domain boundaries and consistent transport invariants. | Requires small parser, request-fact, and renderer contracts with compatibility tests.             |

_Table 1: Alternatives for HTTP extension placement._

## Proposed direction

Select the third option and refine the existing design as follows:

1. Metadata parsers consume borrowed request views and preserve repeated header
   values. Required and optional single-value parsing are separate operations;
   a parser's own contract handles empty values. They never consume the body.
   Applications call them directly or through an optional typed responder
   adapter. Body decoding and domain normalization remain explicit.
2. The engine initializes read-only request facts before hooks: correlation and
   timing, with optional route facts frozen after routing. Correlation policy
   and generation are injected. Authentication is separate. These facts survive
   normal hook failure and are retained sufficiently for supervised timeouts.
3. A configured synchronous public-failure renderer receives the final selected
   public status, stable reason, bounded safe details, and request facts. It
   returns a bounded representation, not a new failure classification. The
   engine preserves private causes, applies required headers, and enforces
   final HTTP constraints. Rendering failure uses the static fallback.
4. Installing/replacing a response body owns its representation metadata. An
   optional SSE helper sets event-stream headers with the body; replacing it
   with an error clears stale stream metadata. Hooks never infer a body kind by
   collecting it or parsing its contents.
5. The engine records finalized HTTP outcomes; the body owner records transfer
   completion; application services record mutation outcomes. There is no
   asynchronous post-commit middleware phase.

Parser details are value-level contracts, not a requirement for a registry or
new extractor ecosystem. The optional typed adapter uses the existing endpoint
invocation boundary; it must preserve policy order, failure conversion, and
`Send` requirements. Syntax and generic bounds remain subject to the ownership
prototype in roadmap phase 2.

The core provides no durable claim store, transaction hook, effect retry, or
universal replay snapshot. A mutation service owns scoped identity, normalized
fingerprints, claim ownership, completion, and uncertain-outcome recovery. An
HTTP adapter maps its typed results. A successful effect remains successful
even when a later response hook fails; cancellation never proves non-execution.

## Ownership and reuse

| Contract                   | Owner and permitted callers                                              | Composition limit                                                                  |
| -------------------------- | ------------------------------------------------------------------------ | ---------------------------------------------------------------------------------- |
| Metadata parser            | Feature/application helper; resource or typed endpoint adapter calls it. | No body read, storage access, or implicit authentication.                          |
| Request facts              | Engine; hooks, renderer, and supervisor read restricted views.           | No generic mutable state bag; correlation grants no authority.                     |
| Public-failure renderer    | Application configures one default; engine invokes it after hooks.       | No domain effects, retry decisions, stream inspection, or status reclassification. |
| Body representation helper | Response/body feature; resource installs it explicitly.                  | No implied stream scheduling, retention, or replay-store access.                   |
| Mutation service           | Application; callable by HTTP and non-HTTP callers.                      | No dependence on response-hook execution or HTTP snapshot shape.                   |

_Table 2: Proposed reuse boundaries; implementation must repeat the helper
sweep._

## Compatibility and verification

Do not treat actix-v2a's proposed PR #92 types as available dependencies. Its
existing transport-independent modules live in a package with an unconditional
Actix dependency; reuse or extraction needs an explicit packaging decision.

A compatibility fixture must preserve selected statuses, code/message/trace
fields, validation paths, pagination tokens and limits, and SSE bytes, or
record an explicit versioned replacement. Consumer-specific envelopes remain
consumer contracts. These rules do not force all applications to use one JSON
shape.

Required evidence includes missing/duplicate/malformed header distinctions, key
propagation into a service, authorization before replay, error rendering after
the last hook failure, correlation on ordinary early exits and 504, JSON
rejection of an SSE request, and mutation success followed by HTTP failure.
Separate durable adapter tests establish mutation atomicity and recovery.

## Outstanding decisions

Roadmap tasks 1.1.2 and 2.1.2 settle request-fact and typed-input signatures.
Task 3.1.3 implements renderer/finalizer integration, task 4.2.1 demonstrates
policy/replay separation, task 5.2.3 tests representation replacement, and task
6.1.3 distinguishes operation and transport telemetry. Required budgets include
correlation syntax/length, error-detail count/bytes, and rendering output size.
SSE convenience packaging and reuse of actix-v2a remain optional until a pilot
requires them. No runtime performance or compiler-specific benefit is inferred.

## Pilot responsibility and budget decisions

The owner accepted Mornington as the proving ground and the HTTP/1.1/Traefik
boundary on 2026-10-13;
[ToR §9.1](terms-of-reference.md#91-mornington-pilot-and-comparative-review)
records that evidence. This acceptance does not promote this whole ADR or the
uncompiled ownership API to Accepted. ADR 001 still requires its spike/traces;
ADR 006 still requires separate consumers and independent extension evidence.

| Responsibility                                                 | Owner                                            | Boundary and permitted composition                                                                                   |
| -------------------------------------------------------------- | ------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------- |
| Immutable request facts and frozen resource identity           | Engine                                           | Initialize once; hooks/renderer/supervisor receive restricted views; no correlation-based authentication             |
| Forward stop, reverse-hook cursor, failure selection           | Engine                                           | L01–L38/C01–C13 in design §6 and E01–E09 in §7.2 are canonical; no second lifecycle registry                         |
| Renderer invocation, typed required headers, HTTP finalization | Engine                                           | Application supplies safe mappings/renderer; H01–H06 apply once; custom code does not own final status               |
| Parser, connection admission, trusted ingress metadata         | Transport adapter                                | Apply CFG01/CFG04–CFG07/CFG30 before the corresponding boundary; parser exits have no context/hook/ID guarantee      |
| Streaming input/output and transfer accounting                 | Body owner, integrated with transport supervisor | Retain request-input policy after ownership transfer; committed failures terminate transfer and never replace status |
| Identity, resource policy, domain mappings, durable effects    | Application                                      | Mornington owns grants/revocation/idempotency/storage; HTTP and non-HTTP drivers use the same application ports      |
| Runtime, generators, clocks, shutdown and application cleanup  | Composition root                                 | Inject dependencies and initialize subscribers once; cleanup and external-task cancellation have their own evidence  |

_Table 3: Responsibility split without a service container or extraction
scheduler. Canonical contract and configuration values remain in the design._

Design §9.2 contains the proving-ground defaults approved on 2026-10-17 and
their workload rationale. This accepts CFG01–CFG33 for the exercise only, not
for 1.0; each row requires evidence-backed review afterwards before 1.0
parameter acceptance. Mornington's hexagonal ports provide direct-domain and
paired driving-adapter validation, as recorded in
[ToR §9.2](terms-of-reference.md#92-proving-ground-approval-and-validation-boundaries).
Application ports remain HTTP-independent under ADR 004. Choose finite limits
over implicit unlimited settings, explicit stream opt-out over sentinel zero,
and static fallback over re-entering custom code during cancellation. Request
execution releases its permit at handoff; streamed bodies and connections
retain their distinct accounting until release. This permits long polling
without pretending that finalized streams disappeared from load.

The renderer cap limits accepted encoded output, and a bounded writer limits
its own accumulation. It cannot limit arbitrary renderer allocations or
synchronous running time. Secondary-diagnostic caps limit retained encoded
details, not primary error payloads or logging-backend queues. Timeouts are
cooperative; blocking trusted code can prevent their timely observation.
Response hooks provide neither durable effects nor asynchronous cancellation
cleanup.

These decisions refine existing owners rather than introducing a new port or
helper. Reuse follows the table's permitted callers and ADR 004; custom
protocol sessions continue to use ADR 005's separate ownership.
StrictHeaderSafe is the pilot's only selectable correlation profile. The
separate Falcon-compatible profile is deferred until X1 specifies its
behaviour; the pilot does not silently redefine that profile or claim X1
compatibility. No upgrade delivery or stabilization follows from Mornington's
no-upgrades boundary.
