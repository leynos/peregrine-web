//! Behavioural checks for missing pinned compiler and linker prerequisites.

#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

#[path = "support/build_tools_preflight.rs"]
mod support;

use std::{env, process::Command};

use support::ToolFixture;

#[test]
fn preflight_ignores_inherited_coverage_build_selection() {
    let test_binary = env::current_exe().expect("locate the preflight test binary");
    let output = Command::new(test_binary)
        .arg("--exact")
        .arg("preflight_child_accepts_default_development_tools")
        .env("CARGO_PROFILE_DEV_CODEGEN_BACKEND", "llvm")
        .env("CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER", "clang")
        .env("CARGO_BUILD_TARGET", "aarch64-unknown-linux-gnu")
        .env("CARGO_ENCODED_RUSTFLAGS", "-C\u{1f}link-arg=-fuse-ld=other")
        .output()
        .expect("run a preflight test with inherited coverage settings");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "default development preflight must ignore inherited build selection: {stdout} {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("1 passed"),
        "the isolated preflight test must execute: {stdout}"
    );
}

#[test]
fn preflight_child_accepts_default_development_tools() {
    let fixture = ToolFixture::new().expect("create fake build tools");
    let output = fixture
        .check(false, None, None)
        .expect("run default development preflight");
    assert!(
        output.status.success(),
        "default development preflight must pass: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn preflight_accepts_the_selected_toolchain_and_linkers() {
    let fixture = ToolFixture::new().expect("create fake build tools");
    for coverage in [false, true] {
        let output = fixture
            .check(coverage, None, None)
            .expect("run build-tool preflight");
        assert!(
            output.status.success(),
            "complete prerequisites must pass: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn preflight_reports_missing_compiler_and_linkers() {
    let fixture = ToolFixture::new().expect("create fake build tools");
    for (coverage, variable, diagnostic) in [
        (false, "CLANG_COMMAND", "clang is missing"),
        (true, "LLD_COMMAND", "ld.lld is missing"),
    ] {
        let output = fixture
            .check(coverage, Some(variable), None)
            .expect("run a missing-tool check");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "missing {variable} must fail preflight"
        );
        assert!(
            stderr.contains(diagnostic),
            "missing {variable} needs guidance: {stderr}"
        );
    }
}

#[test]
fn preflight_rejects_missing_or_diverted_clang_linker() {
    let fixture = ToolFixture::new().expect("create fake build tools");
    fixture
        .directory
        .write("other-ld.mold", "#!/bin/sh\nexit 0\n")
        .expect("create a divergent linker fixture");
    let other_linker = fixture.path.join("other-ld.mold");
    let diverted = fixture
        .check(
            false,
            None,
            Some(("FAKE_LINKER_PATH", other_linker.as_str())),
        )
        .expect("check a Clang selection outside the pinned prefix");
    assert!(
        !diverted.status.success(),
        "a different ld.mold must not satisfy the pinned Clang route"
    );
    assert!(
        String::from_utf8_lossy(&diverted.stderr).contains("not pinned"),
        "diverted Clang selection must explain the pin mismatch: {}",
        String::from_utf8_lossy(&diverted.stderr)
    );
    fixture
        .directory
        .remove_file("bin/ld.mold")
        .expect("remove the pinned Clang linker alias");
    let missing = fixture
        .check(false, None, None)
        .expect("check a missing pinned Clang linker");
    assert!(
        !missing.status.success(),
        "a missing pinned ld.mold must fail preflight"
    );
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("missing or not executable"),
        "a missing linker needs an install hint: {}",
        String::from_utf8_lossy(&missing.stderr)
    );
}

#[test]
fn preflight_rejects_a_missing_pinned_component() {
    let fixture = ToolFixture::new().expect("create fake build tools");
    fixture
        .write_components(Some("rust-analyzer"))
        .expect("remove rust-analyzer from the fake toolchain");
    let output = fixture
        .check(false, None, None)
        .expect("run a missing-component check");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "missing rust-analyzer must fail preflight"
    );
    assert!(
        stderr.contains("missing rust-analyzer"),
        "the missing component must have an actionable diagnostic: {stderr}"
    );
}

#[test]
fn preflight_matches_preview_components_by_their_installed_aliases() {
    let fixture =
        ToolFixture::new().expect("create fake build tools with rustup's installed aliases");
    let complete = fixture
        .check(false, None, None)
        .expect("check both installed preview aliases");
    assert!(
        complete.status.success(),
        "rustup's unsuffixed installed aliases must satisfy the pin"
    );
    for missing in ["llvm-tools-preview", "rustc-codegen-cranelift-preview"] {
        fixture
            .write_components(Some(missing))
            .expect("remove one installed preview alias");
        let output = fixture
            .check(false, None, None)
            .expect("check a missing preview component");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "missing {missing} must fail preflight"
        );
        assert!(
            stderr.contains(&format!("missing {missing}")),
            "missing {missing} needs its manifest name in the diagnostic: {stderr}"
        );
    }
}

#[test]
fn preflight_rejects_a_prefixed_but_different_toolchain() {
    let fixture = ToolFixture::new().expect("create fake build tools");
    fixture
        .directory
        .write(
            "toolchains",
            "nightly-2026-08-27-extra-x86_64-unknown-linux-gnu\n",
        )
        .expect("replace the exact toolchain with a prefixed neighbour");
    let output = fixture
        .check(false, None, None)
        .expect("check exact toolchain matching");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "a prefixed neighbour must not satisfy the pinned toolchain"
    );
    assert!(
        stderr.contains("toolchain nightly-2026-08-27 is not installed"),
        "missing exact toolchain needs an actionable diagnostic: {stderr}"
    );
}

#[test]
fn preflight_rejects_cross_target_and_encoded_flag_overrides() {
    let fixture = ToolFixture::new().expect("create fake build tools");
    for (variable, value, diagnostic) in [
        (
            "CARGO_BUILD_TARGET",
            "aarch64-unknown-linux-gnu",
            "outside the supported native build",
        ),
        (
            "CARGO_ENCODED_RUSTFLAGS",
            "-C\u{1f}link-arg=-fuse-ld=other",
            "overrides the development flags",
        ),
        (
            "CARGO_ENCODED_RUSTFLAGS",
            "",
            "overrides the development flags",
        ),
        (
            "CARGO_PROFILE_DEV_CODEGEN_BACKEND",
            "llvm",
            "overrides Cranelift",
        ),
        (
            "CARGO_PROFILE_DEV_CODEGEN_BACKEND",
            "other",
            "overrides Cranelift",
        ),
        (
            "CARGO_PROFILE_DEV_CODEGEN_BACKEND",
            "",
            "overrides Cranelift",
        ),
        (
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
            "ld",
            "bypasses the pinned Clang linker wrapper",
        ),
        (
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
            "",
            "bypasses the pinned Clang linker wrapper",
        ),
        (
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
            "clang",
            "bypasses the pinned Clang linker wrapper",
        ),
    ] {
        let output = fixture
            .check(false, None, Some((variable, value)))
            .expect("run an unsupported override check");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "unsupported {variable} must fail preflight"
        );
        assert!(
            stderr.contains(diagnostic),
            "unsupported {variable} needs guidance: {stderr}"
        );
    }
}

#[test]
fn preflight_allows_explicit_development_and_coverage_backends() {
    let fixture = ToolFixture::new().expect("create fake build tools");
    for (coverage, backend) in [(false, "cranelift"), (true, "llvm")] {
        let output = fixture
            .check(
                coverage,
                None,
                Some(("CARGO_PROFILE_DEV_CODEGEN_BACKEND", backend)),
            )
            .expect("check an explicitly selected supported backend");
        assert!(
            output.status.success(),
            "{backend} must suit coverage={coverage}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let native_linker = fixture
        .check(
            false,
            None,
            Some((
                "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
                "scripts/native-clang-linker.sh",
            )),
        )
        .expect("check an explicit native linker selection");
    assert!(
        native_linker.status.success(),
        "the native Clang wrapper must retain the pinned mold route: {}",
        String::from_utf8_lossy(&native_linker.stderr)
    );
}
