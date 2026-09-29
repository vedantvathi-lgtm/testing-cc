#!/usr/bin/env bash
# Campaign: N independent processes per kernel; kernel order reshuffled every cycle so
# slow drift (thermals, background) is not confounded with any kernel.
set -u
N=${N:-10}
BIN="${BIN:-$(dirname "$0")/target/release/bce-paired-vulkan-v4}"
LOG="$(dirname "$0")/../runs-v4/campaign-$(date -u +%Y%m%dT%H%M%SZ).log"
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
