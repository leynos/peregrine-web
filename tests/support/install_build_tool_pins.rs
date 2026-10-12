//! Fail-fast contracts for malformed build-tool pins.

#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

use rstest::rstest;

use super::support::InstallerFixture;

#[rstest]
#[case(None, "missing version pin")]
#[case(Some(" \t \n"), "empty version pin")]
#[case(Some("2.41.0\n2.41.1\n"), "expected one line in version pin")]
#[case(Some("2.41 .0\n"), "version pin contains whitespace")]
fn malformed_mold_pins_stop_before_installer_commands(
    #[case] pin: Option<&str>,
    #[case] diagnostic: &str,
) {
    let fixture = InstallerFixture::new().expect("create installer fixture");
    fixture
        .set_mold_version_pin(pin)
        .expect("set malformed mold pin fixture");
    let output = fixture
        .run("Linux", "x86_64", None)
        .expect("run installer with malformed mold pin");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let command_names = fixture
        .command_names()
        .expect("read installer command trace");

    assert!(
        !output.status.success() && stderr.contains(diagnostic),
        "malformed mold pin must fail with {diagnostic:?}: status={:?}; stderr={stderr}",
        output.status.code()
    );
    assert_eq!(
        command_names, "",
        "mold pin validation must stop before installer executors run"
    );
    assert_eq!(
        fixture.scratch_entries().expect("inspect scratch cleanup"),
        0,
        "mold pin validation must not create installer scratch"
    );
}

#[rstest]
#[case("[toolchain]\ncomponents = [\"clippy\"]\n", "no channel found")]
#[case(
    "[toolchain]\nchannel = nightly-2030-01-01\ncomponents = [\"clippy\"]\n",
    "no channel found"
)]
#[case(
    "[toolchain]\nchannel = \"nightly-2030-01-01\"\n",
    "no components found"
)]
#[case(
    "[toolchain]\nchannel = \"nightly-2030-01-01\"\ncomponents = [clippy]\n",
    "no components found"
)]
fn malformed_toolchain_pins_stop_before_installer_commands(
    #[case] contents: &str,
    #[case] diagnostic: &str,
) {
    let fixture = InstallerFixture::new().expect("create installer fixture");
    fixture
        .set_toolchain_pin(contents)
        .expect("set malformed toolchain pin fixture");
    let output = fixture
        .run("Linux", "x86_64", None)
        .expect("run installer with malformed toolchain pin");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let command_names = fixture
        .command_names()
        .expect("read installer command trace");

    assert!(
        !output.status.success() && stderr.contains(diagnostic),
        "malformed toolchain pin must fail with {diagnostic:?}: status={:?}; stderr={stderr}",
        output.status.code()
    );
    assert_eq!(
        command_names, "",
        "toolchain pin validation must stop before installer executors run"
    );
    assert_eq!(
        fixture.scratch_entries().expect("inspect scratch cleanup"),
        0,
        "toolchain pin validation must not create installer scratch"
    );
}
