# Polonius ownership design review

- Date: 2026-09-20.
- Scope: [ownership experiment](polonius-ownership-experiment.md),
  [technical design §3.1](peregrine-design.md), [ADR 002](adr-002-compiler-ownership-experiment.md),
  and [roadmap phase 2](roadmap.md).
- Method: three independent reviewing agents, each covering two Logisphere
  perspectives; findings synthesized and incorporated into the proposal.
- Verdict: **proceed with conditions for the bounded experiment**. This is not
  approval to impose exclusive compiler requirements on consumers.

## 1. Problem, constraints, and core bets

The experiment asks whether exclusive Polonius and new-solver support improves
Peregrine authoring enough to justify its maintenance cost. Entity identity,
resource-owned policy, middleware ordering, error precedence, and ownership
safety are fixed constraints. Implementation shape and compiler adoption remain
open. Success means measurable authoring benefit with semantic parity and a
sustainable consumer contract; compiler acceptance alone is insufficient.

| Bet                                          | Confidence after review                                                  | Remaining evidence                                                                          |
| -------------------------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------- |
| B1: Exclusive borrowing improves authoring.  | High for the isolated Polonius result; limited for framework ergonomics. | Paired resource and middleware edits reviewed independently.                                |
| B2: The new solver earns its requirement.    | Unproven; current outcomes are solver-independent.                       | Representative associated-type and future-bound fixtures, or a costed policy justification. |
| B3: Lifecycle and resource identity survive. | Plausible, conditional on application-owned middleware.                  | Complete traces, exact identity, cancellation, and `Send` contracts.                        |
| B4: Runtime gains justify costs.             | Unknown until representative measurements.                               | Hit/miss distributions, retained memory, compilation costs, and variability.                |
| B5: The compiler contract is sustainable.    | Unknown until external-tool checks.                                      | Per-tool compiler identities, flags, consumer builds, and upgrade rehearsal.                |

_Table 1: Core bets; confidence in a probe is not confidence in product
adoption._

## 2. Findings and incorporated refinements

The panel found no fundamental objection to conducting the experiment. The
following unresolved risks required clarification in the design; their runtime
validation remains future work, with every roadmap task still unchecked.

| Severity and perspective                        | Finding                                                                                               | Refinement and evidence gate                                                                                                                                 |
| ----------------------------------------------- | ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Unresolved risk — Pandalump, architecture       | Endpoint-local middleware ownership could omit hooks before routing or after an unmatched route.      | The application owns the authoritative sequence. Task 2.1.3 includes 404, `OPTIONS *`, and pre-routing completion traces.                                    |
| Unresolved risk — Telefono, contracts           | Generic async resource authoring could lose the baseline `Send` future guarantee at endpoint erasure. | Explicit future bounds and full invocation fixtures are required, including a rejected non-`Send` responder, in 2.1.2–2.1.3.                                 |
| Unresolved risk — Dinolump, long-term viability | The solver hypothesis lacked a concrete representative composition test.                              | Task 2.1.2 now compiles candidate lifetime-indexed, associated-type, and future bounds under both solvers; no difference is a valid result.                  |
| Unresolved risk — Wafflecat, complexity         | The critical-path experiment could expand into a second implementation programme.                     | Define an effort budget and minimal specimen before prototypes; inconclusive results select the compatible design or a bounded extension.                    |
| Unresolved risk — Doggylump, operations         | Published compiler claims depended on temporary files.                                                | The complete [probe archive](compiler-probe-evidence.md) now preserves eight sources and 32 outcomes. Task 2.1.1 still owns the maintained evidence harness. |
| Unresolved risk — Doggylump, operations         | A successful Cargo build could conceal rustdoc, editor, or analyser incompatibility.                  | Task 2.2.2 requires separate identities/configuration, omitted-flag defaults, incompatible-flag diagnostics, and recovery.                                   |
| Unresolved risk — Buzzy Bee, performance        | Hit-path savings may disappear in realistic request-local caches or larger generic stacks.            | Pre-register workload dimensions and numeric budgets; include cold/miss-heavy cases, route/stack scale, repeat counts, and variability in 2.2.1.             |
| Improvement — Pandalump, architecture           | Lifecycle wording required copying parameters despite the borrowing experiment.                       | The algorithm now requires immutable parameters; ownership representation remains an experimental choice.                                                    |
| Improvement — Wafflecat, complexity             | Calling solver adoption a policy choice could evade cost scrutiny.                                    | Task 2.2.3 must state a maintenance benefit and compare policy-only adoption with Polonius-only support.                                                     |
| Improvement — Buzzy Bee, performance            | The matrix omitted the compatible allocation-free-hit control.                                        | The `contains_key`/`get_mut` alternative now appears alongside the other measured probes.                                                                    |

_Table 2: Findings, incorporated design changes, and outstanding validation._

The remaining open question is marginal adoption value. The paired cache
functions expose identical caller signatures. More direct helper control flow
may justify a compiler requirement, but neither application-author benefit nor
a solver-specific benefit has yet been established.

## 3. Pre-mortem

### 3.1. A compiler upgrade blocks consumer releases

Six months after adoption, application compilation succeeds but rustdoc or
Whitaker rejects exclusive source with a different toolchain or flags. Consumer
builds and releases stop. The missed signal was CI testing only the repository
build; B5 was false. Prevent this with per-tool identities and effective flags,
an external consumer, a named upgrade owner, an upgrade rehearsal, and a
last-known-good pin. Tasks 2.2.2–2.2.3 own this evidence.

### 3.2. Miss-heavy requests regress under load

A hit-heavy benchmark favours the new map helper, while production requests
mostly initialize fresh caches. Extra lookups and retained backing storage
reduce capacity and increase tail latency. The missed signals were actual hit
ratios and retained-memory measurements; B4 was false. Task 2.2.1 must compare
cold, mixed, and hit-heavy workloads against pre-registered numeric budgets,
with repeat counts and variability rather than inferred speedups.

### 3.3. Cancellation loses completion work

A middleware author assumes reverse response hooks always run. A dropped
pipeline future skips asynchronous audit work, while operational records imply
normal completion. The blast radius is incomplete audit trails and misleading
request state. The missed signal was a suite testing ordinary unwinding only;
B3 was false. Task 2.1.3 must drop requests at awaited phase boundaries and
distinguish cancellation from normal and failed completion. Ownership cleanup
must never be documented as guaranteed asynchronous auditing or rollback.

## 4. Strongest alternative

Wafflecat's strongest alternative is an NLL-compatible framework using the same
narrow phase capabilities, application-owned lifecycle, resource-owned policy,
and generic resource authoring behind an erased router boundary. Standard entry
APIs or a justified borrowed-entry collection handle difficult helpers.

This retains entity-first ergonomics and middleware composition without an
exclusive Polonius requirement. It trades away some direct early-return helper
expressions. Standard `HashMap::entry` constructs owned keys on hits, while the
compatible double-lookup alternative avoids that construction at another cost.
Neither choice determines whole-framework performance. The comparison must use
this alternative, not a deliberately awkward monolithic-context implementation.

## 5. Ordered conditions and ownership

1. The API maintainer and pilot reviewer bound the specimen, effort, workloads,
   budgets, and author-review rubric before prototype results can bias them.
2. The implementation maintainer preserves reproducible compiler evidence and
   builds equivalent ownership/trait specimens through tasks 2.1.1–2.1.2.
3. The verification maintainer proves observable lifecycle parity and public
   invocation/ownership contracts through task 2.1.3.
4. The performance reviewer and toolchain maintainer complete separate cost
   and downstream-support evidence through tasks 2.2.1–2.2.2.
5. The decision owner records adoption, revision, or rejection in ADR 002
   through task 2.2.3, including any policy-only solver requirement and its
   costs.

These are proposed responsibilities; task 1.1.1 must assign the people. The
review closes documentary ambiguities, not the experimental bets. Production
adoption remains conditional on the evidence above.
