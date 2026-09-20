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
