//! Checks the exact Cargo route and failure propagation for the Act contract target.

use super::{CargoProbe, MakeOptions};

#[test]
fn act_contract_smoke_uses_exact_cargo_route_and_propagates_failure() {
    let probe = CargoProbe::new().expect("create Cargo probe");
    let preflight = probe.path.join("preflight");
    let options = MakeOptions {
        nextest_available: true,
        dry_run: false,
        with_act: false,
        check: preflight.as_str(),
    };
    let expected_cargo = "test --test act_workflow \
                          smoke::real_act_runs_derived_workflow_and_propagates_failure -- \
                          --ignored --exact";

    let success = probe
        .make_command("act-contract-smoke", options)
        .env("CARGO_ENV_LOG", probe.path.join("cargo-env.log").as_str())
        .env("RUSTFLAGS", "inherited-contract-smoke")
        .output()
        .expect("run successful Act contract Make route");
    assert!(
        success.status.success(),
        "act-contract-smoke must execute its Cargo route: {}",
        String::from_utf8_lossy(&success.stderr)
    );
    let calls = probe
        .directory
        .read_to_string("cargo.log")
        .expect("read successful Cargo route");
    assert_eq!(
        calls,
        format!("preflight\n{expected_cargo}\n"),
        "act-contract-smoke must invoke only its exact ignored-test contract after preflight"
    );
    let rustflags = probe
        .directory
        .read_to_string("cargo-env.log")
        .expect("read Cargo environment");
    assert!(
        rustflags.starts_with("inherited-contract-smoke ")
            && rustflags.contains("-D warnings")
            && rustflags.contains("-Zthreads=8"),
        "act-contract-smoke must retain inherited flags and add development flags: {rustflags}"
    );

    probe
        .directory
        .remove_file("cargo.log")
        .expect("clear successful Cargo route");
    let failure = probe
        .make_command("act-contract-smoke", options)
        .env("CARGO_PROBE_STATUS", "23")
        .output()
        .expect("run failing Act contract Make route");
    assert!(
        !failure.status.success(),
        "a failing Act contract test must fail the Make target"
    );
    let failed_calls = probe
        .directory
        .read_to_string("cargo.log")
        .expect("read failed Cargo route");
    assert_eq!(
        failed_calls,
        format!("preflight\n{expected_cargo}\n"),
        "Make must propagate the one failed Act contract invocation without rerunning it"
    );
}
