# Architectural decision record (ADR) 007: Record the provisional resource-policy and body-erasure boundary

## Status

Proposed. The roadmap 1.1.3 spike compiled and exercised the shapes below, so
they are the tested candidate rather than an untried sketch. Acceptance of the
policy vocabulary and enforcement API still belongs to terms-of-reference Q3,
and the ownership direction still depends on roadmap task 2.2.3. Nothing here
is a stable public API or an implemented framework.

## Date

2026-11-01.

## Context and problem statement

The
[technical design §§3–4](peregrine-design.md#3-application-and-context-contracts)
propose a resource-first contract in which a resource owns its policy and its
dependencies, registration freezes the effective method and policy, and a
single conditional invocation replaces route-policy tables. The design named
the policy vocabulary and enforcement registration API as unaccepted until the
Q3 spike, and it deferred the erased body type to
"[selected and compile-tested in the first spike](peregrine-design.md#3-application-and-context-contracts)".

[Roadmap task 1.1.3](roadmap.md) asked four bounded questions: can a resource
own its policy and dependencies; can registration freeze policy before
admission; can an erased body stay `Send` but not `Sync` across a real await;
and can one conditional invocation be expressed without service locators or
shared mutable application state. The
[spike evidence ledger](resource-policy-ownership-spike.md) records the tested
answers. This ADR preserves their outcome so later slices do not rediscover it.

## Decision drivers

- Resource identity and policy must be observable and unchanged across an
  awaited hook, so an enforcer cannot be handed a substitute.
- A request body must cross an await inside a `Send` future without requiring
  `Sync`, because Hyper's incoming body is not `Sync`.
- Cancellation must release exactly the ownership the invocation acquired and
  leave a caller-owned body usable.
- A protected resource must not reach a runner that cannot enforce it, and that
  rejection must be a typed error rather than a runtime check in each responder.
- The boundary must stay dyn-compatible so heterogeneous resources share one
  registration and dispatch path.

## Requirements

### Functional requirements

- A resource declares a method-independent policy that is `Public`,
  `Authenticated`, or a non-empty `Scopes` set.
- Registration rejects a policy that requires enforcement when the runner has
  no enforcer installed.
- Admission observes the retained resource, its declared observation, the
  frozen method, the captured policy, the request path, and the presented
  identity.
- An anonymous protected request is denied before the enforcer runs; an
  enforcer denial reaches neither the responder nor the resource dependency.
- An unselected registration is never invoked.
- An empty scope set is a typed rejection, not a silent permit.

### Technical requirements

- The erased body is `Send` and is not required to be `Sync`.
- The body error type is semantic and retains its upstream source.
- The responder future borrows immutable request metadata and the mutable body
  separately, so neither borrow is `Sync`-bound.
- A fully owned invocation can cross a multi-thread runtime boundary.

## Options considered

| Topic        | Selected shape                                                      | Alternative                                              | Why the alternative was not selected                                                                     |
| ------------ | ------------------------------------------------------------------- | -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Body erasure | `UnsyncBoxBody<Bytes, BodyError>`                                   | `BoxBody<Bytes, BodyError>`                              | `BoxBody` is `Sync`; requiring it would reject Hyper's own incoming body and any `Cell`-bearing body     |
| Resource     | Dyn-compatible `Resource` trait object behind `Arc`                 | Generic resource parameter on every registration call    | Registration storage and the runner become generic or duplicate the dispatch path per concrete type      |
| Enforcement  | `Arc<dyn PolicyEnforcer>` built once at the composition root        | Per-resource generic enforcer or a route-policy registry | A registry reintroduces the table this design removes; a generic parameter infects every registration    |
| Invocation   | `&mut ErasedBody` supplied by the caller                            | Invocation owns the body exclusively                     | Caller ownership keeps the body usable after cancellation; the owned form remains available for spawning |
| Policy gate  | Synchronous `policy()` checked at registration and before admission | Asynchronous policy resolution inside the responder      | A responder-time check cannot deny before the hook and makes every responder repeat the same guard       |

_Table 1: Tested boundary shapes and the alternatives rejected in the spike._

## Proposed direction

Adopt the tested shapes as the candidate for later slices, subject to the
outstanding decisions below:

- A `Send + Sync` dyn-compatible `Resource` owns its typed `Policy` and its
  injected dependencies, and returns a `Send` responder future through
  `async-trait`.
- `adapt_body` erases a concrete `Send` body exactly once into
  `UnsyncBoxBody<Bytes, BodyError>`; upstream failures become
  `BodyError::Upstream` with their source retained.
- Registration captures the method and policy, retains the `Arc<dyn Resource>`
  as the invocation handle, and returns a typed `MissingEnforcer` error when
  the policy requires enforcement and no enforcer is installed.
- The enforcer is an erased async trait object supplied at the composition root.
  Admission receives a request view whose `Identity` is optional; absence is a
  normal condition that policy decides.
- Invocation receives a caller-owned `&mut ErasedBody` and returns an
  `InvocationOutcome` that distinguishes a response, a denial, and a failure.

These are the shapes `peregrine_web::spike` compiles and the evidence ledger
witnesses. They remain provisional: the specimen is not wired into a router,
server, or production authorization path.

## Known risks and limitations

The enforcer is a fixture; no authentication, authorization, or session policy
is implemented. The specimen performs a single synchronous policy read per
invocation, which is enough to freeze identity but says nothing about policy
caching or revocation. `async-trait` boxes each responder future, so the
allocation cost of this erasure is unmeasured.

`UnsyncBoxBody` deliberately excludes `Sync` consumers. Any later requirement
to share a body between threads would need a different erasure and a separated
review rather than a silent bound relaxation.

The compile witnesses are `trybuild` fixtures tied to the pinned nightly; a
compiler change can alter a reviewed `.stderr` baseline even when acceptance is
unchanged. The cancellation witnesses drop a future at one known suspension
point and do not explore cancellation at every await or under load. The
[evidence ledger](resource-policy-ownership-spike.md) lists the limits per
witness.

## Outstanding decisions

Terms-of-reference Q3 and Q5 must still accept or revise the policy vocabulary
and enforcement registration surface; this ADR does not accept them by
recording how they compile. Roadmap task 2.2.3 must select the compiler and
ownership direction before the invocation boundary stabilises, and
[ADR 002](adr-002-compiler-ownership-experiment.md) remains proposed. Open
questions for acceptance:

- Whether `Resource` stays dyn-compatible after the phase 2 comparison, or
  whether generic authoring with an erased adapter is preferable.
- Whether the body bound should relax, tighten, or stay `Send`-only once a
  response body exists.
- What minimum supported compiler and analyser configuration the fixture
  evidence permits.
- Whether the enforcer surface should accept tenant or object-level context
  beyond the initial identity and request view.
