# Case study: Hexagonal applications and Peregrine

- Status: design research; proposed improvements, not implemented APIs.
- Date: 2026-09-20.
- Scope: Corbusier, Wildside, and Episodic as consumers of an HTTP adapter.

Peregrine can make ports and adapters easier to maintain by making each HTTP
resource a small, explicitly constructed adapter around an application service.
Its strongest contribution is local dependency and operation-policy visibility.
It cannot supply application boundaries merely by changing the borrow checker
or moving orchestration into middleware. The recommendations refine the
[technical design](peregrine-design.md), [roadmap](roadmap.md), and proposed
[ADR 004](adr-004-application-port-boundaries.md).

## 1. Evidence and limits

The following immutable source snapshots were inspected. Wildside and Episodic
were fetched and unpacked separately because their local working checkouts
lagged their remote default branches. Existing working files were untouched.

| Application | Inspected commit                           | Relevant implementation                                                                                      |
| ----------- | ------------------------------------------ | ------------------------------------------------------------------------------------------------------------ |
| Corbusier   | `19e1dafd2eee2af7dbcf12320dbc9a56b2b8258a` | Actix HTTP façade, typed request context, generic task service, tenant-scoped repository port.               |
| Wildside    | `095178bd8ce8c35b0854275b710542d75e0ae12e` | Actix command/query ports, state assembly, annotation adapters, module-boundary lint.                        |
| Episodic    | `0740f26ea59ff2f56108cc137833004af09a00d8` | Falcon ASGI resources, injected dependencies, unit-of-work helpers, runtime roots, architecture enforcement. |

_Table 1: Source snapshots; citations below resolve to these commits._

This is a source review, not a migration or benchmark. Consumer test suites
were not run. Examples describe candidate Peregrine APIs; they are not evidence
that a framework exists. Existing Rust services and Python resources illustrate
patterns, not equivalent workloads or language-performance comparisons.

## 2. Corbusier: Keep tenant context and services explicit

### 2.1. Observed implementation

`create_task` receives Actix application data, a `TaskMutationContext`, and a
JSON body. It builds an application request, calls
`state.tasks.create_task(auth.context(), request)`, and maps the result into an
HTTP envelope. `ApiState` owns three application façades behind `Arc<dyn ...>`,
an authenticator, and a clock. `TaskApplication` is an HTTP-adapter façade that
forwards to task services.[^1]

`TaskLifecycleService<R, C>` is generic over a `TaskRepository` and clock. Its
create operation constructs a task and calls `repository.store(ctx, &task)`.
The repository port explicitly requires tenant-scoped operations. The owned
application `RequestContext` carries tenant, user, session, correlation, and
optional causation identifiers. Authentication constructs that context; it is
not simply a collection of untrusted HTTP headers.[^2]

The executable constructs PostgreSQL adapters and services before injecting
`ApiState`. This composition root deliberately sees concrete infrastructure;
ordinary HTTP handlers need not.[^3]

### 2.2. Consequence for Peregrine

A `TasksResource<S>` can retain a concrete service or an application-owned
service port in `self`. Registration should erase the resource only at the
heterogeneous router boundary. It must not require an additional HTTP-only
façade solely to erase a generic service. Existing façade traits remain useful
when they provide a deliberate testing or application contract; their removal
is an experiment, not an automatic improvement.

Actix already permits resource-local dependency injection, as established in
[the first case study](actix-v2a-middleware-case-study.md#1-finding-and-evidence-boundary).
Falcon already constructs resources with dependencies. The proposed benefit is
clearer conventions and a smaller change surface, not exclusive expressiveness.

Peregrine's `RequestFacts` must not replace Corbusier's `RequestContext`. An
adapter explicitly maps verified identity and validated inputs to the
application's context. Correlation supplies observability, not tenant
authority. Object-level and tenant authorization belong in the use case or its
application-owned policy collaborator so that a worker or command-line caller
cannot bypass them by avoiding HTTP middleware.

A resource is an inbound HTTP adapter, not a domain entity. Grouping task HTTP
operations does not justify moving task invariants, persistence, or domain
serialization into the framework's resource trait.

## 3. Wildside: Narrow dependencies without a workflow language

### 3.1. Observed implementation

`HttpState` contains sixteen command/query dependencies. `HttpStatePorts` and
`HttpStateExtraPorts` package construction, while handlers access the shared
bundle. This already separates inbound code from concrete persistence. The
state builder constructs services when a pool is supplied and otherwise selects
fixture implementations; that is application assembly policy.[^4]

The annotation adapter constructs `RouteMutationContext` from a required
session user, route identifier, and optional idempotency key. Its shared
`handle_route_mutation` accepts a `RouteMutationSpec` containing parsing,
request construction, service invocation, and response mapping functions. The
helper has ten type parameters including the returned future. This buys reuse
but also demonstrates the cost of generalizing a short adapter flow.[^5]

Wildside enforces module dependency direction with a repository-local Rust
syntax checker. This matters because a single crate cannot express every module
boundary through Cargo dependencies. Its rules prohibit inbound code from
importing outbound adapters and domain code from importing framework or
infrastructure dependencies.[^6]

### 3.2. Consequence for Peregrine

An `AnnotationsResource` can own just annotation command/query ports. Other
resources can own different small bundles without changing a global HTTP state
constructor. A shared application bundle is still allowed when useful;
Peregrine must not impose one universal state type or runtime service lookup.
Required dependencies are constructor arguments, not optional request
extensions whose absence becomes a late 500.

The responder should normally show four ordinary operations: parse, construct
an application command, await a service, and project the result. Middleware
should not implement those stages through a second generic workflow language.
The optional typed responder adapter from ADR 003 must earn its complexity
against this straightforward baseline.

Fixture selection belongs in an explicit application startup mode, never a
framework fallback for a missing dependency. Likewise, architecture checking is
a consumer development tool, not a request-processing feature. Peregrine should
provide a small example and negative dependency fixtures, without mandating a
particular directory layout or copying Wildside's entire checker.

## 4. Episodic: Falcon proves both the opportunity and the limits

### 4.1. Observed implementation

Episodic uses Falcon ASGI. `create_app(ApiDependencies)` receives an explicit
frozen dependency bundle and registers resource instances. Runtime assembly
creates the SQLAlchemy engine, a unit-of-work factory, readiness checks, and
cleanup functions. The API runtime and Celery worker runtime are distinct
composition roots.[^7]

Resource base classes share GET, history, creation, and update flows through
abstract hooks. The shared creation helper validates required fields, enters
`async with uow_factory()`, calls a service, and serializes its result. This is
an adapter-owned unit-of-work lifetime in the inspected helper; it would be
incorrect to describe every existing Episodic transaction as already owned by
an application service.[^8]

Authorization middleware selects `/v1/` paths, calls an injected authorization
port, and stores the returned principal in request context. Generation metrics
separately classify paths using prefixes and suffixes. Shutdown is represented
by an ASGI lifecycle middleware object; application construction contains a
cast because the exported middleware type does not cover its shutdown-only
shape. These are concrete integration seams, not evidence that Falcon lacks
resource hooks or application lifespan support.[^9]

Episodic's accepted ADR 014 configures Hecate to check Python imports between
domain/ports, application, inbound, outbound, and composition-root groups.
Composition roots are explicit exceptions to adapter isolation. The ADR also
states that runtime-checkable protocols establish structural presence, not
behavioural conformance; deeper orchestration checks remain future work.[^10]

### 4.2. Consequence for Peregrine

Resource construction is already a good fit for hexagonal applications.
Peregrine should preserve that strength without recreating a base-resource
hierarchy. Rust composition can use ordinary functions for parsing and
projection, with a small service dependency for each use case.

Falcon could also use its resource hook and application-defined resource
metadata to remove those path tables. Peregrine's proposed advantage is a
validated, typed convention and final-outcome integration, not an otherwise
unavailable architectural capability.

An immutable, method-specific operation label beside resource policy can remove
the second path-classification table used for generation telemetry. It is
application-supplied bounded metadata, frozen during route resolution, and read
by the engine's final-outcome instrumentation. Authentication still runs before
protected dispatch. A public health route requires an explicit policy;
resource-aware policy cannot infer equivalent behaviour from a path prefix
alone. Unknown routes and pre-routing failures have fixed fallback labels, and
method-not-allowed requests must not invent a selected operation.

A worker needs the same business transaction semantics as HTTP. Moving an
adapter-owned unit-of-work lifetime into an application use-case wrapper is a
candidate application refactoring, not an automatic Peregrine capability. The
wrapper owns factory invocation and success/failure semantics. Middleware must
not start a transaction and commit it because the final HTTP status is
successful. A unit of work is per operation; a shared resource stores a factory
or service, never one mutable transaction for all requests.

Application lifespan should be separate from request middleware. An executable
can own initialized services and explicit asynchronous cleanup, then await
Peregrine's serving/draining completion before closing dependencies still used
by requests or streams. It needs no shutdown-only implementation of the request
middleware trait. Worker shutdown and external durable-job semantics remain
application concerns.

## 5. Proposed adapter shape and its trade-offs

The following is a schematic responder body, not a compiled public API. Types
such as `TaskCommand`, `VerifiedIdentity`, and `TaskReply` belong to the
example application; names do not introduce framework traits.

```rust
struct TasksResource<S: ?Sized> {
    tasks: Arc<S>,
}

// Within a resource responder, after the resource-policy phase:
let context = application_context(verified_identity, input.facts())?;
let payload = decode_create_task(body).await?;
let command = TaskCommand::try_from(payload)?;
let task = self.tasks.create(&context, command).await.map_err(map_task_error)?;
response.set_json(TaskReply::from(task))?;
```

The service receives owned application values and an application context, not
`Context`, `RequestView`, HTTP headers, or a mutable response. The response
projection remains in the HTTP adapter. A worker can construct the same command
and context from its own trusted inputs and invoke the same service. This does
not imply that background jobs may retain a borrowed request or reuse an HTTP
principal without validating their own authority.

Two service-storage options should work without changing the lifecycle:

| Choice                                                                     | Benefit                                                                                        | Cost and condition                                                                                      |
| -------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `TasksResource<ConcreteTaskService>` containing `Arc<ConcreteTaskService>` | Keeps service calls statically dispatched; can avoid a forwarding façade.                      | More generic instantiations; the concrete service must satisfy resource and future bounds.              |
| `TasksResource<dyn TaskCommands>` containing `Arc<dyn TaskCommands>`       | Small application-owned substitution boundary, useful for tests and alternate implementations. | Port must be dyn-compatible, with an explicit async erasure strategy and `Send` futures where required. |

_Table 2: Application service storage is independent of router type erasure._

Neither option makes native async trait methods automatically dyn-compatible.
Neither Polonius nor the new solver supplies missing `Send`, `Sync`, ownership,
or object-safety guarantees. Both compiler prototypes must use the same
application boundary and async-erasure strategy when measuring compiler gains.

| Concern                       | Better fit for Peregrine                                                 | Where middleware becomes unnecessary pain                                                       |
| ----------------------------- | ------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------- |
| Dependencies                  | Constructor-owned resource services and explicit startup validation.     | Service lookup through mutable context hides missing dependencies and broadens access.          |
| Policy and operation identity | Resource metadata reused after route resolution.                         | A global hook that loads domain aggregates becomes a second application service.                |
| Input and output              | Adapter-local parsers, commands, and response projections.               | Hooks that secretly consume bodies or require domain `Serialize` implementations couple layers. |
| Transactions                  | An application service or use-case wrapper owns each unit of work.       | Response status is not a commit signal; cancellation bypasses response hooks.                   |
| Startup and shutdown          | Application root owns adapters; server exposes bounded drain completion. | Request hooks cannot guarantee asynchronous resource cleanup or durable job completion.         |
| Architectural discipline      | Dependency checks plus behavioural port contracts.                       | Runtime middleware cannot enforce compile-time module direction.                                |

_Table 3: Benefits and limits relative to the inspected consumers._

## 6. Axioms, assumptions, and falsifiable hypotheses

### 6.1. Chosen axioms

These are normative design constraints, not discoveries proved by the review.

- A1: Application ports and business values do not depend on Peregrine or HTTP
  types. The composition root may depend on concrete inbound and outbound
  adapters; ordinary inbound adapters may not depend on outbound adapters.
- A2: Resource policy is an HTTP admission check. Application authorization,
  tenant isolation, transaction outcomes, and retry semantics remain valid
  through every supported entrypoint.
- A3: Response completion, durable operation completion, body transfer, and
  application shutdown are distinct events with distinct owners.
- A4: Framework convenience must preserve visible dependencies and ordinary
  application control flow. It must not require a service container, generic
  repository, universal application-context trait, or transaction middleware.

### 6.2. Assumptions to validate

- S1: Representative application services can meet Peregrine's shared-resource
  and `Send` future bounds. A service requiring thread-local state may need a
  separate adapter; the proposal does not promise a local-thread mode.
- S2: One bounded Rust specimen can represent the relevant dependency and
  transaction boundaries of all three applications. It cannot establish that
  every consumer can migrate without broader work.
- S3: Operation names can be supplied from a finite startup declaration. The
  name budget and method mapping need configuration validation; caller-supplied
  paths or arbitrary runtime strings are unsuitable metric labels.
- S4: The application can stop or account for dependent background work before
  closing its adapters. Peregrine can report its own drain outcome, not prove
  that every external worker has stopped.

### 6.3. Experiments and decision rules

The experiments are architecture tests independent of compiler adoption. They
remain planned, with no pass claimed by this document.

| Hypothesis                                                                  | Experiment and evidence                                                                                                                                                                                                                                                                            | Rejection or revision condition                                                                                                                         | Roadmap      |
| --------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ |
| H1: Resource-local injection reduces wiring without obscuring dependencies. | E1: implement the same task/annotation specimen with concrete and dyn service storage; compare against a fair Actix façade and Falcon-style resource baseline, counting forwarding layers and files changed when adding one port. Compile heterogeneous registration and negative ownership cases. | Savings disappear, constructor errors worsen, or the design requires an extra universal façade/container. Retain the simpler explicit service boundary. | 1.3.1; 2.1.2 |
| H2: HTTP adaptation can remain outside the application contract.            | E2: invoke one protected mutation through HTTP and a non-HTTP test driver using the same use case; reject a mismatched tenant, map typed failures, and verify one unit-of-work lifetime per invocation. Build the application fixture without Peregrine/HTTP dependencies.                         | Any use case needs HTTP context, HTTP success controls commit, or the second driver bypasses authorization. Redesign the boundary.                      | 1.3.2; 4.2.1 |
| H3: Co-located operation metadata removes duplicate path classification.    | E3: rename a generation-style route while keeping its operation label; verify routed success, denial, HEAD, 405, route miss, and pre-routing failure labels and policy.                                                                                                                            | Labels require raw paths, drift across methods, or require a second policy table. Keep the existing template labels instead.                            | 3.1.1; 6.1.3 |
| H4: Explicit lifespan ownership avoids lifecycle middleware coupling.       | E4: inject partial startup failure, a slow stream during drain, and failing cleanup; verify dependent resources remain open until users stop and eligible cleanup actions are attempted within the application's budget and blocked actions are reported.                                          | Cleanup requires a response hook, dependency use follows closure, or server completion hides live work. Revise ownership/completion contracts.          | 6.1.2; 6.1.4 |
| H5: Boundary examples can detect architectural regression cheaply.          | E5: add forbidden dependency/import fixtures and a permitted composition-root fixture, plus behavioural tests against a fake and a real outbound adapter. Record checker coverage and blind spots.                                                                                                 | Checks accept the forbidden examples or only prove method presence. Narrow the claims and strengthen the relevant guard.                                | 1.3.2; 6.2.2 |

_Table 4: Testable improvements and their delivery links._

Start with E1 and E2, using a small consumer fixture rather than migrating
three applications. Preserve the same boundary in both compiler variants;
attribute any reduction in forwarding or path tables to API design unless
isolated compiler evidence demonstrates otherwise. Compare change surface and
clarity, not just line counts. Runtime and code-size claims require
measurements.

## 7. References

[^1]: Corbusier
      [task routes](https://github.com/leynos/corbusier/blob/19e1dafd2eee2af7dbcf12320dbc9a56b2b8258a/src/http_api/routes/tasks.rs)
    and [HTTP state and façades](https://github.com/leynos/corbusier/blob/19e1dafd2eee2af7dbcf12320dbc9a56b2b8258a/src/http_api/state.rs).

[^2]: Corbusier
      [task service](https://github.com/leynos/corbusier/blob/19e1dafd2eee2af7dbcf12320dbc9a56b2b8258a/src/task/services/lifecycle.rs),
    [repository port](https://github.com/leynos/corbusier/blob/19e1dafd2eee2af7dbcf12320dbc9a56b2b8258a/src/task/ports/repository.rs),
    [request context](https://github.com/leynos/corbusier/blob/19e1dafd2eee2af7dbcf12320dbc9a56b2b8258a/src/context/request_context.rs),
    and [authentication adapter](https://github.com/leynos/corbusier/blob/19e1dafd2eee2af7dbcf12320dbc9a56b2b8258a/src/http_api/auth.rs).

[^3]: Corbusier
      [executable composition](https://github.com/leynos/corbusier/blob/19e1dafd2eee2af7dbcf12320dbc9a56b2b8258a/src/main.rs).

[^4]: Wildside
      [HTTP state](https://github.com/leynos/wildside/blob/095178bd8ce8c35b0854275b710542d75e0ae12e/backend/src/inbound/http/state.rs)
    and [state builders](https://github.com/leynos/wildside/blob/095178bd8ce8c35b0854275b710542d75e0ae12e/backend/src/server/state_builders.rs).

[^5]: Wildside
      [annotation HTTP adapter](https://github.com/leynos/wildside/blob/095178bd8ce8c35b0854275b710542d75e0ae12e/backend/src/inbound/http/annotations.rs).

[^6]: Wildside
      [architecture checker](https://github.com/leynos/wildside/blob/095178bd8ce8c35b0854275b710542d75e0ae12e/tools/architecture-lint/src/lib.rs).

[^7]: Episodic
      [application assembly](https://github.com/leynos/episodic/blob/0740f26ea59ff2f56108cc137833004af09a00d8/episodic/api/app.py),
    [dependencies](https://github.com/leynos/episodic/blob/0740f26ea59ff2f56108cc137833004af09a00d8/episodic/api/dependencies.py),
    [API runtime](https://github.com/leynos/episodic/blob/0740f26ea59ff2f56108cc137833004af09a00d8/episodic/api/runtime.py),
    and [worker runtime](https://github.com/leynos/episodic/blob/0740f26ea59ff2f56108cc137833004af09a00d8/episodic/worker/runtime.py).

[^8]: Episodic
      [resource base classes](https://github.com/leynos/episodic/blob/0740f26ea59ff2f56108cc137833004af09a00d8/episodic/api/resources/base.py)
    and [shared HTTP handlers](https://github.com/leynos/episodic/blob/0740f26ea59ff2f56108cc137833004af09a00d8/episodic/api/handlers.py).

[^9]: Episodic
      [authorization middleware](https://github.com/leynos/episodic/blob/0740f26ea59ff2f56108cc137833004af09a00d8/episodic/api/authorization.py)
    and [metrics and shutdown middleware](https://github.com/leynos/episodic/blob/0740f26ea59ff2f56108cc137833004af09a00d8/episodic/api/app.py).
    [Falcon middleware documentation](https://falcon.readthedocs.io/en/stable/api/middleware.html)
    confirms separate ASGI startup/shutdown events alongside resource hooks;
    checked with Firecrawl on 2026-09-20.

[^10]: Episodic
       [ADR 014: Architecture enforcement](https://github.com/leynos/episodic/blob/0740f26ea59ff2f56108cc137833004af09a00d8/docs/adr/adr-014-hexagonal-architecture-enforcement.md).
