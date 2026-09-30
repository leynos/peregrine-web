//! Mutation tests for hosted workflow and manual Act route contracts.

use super::{ACT_VALIDATION_WORKFLOW, CI_WORKFLOW, contracts_hold};

#[test]
fn hosted_gate_and_manual_act_have_distinct_routes() {
    assert!(
        contracts_hold(CI_WORKFLOW, ACT_VALIDATION_WORKFLOW),
        "the committed workflows must retain the hosted gate and manual Act route"
    );
}

#[test]
fn workflow_mutations_break_the_contract() {
    for (reason, changed) in [
        (
            "wrong Make goal",
            CI_WORKFLOW.replace("          make lint", "          make typecheck"),
        ),
        (
            "missing Act recursion guard",
            CI_WORKFLOW.replace("WITH_ACT: '0'", "WITH_ACT: '1'"),
        ),
        (
            "coverage guard removed",
            CI_WORKFLOW.replace("if: env.ACT != 'true'", "if: env.ACT == 'true'"),
        ),
        (
            "changed Whitaker action",
            CI_WORKFLOW.replace("6dea5677a84fec60ca51b07202570e3af12ffdb4", "main"),
        ),
        (
            "changed Whitaker input",
            CI_WORKFLOW.replace("cranelift: 'true'", "cranelift: 'false'"),
        ),
        (
            "missing installer",
            CI_WORKFLOW.replace("- name: Install Whitaker", "- name: Missing Whitaker"),
        ),
        (
            "suppressed failure",
            CI_WORKFLOW.replace(
                "- name: Lint\n",
                "- name: Lint\n        continue-on-error: true\n",
            ),
        ),
        (
            "unexpected executor",
            CI_WORKFLOW.replace("          make lint", "          custom-lint"),
        ),
    ] {
        assert!(
            !contracts_hold(&changed, ACT_VALIDATION_WORKFLOW),
            "contract must detect {reason}"
        );
    }
}

#[test]
fn workflow_execution_mutations_break_the_contract() {
    for (reason, changed) in [
        (
            "disabled lint",
            CI_WORKFLOW.replace("- name: Lint\n", "- name: Lint\n        if: false\n"),
        ),
        (
            "disabled job",
            CI_WORKFLOW.replace("build-test:\n", "build-test:\n    if: false\n"),
        ),
        (
            "duplicate PR job",
            CI_WORKFLOW.replace(
                "jobs:\n",
                "jobs:\n  duplicate:\n    runs-on: ubuntu-latest\n    steps:\n      - run: make \
                 test WITH_ACT=1\n",
            ),
        ),
        (
            "unknown shell executor",
            CI_WORKFLOW.replace("          make lint", "          cargo evil"),
        ),
        (
            "missing private cold cache",
            CI_WORKFLOW.replace(
                "DYLINT_DRIVER_PATH: ${{ runner.temp }}/peregrine-whitaker-driver",
                "DYLINT_DRIVER_PATH: ''",
            ),
        ),
        (
            "missing private cache creation",
            CI_WORKFLOW.replace(
                "          mkdir -p \"$DYLINT_DRIVER_PATH\"",
                "          echo no-cache",
            ),
        ),
        (
            "lint backend override",
            CI_WORKFLOW.replace(
                "DYLINT_DRIVER_PATH: ${{ runner.temp }}/peregrine-whitaker-driver",
                "DYLINT_DRIVER_PATH: ${{ runner.temp }}/peregrine-whitaker-driver\n          \
                 CARGO_PROFILE_DEV_CODEGEN_BACKEND: llvm",
            ),
        ),
    ] {
        assert!(
            !contracts_hold(&changed, ACT_VALIDATION_WORKFLOW),
            "contract must detect {reason}"
        );
    }
}

#[test]
fn workflow_environment_mutations_break_the_contract() {
    for (reason, changed) in [
        (
            "lost coverage LLVM",
            CI_WORKFLOW.replace(
                "CARGO_PROFILE_DEV_CODEGEN_BACKEND: llvm",
                "CARGO_PROFILE_DEV_CODEGEN_BACKEND: cranelift",
            ),
        ),
        (
            "disabled PR updates",
            CI_WORKFLOW.replace("[opened, synchronize, reopened]", "[closed]"),
        ),
        (
            "different job cwd",
            CI_WORKFLOW.replace(
                "  build-test:\n",
                "  build-test:\n    defaults:\n      run:\n        working-directory: /tmp\n",
            ),
        ),
        (
            "different lint cwd",
            CI_WORKFLOW.replace(
                "- name: Lint\n",
                "- name: Lint\n        working-directory: /tmp\n",
            ),
        ),
        (
            "different lint shell",
            CI_WORKFLOW.replace("- name: Lint\n", "- name: Lint\n        shell: sh\n"),
        ),
    ] {
        assert!(
            !contracts_hold(&changed, ACT_VALIDATION_WORKFLOW),
            "contract must detect {reason}"
        );
    }
    let enabled_pr_act = ACT_VALIDATION_WORKFLOW.replace(
        "  workflow_dispatch:",
        "  pull_request:\n  workflow_dispatch:",
    );
    assert!(
        !contracts_hold(CI_WORKFLOW, &enabled_pr_act),
        "Act must not repeat full validation on pull requests"
    );
    let wrong_manual_pin =
        ACT_VALIDATION_WORKFLOW.replace("47b337e4f230b591891656534d4ffad868131740", "main");
    assert!(
        !contracts_hold(CI_WORKFLOW, &wrong_manual_pin),
        "manual Act's action revision must stay pinned"
    );
}

#[test]
fn ignores_inert_package_references() {
    for inert in [
        "# sudo apt-get install clang mold",
        "echo sudo apt-get install clang mold",
        "sudo apt-get install clang # mold",
        "sudo apt-get install other || echo clang mold",
        "sudo apt-get install clang mold\\ ",
    ] {
        assert!(
            !super::super::is_linker_install_command(inert),
            "inert package text must not satisfy installation: {inert}"
        );
    }
}
