# Developer Guide

This guide explains the contributor workflow for the generated Peregrine Web
project.

The [technical design](peregrine-design.md) describes the proposed framework;
[ADR 001](adr-001-resource-aware-lifecycle.md) remains proposed. Consult the
[repository layout](repository-layout.md) for file ownership and the
[potential roadmap](roadmap.md) for delivery sequencing. The current library is
a generated stub, so these proposals are not available APIs.

The [compiler ownership experiment](polonius-ownership-experiment.md) records
explicit four-way compiler probes. Its flags are experimental; this proposal
does not change the repository's build settings or consumer requirements.
Implementation must validate downstream and analyser compatibility before
[ADR 002](adr-002-compiler-ownership-experiment.md) can be accepted.

The [extension case study](actix-v2a-middleware-case-study.md) and
[ADR 003](adr-003-http-integration-boundaries.md) distinguish HTTP lifecycle
integration from application services. Prefer metadata parsers and
response/body helpers for value-level work. Durable mutation completion must
not depend on response hooks. These are proposed internal contracts, not
implemented helpers.

The [hexagonal application case study](hexagonal-application-case-study.md) and
[ADR 004](adr-004-application-port-boundaries.md)
define proposed application integration contracts. Construct resource
dependencies explicitly; keep application ports free of framework types, and
permit concrete adapters only at composition roots and their own implementation
boundaries. Examples must separate request middleware from unit-of-work and
application lifespan owners. Boundary checks complement behavioural port tests;
they do not replace them.

The [extension design](pachislot-extension-design.md) and proposed
[ADR 005](adr-005-extension-and-upgrade-boundaries.md) separate correlation
policy from propagation adapters and HTTP upgrade from Pachislot message
processing. An upgrade plan must own everything that outlives the HTTP request.
Keep connection and message lifetimes explicit in examples and verify
compatibility against pinned source behaviour before describing it as supported.

## Local Workflow

After rendering a project, initialize version control and stage its files
before running repository gates:

```sh
git init
git add -A
```

The following workflow describes contributor checks and their file-selection
rules.

Use `make all` as the public entrypoint for formatting, linting, tests, and
spelling. It runs these gates one at a time even when invoked with `make -j`.
`make lint` runs `lint-clippy` (rustdoc, then Clippy) before `lint-whitaker`,
also under `make -j`. Whitaker's temporary Dylint driver is built outside the
repository, so the recipe removes inherited compiler and linker overrides
before starting it. Dylint then checks the repository from its root and
discovers `.cargo/config.toml`; that check uses the development backend and
linker defaults. The recipe sends warnings-as-errors to the repository check
through Dylint's `DYLINT_RUSTFLAGS` input. `make test` prefers
`cargo nextest run` and falls back to `cargo test` when cargo-nextest is not
available. `make check-fmt` verifies Rust formatting with
`cargo fmt --all -- --check` and Markdown formatting via `mdtablefix --check`
using the configured selection and formatting rules. `make fmt` formats Rust
with the pinned `rustfmt`, then applies the Markdown formatter and
`markdownlint-cli2 --fix`. `make typecheck` type-checks without building via
`cargo check`. `make audit` derives the Rust workspace root with
`cargo metadata`, logs workspace member manifests, and runs `cargo audit` once
from the workspace root. PR CI skips `make audit` and the audit-only setup when
`github.actor` is `dependabot[bot]`; that keeps whole-lockfile advisories from
blocking unrelated Dependabot PRs while human PRs retain the audit gate. The
compensating control is `.github/workflows/audit.yml`, which runs weekly and
can also be triggered manually. `make coverage` uses `cargo llvm-cov` with
`lld`.

`make check-fmt` checks Git-tracked Markdown and untracked files Git does not
ignore; files need not be staged. Markdown lint covers every matching file. The
spelling gate enumerates files with `git ls-files`, so it checks tracked files
only. Stage new documentation before relying on spelling results.

The first `make spelling` run regenerates the tracked `typos.toml`; commit that
generated file with the change. The gate derives it from the shared dictionary
and `typos.local.toml`, so add repository-specific exceptions to the local
overlay instead of editing generated output.

Every pull request runs the hosted `build-test` job as the authoritative
repository gate. Its Rust test suite includes workflow-structure and command
contract tests: workflow YAML and pins are checked from source, while real Make
recipes and shell scripts run with controlled external executors. These tests
prove invocation shape, configuration, ordering, and failure propagation
without compiling the repository again for each contract.

Table-driven contract regressions use the `rstest` dev dependency with its
optional asynchronous-timeout and crate-renaming features disabled. Private
test-support helpers remain owned by their named contract modules. Workflow
predicates compose event, action-input, and step-policy checks without changing
their YAML acceptance rules. `MakeOptions` applies caller settings and executor
controls after the harness removes baseline child overrides. `RecordCursor`
belongs only to the Act Make record parser and preserves its existing NUL field
protocol. The unset-environment assertion is shared only by the driver and
configuration boundaries and their regression tests; each boundary owns its
variable names and diagnostics. These helpers are not a cross-suite validation
framework.

The separate `.github/workflows/act-validation.yml` workflow runs only on
manual dispatch. It exercises the full workflow through Act in Docker to check
local Act compatibility, including step routing, action execution, shell
availability, and runner protocols. That result does not establish parity with
GitHub-hosted runners; the hosted `build-test` job remains the repository gate.
Contributors can also opt into this local integration run with
`make test WITH_ACT=1`. It first runs the outer Rust tests, then Act runs the
CI workflow with its ordinary test step and without hosted coverage collection.
On Linux, the host needs Docker, Act, `clang`, `lld`, `python3`, and
`cargo-audit`, plus a GitHub token for nested actions. The ordinary PR
contracts and hosted lint retain the cold Whitaker driver construction and real
repository check, so the stubbed command contracts do not replace compiler
integration evidence.

Use `make act-contract-smoke` for the focused Act compatibility check. It runs
an ignored Rust test against a derived copy of the CI workflow, with explicit
fixture actions and controlled command executors; it checks step routing and
failure propagation without rerunning the full repository suite in Docker. Use
`make act-validation` or `make test WITH_ACT=1` when you need the full local
Act run. To exercise actual Dylint driver construction on both cache states, run
`make whitaker-driver-integration`; it creates a private driver directory and
runs `make lint-whitaker` cold, then warm. Hosted CI's lint step also uses a
fresh runner-temporary driver directory for each job, preserving cold driver
construction as part of the real repository gate.

A scheduled `.github/workflows/mutation-testing.yml` workflow also runs
`cargo-mutants` via the shared reusable workflow, daily at 09:15 UTC by default
and on manual dispatch from the Actions tab. It is informational and does not
gate pull requests. Dependabot keeps its pinned reusable-workflow SHA current.
Mutation testing introduces small source changes and checks that tests fail in
response. Promote surviving mutants into tests that cover the missed behaviour.

Scheduled runs mutate files changed within the detection window; manual runs
mutate the whole crate across shards. When adopting the workflow in another
repository, choose an unclaimed daily slot. The `mutation` job uses a
least-privilege token with `contents: read` and `id-token: write` for workflow
source resolution. The job summary reports per-target outcomes and surviving
mutants, while each shard uploads its `mutants.out/` directory as a
`mutation-report-*` artefact. Runs with no relevant changes finish with a skip
message. Surviving mutants and timeouts leave the run green; investigate failed
runs for usage errors, an already-failing test baseline, or internal errors.

`coverage-main.yml` measures coverage on pushes to `main` and on dispatch from
`main`, and is the only CodeScene caller; `ci.yml` measures pull requests for
their own ratchet, at the same `generate-coverage` revision with
`publish-artefact: 'false'`, and names no CodeScene token, host or command. The
publisher job runs in the `codescene` environment, which admits `main` alone
and holds `CS_ACCESS_TOKEN` as an environment secret. A
`Check CodeScene token availability` step (id `codescene_token`) runs exactly
`echo "available=${{ secrets.CS_ACCESS_TOKEN != '' }}" >> "$GITHUB_OUTPUT"`,
with no `if:` and no `env`. The upload runs only when that output is `true` and
`github.ref` is `refs/heads/main`, takes the token as its `access-token` input
so the workflow binds it in no `env` of its own, and uploads with
`mode: upload` and no checksum input. Publisher runs share the concurrency group
`coverage-main-${{ github.ref }}` with `cancel-in-progress: false`: a running
publisher is never cancelled, and a newer trigger replaces an older pending
run, so the newest trigger's run is the one that publishes. A merge made by the
Dependabot automerge workflow's `GITHUB_TOKEN` fires no push event, so it
publishes nothing until a dispatch from `main` or the next push.
`tests/codescene_publisher.rs` holds the shape over the committed workflows.

## Tooling

Development builds use Cranelift for debug code generation. Every `rustflags`
source in `.cargo/config.toml` enables the parallel `rustc` frontend with
`-Zthreads=8`. On native x86_64 GNU Linux, Cargo selects
`scripts/native-clang-linker.sh` and `-fuse-ld=mold`. The wrapper gives Clang
`BUILD_TOOLS_PREFIX/bin` as its first linker search directory, so bare Cargo
builds use the pinned `ld.mold` rather than a system copy. The wrapper refuses
an absent, diverted, or wrong-version linker before a development link; it
shares this verification with Make preflight through
`scripts/build-tools-common.sh`. Cargo applies one `rustflags` source; an
assigned `RUSTFLAGS` replaces them all, so the Makefile restates both flags in
`DEV_RUST_FLAGS` for targets that assign `RUSTFLAGS`, appending them to any
inherited caller flags (including setup-rust's CI value). Use `make release`
for a production build and `make package` for a verified, publishable Cargo
archive. Both commands assign `RELEASE_RUST_FLAGS`, select LLVM for the dev and
release profiles, clear inherited encoded Rust flags, and select Clang directly
on native x86_64 GNU Linux. The package includes `.cargo/config.toml`; its
verification build therefore needs the same explicit production route. Direct
`cargo build --release` and `cargo package` still inherit Cargo's development
`rustflags` and are not production routes. Coverage assigns its own flags,
selects LLVM for the dev profile, and uses `lld` because LLVM coverage tooling
expects LLVM-compatible code generation and linker behaviour.
`tests/build_standard_contract.rs` checks the configuration sources and recipes.

Whitaker keeps its two Cargo operations separate. Dylint builds its temporary
driver outside the repository and receives no application `RUSTFLAGS`, encoded
flags, backend override, target, or linker override. The repository check runs
from the repository root, where Cargo reads the defaults above; Dylint adds
`-D warnings` and the configured Polonius flags only to that check through
`DYLINT_RUSTFLAGS`. This preserves Cranelift, `-Zthreads=8`, and the native
wrapper/mold route for lint while leaving the coverage route on LLVM and `lld`.
The clean driver boundary also removes target-specific Rust flags, which Cargo
would otherwise append after the configured mold linker argument. Preflight
rejects `--target` and `--target=<triple>` while allowing `--target-dir`.

## Lint baseline

The single-crate lint tables live in the root `Cargo.toml`; new packages in a
future workspace must inherit the workspace lint tables. `clippy.toml` sets
cognitive complexity to 9, argument count to 4, function length to 70 lines,
and nesting depth to 4. Fix findings at their source. Keep exceptions narrow,
documented, and reasoned; do not use lint allowances to defer work. Code that
reads environment values should receive an injected `mockable::Env` or a narrow
equivalent, and tests should use `MockEnv` or a child process with controlled
`Command::env` settings. The pinned nightly in `rust-toolchain.toml` supplies
`clippy`, `rustfmt`, `rust-analyzer`, `rustc-codegen-cranelift-preview`, and
`llvm-tools-preview` for the required lint, formatting, analysis, development,
and coverage routes.

Rust denies unknown, renamed, and removed lints, unsafe code, and missing
documentation. Rustdoc denies missing crate-level documentation, broken
intra-doc links, private intra-doc links, bare URLs, invalid HTML tags, invalid
code-block attributes, and unescaped backticks. Clippy denies assertions
without diagnostic messages and rejects direct process-environment readers,
iterators, and mutation functions. Warnings fail documentation, lint, test, and
doctest gates.

Run `make install-build-tools` after checking out the project. On native x86_64
GNU Linux, it installs the pinned `mold` release and verifies its archive
against `tools/mold/SHA256SUMS`, and installs the repository's pinned nightly
with its required components, including rust-analyzer. On x86_64 GNU Linux,
`make check-build-tools` asks Clang which `ld.mold` it will execute, checks its
link plan and version against `tools/mold/VERSION`, and fails if Clang selects
a different executable. It also checks that the nightly and every pinned
component are installed. It checks clang for native Linux builds, and
`make coverage` also checks clang and lld. Development Make targets run this
check before Cargo. The Makefile exports `BUILD_TOOLS_PREFIX` (default
`~/.local`) for the native Clang wrapper and prepends its `bin` directory to
`PATH` for direct tools. The installer and checker belong to the root build
workflow. Callers may use them directly or through Make targets. CI, Act,
mutation testing, and the main coverage job install the tools before running
tests. The measured coverage command and release build still use their separate
linker flags. The supported `mold` configuration is native x86_64 GNU Linux;
x86_64 Linux musl and cross-target builds do not select this route. Make
preflight rejects cross targets and encoded Rust flags that could bypass the
selected linker or flags. It also rejects a non-Cranelift development backend
override; coverage explicitly selects LLVM after its coverage-specific
preflight. An explicit native target linker must remain the repository's Clang
wrapper so its search directory and the `mold` link argument apply together.

The `mold` 2.41.0 version and x86_64 archive digest in `tools/mold/` come from
the merged Netsuke build-standard follow-up
`4af7b348aa09de5a25fbd2d4f9c396999baedbc8`. The installer downloads the release
archive from `rui314/mold`, verifies the recorded digest, and fails if the
download, checksum, unpack, or toolchain installation fails.

Install `clang`, `lld`, `python3`, and `cargo-audit` before running the full
generated workflow locally on Linux. `make install-build-tools` supplies the
pinned `mold` and nightly toolchain.

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

Security audit jobs may set `CARGO_AUDIT_IGNORES` for narrowly scoped RustSec
advisories that affect unused or tooling-only dependency paths. Keep each
ignore tied to a documented runtime impact analysis, and remove it when the
affected dependency leaves the graph or the project starts using the advised
runtime path.

## Workflow pins and Dependabot

Dependabot owns the upgrade of GitHub Actions and reusable workflows, including
calls into `leynos/shared-actions`. Contract tests that assert a caller's exact
commit SHA create a lockstep dependency: every time Dependabot opens a bump PR,
the test fails until a human edits the pinned constant to match. That defeats
the purpose of automated dependency updates and turns a routine bump into a
manual chore.

The narrow `RUSTFLAGS_PASSTHROUGH_REVISION` exception applies only while no
independent capability probe can establish that the shared `setup-rust` action
accepts the required `rustflags` input. In that case, assert the first revision
that provides the capability and document this boundary beside the test. Remove
the literal revision assertion once an independent capability probe is
available.

Contract tests may still verify the *shape* of a reusable-workflow caller. They
must not verify the specific SHA value.

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

## Markdown formatting

Markdown follows the estate's `markdown-formatting-baseline` rule.

- `make fmt` rewrites Markdown with
  `mdtablefix --in-place --git --include-untracked --wrap --renumber --breaks
  --ellipsis --fences`,
  then runs `markdownlint-cli2 --fix "**/*.md"`.
- `make check-fmt` runs the same mdtablefix command with `--check` in place of
  `--in-place`, and fails when any file would change.
- `--git --include-untracked` selects the Markdown files Git tracks plus the
  untracked files Git does not ignore, so a new document is checked before it
  is staged.
- `.markdownlint-cli2.jsonc` carries the canonical markdownlint configuration.
  Keep its `config` entries and `ignores` globs; add repository-specific rules
  or globs beside them.
- CI installs mdtablefix 0.6.0 with the shared `install-mdtablefix` action
  before `make check-fmt`, and lints Markdown with
  `DavidAnson/markdownlint-cli2-action` over `**/*.md`.
- `tests/markdown_wiring.rs` holds that wiring by contract: `check-fmt` must
  run `mdtablefix --check --git --include-untracked` with its exit status
  reaching Make, the CI job must install mdtablefix before `make check-fmt`,
  and every lint action step must lint `**/*.md`. Each clause is also tested
  against weakened and equivalent fixtures.

Install mdtablefix 0.6.0 or later locally with
`cargo binstall --no-confirm mdtablefix@0.6.0`, or
`cargo install --locked mdtablefix@0.6.0`. Install markdownlint-cli2 with
`bun add --global markdownlint-cli2` or
`npm install --global markdownlint-cli2`.
