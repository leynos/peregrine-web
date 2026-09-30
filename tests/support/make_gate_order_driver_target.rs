//! Exercise the real Make target that owns cold and warm Whitaker driver setup.

use camino::Utf8Path;

use super::{
    probe::GateProbe,
    records::{EnvironmentValue, Invocation},
};

/// Checks failed first-run lint propagates and the target trap removes its cache.
#[test]
fn whitaker_driver_target_stops_after_first_failure_and_cleans_cache() {
    let probe = GateProbe::new().expect("create private gate probes");
    let output = probe
        .run_in_private_scratch("whitaker-driver-integration", Some("whitaker"))
        .expect("run real Make target in private scratch");
    assert!(
        !output.status.success(),
        "the first controlled Whitaker failure must fail the integration target"
    );
    let invocations = probe
        .invocations()
        .expect("parse integration target records");
    assert_eq!(
        probe.stages().expect("read target invocation order"),
        ["mktemp", "preflight", "whitaker"],
        "a failed first Whitaker run must prevent the warm run"
    );
    assert_mktemp_command(&probe, &invocations);
    let whitaker_run = invocations
        .iter()
        .find(|invocation| invocation.stage == "whitaker")
        .expect("record the first controlled Whitaker run");
    assert_eq!(
        whitaker_run.cache_state, "cold",
        "the first invocation must construct the private driver cache"
    );
    let cache_path = match whitaker_run.environment.get("DYLINT_DRIVER_PATH") {
        Some(EnvironmentValue::Value(path)) => path,
        other => panic!("the first invocation must receive its private cache path: {other:?}"),
    };
    assert_eq!(
        cache_path,
        &format!("{}/target/whitaker-driver.fixture", probe.root_path()),
        "the driver cache must stay inside this test's private target directory"
    );
    let relative_cache = Utf8Path::new(cache_path)
        .strip_prefix(probe.root_path())
        .expect("cache path must be contained within private scratch");
    assert!(
        !probe
            .directory_exists(relative_cache)
            .expect("check the trap-owned cache path"),
        "the Make trap must remove the private cache after first-run failure"
    );
}

/// Checks a failed private `mktemp` prevents the first Whitaker invocation.
#[test]
fn whitaker_driver_target_propagates_cache_creation_failure() {
    let probe = GateProbe::new().expect("create private gate probes");
    let output = probe
        .run_in_private_scratch("whitaker-driver-integration", Some("mktemp"))
        .expect("run real Make target with failing private mktemp");
    assert!(
        !output.status.success(),
        "cache creation failure must fail the integration target"
    );
    let invocations = probe.invocations().expect("parse cache failure records");
    assert_eq!(
        probe.stages().expect("read cache failure order"),
        ["mktemp"],
        "cache creation failure must stop before preflight or Whitaker"
    );
    assert_mktemp_command(&probe, &invocations);
    assert!(
        !probe
            .directory_exists(Utf8Path::new("target/whitaker-driver.fixture"))
            .expect("check that no private cache was created"),
        "the failing cache creator must leave no cache directory"
    );
}

/// Verifies the integration target's exact private `mktemp` command.
fn assert_mktemp_command(probe: &GateProbe, invocations: &[Invocation]) {
    let matching = invocations
        .iter()
        .filter(|invocation| invocation.stage == "mktemp")
        .collect::<Vec<_>>();
    assert_eq!(
        matching.len(),
        1,
        "the target must create exactly one cache"
    );
    let expected_arguments = vec![
        "-d".to_owned(),
        format!("{}/target/whitaker-driver.XXXXXX", probe.root_path()),
    ];
    for invocation in matching {
        assert_eq!(
            invocation.executable, "mktemp",
            "the fixed mktemp command must resolve to the controlled executable"
        );
        assert_eq!(
            invocation.working_directory,
            probe.root_path().as_str(),
            "the integration target must run from private scratch"
        );
        assert_eq!(
            invocation.environment.get("BUILD_TOOLS_PREFIX"),
            Some(&EnvironmentValue::Value(probe.root_path().to_string())),
            "the private bin directory must lead tool lookup for this test"
        );
        assert_eq!(
            invocation.arguments, expected_arguments,
            "the target must use the expected temporary cache template"
        );
    }
}
