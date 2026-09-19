# Architectural decision record (ADR) 001: Own the resource-aware lifecycle

## Status

Proposed. This record consolidates the supplied architectural direction; it is
not an assertion that implementation or maintainer acceptance has occurred.

## Date

2026-09-20.

## Context and problem statement

Peregrine's defining requirement is that middleware inspect the resource
selected by routing before its responder executes. The supplied papers also
require mutable request-scoped state and response processing after intentional
early completion or recoverable failure. See the
[terms of reference](terms-of-reference.md) §§5–7 and
[technical design](peregrine-design.md) §§1–7.

A service wrapper alone does not specify where routing becomes visible, which
response hooks run after a short circuit, or how hook failures interact. Those
contracts need an owner.

## Decision drivers

- Preserve the resource object as the source of endpoint behaviour and policy.
- Keep forward processing, short circuits, and response unwinding inspectable.
- Permit heterogeneous resources and middleware without application-wide type
  parameters or a compulsory routing macro.
- Keep the same lifecycle callable in process and through a network adapter.

## Options considered

| Option                                                 | Benefit                                                     | Cost or limitation                                                                       |
| ------------------------------------------------------ | ----------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| A framework-owned pipeline over resource trait objects | Direct resource inspection and a single lifecycle contract. | Owns orchestration, failure semantics, and dispatch allocation costs.                    |
| Compose the entire lifecycle from Tower services       | Reuses service composition machinery.                       | Still needs a design for exposing resolved resources and guaranteeing phase semantics.   |
| Generate a statically dispatched application           | Can reduce dynamic dispatch and allocation.                 | Adds code generation and couples heterogeneous application structure to generated types. |

_Table 1: Alternatives for owning the inner lifecycle._

## Proposed direction

Use a framework-owned pipeline with immutable shared application configuration,
shared resource and middleware objects, and an exclusively borrowed mutable
context per request. Use explicit typed resource metadata; do not depend on
runtime discovery of arbitrary marker traits.

Run request and resource hooks in registration order. For a pipeline future
that runs to completion, invoke every registered response hook once in reverse
order, including hooks whose forward phase did not run. A response hook must
therefore tolerate absent request state. Cancellation and panics do not receive
an asynchronous cleanup guarantee.

Keep Hyper and Tokio at the serving boundary. Preserve an outer adapter point
for Tower; such an adapter must not replace resource-aware processing. Start
with boxed `Send` futures for dynamic async dispatch, measuring their cost. The
detailed proposed contracts, including error precedence and the commitment
boundary for streamed responses, are in the technical design.

## Known risks and limitations

Independent response hooks are simpler to enumerate but require defensive
handling of absent setup state. Dynamic dispatch and boxed futures impose
runtime costs. A custom engine also carries a compatibility and maintenance
burden that cannot be justified by a microbenchmark alone.

In-process middleware is trusted application code. Resource metadata does not
provide authorization unless an enforcement component checks it. Network
cancellation can interrupt asynchronous work and is not a transactional
rollback.

## Outstanding decisions

Acceptance requires a resource-policy API spike, complete lifecycle traces for
failure cases, and agreement on the initial transport boundary. Record compiler
and dependency choices alongside the spike. Revisit this decision if a
representative application needs duplicate route policy or a second internal
lifecycle to implement its requirements.
