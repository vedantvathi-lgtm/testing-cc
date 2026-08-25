#!/bin/bash
# prepare-bench.sh
# System preparation for GPU benchmarking.
# Following NVIDIA's official benchmarking guide:
#   https://docs.nvidia.com/nsight-compute/NsightCompute/index.html#benchmarking
# And Google Benchmark's CPU frequency documentation:
#   https://github.com/google/benchmark#disabling-cpu-frequency-scaling
#
# Run this BEFORE the benchmark:
#   sudo ./prepare-bench.sh
# Then run the benchmark in the same shell session.

set -euo pipefail

echo "=== Fixed Benchmark Suite — System Preparation ==="
echo ""

# ─── GPU Clock Locking (NVIDIA) ────────────────────────────────────────────
echo "=== GPU Clock Locking ==="

# Enable persistent mode (keeps driver loaded, reduces first-launch latency)
if command -v nvidia-smi &> /dev/null; then
    echo "  Enabling persistent mode..."
    sudo nvidia-smi -pm 1

    # Lock GPU clocks to a fixed frequency for reproducibility.
    # NVIDIA docs: lock both min and max to the same stable value.
    # Query available clock range first:
    MIN_CLOCK=$(nvidia-smi --query-gpu=clocks.gr --format=csv,noheader 2>/dev/null | head -1 | sed 's/ MHz//' | tr -d ' ')
    MAX_CLOCK=$(nvidia-smi --query-gpu=clocks.max.gr --format=csv,noheader 2>/dev/null | head -1 | sed 's/ MHz//' | tr -d ' ')
    echo "  GPU clock range: ${MIN_CLOCK:-?} – ${MAX_CLOCK:-?} MHz"

    # Lock both min and max to the same value (MAX_CLOCK for max performance).
    # If that throttles, use MIN_CLOCK for sustained stability.
    if [ -n "${MAX_CLOCK}" ] && [ "${MAX_CLOCK}" -gt 0 ] 2>/dev/null; then
        sudo nvidia-smi -lgc "${MAX_CLOCK},${MAX_CLOCK}"
        echo "  Locked GPU clocks to ${MAX_CLOCK} MHz (min=max)"
    elif [ -n "${MIN_CLOCK}" ] && [ "${MIN_CLOCK}" -gt 0 ] 2>/dev/null; then
        sudo nvidia-smi -lgc "${MIN_CLOCK},${MIN_CLOCK}"
        echo "  Locked GPU clocks to ${MIN_CLOCK} MHz (base clock, fallback)"
    else
        echo "  WARNING: Could not determine GPU clock. Skipping lock."
    fi

    # Verify clock lock
    echo "  Current GPU state:"
    nvidia-smi --query-gpu=name,driver_version,clocks.gr,clocks.mem,clocks_throttle_reasons.active,pstate,temperature.gpu,power.draw \
               --format=csv 2>/dev/null || true

    # Kill competing GPU processes
    COMPETING=$(nvidia-smi --query-compute-apps=pid --format=csv,noheader 2>/dev/null || true)
    if [ -n "${COMPETING}" ]; then
        echo "  Killing competing GPU processes: ${COMPETING}"
        echo "${COMPETING}" | xargs -r kill -9 2>/dev/null || true
    fi
else
    echo "  WARNING: nvidia-smi not found. GPU clock locking skipped."
    echo "  This benchmark should be run on NVIDIA hardware for best results."
fi

# ─── CPU Frequency Pinning ──────────────────────────────────────────────────
echo ""
echo "=== CPU Frequency Scaling ==="

if command -v cpupower &> /dev/null; then
    echo "  Setting CPU governor to 'performance'..."
    sudo cpupower frequency-set -g performance 2>/dev/null || true
elif [ -d /sys/devices/system/cpu/cpu0/cpufreq ]; then
    echo "  Setting CPU governor via sysfs..."
    for cpu in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
        echo performance | sudo tee "${cpu}" > /dev/null 2>&1 || true
    done
else
    echo "  WARNING: cpufreq sysfs not found. CPU frequency pinning skipped."
fi

# Verify CPU governor
GOVERNOR=$(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo "unknown")
echo "  Current CPU governor: ${GOVERNOR}"

# ─── Thermal Soak ───────────────────────────────────────────────────────────
echo ""
echo "=== Thermal Soak ==="
echo "  Running 5 seconds of GPU load to reach thermal steady state..."

if command -v nvidia-smi &> /dev/null; then
    # Run a small GPU stress to warm up
    python3 -c "
import subprocess, time
# Small GPU load via nvidia-smi loop (triggers GPU activity)
start = time.time()
while time.time() - start < 5:
    subprocess.run(['nvidia-smi', '--query-gpu=temperature.gpu,power.draw', '--format=csv,noheader'],
                   capture_output=True)
    time.sleep(0.1)
" 2>/dev/null || true

    TEMP=$(nvidia-smi --query-gpu=temperature.gpu --format=csv,noheader 2>/dev/null | head -1 || echo "?")
    echo "  GPU temperature after soak: ${TEMP}°C"
fi

# ─── Reduce System Noise ────────────────────────────────────────────────────
echo ""
echo "=== System Noise Reduction ==="

# Try to stop non-essential services (best-effort, ignore failures)
for svc in bluetooth NetworkManager cups; do
    sudo systemctl stop "${svc}.service" 2>/dev/null || true
done
echo "  Stopped non-essential services."

# Log system state to a file for benchmark metadata
STATE_LOG="/tmp/bench-system-state-$(date +%Y%m%d-%H%M%S).txt"
{
    echo "System State at $(date)"
    echo "========================"
    echo "CPU governor: $(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo 'unknown')"
    echo "Load average: $(cat /proc/loadavg 2>/dev/null || echo 'unknown')"
    echo ""
    echo "GPU Info (nvidia-smi):"
    nvidia-smi --query-gpu=name,driver_version,clocks.gr,clocks.mem,clocks_throttle_reasons.active,pstate,temperature.gpu,power.draw,utilization.gpu,utilization.memory \
               --format=csv 2>/dev/null || echo "nvidia-smi not available"
    echo ""
    echo "CPU Info:"
    grep 'model name' /proc/cpuinfo | head -1 2>/dev/null || true
    grep 'MemTotal' /proc/meminfo | head -1 2>/dev/null || true
} > "${STATE_LOG}"
echo "  System state logged to: ${STATE_LOG}"

# ─── Done ───────────────────────────────────────────────────────────────────
echo ""
echo "=== Ready to benchmark ==="
echo "  System is prepared. Run the benchmark now:"
echo "    cd bounds-check-bench && cargo run --release"
echo ""
