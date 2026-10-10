#!/usr/bin/env bash
# Fast capability check for the repository's build standard.
#
# Runs before every build target so a missing tool produces an actionable
# installation hint rather than an opaque linker failure deep inside a Cargo
# invocation. Exits non-zero when a required component is absent, unusable, or
# does not match its pin.

set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=scripts/build-tools-common.sh
. "$script_dir/build-tools-common.sh"

# Report on the executable Clang actually selects through the native wrapper.
# Hosts without the x86_64 GNU Linux target table keep their platform linker.
check_mold() {
  local pinned=$1
  if ! uses_mold; then
    note "the pinned mold target is native x86_64 GNU Linux; using the platform linker on $(uname -sm)"
    return 0
  fi
  verify_native_mold "$pinned" true
}

# Report on the toolchain half of the prerequisites: rustup itself and the
# pinned nightly. Any absence is fatal: the standard's parallel frontend is a
# nightly-only flag, and Cranelift is a pinned toolchain component.
check_toolchain() {
  local toolchain=$1 component installed_name component_list installed_components host status=0
  if ! command -v rustup >/dev/null 2>&1; then
    note 'rustup not found on PATH; it is required to select the pinned nightly'
    note 'install it from https://rustup.rs'
    return 1
  fi
  host=$(default_host_triple) || return 1
  if [ -z "$host" ]; then
    note 'cannot determine the rustup default host; check rustup show'
    return 1
  fi
  if ! rustup toolchain list | awk -v expected="$toolchain-$host" \
    '$1 == expected { found = 1 } END { exit !found }'; then
    note "toolchain $toolchain is not installed"
    note 'install it with: make install-build-tools'
    return 1
  fi
  installed_components=$(rustup component list --toolchain "$toolchain" --installed) || {
    note "cannot inspect components for $toolchain; run make install-build-tools"
    return 1
  }
  component_list=$(pinned_components) || return 1
  while IFS= read -r component; do
    # rustup accepts the manifest's preview names but lists these installed
    # components without the suffix (llvm-tools and rustc-codegen-cranelift).
    installed_name=${component%-preview}
    if ! printf '%s\n' "$installed_components" | grep -Fxq "$installed_name-$host"; then
      note "missing $component for $toolchain; run make install-build-tools"
      status=1
    fi
  done <<< "$component_list"
  [ "$status" -eq 0 ] && note "toolchain $toolchain and selected components available"
  return "$status"
}

# Check the linker executables selected by the Cargo target and coverage flags.
check_linkers() {
  local coverage=$1 status=0 clang_command=${CLANG_COMMAND:-clang} lld_command=${LLD_COMMAND:-ld.lld}
  if uses_mold && ! command -v "$clang_command" >/dev/null 2>&1; then
    note 'clang is missing for x86_64 GNU Linux; install the clang system package'
    status=1
  fi
  if [ "$coverage" = true ]; then
    if ! command -v "$clang_command" >/dev/null 2>&1; then
      note 'clang is missing for coverage; install the clang system package'
      status=1
    fi
    if ! command -v "$lld_command" >/dev/null 2>&1; then
      note 'ld.lld is missing for coverage; install the lld system package'
      status=1
    fi
  fi
  return "$status"
}

# The development standard selects a native linker. Cargo's encoded flags take
# precedence over RUSTFLAGS, while an explicit cross target may select a
# different linker; neither route is part of this repository's supported matrix.
check_cargo_overrides() {
  local coverage=$1 target=${CARGO_BUILD_TARGET:-}
  if [ "${CARGO_ENCODED_RUSTFLAGS+x}" = x ]; then
    note 'CARGO_ENCODED_RUSTFLAGS overrides the development flags; unset it before running Make gates'
    return 1
  fi
  if [ -n "$target" ] && { ! uses_mold || [ "$target" != 'x86_64-unknown-linux-gnu' ]; }; then
    note "CARGO_BUILD_TARGET=$target is outside the supported native build; unset it before running Make gates"
    return 1
  fi
  if [ "$coverage" = false ] && [ "${CARGO_PROFILE_DEV_CODEGEN_BACKEND+x}" = x ] \
    && [ "$CARGO_PROFILE_DEV_CODEGEN_BACKEND" != cranelift ]; then
    note 'CARGO_PROFILE_DEV_CODEGEN_BACKEND overrides Cranelift; unset it before development Make gates'
    return 1
  fi
  if [ "$coverage" = false ] && uses_mold \
    && [ "${CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER+x}" = x ] \
    && [ "$CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER" != 'scripts/native-clang-linker.sh' ]; then
    note 'CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER bypasses the pinned Clang linker wrapper; unset it before development Make gates'
    return 1
  fi
}

# Run both checks unconditionally so a developer sees every missing piece in one
# pass rather than fixing them one failed run at a time.
main() {
  local status=0 mold_pin toolchain_pin coverage=false
  case ${1:-} in
    '') ;;
    --coverage) coverage=true ;;
    *) fail "unsupported check-build-tools option: $1" ;;
  esac
  # Resolve the pins into variables first. `fail` exits, but inside a command
  # substitution that exit kills only the subshell, so passing `$(mold_version)`
  # straight into a check would continue with an empty pin and report a
  # nonsensical drift. An assignment propagates the status, so this stops.
  mold_pin=$(mold_version) || return 1
  toolchain_pin=$(pinned_toolchain) || return 1
  check_mold "$mold_pin" || status=1
  check_toolchain "$toolchain_pin" || status=1
  check_linkers "$coverage" || status=1
  check_cargo_overrides "$coverage" || status=1
  [ "$status" -eq 0 ] || note 'capability check failed; see the messages above'
  return "$status"
}

main "$@"
