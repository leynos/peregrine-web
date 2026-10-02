//! Shell executors used by the real Make gate harness.

/// Cargo fixture that validates supported gate commands and nextest selection.
pub(super) const CARGO_SCRIPT: &str = r#"
stage=unexpected-cargo
case "$1" in
  fmt)
    stage=fmt
    [ "$#" -eq 4 ] && [ "$2" = --all ] && [ "$3" = -- ] && [ "$4" = --check ] || stage=unexpected-cargo
    ;;
  doc)
    stage=doc
    [ "$#" -eq 2 ] && [ "$2" = --no-deps ] || stage=unexpected-cargo
    ;;
  clippy)
    stage=clippy
    [ "$#" -eq 6 ] && [ "$2" = --all-targets ] && [ "$3" = --all-features ] &&
      [ "$4" = -- ] && [ "$5" = -D ] && [ "$6" = warnings ] || stage=unexpected-cargo
    ;;
  nextest)
    if [ "$#" -eq 2 ] && [ "$2" = --version ]; then
      stage=nextest-version-probe
    elif [ "$#" -eq 4 ] && [ "$2" = run ] && [ "$3" = --all-targets ] &&
        [ "$4" = --all-features ]; then
      stage=nextest
    else stage=unexpected-cargo; fi
    ;;
  test)
    stage=doctest
    [ "$#" -eq 4 ] && [ "$2" = --doc ] && [ "$3" = --workspace ] &&
      [ "$4" = --all-features ] || stage=unexpected-cargo
    ;;
  *) stage=unexpected-cargo ;;
esac
record_invocation "$GATE_LOG" "$stage" "${0##*/}" none none '' "$@"
case "$stage" in unexpected-*) exit 64 ;; esac
if [ "${GATE_FAIL_AT-}" = "$stage" ]; then exit 1; fi
if [ "$stage" = nextest-version-probe ]; then printf '%s\n' 'cargo-nextest 0.9.0'; fi
"#;

/// Preflight fixture rejects arguments and supports controlled failure.
pub(super) const PREFLIGHT_SCRIPT: &str = r#"
stage=preflight
if [ "$#" -ne 0 ]; then
  stage=unexpected-preflight
  record_invocation "$GATE_LOG" "$stage" "${0##*/}" none none '' "$@"
  exit 64
fi
record_invocation "$GATE_LOG" "$stage" "${0##*/}" none none '' "$@"
[ "${GATE_FAIL_AT-}" != "$stage" ]
"#;

/// Markdown formatter fixture validates the exact check-only command.
pub(super) const MDTABLEFIX_SCRIPT: &str = r#"
stage=markdown-formatting
[ "$#" -eq 8 ] && [ "$1" = --check ] && [ "$2" = --git ] &&
  [ "$3" = --include-untracked ] && [ "$4" = --wrap ] &&
  [ "$5" = --renumber ] && [ "$6" = --breaks ] &&
  [ "$7" = --ellipsis ] && [ "$8" = --fences ] || stage=unexpected-mdtablefix
record_invocation "$GATE_LOG" "$stage" "${0##*/}" none none '' "$@"
[ "$stage" = markdown-formatting ] || exit 64
[ "${GATE_FAIL_AT-}" != "markdown-formatting" ]
"#;

/// markdownlint fixture detects unexpected execution during check-fmt.
pub(super) const MDLINT_SCRIPT: &str = r#"
stage=unexpected-markdownlint
record_invocation "$GATE_LOG" "$stage" "${0##*/}" none none '' "$@"
exit 64
"#;

/// Spelling fixture validates the builder's binding `gate` command.
pub(super) const SPELLING_SCRIPT: &str = r#"
stage=spelling
if [ "$#" -ne 1 ] || [ "$1" != gate ]; then
  stage=unexpected-spelling
  record_invocation "$GATE_LOG" "$stage" "${0##*/}" none none '' "$@"
  exit 64
fi
record_invocation "$GATE_LOG" "$stage" "${0##*/}" none none '' "$@"
[ "${GATE_FAIL_AT-}" != "$stage" ]
"#;

/// Act fixture detects a forbidden nested execution path.
pub(super) const ACT_SCRIPT: &str = r#"
stage=unexpected-act
record_invocation "$GATE_LOG" "$stage" "${0##*/}" none none '' "$@"
exit 64
"#;

/// Whitaker fixture proves private cache warmth and repository Cargo settings.
pub(super) const WHITAKER_SCRIPT: &str = r#"
cache="$DYLINT_DRIVER_PATH/driver-marker"
if [ -f "$cache" ]; then
  cache_state=warm
else
  mkdir -p "$DYLINT_DRIVER_PATH" && : > "$cache" || exit 1
  cache_state=cold
fi
if [ "$#" -ne 4 ] || [ "$1" != --all ] || [ "$2" != -- ] ||
   [ "$3" != --all-targets ] || [ "$4" != --all-features ]; then
  record_invocation "$GATE_LOG" unexpected-whitaker "${0##*/}" "$cache_state" none '' "$@"
  exit 64
fi
record_invocation "$GATE_LOG" whitaker "${0##*/}" "$cache_state" none '' "$@"
if [ "${GATE_FAIL_AT-}" = whitaker ]; then exit 1; fi
record_repository_config() {
  query=$1
  output=$(cargo -Z unstable-options config get "$query") || return 1
  record_invocation "$WHITAKER_ENV_LOG" repository-config cargo none value "$output" -Z unstable-options config get "$query"
}
record_repository_config profile.dev.codegen-backend || exit 1
record_repository_config build.rustflags || exit 1
record_repository_config target.x86_64-unknown-linux-gnu.linker || exit 1
record_repository_config target.x86_64-unknown-linux-gnu.rustflags || exit 1
"#;

/// `mktemp` fixture creates a private driver directory or injects failure.
pub(super) const MKTEMP_SCRIPT: &str = r#"
stage=mktemp
if [ "$#" -ne 2 ] || [ "$1" != -d ] ||
   [ "$2" != "$BUILD_TOOLS_PREFIX/target/whitaker-driver.XXXXXX" ]; then
  stage=unexpected-mktemp
fi
record_invocation "$GATE_LOG" "$stage" "${0##*/}" none none '' "$@"
[ "$stage" = mktemp ] || exit 64
[ "${GATE_FAIL_AT-}" != mktemp ] || exit 1
cache_dir="$BUILD_TOOLS_PREFIX/target/whitaker-driver.fixture"
mkdir "$cache_dir" || exit 1
printf '%s\n' "$cache_dir"
"#;
