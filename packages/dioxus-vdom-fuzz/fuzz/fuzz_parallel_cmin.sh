#!/usr/bin/env bash
set -euo pipefail

# Minimize the corpus, then run cargo-fuzz in parallel once.
#
# Environment overrides:
#   TARGET=vdom_ops
#   WORKERS=8
#   JOBS=8
#   FUZZ_SECONDS=1800
#   CORPUS=corpus/vdom_ops
#   TOOLCHAIN=nightly
#   LIBFUZZER_ARGS="-rss_limit_mb=8192"
#   ARTIFACTS=artifacts/vdom_ops

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$script_dir"

target="${TARGET:-vdom_ops}"
corpus="${CORPUS:-corpus/$target}"
artifacts="${ARTIFACTS:-artifacts/$target}"
toolchain="${TOOLCHAIN:-nightly}"
fuzz_seconds="${FUZZ_SECONDS:-1800}"

is_failure_artifact() {
  local name="${1##*/}"
  case "$name" in
    crash-* | timeout-* | oom-* | leak-*) return 0 ;;
    *) return 1 ;;
  esac
}

first_failure_from_log() {
  local log="$1"
  local line path

  while IFS= read -r line; do
    case "$line" in
      *"Test unit written to "*)
        path="${line#*Test unit written to }"
        path="${path%$'\r'}"
        path="${path%%[[:space:]]*}"

        if is_failure_artifact "$path" && [[ -f "$path" ]]; then
          printf '%s\n' "$path"
          return 0
        fi

        if is_failure_artifact "$path" && [[ -f "../$path" ]]; then
          printf '%s\n' "../$path"
          return 0
        fi
        ;;
    esac
  done <"$log"
}

file_mtime() {
  if stat -f '%m' "$1" >/dev/null 2>&1; then
    stat -f '%m' "$1"
  else
    stat -c '%Y' "$1"
  fi
}

first_new_failure_artifact() {
  local marker="$1"
  local dir="$2"
  local path mtime
  local first_path=""
  local first_mtime=""

  [[ -d "$dir" ]] || return 0

  while IFS= read -r -d '' path; do
    is_failure_artifact "$path" || continue
    mtime="$(file_mtime "$path")"

    if [[ -z "$first_path" ]] ||
      ((mtime < first_mtime)) ||
      (((mtime == first_mtime)) && [[ "$path" < "$first_path" ]]); then
      first_path="$path"
      first_mtime="$mtime"
    fi
  done < <(find "$dir" -type f -newer "$marker" -print0)

  if [[ -n "$first_path" ]]; then
    printf '%s\n' "$first_path"
  fi
}

default_workers="4"
if command -v sysctl >/dev/null 2>&1; then
  default_workers="$(sysctl -n hw.ncpu 2>/dev/null || printf '4')"
elif command -v nproc >/dev/null 2>&1; then
  default_workers="$(nproc 2>/dev/null || printf '4')"
fi

workers="${WORKERS:-$default_workers}"
jobs="${JOBS:-$workers}"

mkdir -p "$corpus" "$artifacts"

echo "target:       $target"
echo "corpus:       $corpus"
echo "artifacts:    $artifacts"
echo "workers/jobs: $workers/$jobs"
echo "epoch:        ${fuzz_seconds}s"
echo

echo "==> minimizing corpus in place"
cargo "+$toolchain" fuzz cmin -s none "$target" "$corpus"

fuzz_log="$(mktemp "${TMPDIR:-/tmp}/fuzz_parallel_cmin.XXXXXX.log")"
artifact_marker="$(mktemp "${TMPDIR:-/tmp}/fuzz_parallel_cmin.XXXXXX.marker")"
trap 'rm -f "$fuzz_log" "$artifact_marker"' EXIT

echo "==> fuzzing for ${fuzz_seconds}s"
set +e
cargo "+$toolchain" fuzz run -s none "$target" "$corpus" -- \
  -jobs="$jobs" \
  -workers="$workers" \
  -max_total_time="$fuzz_seconds" \
  ${LIBFUZZER_ARGS:-} 2>&1 | tee "$fuzz_log"
fuzz_status="${PIPESTATUS[0]}"
set -e

if ((fuzz_status == 0)); then
  exit 0
fi

failure_artifact="$(first_failure_from_log "$fuzz_log" || true)"
if [[ -z "$failure_artifact" ]]; then
  failure_artifact="$(first_new_failure_artifact "$artifact_marker" "$artifacts" || true)"
fi

if [[ -z "$failure_artifact" ]]; then
  echo "==> fuzzing failed with status $fuzz_status, but no new failure artifact was found" >&2
  exit "$fuzz_status"
fi

echo
echo "==> minimizing first failure: $failure_artifact"
set +e
cargo "+$toolchain" fuzz tmin -s none "$target" "$failure_artifact"
tmin_status="$?"
set -e

if ((tmin_status != 0)); then
  echo "==> minimization failed with status $tmin_status" >&2
  exit "$tmin_status"
fi

exit "$fuzz_status"
