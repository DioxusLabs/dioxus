#!/bin/bash
# Re-runs `cargo workspaces publish` until every crate is published, waiting out crates.io 429 rate limits.
# Already-published crates are skipped by cargo-workspaces on each run. Stops on success or any other error.
# Each run's output is shown as it happens and also saved to "<log prefix><run>.log".
#
# Usage: publish-with-retry.sh <repo dir> [log prefix]
set -u
repo="${1:?usage: publish-with-retry.sh <repo dir> [log prefix]}"
prefix="${2:-/tmp/publish-run}"
cd "$repo" || exit 1

for run in $(seq 1 60); do
  log="${prefix}${run}.log"
  echo "=== run ${run} starting at $(date -u '+%H:%M:%S UTC')"
  cargo workspaces publish --publish-as-is --allow-dirty --no-git-push --no-individual-tags 2>&1 | tee "$log"
  status=${PIPESTATUS[0]}
  published=$(grep -c '^info published' "$log")
  if grep -q '429 Too Many Requests' "$log"; then
    retry=$(grep -o 'try again after [A-Za-z]*, [0-9]* [A-Za-z]* [0-9]* [0-9:]* GMT' "$log" | head -1 | sed 's/try again after //')
    target=$(date -j -u -f '%a, %d %b %Y %H:%M:%S GMT' "$retry" +%s 2>/dev/null \
      || date -u -d "$retry" +%s 2>/dev/null || echo 0)
    wait=$(( target - $(date -u +%s) + 5 ))
    [ "$wait" -lt 10 ] && wait=10
    echo "=== run ${run}: published ${published} this run, rate limited until ${retry}; sleeping ${wait}s"
    sleep "$wait"
    continue
  fi
  if grep -q '^info success' "$log" && ! grep -q '^error: unable to publish\|^error: failed to publish' "$log"; then
    echo "=== run ${run}: published ${published} this run; SUCCESS (exit ${status})"
    exit 0
  fi
  echo "=== run ${run}: published ${published} this run; STOPPED on non-rate-limit error (exit ${status}), see ${log}"
  exit 2
done
echo "=== gave up after 60 runs"
exit 3
