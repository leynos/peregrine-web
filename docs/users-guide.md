# User Guide

This guide covers the repository's public build commands and the generated
Peregrine Web project after rendering it from the template. For contributor
gate details and the compiler and linker configuration, see the
[developers' guide](developers-guide.md).

## Build the Repository

Install Git, GNU Make, Bash, and rustup. The installer also needs `curl`,
`tar`, and `sha256sum` on native x86-64 GNU Linux to fetch and verify `mold`.
Install Clang on that host for the native build. From a checkout, install the
pinned build tools and check that the host is ready:

```sh
make install-build-tools
make check-build-tools
```

`make install-build-tools` installs the nightly and components pinned in
`rust-toolchain.toml`. On native x86-64 GNU Linux it also downloads the pinned
`mold` archive and verifies its checksum. That host needs Clang; other hosts
use their platform linker. `make coverage` additionally requires Clang and
`ld.lld`.

The repository's supported build Make targets are:

- `make build` — build the debug library.
- `make test` — run tests and doctests.
- `make lint` — run rustdoc, Clippy, and Whitaker checks.
- `make typecheck` — check the code without building an executable.
- `make release` — build a production release artefact.
- `make package` — build and verify a Cargo package.
- `make coverage` — generate LCOV coverage data.
- `make all` — run formatting, lint, test, and spelling gates in sequence.

Cargo checks and tests default to `--all-targets --all-features`; set
`CARGO_FLAGS` or `TEST_FLAGS` to change those selections. `BUILD_JOBS` can
forward Cargo job options to builds and tests. Development routes preserve
exported `RUSTFLAGS` and add the repository's required flags. The release,
package, and coverage routes set their own compiler and linker options. Make
preflight rejects cross-target and unsupported linker or compiler overrides
before compilation.

Build, test, lint, and typecheck routes check their pinned prerequisites before
compiling. Coverage has its own preflight for its linker requirements. A
missing or mismatched pinned prerequisite, or an unsupported cross-target flag,
stops the relevant preflight with a `build-tools:` diagnostic. Messages for
missing tools include an installation hint where available. These Make targets
use the native host route; cross-target Make builds are unsupported. Detailed
flag behaviour, coverage setup, and CI checks are in the
[developers' guide](developers-guide.md).

## Generated Project Structure

Generated projects use Rust 2024 and include documented starter code. Library
projects render `src/lib.rs`. Application projects render `src/main.rs`,
`src/lib.rs`, release automation, and `[package.metadata.binstall]` metadata
for binary installation.

## Review the proposed Mornington contracts

Peregrine currently exports scaffolding, not a runnable HTTP framework. The
owner selected a Mornington reimplementation as its proving ground: HTTP/1.1
behind Traefik, explicit trusted proxy peers, JSON long polling, and no
upgrades. HTTP/2 and native TLS are deferred. No API below can be invoked yet.

The [technical design](peregrine-design.md) is the canonical proposed contract:
§6 distinguishes ordinary completion from cancellation, §7.2 chooses the
winning failure, §5.1 applies HEAD and no-content rules, and §9.2 lists the
proving-ground configuration. On ordinary completion, every response hook runs
once in reverse order, including hooks without forward setup. A hook failure
selects sanitized 500 while remaining hooks continue. Dropping a future, panic,
disconnect, or deadline cancellation does not promise the remaining hooks.

Before response handoff a supervised execution deadline can produce minimal
504; after handoff, body failure terminates transfer without replacing status.
A successful application mutation can therefore coexist with HTTP/transfer
failure. Response hooks are not rollback or guaranteed cleanup. Mornington's
application owns authorization, durable storage, and effect recovery.

The proposed 30-second execution deadline allows the pilot's 25-second long
poll. Byte, idle, connection, transfer, and drain limits have separate scopes.
Socket/parser limits do not apply to in-process execution; engine/body-owner
limits do. Only the named request-stream total-byte opt-out permits unlimited
bytes; idle and admission accounting remain finite. These are operator-review
inputs, not measured release-capacity guarantees. See
[ToR §9.1](terms-of-reference.md#91-mornington-pilot-and-comparative-review)
for the pinned Actix-design comparator and acceptance scope.
