# Architectural decision record (ADR) 004: Preserve application port boundaries

## Status

Proposed. This refines the unimplemented API and example requirements; it does
not introduce runtime dependencies or require consumer migrations.

## Date

2026-09-20.

## Context and problem statement

The [hexagonal application case study](hexagonal-application-case-study.md)
examines Corbusier's typed tenant context and service façades, Wildside's
command/query injection and architecture checks, and Episodic's Falcon
resources, unit-of-work helpers, and separate runtime roots. Constructor
injection fits the resource model, but resources alone do not prevent broad
state bundles, elaborate adapter helpers, or transport-dependent use cases.

A sweep of Peregrine finds the existing proposed resource dependencies, request
facts, operation services, and serving boundary; implementation remains a
library stub. These existing boundaries should be refined rather than adding a
service container or a second middleware orchestration language.

## Decision drivers

- Keep application ports usable from HTTP and non-HTTP entrypoints.
- Make each resource's dependencies explicit at construction.
- Preserve application authority, tenant isolation, and transaction semantics.
- Avoid duplicate route classification for policy and bounded telemetry.
- Expose server completion without pretending to own application lifespan.

## Options considered

| Option                                                                  | Advantage                                                        | Cost                                                                                                  |
| ----------------------------------------------------------------------- | ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| A universal state/context and transaction middleware                    | Uniform integration point.                                       | Hides dependencies, couples use cases to HTTP, and confuses response completion with durable effects. |
| Leave every boundary to consumer convention                             | Minimal framework surface.                                       | Easy to copy unsafe lifecycle examples or accidentally require framework types in ports.              |
| Explicit resource dependencies and tested application-boundary examples | Small framework surface with demonstrable integration contracts. | Requires representative consumer fixtures and precise ownership documentation.                        |

_Table 1: Application integration options._

## Proposed direction

Select the third option. A resource is an inbound adapter, not a domain entity.
It owns narrow service dependencies supplied by an application composition
root. Registration accepts resources containing concrete services or
application-owned dyn-compatible ports; the router's resource erasure does not
force service erasure. Existing shared-state bundles remain permitted. The
framework neither discovers services nor creates them during requests.

An adapter maps verified identity and parsed transport values into the
application's own context and commands. Application ports do not require
Peregrine, HTTP, serialization, or a universal framework context trait.
Domain-to-wire projection and error mapping stay in the HTTP adapter. Tenant
and object authorization remain enforceable by non-HTTP callers. Resource
policy is complementary admission control, not a replacement.

An application service or use-case wrapper owns each unit-of-work lifetime and
its outcome rules. Resources may store factories, but middleware never commits
on HTTP success and a shared resource never holds one mutable transaction for
all requests. This is a target boundary; Episodic's inspected shared handlers
currently enter unit-of-work scopes in the HTTP adapter.

Extend frozen route facts with optional method-specific operation labels from a
bounded startup declaration. Resources declare these alongside policy;
registration validates the name budget and mapping. Labels do not grant
authority. Final HTTP instrumentation reads the selected operation without
parsing raw paths. Unknown routes, pre-routing failure, and unsupported methods
use fixed fallbacks. HEAD labels follow the selected explicit HEAD responder or
the GET fallback; authentication and policy still evaluate the actual request
method. Reject the feature if it merely adds a second policy registry.

Application roots own startup and asynchronous cleanup. Peregrine exposes its
serving/draining completion and remaining-work outcome; it does not close
injected pools or supervise durable jobs implicitly. Application cleanup must
account for remaining requests, streams, and application work before releasing
dependencies. Startup failure requires partial-construction cleanup. These
contracts do not extend request middleware with lifespan methods.

## Ownership and reuse

| Contract                       | Owner and callers                                                | Limit                                                                       |
| ------------------------------ | ---------------------------------------------------------------- | --------------------------------------------------------------------------- |
| Driving port and command       | Application; all authorized inbound adapters.                    | No framework request/response types or infrastructure dependency.           |
| Identity-to-context projection | Application HTTP adapter; invoked explicitly after verification. | Correlation and untrusted tenant headers cannot establish authority.        |
| Resource dependency bundle     | Application composition root constructs; resource reads.         | No global mutable service registry or implicit fixture fallback.            |
| Operation label                | Resource declaration; router freezes, telemetry reads.           | Finite configured values; not authorization or a business outcome.          |
| Unit-of-work factory and scope | Application use-case wrapper/service.                            | No response-hook commit or rollback guarantee on cancellation.              |
| Adapter lifecycle              | Application root; server reports its own completion.             | No guarantee that external jobs stopped or asynchronous cleanup always ran. |

_Table 2: Boundaries and permitted reuse._

## Consequences and verification

Keep concrete and erased service options in the same bounded fixture, on both
compiler variants. Polonius does not remove `Send`, `Sync`, or
dyn-compatibility requirements. Native async methods need an appropriate
erasure strategy before use through a trait object.

Roadmap step 1.3 compares wiring and invokes one protected use case through
HTTP and a non-HTTP driver. Negative dependency fixtures reject framework
leakage and forbidden adapter imports while permitting composition roots.
Behavioural port tests remain separate from structural checks; adopt no Python
architecture checker as a Rust dependency on the strength of this study.

Tasks 3.1.1 and 6.1.3 test operation metadata, while 6.1.2 and 6.1.4 establish
partial startup, drain, and eligible cleanup behaviour in an application
example. The case study records normative axioms A1–A4, assumptions S1–S4, and
hypotheses H1–H5 with explicit rejection conditions. No benchmark or consumer
migration benefit is considered established until those experiments run.
