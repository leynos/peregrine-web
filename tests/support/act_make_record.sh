record_environment() {
  name=$1
  eval "is_set=\${$name+set}"
  if [ "$is_set" != set ]; then
    printf 'env\0%s\0unset\0\0' "$name"
    return
  fi
  eval "value=\${$name}"
  if [ -z "$value" ]; then
    printf 'env\0%s\0empty\0\0' "$name"
  else
    printf 'env\0%s\0value\0%s\0' "$name" "$value"
  fi
}

record_github_token_presence() {
  if [ "${GITHUB_TOKEN+x}" != x ]; then
    state=unset
  elif [ -z "$GITHUB_TOKEN" ]; then
    state=empty
  else
    state=present
  fi
  printf 'secret\0GITHUB_TOKEN\0%s\0' "$state"
}

record_invocation() {
  {
    executable=${INVOCATION_EXECUTABLE:-${0##*/}}
    printf 'invocation-v1\0%s\0%s\0' "$executable" "$(pwd -P)"
    record_environment WITH_ACT
    record_environment ACT
    record_environment RUSTFLAGS
    record_environment CARGO_ENCODED_RUSTFLAGS
    record_environment CARGO_PROFILE_DEV_CODEGEN_BACKEND
    record_environment CARGO_BUILD_TARGET
    record_environment CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER
    record_github_token_presence
    printf 'argc\0%s\0argv\0' "$#"
    if [ "$#" -gt 0 ]; then
      printf '%s\0' "$@"
    fi
    printf 'end-invocation\0'
  } >> "$INVOCATION_LOG"
}
