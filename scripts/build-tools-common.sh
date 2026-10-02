#!/usr/bin/env bash
# Shared helpers for the build tools the repository's standard needs.
#
# Sourced by scripts/install-build-tools.sh and scripts/check-build-tools.sh so both
# resolve the pinned versions and emit diagnostics identically. Every message is
# prefixed `build-tools:` and written to stderr for consistent diagnostics.

set -euo pipefail

# Locate the repository from this file rather than from the working directory,
# so the entry points run correctly when invoked directly as well as through the
# `install-build-tools` and `check-build-tools` targets. `BASH_SOURCE[0]` is this
# file even when sourced, which is what makes the derivation reliable.
BUILD_TOOLS_HELPER_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
BUILD_TOOLS_REPO_ROOT=$(cd -- "$BUILD_TOOLS_HELPER_DIR/.." && pwd)

# Pin files default to their committed locations. An explicit override still
# wins, which is what lets the tests point the scripts at fixtures. `read_pin`
# validates whichever path is selected, so a missing or empty file is reported
# the same way whether it came from a default or an override.
MOLD_VERSION_FILE="${MOLD_VERSION_FILE:-$BUILD_TOOLS_REPO_ROOT/tools/mold/VERSION}"
MOLD_SHA256SUMS_FILE="${MOLD_SHA256SUMS_FILE:-$BUILD_TOOLS_REPO_ROOT/tools/mold/SHA256SUMS}"
RUST_TOOLCHAIN_FILE="${RUST_TOOLCHAIN_FILE:-$BUILD_TOOLS_REPO_ROOT/rust-toolchain.toml}"

# Prefix for the mold installation tree. The native Clang wrapper passes this
# exact prefix to `-B`, so Clang finds its `ld.mold` ahead of system linkers.
# Make also prepends `bin/` to PATH for direct tool invocations.
BUILD_TOOLS_PREFIX="${BUILD_TOOLS_PREFIX:-$HOME/.local}"

# Emit a diagnostic. Always stderr, so a caller may capture a helper's stdout
# without the diagnostics contaminating the captured value.
note() { printf 'build-tools: %s\n' "$*" >&2; }

# Emit a diagnostic and abort. Used for conditions no caller can recover from,
# such as a missing pin file or an unverifiable download.
fail() {
  printf 'build-tools: %s\n' "$*" >&2
  exit 1
}

# Read a single-line version pin, trimming only its leading and trailing
# whitespace.
#
# A missing, blank, or multi-line pin aborts rather than yielding a version that
# would silently produce a nonsensical download URL. Deleting every whitespace
# character instead would corrupt rather than reject: a stray space would turn
# `1.2 3` into `1.23`, and a second line would concatenate into `1.2.34.5.6`.
# Internal whitespace is likewise rejected, because no pin legitimately contains
# any and silently rewriting one is worse than refusing it.
read_pin() {
  local file=$1 value lines
  [ -f "$file" ] || fail "missing version pin: $file"
  lines=$(grep -c '' <"$file")
  [ "$lines" -le 1 ] || fail "expected one line in version pin: $file, found $lines"
  # `$(...)` strips trailing newlines; the parameter expansions trim spaces and
  # tabs from each end without touching anything between them.
  value=$(cat -- "$file")
  value=${value#"${value%%[![:space:]]*}"}
  value=${value%"${value##*[![:space:]]}"}
  [ -n "$value" ] || fail "empty version pin: $file"
  case $value in
    *[[:space:]]*) fail "version pin contains whitespace: $file" ;;
  esac
  printf '%s' "$value"
}

# The pinned mold release tag, e.g. "2.41.0".
mold_version() { read_pin "$MOLD_VERSION_FILE"; }

# The repository's toolchain, read from `rust-toolchain.toml`.
#
# Deliberately the same toolchain the ordinary gates use, not a second pin. The
# repository pins this nightly for its compiler experiments and Cranelift
# backend, and the parallel frontend is a nightly-only flag.
pinned_toolchain() {
  local file=$RUST_TOOLCHAIN_FILE value
  [ -f "$file" ] || fail "missing version pin: $file"
  value=$(awk -F'"' '/^[[:space:]]*channel[[:space:]]*=/ { print $2; exit }' "$file")
  [ -n "$value" ] || fail "no channel found in: $file"
  printf '%s' "$value"
}

# Read the component list from the same rustup pin used by Cargo. This file
# has one quoted component per line; reject an empty list rather than passing
# a bare compiler installation off as a complete development toolchain.
pinned_components() {
  local file=$RUST_TOOLCHAIN_FILE components
  [ -f "$file" ] || fail "missing toolchain pin: $file"
  components=$(awk -F '"' '/^[[:space:]]*"[^"]+",?[[:space:]]*$/ { print $2 }' "$file")
  [ -n "$components" ] || fail "no components found in: $file"
  printf '%s\n' "$components"
}

# Whether the host can use mold at all; it ships for Linux only.
is_linux() { [ "$(uname -s)" = 'Linux' ]; }

# The committed target table selects mold only for native x86_64 GNU Linux.
default_host_triple() { rustup show | sed -n 's/^Default host: //p'; }
uses_mold() {
  is_linux && [ "$(uname -m)" = 'x86_64' ] \
    && [ "$(default_host_triple)" = 'x86_64-unknown-linux-gnu' ]
}

# The version of the linker executable Clang resolves, or a non-zero status
# when it cannot be run.
installed_mold_version() {
  # `mold --version` prints e.g. "mold 2.41.0 (compatible with GNU ld)". Capture
  # first so a failing mold propagates its status instead of being masked by the
  # exit status of a downstream awk.
  local output
  output=$("$1" --version 2>/dev/null) || return 1
  printf '%s' "$output" | awk 'NR == 1 { print $2 }'
}

# Verify the native Clang route shared by bare Cargo's linker wrapper and the
# Make preflight. Clang searches its installed directory before PATH, so only
# its own `-B` search result and link plan prove which ld.mold will run.
verify_native_mold() {
  local pinned=$1 report_success=${2:-false} clang_command=${CLANG_COMMAND:-clang} resolved expected selected plan installed
  expected="$BUILD_TOOLS_PREFIX/bin/ld.mold"
  if [ ! -x "$expected" ]; then
    note "pinned Clang linker $expected is missing or not executable"
    note 'install it with: make install-build-tools'
    return 1
  fi
  if ! resolved=$("$clang_command" -B "$BUILD_TOOLS_PREFIX/bin" -print-prog-name=ld.mold) \
    || [ -z "$resolved" ]; then
    note 'Clang cannot resolve ld.mold through the pinned linker directory'
    return 1
  fi
  selected=$(readlink -f -- "$resolved") || return 1
  expected=$(readlink -f -- "$expected") || return 1
  if [ "$selected" != "$expected" ]; then
    note "Clang resolves $resolved, not pinned $BUILD_TOOLS_PREFIX/bin/ld.mold"
    note 'run make install-build-tools and check the native Clang linker route'
    return 1
  fi
  if ! plan=$("$clang_command" -B "$BUILD_TOOLS_PREFIX/bin" \
    -### -fuse-ld=mold -x c /dev/null -o /dev/null 2>&1) \
    || ! printf '%s\n' "$plan" | grep -F " \"$resolved\" " >/dev/null; then
    note "Clang's link plan does not execute $resolved"
    return 1
  fi
  if ! installed=$(installed_mold_version "$resolved") || [ -z "$installed" ]; then
    note "Clang's mold at $resolved cannot report its version"
    note 'reinstall it with: make install-build-tools'
    return 1
  fi
  if [ "$installed" != "$pinned" ]; then
    note "mold $installed at $resolved does not match the pin $pinned"
    note 'run make install-build-tools to match'
    return 1
  fi
  # Rustc treats successful linker stderr as a warning; only the standalone
  # preflight reports successful selection.
  if [ "$report_success" = true ]; then
    note "mold $installed at $resolved"
  fi
}
