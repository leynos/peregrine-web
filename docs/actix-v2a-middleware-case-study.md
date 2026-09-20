# Case study: Actix v2a extensions and Peregrine middleware

- Status: source-based assessment and proposed design refinements.
- Date: 2026-09-20.
- Audience: framework, middleware, and application-service authors.
- Companions: [technical design](peregrine-design.md),
  [ADR 003](adr-003-http-integration-boundaries.md), and [roadmap](roadmap.md).

## 1. Finding and evidence boundary

Peregrine can host the functionality examined here, but middleware improves
only part of its integration. The strongest gains are a uniform resource-policy
checkpoint, shared request facts, and one failure-rendering/finalization path.
Most algorithms remain clearer as typed parsers, value types, body adapters, or
application services. Moving them into hooks would add hidden ordering and
state without removing the work.

This assessment compares actix-v2a's existing implementation at `c8f68e8` with
PR #92 at `19771b4`, inspected on 2026-09-20. The PR was open and adds seven
**documentation** files or changes, not runtime capabilities. Its mutation and
HTTP integration work remains unchecked. The local checkout matched the exact
PR head; the runtime sources are unchanged from its base.[^1]

The existing surface includes shared errors, optional UUID idempotency-key
parsing, canonical JSON hashing, record/lookup/snapshot types, pagination, SSE
wire helpers, and OpenAPI schemata. The PR proposes scoped mutation identity,
reservation ownership, conditional completion, uncertain outcomes, replay and
recovery contracts, richer validation/error conversion, correlation, and
telemetry.[^2] These are different evidence classes throughout this document.

Peregrine is still a design. Its examples below are candidate API sketches, not
available APIs or compiled framework examples. Source inspection supports
placement and contract comparisons; it does not establish fewer allocations,
faster requests, reduced production incidents, or measured developer effort.
There is no defensible percentage of implementation work eliminated.

Actix is also a stronger comparator than a handwritten `Transform` boilerplate
example suggests. It already has async function middleware, extractors,
resource-local middleware, and resource-local application data. A helper can
register policy and handler together there too. Peregrine's proposed advantage
is one framework-enforced convention and visible lifecycle, not a capability
that Actix cannot express.[^3]

## 2. Capability and placement comparison

“Clearer” below means less application wiring or fewer ambiguous ownership
boundaries under the proposed contracts, not a measured speedup. “Existing”
means source is present; it does not mean every downstream service uses it.

| Capability and evidence                                                                                    | Best Peregrine placement                                                                                | Assessment and remaining work                                                                                                                                                                |
| ---------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Shared error codes, safe internal-error responses, trace field/header, and Actix conversion: existing.[^4] | Semantic failures plus a configured renderer at finalization.                                           | Clearer integration across hook, handler, routing, and body failures. The error vocabulary and envelope still need adapters; Actix already centralizes much of this through `ResponseError`. |
| Field/index validation and extended error/pagination conversion: proposed.[^2]                             | Pure validators and explicit `From`/mapping functions; renderer consumes safe details.                  | Little middleware advantage. Wrapping validators in asynchronous hooks hides endpoint dependencies.                                                                                          |
| Validated request correlation: proposed; the existing error type already carries a trace value.[^2][^4]    | Immutable request facts created once, propagated by finalization.                                       | Clearer cross-exit invariant. Middleware alone is insufficient for supervised timeouts or failures before a hook ran.                                                                        |
| Optional UUID idempotency header: existing; distinct required extraction: proposed.[^5]                    | Typed synchronous request parser, invoked explicitly by a resource or its typed adapter.                | Comparable to an Actix extractor. A global mandatory-header hook would wrongly reject unrelated requests.                                                                                    |
| Canonical JSON hash: existing; normalized intent and fingerprint profiles: proposed.[^6]                   | Pure helper called after application validation and normalization.                                      | No middleware gain. Hashing wire bytes or generic JSON before normalization can identify the wrong operation.                                                                                |
| Scoped identity, atomic claim, owner-checked completion, recovery: proposed.[^2]                           | Application service plus a durable adapter and conformance tests.                                       | No lifecycle shortcut. HTTP hooks cannot make domain and deduplication writes atomic or fence external effects.                                                                              |
| Current authorization before replay: proposed.[^2]                                                         | Resource policy before dispatch; application service rechecks object policy and reconstructs results.   | A clearer common entry gate, with domain work remaining explicit. A saved success must never bypass current access or redaction.                                                             |
| Cursor encoding, limits, links, and envelopes: existing; error conversion extensions: proposed.[^7]        | Pure pagination types and an explicit responder helper.                                                 | Existing helpers already fit. Middleware cannot choose a resource's ordering, query, or permitted cursor fields.                                                                             |
| `Last-Event-ID`, cache-header, event-ID, and frame validation: existing.[^8]                               | Typed parser and optional event-stream response constructor.                                            | Modest header-adapter simplification; the wire helpers remain useful unchanged in principle. No global SSE hook.                                                                             |
| Heartbeat policy, comment frames, and `stream_reset`: existing wire helpers.[^8]                           | An owned stream combinator or application stream, with injected time.                                   | Response hooks end before transfer. They cannot schedule ongoing heartbeats or perform replay-store recovery.                                                                                |
| Bounded mutation telemetry: proposed.[^2]                                                                  | Application service emits mutation outcomes; engine emits HTTP finalization and body-transfer outcomes. | Cleaner separation of events. A response status is not evidence that a mutation committed.                                                                                                   |
| OpenAPI error and replay schemata: existing.[^9]                                                           | Contract/schema generation and compatibility tests.                                                     | No middleware advantage. Runtime hooks do not describe an endpoint's wire contract.                                                                                                          |

_Table 1: The reusable core mostly survives; middleware replaces selected
wiring._

The important split is **HTTP policy and presentation versus domain
execution**. Even rows with a clear integration gain retain their parsers,
semantic types, validation, and tests. No finding justifies replacing actix-v2a
with a universal Peregrine middleware package.

## 3. Where Peregrine improves the integration

### 3.1. Resource identity supplies a consistent policy checkpoint

A protected mutation can group dependencies and operation policy on its
resource. The engine supplies that actual resource to authorization before
dispatch. The same checkpoint covers synthetic HEAD/OPTIONS responses and
applies again on an HTTP replay request. This removes the need for an
application convention linking a route name to policy; Actix can achieve
equivalent co-location with a resource-registration helper and shared data.[^3]

Candidate resource authoring keeps the application service call visible:

```rust,no_run
async fn on_post(&self, input: RequestView<'_>, body: &mut BodySlot)
    -> Result<InvoiceReply, Failure>
{
    let key = input.required_header::<IdempotencyKeyHeader>()?;
    let intent = CreateInvoice::validate_and_normalize(body.json().await?)?;
    Ok(self.invoices.create(input.principal()?, key, intent).await?)
}
```

This is an illustrative typed responder adapter: it converts `InvoiceReply`
into the response draft after the call. The existing mutable-response style
remains available. `IdempotencyKeyHeader` denotes an application parser adapter
returning the existing key value, not an implemented Peregrine or actix-v2a
type. `create` owns operation scope, fingerprinting, reservation, effect
execution, completion, and replay policy; its implementation is not hidden in
the example's middleware.

An application using Actix and actix-v2a can write an equally short handler.
Peregrine's improvement is the consistent resource-policy gate and shared
failure path around it, not the number of lines in this function. Required-key
parsing must retain the key through the service call; middleware presence alone
cannot prove that a handler uses it.

### 3.2. One finalizer can preserve error context without body interception

The existing Actix adapter already renders a shared error and propagates its
trace header; conversion preserves an existing shared error when possible. It
also redacts internal failures and retains otherwise unmapped HTTP statuses.
The PR extends this contract rather than starting from scratch.[^4]

Peregrine should preserve that typed information until all response hooks have
finished. A configured renderer sees the final selected public failure and
immutable request facts. It does not collect and deserialize an arbitrary
response body to discover whether it looks like JSON error output.

| Existing Actix integration                                                                                                   | Proposed Peregrine integration                                                                                                            |
| ---------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `extract_idempotency_key(req.headers())` returns an optional key; `map_idempotency_key_error` converts failures.             | `input.optional_header::<IdempotencyKeyHeader>()?` uses an explicit typed parser/error mapping and returns the same semantic distinction. |
| `Error::error_response()` creates the shared JSON envelope and trace header. Other error paths must enter that contract too. | Framework failures and explicitly mapped application failures meet the same finalizer with a selected status and request facts.           |
| Resource/scope middleware can attach response headers, with ordering chosen by the application.                              | Finalization owns correlation propagation after hook failures and representation replacement.                                             |

_Table 2: A narrower framework convention, not an absence of Actix equivalents._

For example, a handler produces a 409 conflict, then a response hook fails. The
final failure becomes a sanitized 500 while the conflict remains a diagnostic
cause. The renderer and correlation header must describe that final result.
Rendering in an earlier “error middleware” would make correctness depend on
where it sits in reverse response order.

### 3.3. Shared facts survive ordinary short circuits

Peregrine's independent response phase runs all registered response hooks on
ordinary completion, even when an earlier request hook rejected the request.
That is convenient for security headers and diagnostic context, but a hook
cannot assume its own request setup ran. An immutable `RequestFacts` value,
initialized at context creation, gives both renderer and hooks a safe minimum:
a validated/generated correlation ID and request timing, with an optional
frozen route added by the engine. It is not an authentication credential.

This improvement belongs partly in the engine rather than in middleware.
Supervised 504 generation must reuse these facts without resuming cancelled
hooks. Parser/connection failures before context creation, shutdown, and panics
remain outside this guarantee. An Actix application can implement the same
policy; Peregrine can make its coverage a documented default.

## 4. The counterexample: idempotency is not an unwind operation

The tempting middleware design is:

```text
resource hook: hash body, reserve key, put claim in extensions
responder: execute mutation
response hook: save response, mark claim complete
```

It fails as a general contract:

1. Hashing before validation can disagree with normalized operation intent.
   Raw JSON canonicalization sorts keys; it does not normalize domain meaning.
2. Another resource hook may reject after reservation. Independent response
   hooks run even when their request/resource setup did not run, so completion
   code must distinguish no claim, an unused claim, executed work, and an
   uncertain effect. A response status does not supply that distinction.
3. The mutation may commit before a later response hook replaces its HTTP
   response with 500. Saving the final error as the durable mutation outcome
   loses the successful domain result; saving the earlier response may retain
   stale redaction or one-time secrets.
4. Cancellation may skip the completion hook entirely. Asynchronous cleanup
   cannot be guaranteed by `Drop`; an expired lease does not prove safe retry.
5. A generic snapshot cannot transparently capture an unbounded stream while
   preserving backpressure. Replay also needs current authorization and an
   explicit decision about safe stored fields.

PR #92 already recognizes these boundaries and proposes consumer-owned durable
adapters. Peregrine should preserve them, including non-HTTP callers.[^2] Actix
middleware has the same durable-effect limitation; Peregrine's independent
response hooks add the particular obligation to handle missing forward setup.

The preferable sequence is explicit, with transport mapping outside the service:

```text
resource policy -> parse key -> validate and normalize intent
application service -> claim scoped identity and compare fingerprint
  acquired      -> effect + result in one transaction, or reconcile separately
  completed     -> recheck current access/redaction and reconstruct safe result
  pending       -> bounded wait or report pending, without duplicate execution
  conflict      -> report incompatible intent
  indeterminate -> lookup/reconcile; never infer permission to repeat the effect
HTTP adapter -> map typed outcome -> response hooks -> finalization
```

Atomic domain/result commit and separately reserved external effects require
different adapter evidence. Stale-owner completion rejection alone does not
fence an old worker from performing its external effect. Operation outcomes,
retention, and cancellation belong to the service's contract in either
framework. Retry metadata must distinguish querying an outcome from
re-executing an effect.

## 5. Where middleware adds pain

### 5.1. A context bag hides inputs that Actix names explicitly

`web::Query<PageParams>` and custom `FromRequest` types advertise dependencies.
Replacing those with `ctx.extensions().get::<T>()` can move a compile-time
input into an optional runtime lookup, coupled to undocumented hook
ordering.[^3] Use synchronous typed parsers on request views and ordinary
values in resource methods. Provide an optional typed responder adapter for
applications wanting explicit inputs; do not require one hook per field or a
generic extractor scheduler. Missing authenticated identity remains a semantic
authorization condition, not a panic from a missing extension.

Peregrine's sequential exclusive context borrow can also obstruct independent
work. Narrow views help, but introducing lifetime-heavy adapters merely to
parse a UUID is needless complexity. None of this case study requires Polonius
or the new solver; those remain a separate measured experiment.

### 5.2. Independent response hooks reverse the usual setup intuition

A hook can run without its forward setup and before another hook replaces the
response. Such hooks must be idempotent with respect to their own header
changes, tolerate absent optional state, and treat status as provisional.
Putting a database transaction in extensions would make its lifetime harder to
understand without establishing rollback or commit guarantees.

Keep final status metrics and correlation propagation in finalization. Retain
ordinary response hooks for transformations that belong before commitment; do
not invent a second configurable unwind order or an asynchronous commit hook.
The fixed lifecycle is only simpler when its exclusions are explicit.

Late error serialization has a cost: a response hook cannot sign or compress
final error bytes that do not yet exist. Such transformations need a separately
designed post-render body adapter; they are not implied by the initial hook
contract. The static serialization-failure fallback may omit a body correlation
field, but finalization still preserves the validated correlation header.

### 5.3. Pagination and SSE are mostly ordinary code

Actix-v2a already caps page limits, rejects zero, bounds cursor input, accepts
its supported Base64 forms, and preserves unrelated query parameters in links.
Middleware must not silently change those contracts, reconstruct public URLs
from untrusted forwarding headers, or turn unsigned cursors into authorization
claims.[^7]

Its SSE adapter currently converts Actix header values into an owned vector of
`SseHeader` values, then calls a transport-independent parser. A borrowed raw
header iterator could avoid that intermediate representation. That is a small
adapter/API improvement available in Actix too, not a consequence of Peregrine
or a demonstrated latency gain.[^8]

SSE framing, the 20-second default heartbeat policy, and reset frames already
have shared helpers. An optional event-stream constructor can apply media type
and cache policy atomically when a stream is installed. A route-wide response
hook that stamps SSE headers on a JSON authorization failure would be wrong.
Heartbeat scheduling, event retention, subscriber cancellation, and
replay-store access remain stream/application responsibilities. After
commitment, an error cannot be converted into a new JSON response.[^8]

### 5.4. Abstraction costs remain real

Typed adapter code, boxed async boundaries, middleware traversal, and `Send`
requirements still have costs. Actix's function middleware already removes much
manual `Transform` machinery; its worker-local designs can also accommodate
state that Peregrine's proposed `Send + Sync` resources cannot.[^3] Do not
claim that a shorter hook implementation removes allocations or trait
complexity from the framework. Measure total helper, registration, and
diagnostic complexity.

The actix-v2a package currently depends on Actix unconditionally even though
many modules contain transport-independent logic. Direct reuse may therefore
bring an unwanted dependency. An upstream feature split or companion core crate
would need a separate packaging decision; simply renaming an import is not a
Peregrine integration plan.[^10]

## 6. Resulting architecture and API refinements

[ADR 003](adr-003-http-integration-boundaries.md) records these proposed
choices. The repository sweep found existing design concepts for phase views,
private failure state, typed extensions, finalization, and owned streaming
bodies, but no implemented framework helpers beyond the library stub. Refine
those concepts rather than create parallel middleware subsystems.

| Refinement                                              | Ownership and permitted composition                                                                                                                                                       | Acceptance evidence                                                                                                                                                    |
| ------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Typed request parsers and optional responder adaptation | Parsers inspect borrowed metadata, preserve repeated headers, and return semantic values/errors. Resources or typed endpoint adapters call them explicitly; body decoding stays separate. | Required/optional/duplicate/malformed key cases; key reaches service; empty SSE replay cursor remains absent; no body consumption by metadata parsing.                 |
| Immutable request facts                                 | Engine initializes correlation and timing once; hooks read them; route facts become available after routing. Applications configure trust and generation.                                 | Consistent ID on ordinary early exits, framework failures, hook failure, and supervised 504; concurrent isolation and pre-context exclusions.                          |
| Configurable public-failure renderer                    | Application selects envelope policy; engine selects final status, retains causes, enforces headers, and commits. Renderer receives safe structured data only.                             | Explicit status/reason preserved; final hook failure invalidates earlier presentation; serialization fallback; existing wire fields preserved or explicitly versioned. |
| Explicit response representation                        | Installing a body records its kind and associated headers; replacing it clears stale representation metadata. Optional SSE support is an ordinary response/body helper.                   | JSON failures are never labelled SSE; streams are not collected; HEAD suppression and post-commit failure remain correct.                                              |
| Separate operation and transport observations           | Application service records mutation outcome; engine reports finalized HTTP outcome; body owner reports transfer completion.                                                              | Successful mutation followed by HTTP failure remains two different outcomes; cancellation is not counted as completed transfer.                                        |

_Table 3: Bounded enhancements; all remain proposed until implemented and
tested._

No new transaction engine, idempotency middleware, per-feature lifecycle phase,
OpenAPI generator, or general dependency-injection registry follows from this
case study. Parser and response helper examples should remain usable without
adopting a broader middleware framework.

## 7. Delivery and verification

The [roadmap](roadmap.md) incorporates these findings into contract definition,
ownership prototypes, lifecycle rendering, protected-resource interactions,
streaming, and instrumentation. The main design includes the marked integration
improvements as examples, with proposed API names explicitly labelled.

A meaningful comparison implements equivalent Actix and Peregrine resource
fixtures: missing/duplicate keys, changed authorization on replay, one
successful mutation followed by a response-hook failure, JSON rejection on an
SSE route, and a cancelled stream. Count application glue and registration
assumptions, not just hook lines. This is future comparison work, not a result
of this review.

Keep compatibility assertions for code/message/trace fields, selected HTTP
status, validation paths, cursor tokens, page-limit behaviour, and SSE wire
bytes. Consumers with additional envelope fields must supply their own
fixtures; the PR's consumer observations are not independently reproduced here.
Durable mutation guarantees require the separate restart, concurrency, and
ambiguous- acknowledgement harness specified by PR #92. HTTP pipeline tests
cannot replace it.

## 8. Sources

All sources were inspected on 2026-09-20. Commit links freeze the reviewed
actix-v2a state. Live Actix documentation supplied the comparison baseline.

[^1]: [PR #92](https://github.com/leynos/actix-v2a/pull/92),
    head `19771b4762012b48baf5ab46671e2475e404955d`, base
    `c8f68e8ad3370550ffd0bb7ebff3f5e8d2d29b53`.

[^2]: [Shared mutation and HTTP design](https://github.com/leynos/actix-v2a/blob/19771b4762012b48baf5ab46671e2475e404955d/docs/shared-mutation-contract-design.md)
    and [HTTP integration ADR](https://github.com/leynos/actix-v2a/blob/19771b4762012b48baf5ab46671e2475e404955d/docs/adr-004-shared-http-integration.md).

[^3]: Actix [middleware](https://actix.rs/docs/middleware/),
    [function middleware](https://docs.rs/actix-web/4.15.0/actix_web/middleware/fn.from_fn.html),
    [resource data](https://docs.rs/actix-web/4.15.0/actix_web/web/struct.Data.html),
    [extractors](https://docs.rs/actix-web/4.15.0/actix_web/trait.FromRequest.html),
    and [worker-local state](https://actix.rs/docs/server/#multi-threading).

[^4]: [Error value](https://github.com/leynos/actix-v2a/blob/c8f68e8ad3370550ffd0bb7ebff3f5e8d2d29b53/src/error.rs)
    and [HTTP adapter](https://github.com/leynos/actix-v2a/blob/c8f68e8ad3370550ffd0bb7ebff3f5e8d2d29b53/src/http/error.rs).

[^5]: [Idempotency header parser](https://github.com/leynos/actix-v2a/blob/c8f68e8ad3370550ffd0bb7ebff3f5e8d2d29b53/src/idempotency/http.rs)
    and [record/lookup types](https://github.com/leynos/actix-v2a/blob/c8f68e8ad3370550ffd0bb7ebff3f5e8d2d29b53/src/idempotency/record.rs).

[^6]: [Canonical JSON hashing](https://github.com/leynos/actix-v2a/blob/c8f68e8ad3370550ffd0bb7ebff3f5e8d2d29b53/src/idempotency/payload.rs).

[^7]: [Pagination contracts](https://github.com/leynos/actix-v2a/blob/c8f68e8ad3370550ffd0bb7ebff3f5e8d2d29b53/src/pagination/mod.rs),
    [cursor codec](https://github.com/leynos/actix-v2a/blob/c8f68e8ad3370550ffd0bb7ebff3f5e8d2d29b53/src/pagination/cursor.rs),
    [parameters](https://github.com/leynos/actix-v2a/blob/c8f68e8ad3370550ffd0bb7ebff3f5e8d2d29b53/src/pagination/params.rs),
    and [links/envelope](https://github.com/leynos/actix-v2a/blob/c8f68e8ad3370550ffd0bb7ebff3f5e8d2d29b53/src/pagination/envelope.rs).

[^8]: [SSE module](https://github.com/leynos/actix-v2a/blob/c8f68e8ad3370550ffd0bb7ebff3f5e8d2d29b53/src/sse/mod.rs),
    [Actix adapter](https://github.com/leynos/actix-v2a/blob/c8f68e8ad3370550ffd0bb7ebff3f5e8d2d29b53/src/sse/actix_adapter.rs),
    and [user guide](https://github.com/leynos/actix-v2a/blob/c8f68e8ad3370550ffd0bb7ebff3f5e8d2d29b53/docs/users-guide.md).

[^9]: [OpenAPI schemata](https://github.com/leynos/actix-v2a/blob/c8f68e8ad3370550ffd0bb7ebff3f5e8d2d29b53/src/openapi/schemas.rs).

[^10]: [Package dependencies](https://github.com/leynos/actix-v2a/blob/c8f68e8ad3370550ffd0bb7ebff3f5e8d2d29b53/Cargo.toml).
