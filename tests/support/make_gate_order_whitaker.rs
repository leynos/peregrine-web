//! Fake Whitaker process and repository-route assertions for Make gate tests.

/// Captures the environment, Dylint arguments, private cache state, and Cargo config.
pub(super) const PROBE_SCRIPT: &str = concat!(
    "#!/bin/sh\n",
    "cache=\"$DYLINT_DRIVER_PATH/driver-marker\"\n",
    "if [ -f \"$cache\" ]; then cache_state=warm; else ",
    "mkdir -p \"$DYLINT_DRIVER_PATH\" && : > \"$cache\" || exit 1; ",
    "cache_state=cold; fi\n",
    "{ printf 'RUSTFLAGS=%s\\nENCODED=%s\\nBACKEND=%s\\n' ",
    "\"${RUSTFLAGS-<unset>}\" \"${CARGO_ENCODED_RUSTFLAGS-<unset>}\" ",
    "\"${CARGO_PROFILE_DEV_CODEGEN_BACKEND-<unset>}\"; ",
    "printf 'TARGET=%s\\nLINKER=%s\\nCFLAGS=%s\\nLDFLAGS=%s\\n' ",
    "\"${CARGO_BUILD_TARGET-<unset>}\" ",
    "\"${CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER-<unset>}\" ",
    "\"${CFLAGS-<unset>}\" \"${LDFLAGS-<unset>}\"; ",
    "printf 'DYLINT_RUSTFLAGS=%s\\nCACHE=%s\\nCWD=%s\\nARGS=' ",
    "\"${DYLINT_RUSTFLAGS-<unset>}\" \"$cache_state\" \"$PWD\"; ",
    "printf '<%s>' \"$@\"; printf '\\n'; ",
    "printf 'REPO_BACKEND=%s\\n' \"$(cargo -Z unstable-options config get ",
    "profile.dev.codegen-backend)\"; ",
    "printf 'REPO_BUILD_FLAGS=%s\\n' \"$(cargo -Z unstable-options config get ",
    "build.rustflags)\"; ",
    "printf 'REPO_LINKER=%s\\n' \"$(cargo -Z unstable-options config get ",
    "target.x86_64-unknown-linux-gnu.linker)\"; ",
    "printf 'REPO_TARGET_FLAGS=%s\\n' \"$(cargo -Z unstable-options config get ",
    "target.x86_64-unknown-linux-gnu.rustflags)\"; } >> \"$WHITAKER_ENV_LOG\"\n",
    "printf '%s\\n' whitaker >> \"$GATE_LOG\"\n",
    "[ \"${GATE_FAIL_AT:-}\" != whitaker ]\n",
);

/// Checks the repository-side Dylint invocation rather than only its outer environment.
pub(super) fn assert_repository_route(log: &str, cache_state: &str) {
    for setting in [
        "RUSTFLAGS=<unset>",
        "ENCODED=<unset>",
        "BACKEND=<unset>",
        "TARGET=<unset>",
        "LINKER=<unset>",
        "CFLAGS=<unset>",
        "LDFLAGS=<unset>",
        "DYLINT_RUSTFLAGS=-D warnings",
        "ARGS=<--all><--><--all-targets><--all-features>",
        "REPO_BACKEND=profile.dev.codegen-backend = \"cranelift\"",
        "REPO_BUILD_FLAGS=build.rustflags = [\"-Zthreads=8\"]",
        "REPO_LINKER=target.x86_64-unknown-linux-gnu.linker = \"scripts/native-clang-linker.sh\"",
        "REPO_TARGET_FLAGS=target.x86_64-unknown-linux-gnu.rustflags = [\"-Zthreads=8\", \"-C\", \
         \"link-arg=-fuse-ld=mold\"]",
    ] {
        assert!(
            log.contains(setting),
            "Whitaker repository checking must retain {setting}: {log}"
        );
    }
    assert!(
        log.contains(&format!("CACHE={cache_state}")),
        "the private probe cache must exercise {cache_state} state: {log}"
    );
    assert!(
        log.contains(concat!("CWD=", env!("CARGO_MANIFEST_DIR"))),
        "the repository Cargo configuration is discovered from its root: {log}"
    );
}
