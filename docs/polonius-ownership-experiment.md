# Polonius and new-solver ownership experiment

- Status: proposed experiment; compiler probes are evidence, not a framework.
- Date: 2026-09-20.
- Audience: framework authors, middleware authors, and prospective consumers.
- Companions: [technical design](peregrine-design.md),
  [roadmap](roadmap.md), [ADR 002](adr-002-compiler-ownership-experiment.md),
  and [six-perspective review](polonius-design-review.md).

## 1. The GIST idea and its scope

**Idea:** designing Peregrine exclusively for Polonius and the new trait solver
may allow more direct borrowing APIs and simpler resource-aware middleware
composition than retaining old-checker compatibility. The experiment serves
Goals G1, G2, and G4: endpoint cohesion, explainable processing, and
maintainable adoption. Its outcome is adoption, revision, or rejection, not
assumed success.

'Pure' means the experimental implementation and its consumers use an
explicitly pinned compiler with `-Zpolonius=next -Znext-solver=globally`,
without maintaining an old-checker fallback implementation or conditional
compatibility path in that candidate. It does not mean allocation-free, wholly
static dispatch, no owned data, no `Arc`, or permission to violate exclusive
borrowing. Comparison fixtures are experimental controls, not a promise of two
maintained products.

Candidate A remains compatible with non-lexical lifetimes (NLL), using explicit
`-Zpolonius=off -Znext-solver=no` on the same compiler for attribution.
Candidate B uses both new mechanisms exclusively. Neither candidate is called
stable-Rust compatible on that evidence alone; a pinned nightly with flags
disabled is not a stable compiler support test.

The hypothesis has two separate parts. Polonius must improve a demonstrated
borrowing pattern. The new solver must be evaluated independently for trait
composition and associated-type constraints. A benefit attributable only to
Polonius is not evidence that the new solver is necessary. The experiment may
recommend retaining the new solver as a supported toolchain policy, but must
name that as a policy choice rather than an ergonomic result.

Both candidates preserve resource-owned dependencies and policy, routing before
resource hooks, method dispatch, early responses, reverse response hooks,
semantic errors, and the commitment boundary for streams. Both may use the same
phase-specific views. Comparing a monolithic context only against a split-view
candidate would confound architecture with compiler behaviour.

## 2. Compiler evidence and reproduction

The 2026-09-20 probes used the repository's `nightly-2026-08-27` compiler:
`rustc 1.100.0-nightly (bff8e12ff 2026-08-26)`, edition 2024. Every case
explicitly selected both flags rather than relying on nightly defaults. The
upstream Polonius announcement describes flow-sensitive lifetime reasoning and
remaining unsupported patterns; it does not promise arbitrary borrowing
relief.[^1]

| Probe                                                           | NLL, old solver | NLL, new solver | Polonius, old solver | Polonius, new solver |
| --------------------------------------------------------------- | --------------- | --------------- | -------------------- | -------------------- |
| Fallible cache: branch before borrowing                         | Pass            | Pass            | Pass                 | Pass                 |
| Fallible cache: return existing borrow, then initialize on miss | E0502           | E0502           | Pass                 | Pass                 |
| Map cache: standard `Entry` implementation                      | Pass            | Pass            | Pass                 | Pass                 |
| Map cache: early borrowed hit, `Entry` on miss                  | E0499           | E0499           | Pass                 | Pass                 |
| Map cache: `contains_key` followed by `get_mut`                 | Pass            | Pass            | Pass                 | Pass                 |
| Split-field async resource responder                            | Pass            | Pass            | Pass                 | Pass                 |
| Native async responder through `dyn Resource`                   | E0038           | E0038           | E0038                | E0038                |
| Simultaneous overlapping mutable borrows                        | E0499           | E0499           | E0499                | E0499                |

_Table 1: Observed compiler results; error codes identify expected rejection._

The matrix establishes a Polonius-specific benefit and no solver-specific
benefit in these examples. It does not validate the proposed pipeline, its
runtime costs, or deployment support. The new solver changes trait solving and
associated-type normalization; it does not automatically enable every language
feature whose development it supports.[^2]

For each complete example module, reproduce the four compiler configurations
with the following command, substituting the input file and flag pair. The
paired cache functions in §3 need only the shared imports and error type shown
there. Library-mode compilation does not require a `main` function. The Option
comparison supplies method bodies rather than complete modules.

```sh
rustc +nightly-2026-08-27 --edition=2024 --crate-type=lib \
  -Zpolonius=off -Znext-solver=no example.rs
rustc +nightly-2026-08-27 --edition=2024 --crate-type=lib \
  -Zpolonius=off -Znext-solver=globally example.rs
rustc +nightly-2026-08-27 --edition=2024 --crate-type=lib \
  -Zpolonius=next -Znext-solver=no example.rs
rustc +nightly-2026-08-27 --edition=2024 --crate-type=lib \
  -Zpolonius=next -Znext-solver=globally example.rs
```

The [probe archive](compiler-probe-evidence.md) preserves all eight complete
fixtures and the observed results, including runtime cache assertions. The
implementation experiment must preserve its fixtures and command outputs as
reviewable artefacts. Check expected error categories, not complete unstable
compiler wording. A compiler change that accepts a negative aliasing fixture
requires investigation; one that accepts a formerly rejected positive pattern
requires updating the attribution rather than preserving an obsolete claim.

## 3. Side-by-side code and ergonomic consequences

### 3.1. A fallible request-local map cache

This example represents already-owned derived request data, not request-body
consumption or the immutable resource registry. It preserves the same public
signature in both candidates. A fallible initializer runs only on a miss;
failure leaves the map unchanged. `DecodeError` stands for a semantic
application error. These shared definitions precede either implementation:

```rust
use std::collections::{HashMap, hash_map::Entry};

#[derive(Debug, PartialEq)]
pub struct DecodeError;
```

The comparison table places the essential control flow side by side. Complete
function bodies follow so the two alternatives can be compiled independently.

| Old-checker-compatible design                       | Exclusive Polonius design                                       |
| --------------------------------------------------- | --------------------------------------------------------------- |
| `match cache.entry(key.to_owned()) { ... }`         | `if let Some(value) = cache.get_mut(key) { return Ok(value); }` |
| `Entry::Occupied(e) => Ok(e.into_mut())`            | `match cache.entry(key.to_owned()) { ... }` on the miss path.   |
| `Entry::Vacant(e) => Ok(e.insert(decode()?))`       | The same `Entry` match handles fallible miss initialization.    |
| Resource code calls `decoded(cache, key, decode)?`. | Resource code calls the same `decoded(cache, key, decode)?`.    |

_Table 2: Identical caller ergonomics; different helper control flow._

**A: old-checker-compatible implementation.** The standard entry API is
compact, returns a borrow, and needs no panic, cloned value, or unsafe code.

```rust
pub fn decoded<'a>(
    cache: &'a mut HashMap<String, String>,
    key: &str,
    decode: impl FnOnce() -> Result<String, DecodeError>,
) -> Result<&'a mut String, DecodeError> {
    match cache.entry(key.to_owned()) {
        Entry::Occupied(entry) => Ok(entry.into_mut()),
        Entry::Vacant(entry) => Ok(entry.insert(decode()?)),
    }
}
```

**B: design for Polonius exclusively.** The hit path returns its borrow before
the miss path owns a key. NLL rejects the later mutable borrow; Polonius
accepts it with either solver in the measured matrix.

```rust
pub fn decoded<'a>(
    cache: &'a mut HashMap<String, String>,
    key: &str,
    decode: impl FnOnce() -> Result<String, DecodeError>,
) -> Result<&'a mut String, DecodeError> {
    if let Some(value) = cache.get_mut(key) {
        return Ok(value);
    }
    match cache.entry(key.to_owned()) {
        Entry::Occupied(entry) => Ok(entry.into_mut()),
        Entry::Vacant(entry) => Ok(entry.insert(decode()?)),
    }
}
```

B avoids constructing an owned string key on a hit and performs one map lookup
there. It is longer than A and performs an extra lookup on a miss. This is a
hit-path cost opportunity and a direct early-return borrowing style, not a
universal simplification or measured speedup. An NLL-compatible `contains_key`/
`get_mut` implementation also avoids hit-path key ownership, at the cost of two
hit lookups. A borrowed-entry collection API is another alternative to evaluate
before attributing a library limitation to NLL.

### 3.2. When the compiler buys only a different expression

For one cached string, assume `self.decoded: Option<String>` and the same
fallible initializer. The following method bodies share the signature
`fn decoded(&mut self, decode: impl FnOnce() -> Result<String, DecodeError>)`
returning `Result<&str, DecodeError>`.

| Old-checker-compatible method body                                                                               | Exclusive Polonius method body                                                                                         |
| ---------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `if self.decoded.is_none() {` `self.decoded = Some(decode()?);` `}` `self.decoded.as_deref().ok_or(DecodeError)` | `if let Some(value) = self.decoded.as_deref() {` `return Ok(value);` `}` `Ok(self.decoded.insert(decode()?).as_str())` |

_Table 3: Both cache once and return a borrow; neither clones the cached value._

The old-compatible final error arm is defensive and unreachable after
successful initialization. B avoids representing that impossible state as an
error, but neither version saves a payload allocation or changes the caller
signature. Both allow retry after failed initialization; this helper is not the
terminal body-consumption state machine in technical design §8. For an
infallible initializer, `Option::get_or_insert_with` already provides a concise
compatible solution; selecting Polonius solely for that case buys nothing
demonstrated.

### 3.3. Resource responders and middleware phase views

The framework's resource methods remain grouped on the resource object in both
candidates. The useful ownership change is to make phase access explicit:

| Whole-context draft                                      | Split-view candidate, available with either checker                                                |
| -------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `on_get(&self, ctx: &mut Context)`                       | `on_get(&self, input: RequestView<'_>, body: &mut BodySlot, response: &mut ResponseDraft)`         |
| Handler can request all mutable context state.           | Metadata is borrowed read-only; body and response are distinct mutable capabilities.               |
| `process_resource(ctx, resource)`                        | `process_resource(resource, route, request_state)` retains the selected resource and frozen route. |
| Public completion state needs a monotonic mutation rule. | A typed `Continue`/`Respond` hook result lets the engine own the transition.                       |

_Table 4: Candidate API shapes, not implemented Peregrine signatures._

NLL already permits disjoint field borrows to cross `.await`. Polonius does not
make this split possible; the ownership boundary does. A generic responder can
also return an unboxed future with either checker. Type erasure at the router
boundary is a separate choice, not a new-solver feature. Native async methods
remain unsuitable for direct `dyn Resource` dispatch under the tested flags.[^3]

## 4. The candidate ownership and composition contract

Both implementations use the same proposed lifetime boundaries:

1. The request future owns a request envelope. Request hooks can rewrite the
   effective target before routing. Freezing the target ends that capability.
2. Route matching produces offsets or short-lived borrowed views into frozen
   URI storage owned outside the mutable request state. It does not store a
   self-reference inside a movable context. Owned route parameters remain a
   valid fallback if measurements do not justify the extra lifetime surface.
3. The application owns the authoritative middleware sequence for all phases,
   including requests with no selected resource. A typed endpoint adapter
   retains the actual resource object, its immutable policy, and responder
   invocation; it may borrow that same stack. Resource hooks receive that
   object before method dispatch. Metadata copied into an unrelated route
   registry is not a substitute for resource identity.
4. The pipeline loans phase-specific views for each awaited call and regains
   them on completion. Mutation of body state cannot invalidate frozen route
   storage. Middleware requesting unrelated mutable capabilities must declare
   them rather than obtaining a universal context borrow.
5. The handler populates a response draft. Hooks return explicit continuation
   or early-response decisions; the engine retains failure precedence and
   invokes every response hook once in reverse order on ordinary completion.
6. Finalization consumes the response draft. Escaping response streams and
   detached work own their backing data, use shared ownership, or copy the
   small data they need. They cannot retain request-local borrows after the
   pipeline future ends.

Borrowing across a request-scoped `.await` is allowed when the owner lives long
enough. Sending detached work to another task does not extend that lifetime.
Polonius changes neither this condition nor aliasing, `Send`, or `Sync` rules.
No unsafe lifetime extension is part of either candidate.

A fixed application-owned middleware stack can compose generically, with a
dyn-compatible invocation boundary for heterogeneous router storage. The typed
adapter must preserve the baseline `Send` future guarantee, with explicit
bounds on returned futures; native async trait syntax alone does not promise
it. Include a multithread-executor-compatible invocation fixture and a negative
case retaining non-`Send` state across an await.

Runtime-selected middleware may still require erased calls or futures. The
experiment records allocations at each actual boundary; it promises neither one
allocation per request nor a wholly unboxed pipeline.

The engine owns all three phases even if generics assemble their hooks. A
wrapper that unwinds only middleware entered during forward processing is not
equivalent to Peregrine's independent response-hook contract. Cancellation
still prevents asynchronous completion hooks; ownership cleanup must not be
presented as guaranteed auditing or rollback.

## 5. Evidence, acceptance, and falsification

The implementation experiment compares the same protected resource, policy
inputs, routing corpus, middleware sequence, body modes, and dependency mocks.
First keep representations and dispatch strategy fixed while changing checker
and solver flags. Then evaluate phase views and boundary type erasure as
separate architectural changes available to both compiler candidates. Compile
the actual candidate lifetime-indexed middleware and associated-type bounds,
including returned-future `Send` requirements, under both solvers. Reduce any
discrepancy to a minimal reproducer; record "no difference found" if applicable.

| Bet                                                          | Evidence needed                                                                                                             | Reject or revise when                                                                                                    |
| ------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| B1: Exclusive borrowing improves implementation ergonomics.  | Paired helpers and representative resource/middleware edits reviewed by authors unfamiliar with the implementation.         | Gains reduce to relocating complexity, or the best compatible API is equally clear.                                      |
| B2: The new solver helps the required composition contracts. | Minimal trait-composition reproducer accepted only with the new solver, or a separately justified compiler-policy decision. | No solver-specific requirement is found and the dependency is described as an API necessity.                             |
| B3: Entity-first policy and lifecycle semantics survive.     | Exact resource identity, responder invocation counts, and complete trace comparisons for normal and failure paths.          | Authorization, hook order, or failure precedence changes between candidates.                                             |
| B4: Runtime savings exceed added costs where relevant.       | Allocation/key-construction counts, hit/miss workloads, retained memory, throughput, and latency distributions.             | Hit savings disappear under the pilot workload, misses regress materially, or borrowed storage retains excessive memory. |
| B5: Downstream compiler support is sustainable.              | Clean consumer package builds, editor diagnostics, documentation, lint/proof tooling, and a compiler upgrade rehearsal.     | Consumers require undocumented flags, unsupported toolchain combinations, or unmaintainable compatibility machinery.     |

_Table 5: Experimental bets and falsification evidence._

Before prototype work, the maintainer and pilot reviewer must record an effort
budget and minimum specimen: one protected resource, one fallible cache, a
small fixed middleware sequence, and synthetic request/stream boundaries. Full
transport and production policy delivery are out of scope. At budget exhaustion
or inconclusive results, choose the compatible design or record a bounded
extension decision; the experiment must not block delivery indefinitely.

Before measurements, record the workload, cache cardinality and key sizes,
hit/miss ratios, route count, middleware depth, concurrency, body/stream sizes,
numeric regression budgets, and review rubric. Include cold/miss-heavy and
hit-heavy workloads, a larger route/stack composition, repeat counts, and
variability. Measure clean and incremental build time, binary size, and future
sizes as well as request costs; generic composition can move cost from runtime
to compilation and code size. Performance conclusions require optimized builds
with the same code-generation backend and dependency set, not a comparison of
Cranelift against LLVM.

Minimum semantic cases include cache hit/miss/initializer failure, protected
HEAD/OPTIONS, 404, `OPTIONS *`, early completion before routing or setup,
handler error followed by response hook error, dropped request futures, and
attempted stream/task escapes. A borrowed value must not outlive invalidated
backing storage. Preserve negative compile cases for overlapping borrows and
native async dyn dispatch alongside the positive comparisons.

Adopt candidate B only when B1 and B3–B5 pass and B2 has an explicit
disposition. If ergonomics improve equally under NLL, adopt the ownership
design without claiming a compiler benefit. If only Polonius contributes,
either justify the new solver separately or revise the combined requirement. A
policy-only solver requirement must name its maintenance benefit, compare it
with Polonius-only support, and account for consumer/tooling costs. If neither
has a material benefit, retain the compatible implementation. In all cases
select one maintained product direction and retain comparisons only as test
evidence.

## 6. Toolchain and delivery boundaries

No compiler flags, package dependencies, runtime APIs, or support promises are
changed by this proposal. A later adoption decision must specify the exact
compiler and flags for library, consumer, test, and documentation compilation.
Cargo configuration from a dependency repository is not a consumer-wide build
contract; demonstrate a downstream build without the parent checkout's config.

The repository's current Whitaker invocation uses its own compiler toolchain.
Test whether it can analyse the exclusive source with the required flags before
adoption. Rustdoc, Clippy, rust-analyzer, coverage, and the proposed Verus
proof workflow also need explicit compatibility evidence. Compiler flags are
not Cargo features and must not create hidden feature combinations. Record the
configuration route separately for `RUSTFLAGS`, `RUSTDOCFLAGS`, editor checks,
and each analyser; success in one does not establish the others. A clean
consumer must also exercise deliberately omitted flags and record effective
defaults. Omission may still compile on a particular nightly; do not assume
failure. Disabling a mechanism needed by the source must produce a useful compiler
diagnostic, with recovery instructions. A policy-only solver requirement may
still compile when disabled; document its configuration check or unsupported
status separately rather than promising automatic compiler rejection.

Pin a last-known-good toolchain and preserve a reproducible consumer example.
For a compiler regression, restore that toolchain or defer the upgrade; do not
silently install an old-checker fallback in the exclusive candidate. Record who
owns compiler updates, how diagnostic changes are triaged, and how loss of a
required analyser affects release readiness. No required gate may be silently
removed to make the experimental branch green.

The [roadmap](roadmap.md) puts this experiment before production delivery
slices. [ADR 002](adr-002-compiler-ownership-experiment.md) remains proposed
until the adoption task records evidence and a decision. Existing goals and
HTTP/lifecycle invariants remain in force whichever candidate is selected.

## 7. References

[^1]: [Polonius Alpha announcement](https://blog.rust-lang.org/2026/08/04/enabling-polonius-alpha-on-nightly/),
    checked 2026-09-20. Explicit flag probes establish the local compiler result.

[^2]: [Next-generation trait solver design goals](https://goals.rust-lang.org/2025h2/next-solver.html)
    and [compiler guide](https://rustc-dev-guide.rust-lang.org/solve/trait-solving.html),
    checked 2026-09-20. The former describes goals, not current feature promises.

[^3]: [Rust Reference: dyn compatibility](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility),
    checked 2026-09-20 and corroborated by the local negative compile probe.
