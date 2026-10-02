#!/usr/bin/env bash
# Campaign: N independent processes per kernel; kernel order reshuffled every cycle so
# slow drift (thermals, background) is not confounded with any kernel.
set -u
N=${N:-10}
HERE="$(cd "$(dirname "$0")" && pwd)"
BIN="${BIN:-$HERE/target/release/bce-paired-vulkan-v4}"
LOG="$HERE/../runs-v4/campaign-$(date -u +%Y%m%dT%H%M%SZ).log"
# Build right before running, embedding the source revision the binary is built from;
# the harness refuses to run if the wgpu source changes after this build.
WGPU="$(cd "$HERE/../../wgpu" && pwd)"
export BCE_BUILD_WGPU_REV="$(git -C "$WGPU" rev-parse HEAD)"
export BCE_BUILD_NAGA_DIFF_SHA="$(git -C "$WGPU" diff HEAD -- naga/src | sha256sum | cut -d' ' -f1)"
TARGET_DIR="$(cd "$(dirname "$BIN")/.." && pwd)"   # .../target or .../target-multi
( cd "$HERE" && CARGO_TARGET_DIR="$TARGET_DIR" cargo build --release -q ) || { echo "build failed"; exit 1; }
mapfile -t KERNELS < <("$BIN" --list)
echo "campaign N=$N kernels=${KERNELS[*]} start=$(date -u +%FT%TZ)" | tee -a "$LOG"
for cycle in $(seq 1 "$N"); do
  mapfile -t ORDER < <(printf '%s\n' "${KERNELS[@]}" | shuf --random-source=<(yes "$cycle"))
  for k in "${ORDER[@]}"; do
    echo "cycle=$cycle kernel=$k load=$(cut -d' ' -f1-3 /proc/loadavg)" >> "$LOG"
    "$BIN" "$k" >> "$LOG" 2>&1 || echo "FAILED cycle=$cycle kernel=$k exit=$?" | tee -a "$LOG"
  done
done
echo "end=$(date -u +%FT%TZ)" | tee -a "$LOG"
