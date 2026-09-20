# Architectural decision record (ADR) 002: Compare exclusive compiler ownership

## Status

Proposed. This record proposes a design experiment, not exclusive compiler
adoption or an implemented public API. The experiment has not yet passed its
product acceptance criteria.

## Date

2026-09-20.

## Context and problem statement

Peregrine's resource-first model and visible middleware lifecycle do not
require one universally mutable context or one erased call per hook. Narrow
phase views and typed endpoint adapters can preserve the model while exposing
ownership more precisely. Polonius accepts some direct conditional-return
borrowing patterns rejected by NLL; the new solver's separate value needs
demonstration.

The
[technical design §3.1](peregrine-design.md#31-experimental-compiler-and-ownership-direction)
and [experiment specification](polonius-ownership-experiment.md) define the
comparison. [ADR 001](adr-001-resource-aware-lifecycle.md) still governs the
proposed lifecycle semantics, subject to acceptance.

## Decision drivers

- Preserve resource identity, resource-owned policy, and independent response
  hooks across implementation strategies.
- Compare the best compatible API with the exclusive compiler candidate.
- Attribute improvements to ownership design, Polonius, and the solver
  separately.
- Establish a usable downstream build contract before imposing compiler flags.

## Options considered

| Option                                             | Benefit                                                                              | Cost                                                                                  |
| -------------------------------------------------- | ------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------- |
| NLL-compatible ownership redesign                  | Narrow capabilities and entity-first ergonomics without a Polonius-only requirement. | Some conditional borrowing helpers require different control flow or collection APIs. |
| Pure Polonius plus new-solver candidate            | Direct early-return borrowing and freedom to test solver-dependent composition.      | Compiler binding, downstream setup, and analyser compatibility require evidence.      |
| Polonius-only requirement with solver independence | Retains demonstrated borrowing benefits if solver necessity is unproven.             | Changes the combined hypothesis and needs an explicit revised support decision.       |
| Permanently maintain both implementations          | Offers consumers a checker choice.                                                   | Doubles support and semantic-conformance work; excluded from the proposed product.    |

_Table 1: Compiler-posture alternatives._

## Proposed direction

Conduct the
[roadmap phase 2](roadmap.md#2-test-a-pure-polonius-and-new-solver-implementation)
experiment before selecting a production ownership model. Build comparable
resource and middleware prototypes; use a four-way flag matrix to separate
compiler mechanisms. Candidate B has no compatibility fallback. Standard entry
APIs and split views are permitted in candidate A; they are not concessions
reserved for the new compiler.

Retain all HTTP, authorization, error, and cancellation invariants in both
candidates. Explicitly permit owned data and erased dispatch where lifetime or
heterogeneous storage boundaries need them. Treat compiler adoption as a
maintained consumer contract, not a local development preference.

## Known risks and limitations

Measured helper acceptance does not demonstrate a better framework API. A
single-lookup hit path can add work on misses. Generic middleware can increase
compile time and binary size. Request-local borrows cannot escape through a
response stream or detached task without transferring suitable ownership. The
new solver has no demonstrated unique benefit in the current probe set.

The [six-perspective review](polonius-design-review.md) supports a bounded
experiment with conditions. Its findings are incorporated in the specification
and phase 2 acceptance criteria; prototype evidence remains outstanding.

## Outstanding decisions

The API maintainer and pilot reviewer must set evaluation budgets, review the
paired implementation, and record adoption, revision, or rejection. Acceptance
requires downstream builds and all required analyser gates, including the
repository's separate Whitaker compiler, to work with the selected contract.
Record any choice to require the solver for policy reasons separately from
claims of ergonomic necessity. Until then, both this ADR and the implementation
direction remain proposed.
