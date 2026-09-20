# Documentation contents

[Documentation contents](contents.md) is the index for Peregrine Web's
documentation set.

## Project guides

- [User guide](users-guide.md) explains how to use the generated project and
  its public build and test commands.
- [Developer guide](developers-guide.md) explains the local workflow and
  implementation tooling for contributors.
- [Repository layout](repository-layout.md) explains the generated project's
  top-level files, directories, and ownership boundaries.
- [Documentation style guide](documentation-style-guide.md) defines the
  spelling, structure, Markdown, Architecture Decision Record (ADR), Request
  for Comments (RFC), and roadmap conventions used by this documentation set.

## Product direction and design

- [Terms of reference](terms-of-reference.md) defines the intended users,
  goals, scope, assumptions, and unresolved decisions from the source papers.
- [Technical design](peregrine-design.md) proposes the resource, lifecycle,
  body, transport, and verification contracts.
- [Potential GIST roadmap](roadmap.md) links Goals, Ideas, Steps, and Tasks to
  review-sized delivery slices and observable success criteria.
- [ADR 001: Resource-aware lifecycle](adr-001-resource-aware-lifecycle.md)
  records the proposed choice to own the inner middleware pipeline.
- [Compiler ownership experiment](polonius-ownership-experiment.md) compares
  old-checker-compatible and exclusive Polonius/new-solver designs with code
  examples, measured acceptance, and adoption criteria.
- [ADR 002: Compiler ownership experiment](adr-002-compiler-ownership-experiment.md)
  records the proposed experiment and the conditions for a compiler commitment.
- [Compiler probe evidence](compiler-probe-evidence.md): complete isolated
  compiler fixtures and recorded outcomes.
- [Polonius ownership design review](polonius-design-review.md): six-perspective
  findings, pre-mortem, and adoption conditions.

## Rust reference material

- [Reliable testing in Rust via dependency injection](reliable-testing-in-rust-via-dependency-injection.md)
  explains how to keep tests deterministic by injecting environment, clock,
  filesystem, and other external dependencies.
- [Rust doctest Don't Repeat Yourself guide](rust-doctest-dry-guide.md)
  explains how to write maintainable, executable Rust documentation examples.
- [Rust testing with `rstest` fixtures](rust-testing-with-rstest-fixtures.md)
  explains fixture-based, parameterized, and asynchronous testing with `rstest`.

## Engineering practice

- [Complexity antipatterns and refactoring strategies](complexity-antipatterns-and-refactoring-strategies.md)
  explains cognitive complexity, the bumpy-road antipattern, and refactoring
  approaches for maintainable code.
- [Scripting standards](scripting-standards.md) explains the preferred Python
  scripting stack, command execution patterns, and test expectations for helper
  scripts.
