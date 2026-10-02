#!/usr/bin/env bash
# realworld-1 campaign (spec 005): every workload N times per adapter, order reshuffled each cycle.
# Before EVERY run: wait until no other GPU compute app runs and 1-min load < 2 (noise gate).
# Between adapters: 90 s settling (fix for the 2026-09-29 Intel control deviation).
set -u
N_NV=${N_NV:-10}
N_INTEL=${N_INTEL:-5}
HERE="$(cd "$(dirname "$0")" && pwd)"
WGPU="$(cd "$HERE/../../wgpu" && pwd)"
TARGET="$(cd "$HERE/../paired-vulkan-v4" && pwd)/target"
BIN="$TARGET/release/bce-realworld-v1"
LOG="$HERE/../runs-realworld/campaign-$(date -u +%Y%m%dT%H%M%SZ).log"
export BCE_BUILD_WGPU_REV="$(git -C "$WGPU" rev-parse HEAD)"
export BCE_BUILD_NAGA_DIFF_SHA="$(git -C "$WGPU" diff HEAD -- naga/src | sha256sum | cut -d' ' -f1)"
( cd "$HERE" && CARGO_TARGET_DIR="$TARGET" cargo build --release -q ) || { echo "build failed" | tee -a "$LOG"; exit 1; }
mapfile -t WORK < <("$BIN" --list)
quiet() {
  while true; do
    apps=$(nvidia-smi --query-compute-apps=pid --format=csv,noheader 2>/dev/null | wc -l)
    load=$(cut -d' ' -f1 /proc/loadavg)
    if [ "$apps" -eq 0 ] && awk "BEGIN{exit !($load < 2.0)}"; then return; fi
    echo "$(date -u +%FT%TZ) waiting apps=$apps load=$load" >> "$LOG"
    sleep 30
  done
}
campaign() {  # $1 = N, $2 = adapter substring or empty
  echo "campaign N=$1 adapter=${2:-default} workloads=${#WORK[@]} start=$(date -u +%FT%TZ)" | tee -a "$LOG"
  for cycle in $(seq 1 "$1"); do
    mapfile -t ORDER < <(printf '%s\n' "${WORK[@]}" | shuf --random-source=<(yes "$cycle"))
    for w in "${ORDER[@]}"; do
      quiet
      echo "cycle=$cycle workload=$w load=$(cut -d' ' -f1-3 /proc/loadavg)" >> "$LOG"
      if [ -n "${2:-}" ]; then BCE_ADAPTER="$2" "$BIN" "$w" >> "$LOG" 2>&1; else "$BIN" "$w" >> "$LOG" 2>&1; fi || echo "FAILED cycle=$cycle workload=$w exit=$?" | tee -a "$LOG"
    done
  done
  echo "end=$(date -u +%FT%TZ)" | tee -a "$LOG"
}
campaign "$N_NV" ""
sleep 90
campaign "$N_INTEL" "Intel"
echo "campaigns finished $(date -u +%FT%TZ)" | tee -a "$LOG"
