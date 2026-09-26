# Peregrine Web

_A study in architectural heresy: an entity-first web framework for Rust._

Falcon had a good idea: put related HTTP operations on a resource, give that
resource its dependencies, and let middleware see what the request is actually
about. Peregrine proposes bringing that architecture to Rust, with explicit
ownership and a request lifecycle you can follow without a séance.

**Status: design and experiments.** The repository currently contains a library
scaffold, a greeting function, and one placeholder test. The framework APIs
below are proposed; there is no HTTP server to run yet.

______________________________________________________________________

## Why Peregrine?

A route selects a resource. The resource groups its HTTP operations and
declares its policy. Middleware sees that same resource before the responder
runs.

That last part is the reason for the project.

- **Keep the endpoint together.** Methods, dependencies, and admission policy
  belong beside the resource they describe.
- **Make the lifecycle inspectable.** Request hooks, routing, resource hooks,
  dispatch, and response hooks have explicit ordering and failure rules.
- **Keep application boundaries intact.** A resource is an HTTP adapter around
  application services. Tenant authority, transactions, and business rules stay
  usable from workers and other entrypoints.
- **Make the trade-offs earn their keep.** Compare ownership ergonomics,
  dispatch costs, and maintenance effort against fair alternatives.

The good parts of an older architecture deserve a fair hearing. They can bring
benchmarks to the hearing, too.

______________________________________________________________________

## Quick start

### Get the scaffold

Install Git and a rustup-managed Rust toolchain. The checkout selects its
pinned nightly and components automatically. On x86-64 Linux, its build
configuration also requires `clang` and `mold`. See the
[users' guide](docs/users-guide.md) for build-tool details.

```bash
git clone https://github.com/leynos/peregrine-web.git
cd peregrine-web
cargo test
```

A successful run currently reports one passing placeholder test. That checks
the scaffold, not the proposed HTTP contracts.

### Explore the current library

```bash
cargo doc --no-deps
```

Open `target/doc/peregrine_web/index.html` to inspect the current library API.
The [technical design](docs/peregrine-design.md) contains the proposed resource
and middleware examples; the [roadmap](docs/roadmap.md) tracks the work needed
to make them runnable.

______________________________________________________________________

## The proposed framework

- **Resource-first routing.** Trait-based resources group HTTP responders and
  declare supported methods and typed policy. Resources can own concrete
  services or application-defined trait objects.
- **Three middleware phases.** Request and resource hooks run forwards;
  response hooks run in reverse order on ordinary completion, including early
  responses and errors. Cancellation has its own contract.
- **Policy at the destination.** Authorization runs after routing and before
  dispatch, including automatic HEAD and OPTIONS handling. Application services
  retain tenant and object-level checks across entrypoints.
- **Explicit ownership.** A request owner controls lifecycle state, with narrow
  phase views under evaluation. Shared resources retain their dependencies;
  streams own what they need after response commitment.
- **Consistent HTTP presentation.** Typed input helpers, immutable request
  facts, and final failure rendering keep transport concerns at the boundary.
- **Bounded serving and bodies.** Hyper and Tokio provide the proposed transport
  foundation, with explicit body limits, backpressure, admission, deadlines,
  and graceful drain. The application owns startup and adapter cleanup.

Dynamic dispatch is a deliberate option at the heterogeneous router boundary.
Its cost still needs measuring. A database query is not a benchmark exemption.

### The compiler experiment

A key experiment asks whether designing exclusively for **Polonius and the new
trait solver** makes borrowing and middleware composition simpler while keeping
the resource-first API. It compares that candidate with an implementation
compatible with the old checker, using the same ownership boundaries.

The isolated probes show Polonius helping specific early-return borrowing
patterns. They have not shown a solver-specific ergonomic gain. Narrow phase
views work with either checker; overlapping mutable borrows and native async
trait-object dispatch do not become valid by wishing harder.

The repository's nightly pin is not a decision to require those experimental
flags from consumers. See the
[ownership experiment](docs/polonius-ownership-experiment.md) for side-by-side
examples, evidence, and adoption criteria.

______________________________________________________________________

## Learn more

- [Terms of reference](docs/terms-of-reference.md) — purpose, scope, and open
  decisions.
- [Technical design](docs/peregrine-design.md) — proposed APIs and lifecycle
  contracts.
- [Roadmap](docs/roadmap.md) — goals, ideas, experiments, and delivery tasks.
- [Actix extension case study](docs/actix-v2a-middleware-case-study.md) — where
  middleware helps and where ordinary helpers or services are clearer.
- [Hexagonal application case study](docs/hexagonal-application-case-study.md)
  — lessons from Corbusier, Wildside, and Episodic.
- [Pachislot extension design](docs/pachislot-extension-design.md) — correlation
  propagation and a proposed WebSocket companion hosted inside Peregrine.
- [Users' guide](docs/users-guide.md) — current scaffold and build commands.
- [Developers' guide](docs/developers-guide.md) — contributor workflow and
  proposed integration boundaries.
- [Documentation contents](docs/contents.md) — the complete documentation map.

______________________________________________________________________

## Licence

ISC — see [LICENSE](LICENSE) for details.

______________________________________________________________________

## Contributing

Design critiques, small compiler reproductions, and realistic application
examples are welcome. Help establish which ideas make a service easier to
maintain and which merely give the types more exercise.

Start with the [developers' guide](docs/developers-guide.md), follow
[AGENTS.md](AGENTS.md), and connect proposed changes to the roadmap's evidence
criteria. Framework implementation remains ahead of us.

A [df12 Productions](https://df12.studio/) project.
