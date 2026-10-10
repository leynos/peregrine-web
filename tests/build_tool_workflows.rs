//! Behavioural cases for reachable workflow suite provisioning.

#[path = "support/build_tool_workflows.rs"]
mod support;

use support::{workflow, workflow_violations, workflows};

#[test]
fn every_reachable_suite_is_provisioned() {
    let documents = workflows().expect("discover and parse committed workflows");
    let problems = workflow_violations(&documents).expect("inspect suite routes");
    assert!(
        problems.is_empty(),
        "workflow suite provisioning failed: {problems:#?}"
    );
}

#[test]
fn newly_added_linux_suite_cannot_escape_provisioning() {
    for runner in [
        "ubuntu-latest",
        "[self-hosted, linux]",
        "{labels: [self-hosted, linux]}",
    ] {
        let source = format!(
            "jobs:\n  new-suite:\n    runs-on: {runner}\n    steps:\n      - run: cargo test\n"
        );
        let document = workflow(&source).expect("parse a synthetic suite job");
        let problems = workflow_violations(&[("new.yml".to_owned(), document)])
            .expect("inspect a newly added suite");
        assert!(
            !problems.is_empty(),
            "Linux runner {runner} must require provisioning"
        );
    }
    let matrix = "jobs:\n  new-suite:\n    runs-on: '${{ matrix.os }}'\n    strategy:\n      \
                  matrix:\n        os: [ubuntu-latest, windows-latest]\n    steps:\n      - run: \
                  make test\n";
    let document = workflow(matrix).expect("parse a matrix suite job");
    let problems = workflow_violations(&[("matrix.yml".to_owned(), document)])
        .expect("inspect a matrix suite");
    assert!(
        !problems.is_empty(),
        "a Linux matrix case must require provisioning"
    );
    let mapped_matrix = "jobs:\n  suite:\n    runs-on: {labels: [self-hosted, '${{ matrix.os \
                         }}']}\n    strategy:\n      matrix:\n        include:\n          - os: \
                         ubuntu-latest\n          - os: windows-latest\n    steps:\n      - run: \
                         cargo test\n";
    let mapped_document = workflow(mapped_matrix).expect("parse a mapped matrix runner");
    let mapped_problems = workflow_violations(&[("mapped-matrix.yml".to_owned(), mapped_document)])
        .expect("inspect a mapped matrix suite");
    assert!(
        !mapped_problems.is_empty(),
        "mapped Linux matrix labels must require provisioning"
    );
}

#[test]
fn ordered_linux_installation_allows_a_new_suite() {
    let source = "jobs:\n  suite:\n    runs-on: ubuntu-latest\n    steps:\n      - run: sudo \
                  apt-get install clang mold\n      - run: make install-build-tools\n      - run: \
                  make test\n";
    let document = workflow(source).expect("parse an ordered suite fixture");
    let problems =
        workflow_violations(&[("new.yml".to_owned(), document)]).expect("inspect an ordered suite");
    assert!(
        problems.is_empty(),
        "an ordered Linux suite must pass: {problems:?}"
    );
    let indirect = workflow(
        "jobs:\n  suite:\n    runs-on: ubuntu-latest\n    steps:\n      - run: make all\n",
    )
    .expect("parse an indirect Make suite fixture");
    let indirect_problems = workflow_violations(&[("indirect.yml".to_owned(), indirect)])
        .expect("inspect an indirect Make suite");
    assert!(
        !indirect_problems.is_empty(),
        "make all reaches tests and requires tools"
    );
}

#[test]
fn direct_cargo_and_unresolved_shell_routes_are_checked() {
    for command in ["cargo test", "cargo +nightly test", "cargo --verbose test"] {
        let source = format!(
            "jobs:\n  suite:\n    runs-on: ubuntu-latest\n    steps:\n      - run: {command}\n"
        );
        let direct = workflow(&source).expect("parse a direct Cargo suite fixture");
        let problems = workflow_violations(&[("direct.yml".to_owned(), direct)])
            .expect("inspect direct Cargo tests");
        assert!(
            !problems.is_empty(),
            "direct Cargo suite {command} must require tools"
        );
    }
    for command in [
        "make -C . test",
        "make $GOAL",
        "bash scripts/run-tests.sh",
        "./scripts/run-tests.sh",
    ] {
        let source = format!(
            "jobs:\n  suite:\n    runs-on: ubuntu-latest\n    steps:\n      - run: {command}\n"
        );
        let document = workflow(&source).expect("parse an indirect command fixture");
        assert!(
            workflow_violations(&[("indirect.yml".to_owned(), document)]).is_err(),
            "unresolved workflow command must fail closed: {command}"
        );
    }
}

#[test]
fn rejects_ineffective_installers_and_unprovable_workflows() {
    for install in [
        "",
        "      - run: make install-build-tools\n",
        "      - run: make install-build-tools\n        if: false\n",
        "      - run: make install-build-tools\n        continue-on-error: true\n",
    ] {
        let source = format!(
            "jobs:\n  suite:\n    runs-on: ubuntu-latest\n    steps:\n      - run: sudo apt-get \
             install clang mold\n      - run: cargo test\n{install}"
        );
        let document = workflow(&source).expect("parse an ineffective installer fixture");
        let problems = workflow_violations(&[("new.yml".to_owned(), document)])
            .expect("inspect installer placement");
        assert!(
            !problems.is_empty(),
            "ineffective installer must fail: {source}"
        );
    }
    assert!(
        workflow("jobs:\n  same: {}\n  same: {}\n").is_err(),
        "duplicate workflow job keys must be rejected"
    );
    let problems = workflow_violations(&[]).expect("inspect the empty workflow set");
    assert!(
        !problems.is_empty(),
        "an empty suite reading must be rejected"
    );
    let unknown = workflow(
        "jobs:\n  suite:\n    runs-on: '${{ matrix.missing }}'\n    steps:\n      - run: cargo \
         test\n",
    )
    .expect("parse an unresolved runner fixture");
    let unknown_problems = workflow_violations(&[("unknown.yml".to_owned(), unknown)])
        .expect("inspect an unresolved runner");
    assert!(
        !unknown_problems.is_empty(),
        "an unresolved suite runner must be rejected"
    );
    let coverage = workflow(
        "jobs:\n  coverage:\n    runs-on: ubuntu-latest\n    steps:\n      - run: sudo apt-get \
         install clang mold\n      - run: make install-build-tools\n      - run: make coverage\n",
    )
    .expect("parse a coverage fixture without lld");
    let coverage_problems = workflow_violations(&[("coverage.yml".to_owned(), coverage)])
        .expect("inspect coverage prerequisites");
    assert!(
        !coverage_problems.is_empty(),
        "coverage requires the lld package before installation"
    );
}

#[test]
fn mutation_setup_rejects_missing_late_and_masked_installers() {
    for setup in [
        "sudo apt-get install clang lld mold",
        "make install-build-tools\nsudo apt-get install clang lld mold",
        "sudo apt-get install clang lld mold\nmake install-build-tools || true",
        "sudo apt-get install clang lld mold\ncargo-mutants run\nmake install-build-tools",
    ] {
        let source = format!(
            "jobs:\n  mutation:\n    uses: example/.github/workflows/mutation-cargo.yml@123\n    \
             with:\n      setup-commands: |\n{}\n",
            setup
                .lines()
                .map(|line| format!("        {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        let document = workflow(&source).expect("parse a mutation setup fixture");
        let problems = workflow_violations(&[("mutation.yml".to_owned(), document)])
            .expect("inspect mutation setup ordering");
        assert!(
            !problems.is_empty(),
            "ineffective mutation setup must fail: {setup}"
        );
    }
    let continued = workflow(
        "jobs:\n  mutation:\n    uses: example/.github/workflows/mutation-cargo.yml@123\n    \
         continue-on-error: true\n    with:\n      setup-commands: |\n        sudo apt-get \
         install clang lld mold\n        make install-build-tools\n",
    )
    .expect("parse a continued mutation fixture");
    let problems = workflow_violations(&[("mutation.yml".to_owned(), continued)])
        .expect("inspect continued mutation setup");
    assert!(
        !problems.is_empty(),
        "mutation failure masking must be rejected"
    );
}

#[test]
fn non_linux_suite_does_not_require_mold() {
    let source = "jobs:\n  windows-suite:\n    runs-on: windows-latest\n    steps:\n      - run: \
                  cargo test\n";
    let document = workflow(source).expect("parse a Windows suite fixture");
    let problems = workflow_violations(&[("windows.yml".to_owned(), document)])
        .expect("inspect a Windows suite");
    assert!(
        problems.is_empty(),
        "non-Linux suite must keep its platform linker: {problems:?}"
    );
}
