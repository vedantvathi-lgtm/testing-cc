#!/usr/bin/env bash
# Bounded race-location probe (spec 005): 200 interleaved outputs per slot, pixel sets in memory,
# no images saved. Usage: race_probe.sh <binary> <out.jsonl> <adapter> <scene-stem>...
# Moves ONLY directories marked DIAGNOSTIC-ONLY.txt into runs-realworld/_diag-20260930.
set -u
BIN=$1; OUT=$2; ADAPTER=$3; shift 3
HERE="$(cd "$(dirname "$0")" && pwd)"
RUNS="$HERE/../runs-realworld"
export BCE_BUILD_WGPU_REV="$(git -C "$HERE/../../wgpu" rev-parse HEAD)"
export BCE_BUILD_NAGA_DIFF_SHA="$(git -C "$HERE/../../wgpu" diff HEAD -- naga/src | sha256sum | cut -d' ' -f1)"
for stem in "$@"; do
  BCE_ADAPTER="$ADAPTER" BCE_DETERMINISM=200 timeout 3000 "$BIN" "vello:$stem" 2>/dev/null | grep '^race-summary' | sed 's/^race-summary //' >> "$OUT"
  for d in "$RUNS"/2*/; do
    [ -f "$d/DIAGNOSTIC-ONLY.txt" ] && mv "$d" "$RUNS/_diag-20260930/"
  done
  echo "$(date -u +%FT%TZ) $ADAPTER $stem done; disk: $(df -h / | tail -1 | awk '{print $4}') free" >> "$RUNS/_diag-20260930/race-rerun.log"
done
