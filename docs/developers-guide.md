# Developer Guide

This guide explains the contributor workflow for the generated
Peregrine Web project.

The [technical design](peregrine-design.md) describes the proposed framework;
[ADR 001](adr-001-resource-aware-lifecycle.md) remains proposed. Consult the
[repository layout](repository-layout.md) for file ownership and the
[potential roadmap](roadmap.md) for delivery sequencing. The current library
is a generated stub, so these proposals are not available APIs.

The [compiler ownership experiment](polonius-ownership-experiment.md) records
explicit four-way compiler probes. Its flags are experimental; this proposal
does not change the repository's build settings or consumer requirements.
Implementation must validate downstream and analyser compatibility before
[ADR 002](adr-002-compiler-ownership-experiment.md) can be accepted.

The [extension case study](actix-v2a-middleware-case-study.md) and
[ADR 003](adr-003-http-integration-boundaries.md) distinguish HTTP lifecycle
integration from application services. Prefer metadata parsers and response/body
helpers for value-level work. Durable mutation completion must not depend on
response hooks. These are proposed internal contracts, not implemented helpers.

The [hexagonal application case study](hexagonal-application-case-study.md) and
[ADR 004](adr-004-application-port-boundaries.md) define proposed application
integration contracts. Construct resource dependencies explicitly; keep
application ports free of framework types, and permit concrete adapters only
at composition roots and their own implementation boundaries. Examples must
separate request middleware from unit-of-work and application lifespan owners.
Boundary checks complement behavioural port tests; they do not replace them.

The [extension design](pachislot-extension-design.md) and proposed
[ADR 005](adr-005-extension-and-upgrade-boundaries.md) separate correlation policy
from propagation adapters and HTTP upgrade from Pachislot message processing.
An upgrade plan must own everything that outlives the HTTP request. Keep
connection and message lifetimes explicit in examples and verify compatibility
against pinned source behaviour before describing it as supported.

## Local Workflow

Use `make all` as the public entrypoint for formatting, linting, and tests.
`make lint` runs rustdoc, Clippy, and Whitaker. `make test` prefers
`cargo nextest run` and falls back to `cargo test` when cargo-nextest is not
available. `make check-fmt` verifies Rust formatting with
`cargo fmt --all -- --check`, and `make fmt` formats Rust sources with
nightly `rustfmt` and Markdown with `mdformat`. `make typecheck`
type-checks without building via `cargo check`. `make audit` derives the
Rust workspace root with `cargo metadata`, logs workspace member manifests,
and runs `cargo audit` once from the workspace root. PR CI skips `make audit`
and the audit-only setup when `github.actor` is `dependabot[bot]`; that keeps
whole-lockfile advisories from blocking unrelated Dependabot PRs while human
PRs retain the audit gate. The compensating control is
`.github/workflows/audit.yml`, which runs weekly and can also be triggered
manually. `make coverage` uses `cargo llvm-cov` with `lld`.

GitHub Actions Act validation lives in `.github/workflows/act-validation.yml`.
The main `.github/workflows/ci.yml` workflow deliberately does not run
`make test WITH_ACT=1`; the separate Act workflow runs those slower
container-backed checks in parallel.

A scheduled `.github/workflows/mutation-testing.yml` workflow also runs
`cargo-mutants` via the shared reusable workflow, daily and on manual
dispatch. It is informational and does not gate pull requests. Dependabot
keeps its pinned reusable-workflow SHA current. See the user guide's
"Scheduled Mutation Testing" section for behaviour, and promote surviving
mutants into new tests.

## Tooling

Development builds use Cranelift for debug code generation. On Linux targets,
`.cargo/config.toml` configures clang to link with `mold` so debug builds link
quickly. Coverage generation uses `lld` because LLVM coverage tooling expects
LLVM-compatible linker behaviour.

Install `clang`, `lld`, `mold`, `python3`, and `cargo-audit` before running the
full generated workflow locally on Linux.

## Spelling policy

Markdown uses en-GB-oxendict spelling enforced by the shared
`typos-config-builder` gate. Run `make spelling`.

`typos.toml` is generated output. The gate regenerates it on every run from the
live shared estate dictionary and the `typos.local.toml` overlay, so a word
added to the shared dictionary needs no change here. Because the dictionary is
live, `typos.toml` must never be drift checked in continuous integration. Add
narrow repository-specific identifier, API, proper-name, or fixture exceptions
to `typos.local.toml`; hand-editing `typos.toml` is not supported and any edits
are overwritten on the next run.

### Security audit ignores

Security audit jobs may set `CARGO_AUDIT_IGNORES` for narrowly scoped
RustSec advisories that affect unused or tooling-only dependency paths. Keep
each ignore tied to a documented runtime impact analysis, and remove it when
the affected dependency leaves the graph or the project starts using the
advised runtime path.

## Workflow pins and Dependabot

Dependabot owns the upgrade of GitHub Actions and reusable workflows,
including calls into `leynos/shared-actions`. Contract tests that assert a
caller's exact commit SHA create a lockstep dependency: every time Dependabot
opens a bump PR, the test fails until a human edits the pinned constant to
match. That defeats the purpose of automated dependency updates and turns a
routine bump into a manual chore.

The narrow `RUSTFLAGS_PASSTHROUGH_REVISION` exception applies only while no
independent capability probe can establish that the shared `setup-rust` action
accepts the required `rustflags` input. In that case, assert the first revision
that provides the capability and document this boundary beside the test. Remove
the literal revision assertion once an independent capability probe is
available.

Contract tests may still verify the *shape* of a reusable-workflow caller.
They must not verify the specific SHA value.

- Do assert the workflow references the correct reusable workflow path.
- Do assert the ref is pinned to a full 40-character commit SHA, not a
  mutable branch such as `main` or `rolling`.
- Do assert the expected `on:` triggers, least-privilege `permissions:`, and
  the inputs the caller relies on.
- Do not hard-code the current SHA value as an expected string. Match it with
  a pattern instead.
- Do not fail a test purely because Dependabot bumped the pinned SHA.

```python
import re

SHA_RE = re.compile(r"^[0-9a-f]{40}$")

def test_uses_pinned_full_sha(caller_step):
    ref = caller_step["uses"].split("@")[-1]
    assert SHA_RE.match(ref), f"expected a 40-hex commit SHA, got {ref!r}"
```

If a workflow's behaviour genuinely depends on a feature only present from a
particular commit onwards, express that as a comment or a changelog note, not
as a test assertion on the SHA string. The sole exception is the
`RUSTFLAGS_PASSTHROUGH_REVISION` boundary above: until an independent probe can
confirm that `setup-rust` supports `rustflags`, document and assert the first
capable revision. Remove that literal revision assertion once the probe exists.
