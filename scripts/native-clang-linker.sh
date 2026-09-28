#!/usr/bin/env bash
# Give Clang the pinned mold directory before its installed linker directory.
# Cargo selects this wrapper for the x86_64 GNU Linux target; release links
# without `-fuse-ld=mold` still use their ordinary linker.

set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=scripts/build-tools-common.sh
. "$script_dir/build-tools-common.sh"

for argument in "$@"; do
  if [ "$argument" = '-fuse-ld=mold' ]; then
    uses_mold || fail 'the mold build route requires a native x86_64-unknown-linux-gnu host'
    pin=$(mold_version) || exit 1
    verify_native_mold "$pin" || exit 1
    break
  fi
done

exec "${CLANG_COMMAND:-clang}" -B "$BUILD_TOOLS_PREFIX/bin" "$@"
