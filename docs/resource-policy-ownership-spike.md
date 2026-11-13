# Resource-policy and ownership spike evidence

This ledger records what the experimental specimen under `src/spike/` actually
demonstrated for roadmap task 1.1.3 on the pinned compiler. It maps each
witness in the
[spike plan](execplans/1-1-3-compile-resource-policy-and-async-ownership-spike.md)
to its fixture, command, and observed result, and it states what the
experiment does not establish.

The specimen is exported as `peregrine_web::spike` so compile fixtures exercise
the real library surface. It is not a framework, router, server, or
authorization provider, and nothing here promises a stable interface.

## Scope

The specimen answers four bounded questions:

- Can a resource object own its typed policy and its injected dependencies?
- Can registration freeze the effective method and policy before admission?
- Can an erased request body stay `Send` but not `Sync` across a real await?
- Can one conditional invocation be expressed without route-policy tables,
  service locators, or shared mutable application state?

Runtime identity, body-adapter, and cancellation witnesses are recorded in the
M2 sections of this ledger. Full HTTP serving, production authorization, and
lifecycle proofs remain out of scope.

## Design traceability

Each witness group answers a design obligation rather than a free-standing
experiment. Every result below was observed on the compiler recorded in
[Compiler and build posture](#compiler-and-build-posture); the reproducing
commands appear with each group, and the limits follow each table.

Table: Witness groups, their design and ADR anchors, and the obligation each
group partially discharges

| Group      | Design anchor                                                                                                                                         | ADR anchor                                                            | Obligation                                                                              |
| ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| V-OWN      | [§3.1](peregrine-design.md#31-experimental-compiler-and-ownership-direction), [§11 I6](peregrine-design.md#11-correctness-and-evidence)               | [ADR 002](adr-002-compiler-ownership-experiment.md) (proposed)        | V6: public async objects and futures meet their ownership bounds                        |
| V-IDENTITY | [§4](peregrine-design.md#4-resources-method-metadata-and-policy), [§11 I3](peregrine-design.md#11-correctness-and-evidence)                           | [ADR 007](adrs/adr-0007-resource-policy-and-body-erasure-boundary.md) | V3: denied resource policy never reaches a responder, with identity frozen at admission |
| V-BODY     | [§3](peregrine-design.md#3-application-and-context-contracts), [§11 I4](peregrine-design.md#11-correctness-and-evidence)                              | [ADR 007](adrs/adr-0007-resource-policy-and-body-erasure-boundary.md) | V4/I4 body-adapter subset: frames, trailers, and terminal errors survive erasure        |
| V-CANCEL   | [§3.1](peregrine-design.md#31-experimental-compiler-and-ownership-direction), [§11.1](peregrine-design.md#111-contract-review-and-later-verification) | [ADR 007](adrs/adr-0007-resource-policy-and-body-erasure-boundary.md) | V6 owned-escape counterparts and the cancellation boundary of C09–C12                   |

The V4/I4 mapping is partial on purpose: the spike proves the adapter preserves
frames and errors, not the collection limit or backpressure behaviour that
design §11 assigns to later slices.

## Compiler and build posture

Table: Compiler, toolchain, and build configuration observed for this evidence

| Item                  | Observed value                                                                       |
| --------------------- | ------------------------------------------------------------------------------------ |
| `rustc -Vv`           | `rustc 1.100.0-nightly (bff8e12ff 2026-08-26)`, LLVM 23.1.0                          |
| `cargo --version`     | `cargo 1.100.0-nightly (e8cb624d5 2026-08-22)`                                       |
| `rust-toolchain.toml` | `nightly-2026-08-27`, with Clippy, LLVM tools, Cranelift, rust-analyzer, and rustfmt |
| Development backend   | `codegen-backend = "cranelift"` in `.cargo/config.toml`                              |
| Linker                | `scripts/native-clang-linker.sh` with `-C link-arg=-fuse-ld=mold`                    |
| Rust flags            | `-Zthreads=8` from Cargo configuration; `RUSTFLAGS` unset                            |
| Compile admission     | `RUSTC_WRAPPER` and `RUSTDOC` point at the host `build-limits` adapters              |

Every command in this ledger was run through Cargo with those wrappers in
place. No compiler invocation bypassed admission.

Snapshots of the full `rustc -Vv` text and the gate logs are kept on the
recording host under `/tmp`.

The table records the M1 evidence run. The repository's pinned nightly has
since moved, so the compiler-dependent observations were re-recorded rather
than inferred; see
[Re-recording on the pinned nightly](#re-recording-on-the-pinned-nightly). The
M1 rows are retained unchanged so the original evidence stays traceable.

### Borrow-checker posture

The toolchain, Cargo configuration, and Makefile select no `-Zpolonius` value,
so the compiler's default applies. Current nightlies enable Polonius Alpha by
default, but the effective posture is recorded from a canary rather than from
the channel name. The temporary example below returns `b` on one path and `a`
on another after a shared reborrow:

```rust
fn reborrow(a: &mut u8) -> &mut u8 {
    let b = &mut *a;
    if true { b } else { a }
}
```

- `cargo rustc --example polonius_canary` exited 0.
- `cargo rustc --example polonius_canary -- -Zpolonius=off` exited 101 with
  `error[E0499]: cannot borrow *a as a mutable more than once at a time`.

Recorded posture: `polonius-alpha`. The example was removed before the M1
commit; the source above, the two commands, and the recorded exit statuses are
the retained evidence. Logs are in `/tmp/canary-1-1-3-*.out` on the recording
host. This experiment makes no compiler commitment, and the default posture
could change with a future nightly.

### Re-recording on the pinned nightly

`rust-toolchain.toml` moved from `nightly-2026-08-27` to `nightly-2026-11-06`
after the M1 evidence was recorded. The compiler-dependent observations were
therefore re-taken on that pin instead of being carried over from the earlier
run.

Table: Compiler identity and borrow-checker posture re-recorded on the pin

| Item                                                      | Re-recorded value                                           |
| --------------------------------------------------------- | ----------------------------------------------------------- |
| `rust-toolchain.toml`                                     | `nightly-2026-11-06`                                        |
| `rustc -Vv`                                               | `rustc 1.101.0-nightly (249c1e4e3 2026-11-05)`, LLVM 23.1.5 |
| `cargo --version`                                         | `cargo 1.101.0-nightly (3c747ea3e 2026-11-03)`              |
| `cargo rustc --example polonius_canary`                   | exited 0                                                    |
| `cargo rustc --example polonius_canary -- -Zpolonius=off` | exited 101 with E0499                                       |

The canary source above produced both results, so the posture observed on the
pinned nightly is still `polonius-alpha`; it is not inferred from the channel
name. The compile-fail baselines under `tests/ui/ownership/fail/` were
regenerated on this pin as well. Their differences were diagnostic rendering
only: one removed note, two re-anchored labels, and one added lifetime label.
The fixtures still fail on the same errors, so the contract they pin is
unchanged.

### Dependency review

The spike plan requires exact release compatibility, minimum-compiler
declarations, licences, and lockfile resolution to be checked at M1 rather than
assumed. The review below was taken from the committed `Cargo.lock` with
`cargo metadata --format-version 1 --locked --offline`, which reads the licence,
`rust-version`, and feature declarations of every resolved package. `--locked`
also fails if the lockfile and the manifests disagree, so a successful run is
the lockfile-resolution evidence: 225 non-root packages resolve from 22 direct
requirements without a lockfile rewrite.

Table: Direct runtime dependencies, their requirements, and their resolved
metadata

| Dependency       | Requirement | Resolved | Licence           | `rust-version` | Features enabled |
| ---------------- | ----------- | -------- | ----------------- | -------------- | ---------------- |
| `async-trait`    | `^0.1.92`   | 0.1.92   | MIT OR Apache-2.0 | 1.71           | default          |
| `bytes`          | `^1.12.1`   | 1.12.1   | MIT               | 1.57           | `default`, `std` |
| `http`           | `^1.5.0`    | 1.5.0    | MIT OR Apache-2.0 | 1.57.0         | `default`, `std` |
| `http-body`      | `^1.1.0`    | 1.1.0    | MIT               | 1.61           | default          |
| `http-body-util` | `^0.1.5`    | 0.1.5    | MIT               | 1.61           | default          |
| `thiserror`      | `^2.0.21`   | 2.0.21   | MIT OR Apache-2.0 | 1.77           | `default`, `std` |

Table: Direct development dependencies, their requirements, and their resolved
metadata

| Dependency          | Requirement | Resolved          | Licence                                             | `rust-version` | Features enabled                                                                                                  |
| ------------------- | ----------- | ----------------- | --------------------------------------------------- | -------------- | ----------------------------------------------------------------------------------------------------------------- |
| `camino`            | `^1.2.0`    | 1.2.6             | MIT OR Apache-2.0                                   | 1.61.0         | `serde1`                                                                                                          |
| `cap-std`           | `^4.0.3`    | 4.0.3             | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | not declared   | `default`, `fs_utf8`, `camino`                                                                                    |
| `googletest`        | `^0.14.4`   | 0.14.4            | Apache-2.0                                          | 1.85.0         | default                                                                                                           |
| `hyper`             | `^1.12.0`   | 1.12.0            | MIT                                                 | 1.63           | `default`, `http1`, `server`                                                                                      |
| `hyper-util`        | `^0.1.21`   | 0.1.21            | MIT                                                 | 1.85           | `default`, `tokio`                                                                                                |
| `mockable`          | `^3.0.0`    | 3.0.0             | MIT, by `license-file`                              | not declared   | none (default features off)                                                                                       |
| `pretty_assertions` | `^1.4.1`    | 1.4.1             | MIT OR Apache-2.0                                   | not declared   | `default`, `std`                                                                                                  |
| `proptest`          | `^1.9.0`    | 1.11.0            | MIT OR Apache-2.0                                   | 1.85           | `default`, `bit-set`, `fork`, `regex-syntax`, `rusty-fork`, `std`, `tempfile`, `timeout`                          |
| `rstest`            | `^0.27.0`   | 0.27.0            | MIT OR Apache-2.0                                   | 1.85.0         | none (default features off)                                                                                       |
| `rstest-bdd`        | `^0.6.0`    | 0.6.0             | ISC                                                 | 1.88           | `default`, `diagnostics`, `serde`, `serde_json`                                                                   |
| `rstest-bdd-macros` | `^0.6.0`    | 0.6.0             | ISC                                                 | 1.88           | `default`                                                                                                         |
| `serde`             | `^1.0.228`  | 1.0.229           | MIT OR Apache-2.0                                   | 1.56           | `alloc`, `default`, `derive`, `std`                                                                               |
| `serde_yaml`        | `^0.9.34`   | 0.9.34+deprecated | MIT OR Apache-2.0                                   | 1.64           | default                                                                                                           |
| `tokio`             | `^1.54.1`   | 1.54.1            | MIT                                                 | 1.85           | `default`, `io-util`, `macros`, `net`, `rt`, `rt-multi-thread`, `sync`, `time` (plus enabled transitive features) |
| `toml`              | `^1.1.3`    | 1.1.6+spec-1.1.0  | MIT OR Apache-2.0                                   | 1.85           | `default`, `display`, `parse`, `serde`, `std`                                                                     |
| `trybuild`          | `^1.0.122`  | 1.0.122           | MIT OR Apache-2.0                                   | 1.88           | default                                                                                                           |

Every requirement is a caret requirement in `Cargo.toml`, and every resolved
version lies inside its requirement, so the lockfile records a
SemVer-compatible resolution rather than an override.

Two declarations are not SPDX `license` fields and were checked by hand.
`mockable` 3.0.0 declares `license-file = "LICENSE"`, whose text is the MIT
licence, so the missing SPDX field is a metadata gap in that crate rather than
an unlicensed dependency. `cap-std` 4.0.3 declares its triple-licensed
`Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` field but no
`rust-version`; its `fs_utf8` feature is the only one enabled, and it is what
pulls in `camino`, which the manifest also requests directly.

Every other package in the resolved graph offers a permissive option. The
non-permissive-looking declarations are all dual or triple licences whose
permissive branch is selectable: `r-efi` offers MIT, `self_cell` offers
Apache-2.0, `ryu` offers Apache-2.0, and the Unicode data crates carry
Unicode-3.0. No package in the graph is GPL-only or otherwise incompatible with
this repository's ISC licence.

Minimum-compiler declarations are all below the pinned toolchain. Of the 225
resolved packages, 169 declare `rust-version` and the highest is 1.91
(`cargo-platform`, a transitive build dependency of `trybuild`); the pinned
compiler is 1.100.0-nightly. The 56 packages without a declaration, including
the three direct ones above, therefore state no requirement this toolchain
could fail to meet. That is a statement about the declaration only: it is not
evidence that such a package would build on an older compiler, and none was
tested. What the gate evidence does establish is that all 225 compile on the
pinned toolchain.

Feature enablement was reviewed against actual use. Hyper is limited to `http1`
and `server`, and `hyper-util` to `tokio`; both appear only in the loopback
fixture. Tokio's `io-util`, `macros`, `net`, `rt`, `rt-multi-thread`, `sync`,
and `time` are each exercised by the network fixture, the asynchronous
witnesses, and the example. `cap-std` enables only `fs_utf8` on top of its
empty default set, and `rstest` and `mockable` are brought in with default
features disabled. This is the smallest set that compiles the specimen; task
1.2.1 still owns the maintained dependency and feature baseline.

The resolved graph contains eight duplicate crate names at different versions:
`getrandom`, `io-lifetimes`, `r-efi`, `syn`, `thiserror`, `thiserror-impl`,
`toml`, and `windows-sys`. Each duplicate is transitive, arriving through
`trybuild`, `proptest`, or the `cap-std` family; no direct requirement is
duplicated, and pruning them would mean changing a dev-dependency rather than
this spike's manifest.

The tables above record the M1 audit as it stood when the compile evidence was
first taken, and are left unchanged as historical evidence. One direct
development dependency was added afterwards, for the witnesses the review asked
for. `serde_json` `^1.0.154` resolves to 1.0.154, licensed `MIT OR Apache-2.0`,
with `rust-version` 1.71 and its default features, under the same
`cargo metadata --format-version 1 --locked --offline` command. It was already
resolved transitively, so the addition changed no resolved package and the
graph still contains 225 non-root packages. Only
`tests/resource_policy_example_executable.rs` uses it, to read the built
example's path from Cargo's JSON artefact messages and run it directly.

## V-OWN: compile witnesses

`tests/ownership_contract.rs` runs `trybuild` over
`tests/ui/ownership/pass/*.rs` and `tests/ui/ownership/fail/*.rs`. Positive
fixtures must compile against the exported spike module. Negative fixtures must
fail for one named ownership, borrow, or trait requirement, and each reviewed
`.stderr` baseline records the diagnostic the pinned compiler emitted.

Table: V-OWN compile-pass witnesses and what each one establishes

| Witness                             | Fixture                                | Establishes                                                                        |
| ----------------------------------- | -------------------------------------- | ---------------------------------------------------------------------------------- |
| Heterogeneous collection            | `pass/heterogeneous_collection.rs`     | Two concrete resource types share one erased interface in a collection             |
| Shared resource state               | `pass/shared_resource_state.rs`        | Two registrations retain the same resource allocation                              |
| Disjoint field borrows across await | `pass/disjoint_fields_across_await.rs` | The real `ResourceRunner::invoke` future stays `Send` over disjoint context fields |
| `Send` but not `Sync` body          | `pass/send_not_sync_body.rs`           | An erased `Cell`-bearing body crosses an await while the future stays `Send`       |
| Owned escape counterparts           | `pass/owned_escapes.rs`                | Every rejected escape has an owned form that compiles                              |
| `Rc` ends before await              | `pass/rc_scope_ends_before_await.rs`   | A non-`Send` local scoped before the await does not prohibit a `Send` future       |

Table: V-OWN compile-fail witnesses, the requirement each one violates, and the
diagnostic the pinned compiler emitted

| Witness                        | Fixture                                    | Requirement                                         | Observed diagnostic                                           |
| ------------------------------ | ------------------------------------------ | --------------------------------------------------- | ------------------------------------------------------------- |
| `Rc`-backed resource           | `fail/rc_backed_resource.rs`               | `Resource: Send + Sync`                             | `E0277`: `Rc<String>` cannot be shared between threads safely |
| Non-`Send` state across await  | `fail/non_send_state_across_await.rs`      | Spawned futures must be `Send`                      | `future cannot be sent between threads safely`                |
| Overlapping mutable loans      | `fail/overlapping_mutable_loans.rs`        | Rust's exclusive-borrow rule                        | `E0499`: cannot borrow as mutable more than once at a time    |
| Response body captures request | `fail/response_captures_request_borrow.rs` | Owned bodies cannot retain request-local references | `E0521`: borrowed data escapes outside of function            |
| Spawned task borrows request   | `fail/spawn_borrows_request.rs`            | Detached tasks require `'static` captures           | `E0597`: `headers` does not live long enough                  |
| Missing policy                 | `fail/missing_policy.rs`                   | `Resource::policy` is required                      | `E0046`: not all trait items implemented, missing `policy`    |

## Reproducing the compile evidence

```sh
make test TEST_FLAGS='--test ownership_contract'
```

The positive fixtures must pass and every negative fixture must match its
reviewed `.stderr` baseline. `trybuild` normalizes machine-specific paths, so a
compiler change can alter the baseline even when acceptance is unchanged.
Compiler changes therefore need a deliberate baseline review, never a blind
`TRYBUILD=overwrite`.

## What the compile evidence does not establish

- `trybuild` compiles fixtures; it does not exercise runtime scheduling,
  cancellation, or network I/O. The M2 witnesses cover those.
- A dishonest resource can return a policy that differs from its behaviour.
  The specimen records the contract and does not detect that violation.
- The fixtures record acceptance on one dated nightly. They do not claim a
  minimum supported Rust version or forward compatibility.
- The specimen's policy gate demonstrates admission ordering only. It is not
  tenant or object authorization for an application.

## V-IDENTITY: identity and admission witnesses

`tests/resource_policy_identity.rs` owns the identity witnesses and
`tests/resource_policy_spike.rs` owns the admission outcomes; both drive the
runner directly. The fixture enforcer records one observation, suspends on a
`tokio::sync::Notify` rendezvous, then records a second observation after it
resumes, so every equality below spans a real await boundary rather than a
synchronous stand-in. Its resources and helpers live in
`tests/support/spike/fixtures.rs`, which the cancellation binary includes as
well. `tests/resource_policy_spike_bdd.rs` repeats the outcomes as `rstest-bdd`
scenarios bound to `tests/features/resource_policy_spike.feature`.

Table: V-IDENTITY witnesses and the behaviour each one establishes

| Witness                    | Test                                               | Establishes                                                                                                  |
| -------------------------- | -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| Identity across suspension | `identity_and_policy_survive_the_awaited_hook`     | Handle, observation, method, policy, path, and principal are identical before and after the enforcer's await |
| Repeated admissions        | `repeated_admissions_observe_the_frozen_policy`    | Two admissions of one registration observe the same registration-time policy snapshot                        |
| Same-type instances        | `same_type_instances_keep_distinct_identity`       | Two instances of one concrete type keep distinct allocation identities and only the selected one responds    |
| Denial before the hook     | `missing_identity_is_denied_before_the_hook_runs`  | An anonymous protected request is denied with `MissingIdentity` and never reaches the enforcer               |
| Enforcer denial            | `enforcer_denial_withholds_the_request`            | A `Decision::Deny` reaches neither the responder nor the dependency                                          |
| Protected registration     | `protected_registration_requires_an_enforcer`      | A protected resource cannot register without an enforcement hook                                             |
| Public registration        | `public_resource_permits_direct_invocation`        | A public resource registers and responds with no enforcer installed                                          |
| Unselected resource        | `an_unselected_resource_is_not_invoked`            | Invoking one registration leaves a second registered resource untouched                                      |
| Scope delegation           | `scoped_policy_delegates_without_computing_scopes` | A scoped policy defers its decision to the enforcer                                                          |
| Empty scope rejection      | `empty_scope_set_is_rejected`                      | An empty scope set is a typed rejection, not a silent permit                                                 |

The BDD suite covers five of these as scenarios: public direct invocation,
protected denial before invocation, protected registration without enforcement,
a present identity permitting a protected invocation, and an absent identity
being denied before the hook.

`tests/resource_policy_registration.rs` adds the registration-time boundary
witnesses. Its resource answers `GET` publicly and every other method as
authenticated, so the captured policy shows which method registration asked
about. Its public-reach witness deliberately does not use the rendezvous
fixture: it installs a denying enforcer that counts consultations and returns
immediately, so a regression that admits a public request through the enforcer
fails on the count instead of blocking on a suspension no driver releases.

A single-scope fixture cannot show that a multi-scope policy survives
registration intact, because an implementation that kept only the first scope
would satisfy every one-scope witness. `multi_scoped_catalogue` therefore
declares three scopes out of alphabetical order, and
`every_declared_scope_reaches_registration_and_the_enforcer` compares the full
ordered list both in the policy captured by registration and in the pre- and
post-suspension observations the enforcer records.

Table: Registration-time witnesses and the boundary each one establishes

| Witness                          | Test                                                           | Establishes                                                                                                                                                               |
| -------------------------------- | -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Policy frozen per method         | `registration_captures_the_policy_for_the_frozen_method`       | A resource with a per-method policy captures the policy belonging to the method it was registered with                                                                    |
| Registration without an enforcer | `registration_without_an_enforcer_accepts_only_public_methods` | A public method registers without an enforcer, while a protected one fails with `RegistrationError::MissingEnforcer` naming the method and policy                         |
| Public reach                     | `a_public_resource_never_reaches_a_denying_enforcer`           | A public registration responds and its denying enforcer records zero consultations                                                                                        |
| Invocation without an enforcer   | `a_protected_invocation_without_an_enforcer_is_denied`         | A protected registration made with an enforcer is denied with `MissingEnforcer` when invoked through a runner without one, leaving the responder and dependency untouched |
| Full scope retention             | `every_declared_scope_reaches_registration_and_the_enforcer`   | A three-scope policy keeps every scope in declaration order at registration and in both enforcer observations                                                             |

`tests/resource_policy_example_executable.rs` covers the process contract the
user guide states for the example rather than its internal assertions. It
builds the example, reads the executable's path from Cargo's JSON artefact
messages, and runs that executable directly, so both standard output and
standard error are asserted empty without Cargo's own status and
package-cache-lock messages in the way. A fault-injection run that wrote one
line to standard error failed the witness with that text, which is the negative
control for the assertion.

## V-BODY: adapter and network witnesses

`tests/body_adapter_spike.rs` compares frames, not concatenated bytes, because
a byte oracle cannot see a dropped empty frame or a moved boundary.
`tests/body_adapter_property.rs` generates the same frame classes from
`tests/support/spike/cases.rs`, so the named witnesses and the property cannot
drift apart. `tests/network_body_spike.rs` drives a real Hyper `Incoming` body
through the same `adapt_body` and `ResourceRunner::invoke` seam on an ephemeral
loopback port. `tests/spike_fixture_bounds.rs` pins the fixture's
pre-accumulation trailer bound from below, above, across frames, and for
repeated trailer names.

Table: V-BODY witnesses and the behaviour each one establishes

| Witness              | Test                                                       | Establishes                                                                                         |
| -------------------- | ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| Empty input          | `empty_input_yields_no_frames`                             | An empty body emits no frames                                                                       |
| Frame boundaries     | `several_data_frames_preserve_boundaries`                  | Three frames, including an empty one, survive unchanged                                             |
| Trailers             | `trailers_are_forwarded_verbatim`                          | Every trailer name and value is forwarded unchanged                                                 |
| Pending then ready   | `pending_then_ready_is_polled_again`                       | A pending poll does not lose the next frame; polls cover pending, data, and end                     |
| Terminal error       | `terminal_error_is_reported_once`                          | An upstream error surfaces as `BodyError::Upstream` with its text intact                            |
| Partial consumption  | `dropping_a_partially_consumed_body_releases_it_once`      | A half-read body owns its resources until dropped, then releases them once                          |
| Frame classes        | `every_documented_frame_class_is_reproduced`               | Each emptiness/trailer combination is reproduced frame by frame                                     |
| Empty with trailers  | `empty_body_with_trailers_yields_only_the_trailers_frame`  | An empty body with trailers yields exactly one trailers frame                                       |
| Poll bound           | `synthetic_poll_bound_is_enforced`                         | The fixture body fails after its documented poll bound instead of looping                           |
| Generated partitions | `generated_partitions_are_reproduced_frame_by_frame`       | Sixty-four generated partitions, with optional trailers, reproduce frame by frame                   |
| One invocation path  | `synthetic_and_network_requests_share_one_invocation_path` | A synthetic and a network request carry `hello` byte for byte to the same dependency, one call each |
| Network trailers     | `chunked_request_trailers_reach_the_resource`              | Chunked body bytes and trailers arrive at the resource unchanged through Hyper                      |
| Truncated request    | `truncated_content_length_surfaces_the_body_error`         | A short body surfaces the adapter's error, not a silent success                                     |
| Trailer bound        | `exactly_four_distinct_trailer_pairs_are_accepted`         | Four trailer pairs are accepted and stored                                                          |
| Trailer overflow     | `trailer_pairs_accumulate_across_frames_before_rejection`  | Pairs accumulate across frames, and the fifth is rejected before it is copied                       |
| Repeated names       | `a_repeated_trailer_name_counts_individual_values`         | A repeated trailer name counts as one pair per value, not one pair per name                         |

Frame-level tests compile against a `Cell`-bearing synthetic body that is
`Send` but not `Sync`, so the tests themselves witness the claimed bound.

## V-CANCEL: cancellation and drop witnesses

`tests/resource_policy_cancellation.rs` owns these witnesses. They are a
separate binary from the identity suite so each file stays within the
repository's 400-line limit and each set can be reproduced on its own target.

Table: V-CANCEL witnesses and the behaviour each one establishes

| Witness                    | Test                                                            | Establishes                                                                                        |
| -------------------------- | --------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| Owned body, cancelled call | `dropping_a_pending_invocation_releases_the_owned_body_once`    | Dropping a pending invocation releases the owned body exactly once and never reaches the responder |
| Body and request guard     | `dropping_a_pending_invocation_releases_body_and_request_guard` | Cancellation releases the body and a separate request-local guard exactly once each                |
| Borrowed arguments         | `a_dropped_future_returns_its_borrowed_arguments`               | A cancelled invocation leaves a caller-owned body usable and still owned by the caller             |
| Spawned invocation         | `a_fully_owned_invocation_can_be_spawned`                       | A fully owned invocation crosses a multi-thread runtime boundary and responds once                 |

## Negative controls

Each control below was injected after the suites were green, observed to fail
the named tests, and then reverted before the green run recorded above. This
substitutes for a red-first stage on the runtime witnesses, which M1 had
already implemented.

Table: Injected faults and the tests that detected each one

| Fault                         | Injected at                                            | Failing tests                                                                                   |
| ----------------------------- | ------------------------------------------------------ | ----------------------------------------------------------------------------------------------- |
| Substituted allocation handle | Fixture observer in `tests/support/spike/admission.rs` | `identity_and_policy_survive_the_awaited_hook`                                                  |
| Invocation despite denial     | `src/spike/invoke.rs`, `Decision::Deny` arm            | `missing_identity_is_denied_before_the_hook_runs`, `enforcer_denial_withholds_the_request`      |
| Dropped empty data frames     | `src/spike/body.rs`, `adapt_body`                      | `several_data_frames_preserve_boundaries`, `generated_partitions_are_reproduced_frame_by_frame` |

The handle substitution changed only the fixture's observation, which is the
point: the equality assertion fails when the observed allocation differs from
the registration's retained allocation.

## Reproducing the runtime evidence

```sh
make test TEST_FLAGS='--test resource_policy_spike'
make test TEST_FLAGS='--test resource_policy_identity'
make test TEST_FLAGS='--test resource_policy_registration'
make test TEST_FLAGS='--test resource_policy_cancellation'
make test TEST_FLAGS='--test resource_policy_spike_bdd'
make test TEST_FLAGS='--test body_adapter_spike'
make test TEST_FLAGS='--test body_adapter_property'
make test TEST_FLAGS='--test network_body_spike'
make test TEST_FLAGS='--test spike_fixture_bounds'
make test TEST_FLAGS='--test resource_policy_example'
make test TEST_FLAGS='--test resource_policy_example_executable'
cargo run --example resource_policy_spike
```

All eleven targets passed after the faults were reverted, and the example
exited zero without writing to either stream. `resource_policy_example` loads
the example file as a module, so the assertions the reader is invited to run
are exercised by `cargo test` as well; `resource_policy_example_executable`
builds the example and runs the executable directly, so the documented exit
status and silence are checked where a reader would see them. The network cases
bind an ephemeral loopback port and join their server task rather than sleeping.

## What the runtime evidence does not establish

- The enforcer is a fixture. No authentication, authorization, or session
  policy is implemented or evaluated.
- The network harness serves one request with Hyper's lower-level connection
  API; it does not demonstrate routing, connection reuse, or load behaviour.
- The cancellation witnesses drop in-flight invocations at one known suspension
  point. They do not explore cancellation at every await or under load.
- Frame-level equality holds for the generated partitions in this suite, not
  for arbitrary bodies or arbitrary upstream error types.
