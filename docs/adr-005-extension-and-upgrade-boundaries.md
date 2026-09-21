# Architectural decision record (ADR) 005: Host owned protocol extensions

## Status

Proposed. Pachislot is a named consumer requirement; the APIs and compatibility
claims remain experimental. No runtime or dependency is added by this record.

## Date

2026-09-21.

## Context and problem statement

The requested Pachislot companion should reproduce falcon-pachinko capabilities
inside a Peregrine service, with named AsyncAPI channels, typed messages, and
operation traits. falcon-correlate supplies a second concrete extension model:
request correlation, logging, and downstream propagation.

The [extension design](pachislot-extension-design.md) inspects pinned source
revisions and primary protocol documentation. Peregrine already proposes
immutable request facts, finalization, application-owned services, and shutdown
tracking, but explicitly defers upgrades and rejects informational statuses as
ordinary final responses. An extension cannot supply a sound WebSocket runtime
by bypassing those contracts.

## Decision drivers

- Keep correlation selection single-owned and preserve configurable semantics.
- Host an endpoint inside the existing service and admission-policy boundary.
- End the HTTP lifecycle at protocol commitment without losing task ownership.
- Preserve Pachinko wire modes and distinguish observed from intended behaviour.
- Map AsyncAPI identities explicitly without confusing metadata and Rust traits.

## Options considered

| Option                                             | Benefit                                                               | Cost                                                                                                 |
| -------------------------------------------------- | --------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Run a separate WebSocket process                   | Independent deployment and resource limits.                           | New transport, authentication, service-access, and operational contracts; not the requested default. |
| Put sockets and messages in HTTP middleware        | Reuses familiar hook names.                                           | Retains invalid request lifetimes and conflates HTTP completion with connection/message processing.  |
| Add a typed upgrade boundary and companion runtime | Shared listener and resource policy with explicit ownership transfer. | Requires a final-outcome variant, handshake-aware hooks, and upgraded-session accounting.            |

_Table 1: Hosting alternatives._

## Proposed direction

Select the third option, treating sidecar as an in-process companion library.
Peregrine owns a protocol upgrade capability independent of Pachislot. A
Pachislot integration adapter mounts channel endpoints through this boundary;
Peregrine core does not interpret AsyncAPI or own a room registry.

Configure correlation selection before request hooks. Preserve the
falcon-correlate profile's generator, optional validator, trusted-source rules,
and echo switch, subject to explicit bounded/header-safe validation. Outbound
HTTP and job propagation are separate adapters. Owned values and future-scoped
instrumentation replace reliance on response-hook cleanup of ambient state.

An explicitly registered upgrade endpoint shares resource metadata and policy,
but has its own method/handshake contract. It cannot inherit automatic HEAD
upgrade behaviour. An engine-created `UpgradePlan` is mutually exclusive with
an ordinary response. Response hooks complete before commitment; rejection,
replacement, and hook failure invalidate the plan. Only successful finalization
can emit a 101. The supervisor owns the pending hand-off before that response
is returned; the session starts only after the transport resolves the upgrade.

The one-shot plan transfers owned I/O, immutable connection facts, and session
ownership. It never transfers a borrowed HTTP context. Session permits and
shutdown tracking outlive HTTP execution permits. Failures after commitment
produce message/close outcomes, not HTTP errors. The application coordinates
worker shutdown and adapter cleanup with the session runtime.

Pachislot's frozen registry maps channel identity/address, message
identity/wire tag, and operation identity/direction to typed entity
capabilities. Start with one channel per WebSocket connection under the
standard binding. Multiplexing is a separately versioned protocol choice.
AsyncAPI operation traits remain metadata merged by specification rules; Rust
receive/emission traits implement behaviour and map to explicit operation
descriptors. Runtime authorization binds the metadata to application policy
rather than treating documentation as enforcement.

## Consequences and verification

A shared endpoint factory and an owned per-connection session have different
lifetimes and concurrency bounds. Keep per-message mutable state in the
session, with bounded writers and explicit application-context projection.
Preserve Pachinko hook-layer ordering and codec behaviour through a named
compatibility profile; do not copy Peregrine's independent response-hook rules
onto messages. Compatibility does not require Python decorators, app
monkey-patching, or a service container.

Roadmap task 1.4.1 fixes the correlation profile and compiles the ownership
seam before API stabilization. Tasks 7.3.1–7.3.3 test transport hand-off,
compatibility and AsyncAPI coherence, then operational isolation. The extension
design names experiments X1–X5, including negative ownership cases and
differential client traces. Native sockets remain necessary to establish
handshake and close behaviour; simulators alone are insufficient.

Exact response/close mappings, budgets, publication layout, supported schema
subset, and the compatibility baseline require experimental closure. HTTP/2
extended CONNECT and a distributed connection backend remain separate work.
Polonius/new-solver adoption remains independent: neither mechanism makes an
HTTP borrow safe to retain in a long-lived upgraded session.
