# Architectural decision record (ADR) 006: Gate extension-interface stabilization

## Status

Accepted as the validation policy for experimental extension interfaces. ADR
005's API shape remains proposed. No implementation, compatibility result, or
interface stabilization is implied by this decision.

## Date

2026-09-21.

## Context and problem statement

Correlation and owned upgrades must be reusable extension boundaries rather
than accommodations for two named libraries. ADR 005 and experiments X1–X5
specify ownership and compatibility, but previously left companion delivery
implicit and did not require independent evidence before stabilization.

## Decision drivers

- Deliver `peregrine-correlate` as a separate consumer of public interfaces.
- Separate Pachislot implementation from compatibility verification.
- Demonstrate removal, substitution, composition, and failure symmetry.
- Prevent compatibility with one consumer from becoming a generality claim.

## Options considered

Stabilizing after the ownership spike is cheap but demonstrates only that a
candidate API compiles. Stabilizing after Pachislot compatibility adds real
transport evidence but may preserve library-specific assumptions. Requiring
both companion libraries and an independent adapter costs more integration work
but directly tests the claimed boundary. Select the third option.

## Decision

Keep the extension interfaces experimental until all gates below pass on one
identified Peregrine revision and compiler configuration. Deliver the libraries
as separate crates consuming public Peregrine interfaces; separate repositories
or registry publication are not prerequisites. Core must not depend on either
companion or select behaviour by their concrete types or names.

The independent adapter implements a minimal WebSocket echo session directly
against the public upgrade capability. It must not depend on Pachislot or reuse
its runtime, registry, or integration glue. A shared general-purpose WebSocket
codec is permitted. Its purpose is boundary substitution, not Pachinko parity.

### Gate A: Deliver concrete consumers

- `peregrine-correlate` implements the selected correlation profile, request
  facts, response echo, and explicit outbound propagation adapters. Run X1
  fixtures, including trust, invalid/duplicate headers, generator failure,
  cancellation, and concurrent-request isolation. Declare any unsupported
  Python-specific integration instead of claiming full parity.
- Pachislot implements its entity/channel/message registry, operation traits,
  codecs, dispatch, message hooks, rooms, workers, bounded writers, and owned
  session lifecycle. Provide a runnable endpoint inside a Peregrine service.
  Then run X3 and X4 against the pinned reference and supported schema subset.
- The independent echo adapter passes real socket upgrade, message, close,
  admission, and shutdown checks through the same public interface.

### Gate B: Demonstrate generality

| Check            | Required evidence and rejection condition                                                                                                                                                                                                                                                                                      |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Removal          | Build and run the HTTP service without either companion dependency, then remove each companion independently from the combined specimen. Unrelated HTTP behaviour and the remaining consumer still work. Reject hidden initialization, feature, or shutdown dependencies on the removed library.                               |
| Substitution     | Mount the independent echo adapter in place of Pachislot using the same upgrade contract and core binary configuration. Policy, ownership transfer, admission, and shutdown remain valid without core edits or Pachislot glue. No channel-routing parity is required.                                                          |
| Composition      | Run correlation with ordinary HTTP middleware and each WebSocket adapter, including a service hosting both adapters. Selection occurs once; HTTP, connection, message, and outbound-effect facts retain their declared scopes. Reject concrete-type checks, companion-specific ordering exceptions, or leaked ambient context. |
| Failure symmetry | Apply equivalent faults to both adapters at policy rejection, handshake validation, response-hook failure, transfer failure, disconnect, cancellation, capacity exhaustion, and shutdown. Shared boundary outcomes obey the same lifecycle and accounting rules; protocol-specific message outcomes may differ explicitly.     |

_Table 1: Mandatory extension-generality checks._

Failure symmetry requires rejection before commitment to remain an HTTP
outcome, with no session started. After commitment, failures use session/close
outcomes and cannot attempt another HTTP response. Every acquired permit,
reservation, and supervisor entry must have one terminal release or transfer;
failed and cancelled paths must not leak tasks, correlation, or identity. Test
failed startup and shutdown cleanup as well as successful operation.

### Gate C: Validate ownership, compatibility, and operations

Run the full X1–X5 matrix in the
[extension design](pachislot-extension-design.md#7-compatibility-and-experimental-gates),
including compile-fail HTTP-borrow escape cases and native-socket evidence.
Retain explicit budgets for connections, messages, queues, and shutdown, plus
observed bounds under stalled peers and fan-out failures. Preserve the existing
HTTP lifecycle and the shared application-port specimen.

Use unit and behavioural regressions, generated property tests for lifecycle
and isolation invariants, and substantive Verus proofs for non-trivial pure
state-transition invariants. Proofs must apply to production-used kernels or
have a refinement connection to them. Document assumptions and proof scope; a
model that assumes the desired safety property is insufficient. Socket and
cancellation tests remain necessary for effects outside that proof boundary.

### Gate D: Make an explicit stabilization decision

Archive the consumer/core revisions, toolchains, fixture versions, commands,
traces, compatibility deviations, property-test results, proof claims and
assumptions, and resource measurements. All applicable repository quality gates
must pass. A table must map every gate above and X1–X5 to its evidence.

The designated maintainer records an accept, revise, or defer decision and
updates ADR 005 and the public design status. Missing or failing evidence
blocks stabilization; a waiver cannot be described as a passed gate. API
changes that invalidate evidence require rerunning the affected checks and
combined integration before accepting the new revision. A core HTTP pilot may
ship earlier only with these interfaces explicitly experimental or unavailable.

## Roadmap traceability

| Delivery or gate                               | Roadmap tasks |
| ---------------------------------------------- | ------------- |
| Ownership and correlation contract spike       | 1.4.1         |
| HTTP hooks and upgrade transport               | 3.1.3; 7.3.1  |
| Pachislot implementation                       | 7.3.4         |
| Pachinko and AsyncAPI compatibility            | 7.3.2         |
| Operational isolation and X5                   | 7.3.3         |
| Separate correlation consumer and X1           | 7.4.1         |
| Independent adapter and four generality checks | 7.4.2         |
| Evidence review and stabilization decision     | 7.4.3         |

_Table 2: Delivery and acceptance ownership._

## Consequences

A successful spike or compatibility demonstration cannot alone stabilize the
interfaces. Three independently integrated consumers increase maintenance and
verification work, but make special-case assumptions observable. Failed gates
should simplify or revise the boundary before adding another extension hook.
