#!/usr/bin/env bash
# Start the v4 campaigns only when the machine is quiet (benchmarking notes §11):
# no other GPU compute apps AND 1-minute load average < 2, for 3 consecutive checks 60 s apart.
set -u
cd "$(dirname "$0")/.."
quiet=0
while [ "$quiet" -lt 3 ]; do
  apps=$(nvidia-smi --query-compute-apps=pid --format=csv,noheader)
  load=$(cut -d' ' -f1 /proc/loadavg)
  if [ -z "$apps" ] && awk "BEGIN{exit !($load < 2.0)}"; then quiet=$((quiet+1)); else quiet=0; fi
  echo "$(date -u +%FT%TZ) apps=[${apps//$'\n'/,}] load=$load quiet=$quiet"
  [ "$quiet" -lt 3 ] && sleep 60
done
N=10 ./paired-vulkan-v4/run_campaign.sh
BCE_ADAPTER=intel BIN="$PWD/paired-vulkan-v4/target-multi/release/bce-paired-vulkan-v4" N=5 ./paired-vulkan-v4/run_campaign.sh
echo "campaigns finished $(date -u +%FT%TZ)"
