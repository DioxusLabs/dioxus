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
#   MIN_CORPUS=/private/tmp/dioxus-vdom-fuzz/vdom_ops-minimized
#   TOOLCHAIN=nightly
#   LIBFUZZER_ARGS="-rss_limit_mb=8192"

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$script_dir"

target="${TARGET:-vdom_ops}"
corpus="${CORPUS:-corpus/$target}"
min_corpus="${MIN_CORPUS:-/private/tmp/dioxus-vdom-fuzz/$target-minimized}"
toolchain="${TOOLCHAIN:-nightly}"
fuzz_seconds="${FUZZ_SECONDS:-1800}"

default_workers="4"
if command -v sysctl >/dev/null 2>&1; then
  default_workers="$(sysctl -n hw.ncpu 2>/dev/null || printf '4')"
elif command -v nproc >/dev/null 2>&1; then
  default_workers="$(nproc 2>/dev/null || printf '4')"
fi

workers="${WORKERS:-$default_workers}"
jobs="${JOBS:-$workers}"

mkdir -p "$corpus" "$min_corpus"

minimize_corpus() {
  echo "==> minimizing corpus"
  tmp_corpus="${min_corpus}.tmp"
  rm -rf "$tmp_corpus"
  mkdir -p "$tmp_corpus"

  cargo "+$toolchain" fuzz cmin "$target" "$tmp_corpus"

  echo "==> replacing live corpus with minimized corpus"
  old_corpus="${corpus}.old"
  rm -rf "$old_corpus"
  if [ -d "$corpus" ]; then
    mv "$corpus" "$old_corpus"
  fi
  mv "$tmp_corpus" "$corpus"
  rm -rf "$old_corpus"
}

echo "target:       $target"
echo "corpus:       $corpus"
echo "min corpus:   $min_corpus"
echo "workers/jobs: $workers/$jobs"
echo "epoch:        ${fuzz_seconds}s"
echo

minimize_corpus

echo "==> fuzzing for ${fuzz_seconds}s"
cargo "+$toolchain" fuzz run "$target" "$corpus" -- \
  -jobs="$jobs" \
  -workers="$workers" \
  -max_total_time="$fuzz_seconds" \
  ${LIBFUZZER_ARGS:-}
