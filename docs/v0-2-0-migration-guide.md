# Migration guide: v0.1.0 to v0.2.0

## Who should read this

Read this guide if you depend on `peregrine-web` and are moving from v0.1.0 to
v0.2.0. The release adds one public module, `peregrine_web::spike`, and adds
the runtime dependencies that module needs. It removes nothing and deprecates
nothing.

The new module is an experimental specimen, not a framework surface. It carries
no stability promise: its names may move, change, or disappear in any later
release, including a patch release. Adopting it is optional, and no consumer is
required to act.

Consumers who never name `peregrine_web::spike` need no source change. Their
only decision is whether the added runtime dependencies pass their own
supply-chain policy.

## Impact at a glance

| Priority          | Area                   | What to do                                                                          |
| ----------------- | ---------------------- | ----------------------------------------------------------------------------------- |
| Optional          | `peregrine_web::spike` | Compile against it only in experiments; expect it to change without notice.         |
| Optional          | Dependency requirement | Raise the requirement to `0.2` only if you want the new module.                     |
| Review if enabled | Supply-chain policy    | Re-run `cargo deny`, `cargo vet`, or vendoring checks against the new dependencies. |

_Table 1: Obligations when moving from v0.1.0 to v0.2.0._

There are no required changes and no recommended changes. The numbered section
below covers the one optional adoption decision.

## 1. Decide whether to compile against the experimental module (optional)

`peregrine_web::spike` exists so the repository can record evidence about
resource-owned policy, admission ordering, erased request bodies, and
cancellation. It is deliberately incomplete: there is no router, server,
authentication, or authorization provider. The
[spike evidence ledger](resource-policy-ownership-spike.md) states what the
witnesses establish and what they leave open, and
[ADR 007](adr-007-resource-policy-and-body-erasure-boundary.md) records the
provisional shapes and the alternatives they rejected.

If you want to inspect that evidence, raise the dependency requirement and keep
the import inside a throwaway experiment. If you do not, do nothing: the v0.1.0
surface compiles unchanged against v0.2.0.

Adoption is not a commitment by the project. Resource ownership, the policy
vocabulary, and the body-erasure boundary are all still under review, so code
written against this module is expected to break.

## What does not need to change

- `peregrine_web::greet` and its signature are unchanged.
- The crate edition, licence, and public module paths other than the new
  `spike` module are unchanged.
- Nothing in v0.1.0 is deprecated or removed, so existing source needs no
  edit and compiles without new warnings.
- No feature flags, configuration files, or environment variables are added.

## Migration checklist

- [ ] Confirm the resolved version is 0.2.0 or later with
  `cargo tree -i peregrine-web`.
- [ ] Confirm the crate still builds with no source change.
- [ ] If a supply-chain policy is enabled, review the new runtime
  dependencies against it.
- [ ] Decide whether any code will use `peregrine_web::spike`.
- [ ] If it will, isolate that use in an experiment and expect breakage in
  later releases.

## Common errors and fixes

- **Error:** ``error[E0432]: unresolved import `peregrine_web::spike` ``, whose
  detail line reads ``could not find `spike` in `peregrine_web` ``.
  - **Fix:** The dependency still resolves to v0.1.0. Raise the requirement in
    `Cargo.toml` to `"0.2"`, then run `cargo update -p peregrine-web`.
