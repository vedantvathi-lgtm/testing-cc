#!/usr/bin/env python3
"""
generate_comparison.py — Cross-backend comparison chart for DAVE sonar.

Loads:
  1. wgpu criterion JSON  (from target/criterion/ or criterion JSON export)
  2. SYCL benchmark JSON  (from sonar_sycl/bench/ stdout)
  3. CUDA timing CSV       (optional, from CUDA profiling)

Produces a multi-panel comparison chart as PNG + terminal table.

Usage:
    python generate_comparison.py [--wgpu-json PATH] [--sycl-json PATH]
                                  [--cuda-csv PATH] [--output PATH]
                                  [--terminal-only]
"""

import argparse
import csv
import json
import os
import sys
import glob
from pathlib import Path
from typing import Dict, List, Optional, Tuple

# ═══════════════════════════════════════════════════════════════════════
# Data loaders
# ═══════════════════════════════════════════════════════════════════════

def load_wgpu_criterion(path: str) -> Dict[str, dict]:
    """Load wgpu criterion benchmark results.

    Supports both:
    - Criterion's default target/criterion/ directory structure
    - Exported JSON files (from criterion-json or manual export)

    Returns: {config_label: {median_us, q1_us, q3_us, ...}}
    """
    results = {}

    criterion_path = Path(path)
    if not criterion_path.exists():
        # Try the standard criterion directory
        criterion_path = Path("sonar_wgpu/target/criterion")
        if not criterion_path.exists():
            criterion_path = Path("target/criterion")

    if criterion_path.is_dir():
        # Standard criterion directory structure:
        #   target/criterion/sonar_frame/total_frame/<config>/new/estimates.json
        bench_dir = criterion_path / "sonar_frame" / "total_frame"
        if bench_dir.exists():
            for config_dir in sorted(bench_dir.iterdir()):
                if not config_dir.is_dir():
                    continue
                label = config_dir.name
                estimates = config_dir / "new" / "estimates.json"
                if not estimates.exists():
                    continue
                with open(estimates) as f:
                    data = json.load(f)
                mean = data.get("mean", {})
                median = data.get("median", {})
                std_dev = data.get("std_dev", {})
                results[label] = {
                    "median_us": median.get("point_estimate", 0) / 1e3,
                    "q1_us": median.get("confidence_interval", {}).get("lower_bound", median.get("point_estimate", 0) * 0.98) / 1e3,
                    "q3_us": median.get("confidence_interval", {}).get("upper_bound", median.get("point_estimate", 0) * 1.02) / 1e3,
                    "mean_us": mean.get("point_estimate", 0) / 1e3,
                    "std_dev_us": std_dev.get("point_estimate", 0) / 1e3,
                }

    elif criterion_path.is_file():
        # Direct JSON file
        with open(criterion_path) as f:
            data = json.load(f)

        # Handle criterion JSON export format
        if "benchmarks" in data:
            for bench in data["benchmarks"]:
                label = bench.get("full_id", bench.get("id", "unknown"))
                # Extract just the config label (e.g., "8b_16r_16f")
                parts = label.split("/")
                config = parts[-1] if parts else label
                results[config] = {
                    "median_us": bench.get("median_us", 0),
                    "q1_us": bench.get("q1_us", bench.get("median_us", 0) * 0.98),
                    "q3_us": bench.get("q3_us", bench.get("median_us", 0) * 1.02),
                    "mean_us": bench.get("mean_us", bench.get("median_us", 0)),
                    "std_dev_us": bench.get("std_dev_us", 0),
                }
        elif "sonar_frame" in data:
            # Nested format
            for config, vals in data["sonar_frame"].items():
                results[config] = {
                    "median_us": vals.get("median_us", 0),
                    "q1_us": vals.get("q1_us", vals.get("median_us", 0) * 0.98),
                    "q3_us": vals.get("q3_us", vals.get("median_us", 0) * 1.02),
                    "mean_us": vals.get("mean_us", vals.get("median_us", 0)),
                    "std_dev_us": vals.get("std_dev_us", 0),
                }

    return results


def load_sycl_json(path: str) -> Dict[str, dict]:
    """Load SYCL benchmark JSON (from bench_sycl stdout).

    Returns: {config_label: {median_us, q1_us, q3_us, ...}}
    """
    results = {}
    with open(path) as f:
        data = json.load(f)

    for bench in data.get("benchmarks", []):
        label = bench.get("id", "unknown")
        results[label] = {
            "median_us": bench.get("median_us", 0),
            "q1_us": bench.get("q1_us", bench.get("median_us", 0) * 0.98),
            "q3_us": bench.get("q3_us", bench.get("median_us", 0) * 1.02),
            "mean_us": bench.get("mean_us", bench.get("median_us", 0)),
            "std_dev_us": bench.get("std_dev_us", 0),
        }

    return results


def load_cuda_csv(path: str) -> Dict[str, dict]:
    """Load CUDA timing CSV.

    Expected columns: config, median_us, q1_us, q3_us [, upload_us, scatter_us, ...]

    Returns: {config_label: {median_us, q1_us, q3_us, ...}}
    """
    results = {}
    with open(path) as f:
        reader = csv.DictReader(f)
        for row in reader:
            label = row.get("config", row.get("label", "unknown"))
            median = float(row.get("median_us", row.get("total_us", 0)))
            q1_val = row.get("q1_us", "")
            q3_val = row.get("q3_us", "")
            results[label] = {
                "median_us": median,
                "q1_us": float(q1_val) if q1_val else median * 0.95,
                "q3_us": float(q3_val) if q3_val else median * 1.05,
                "mean_us": median,
                "std_dev_us": 0.0,
            }

    return results


# ═══════════════════════════════════════════════════════════════════════
# Terminal table renderer
# ═══════════════════════════════════════════════════════════════════════

BACKEND_COLORS = {
    "cuda":  "\033[91m",  # Red
    "wgpu":  "\033[94m",  # Blue
    "sycl":  "\033[92m",  # Green
    "reset": "\033[0m",
}

def print_terminal_table(
    all_data: Dict[str, Dict[str, dict]],
    configs: List[str],
):
    """Print a formatted comparison table to the terminal."""
    backends = sorted(all_data.keys())
    c = BACKEND_COLORS

    # Header
    print()
    print(f"  {'Config':<16}", end="")
    for b in backends:
        color = c.get(b, "")
        print(f"  {color}{b.upper():>14}{c['reset']}", end="")
    print()
    print("  " + "─" * (16 + 16 * len(backends)))

    # Find best per config
    for config in configs:
        present = {b: all_data[b].get(config) for b in backends if config in all_data[b]}
        if not present:
            continue

        best_time = min(v["median_us"] for v in present.values())
        print(f"  {config:<16}", end="")
        for b in backends:
            if config in all_data[b]:
                d = all_data[b][config]
                t = d["median_us"]
                color = c.get(b, "")
                marker = " ★" if abs(t - best_time) < 0.01 * best_time else ""
                print(f"  {color}{t:>11.1f} µs{c['reset']}{marker}", end="")
            else:
                print(f"  {'—':>14}", end="")
        print()

    # Speedup summary
    print()
    print("  Speedup vs wgpu (lower is better):")
    if "wgpu" not in all_data:
        print("  (no wgpu data — cannot compute speedup)")
        return

    for b in backends:
        if b == "wgpu":
            continue
        color = c.get(b, "")
        print(f"  {color}{b.upper()}{c['reset']}", end="")
        for config in configs:
            if config in all_data.get(b, {}) and config in all_data["wgpu"]:
                sycl_t = all_data[b][config]["median_us"]
                wgpu_t = all_data["wgpu"][config]["median_us"]
                if wgpu_t > 0:
                    speedup = wgpu_t / sycl_t
                    print(f"  {speedup:.2f}×", end="")
                else:
                    print(f"  {'—':>6}", end="")
            else:
                print(f"  {'—':>6}", end="")
        print()
    print()


# ═══════════════════════════════════════════════════════════════════════
# Chart renderer (matplotlib)
# ═══════════════════════════════════════════════════════════════════════

def generate_chart(
    all_data: Dict[str, Dict[str, dict]],
    configs: List[str],
    output_path: str,
):
    """Generate a grouped bar chart comparing backends."""
    try:
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
        import numpy as np
    except ImportError:
        print("  [!] matplotlib not installed — skipping chart generation.", file=sys.stderr)
        print("      Install with: pip install matplotlib", file=sys.stderr)
        return

    backends = sorted(all_data.keys())
    n_configs = len(configs)
    n_backends = len(backends)

    colors = {
        "cuda": "#e74c3c",
        "wgpu": "#3498db",
        "sycl": "#2ecc71",
    }

    fig, axes = plt.subplots(1, 2, figsize=(14, 6), gridspec_kw={"width_ratios": [3, 2]})

    # ── Panel 1: Grouped bar chart (total frame time) ──
    ax1 = axes[0]
    x = np.arange(n_configs)
    width = 0.7 / max(n_backends, 1)

    for bi, backend in enumerate(backends):
        means = []
        errors = []
        for config in configs:
            d = all_data.get(backend, {}).get(config)
            if d:
                means.append(d["median_us"])
                errors.append((d["median_us"] - d["q1_us"], d["q3_us"] - d["median_us"]))
            else:
                means.append(0)
                errors.append((0, 0))

        offset = (bi - n_backends / 2 + 0.5) * width
        bars = ax1.bar(x + offset, means, width, label=backend.upper(),
                       color=colors.get(backend, "#95a5a6"), edgecolor="white", linewidth=0.5)

        # Error bars
        if any(e[0] > 0 or e[1] > 0 for e in errors):
            err_low = [e[0] for e in errors]
            err_high = [e[1] for e in errors]
            ax1.errorbar(x + offset, means, yerr=[err_low, err_high],
                        fmt="none", ecolor="#333", capsize=3, linewidth=1)

        # Value labels on bars
        for bi2, (bar, mean) in enumerate(zip(bars, means)):
            if mean > 0:
                ax1.text(bar.get_x() + bar.get_width() / 2, bar.get_height() + 0.02 * max(means),
                        f"{mean:.1f}", ha="center", va="bottom", fontsize=7, fontweight="bold")

    ax1.set_xlabel("Workload Configuration", fontsize=10)
    ax1.set_ylabel("Median Frame Time (µs)", fontsize=10)
    ax1.set_title("Sonar Pipeline — Total Frame Time", fontsize=12, fontweight="bold")
    ax1.set_xticks(x)
    ax1.set_xticklabels(configs, fontsize=8)
    ax1.legend(fontsize=9)
    ax1.grid(axis="y", alpha=0.3)
    ax1.set_yscale("log")

    # ── Panel 2: Speedup heatmap ──
    ax2 = axes[1]
    if "wgpu" in all_data:
        speedup_data = []
        labels_y = []
        for backend in backends:
            if backend == "wgpu":
                continue
            row = []
            for config in configs:
                s = all_data.get(backend, {}).get(config, {}).get("median_us", 0)
                w = all_data.get("wgpu", {}).get(config, {}).get("median_us", 0)
                if s > 0 and w > 0:
                    row.append(w / s)
                else:
                    row.append(0)
            speedup_data.append(row)
            labels_y.append(backend.upper())

        if speedup_data:
            speedup_arr = np.array(speedup_data)
            im = ax2.imshow(speedup_arr, cmap="RdYlGn", aspect="auto", vmin=0.5, vmax=3.0)
            ax2.set_xticks(range(len(configs)))
            ax2.set_xticklabels(configs, fontsize=8, rotation=30, ha="right")
            ax2.set_yticks(range(len(labels_y)))
            ax2.set_yticklabels(labels_y, fontsize=9)
            ax2.set_title("Speedup vs wgpu", fontsize=12, fontweight="bold")

            for i in range(len(labels_y)):
                for j in range(len(configs)):
                    val = speedup_arr[i, j]
                    if val > 0:
                        color = "white" if val < 0.8 or val > 2.0 else "black"
                        ax2.text(j, i, f"{val:.2f}×", ha="center", va="center",
                                fontsize=10, fontweight="bold", color=color)

            fig.colorbar(im, ax=ax2, shrink=0.8, label="Speedup (×)")
        else:
            ax2.text(0.5, 0.5, "No comparison data", ha="center", va="center",
                    transform=ax2.transAxes, fontsize=12, color="#999")
    else:
        ax2.text(0.5, 0.5, "No wgpu baseline data", ha="center", va="center",
                transform=ax2.transAxes, fontsize=12, color="#999")

    plt.tight_layout()
    plt.savefig(output_path, dpi=150, bbox_inches="tight")
    plt.close()
    print(f"  Chart saved to: {output_path}")


# ═══════════════════════════════════════════════════════════════════════
# Auto-discover benchmark files
# ═══════════════════════════════════════════════════════════════════════

def auto_discover_wgpu_json() -> Optional[str]:
    """Try to find criterion output in standard locations."""
    candidates = [
        "sonar_wgpu/target/criterion",
        "target/criterion",
        "wgpu_test_run/wgpu_bench_results.json",
    ]
    for c in candidates:
        if Path(c).exists():
            return c
    return None


def auto_discover_sycl_json() -> Optional[str]:
    """Try to find SYCL benchmark JSON."""
    candidates = [
        "dave/sonar_sycl/bench_results.json",
        "dave/sonar_sycl/bench/bench_results.json",
    ]
    for c in candidates:
        if Path(c).exists():
            return c
    return None


def auto_discover_cuda_csv() -> Optional[str]:
    """Try to find CUDA timing CSV."""
    candidates = [
        "cuda_bench_results.csv",
        "dave/cuda_bench_results.csv",
    ]
    for c in candidates:
        if Path(c).exists():
            return c
    return None


# ═══════════════════════════════════════════════════════════════════════
# Main
# ═══════════════════════════════════════════════════════════════════════

def main():
    parser = argparse.ArgumentParser(
        description="Generate cross-backend sonar benchmark comparison chart")
    parser.add_argument("--wgpu-json", help="Path to wgpu criterion JSON or directory")
    parser.add_argument("--sycl-json", help="Path to SYCL benchmark JSON")
    parser.add_argument("--cuda-csv", help="Path to CUDA timing CSV")
    parser.add_argument("--output", default="benchmark_comparison.png",
                       help="Output chart path (default: benchmark_comparison.png)")
    parser.add_argument("--terminal-only", action="store_true",
                       help="Only print terminal table, skip chart")
    args = parser.parse_args()

    # Auto-discover if not specified
    wgpu_path = args.wgpu_json or auto_discover_wgpu_json()
    sycl_path = args.sycl_json or auto_discover_sycl_json()
    cuda_path = args.cuda_csv or auto_discover_cuda_csv()

    print("═══════════════════════════════════════════════════════════════")
    print("  DAVE Sonar — Cross-Backend Benchmark Comparison")
    print("═══════════════════════════════════════════════════════════════")
    print()

    all_data: Dict[str, Dict[str, dict]] = {}

    if wgpu_path:
        print(f"  Loading wgpu data from: {wgpu_path}")
        wgpu_data = load_wgpu_criterion(wgpu_path)
        if wgpu_data:
            all_data["wgpu"] = wgpu_data
            print(f"    → {len(wgpu_data)} configs loaded")
        else:
            print(f"    → No data found (directory may be empty)")
    else:
        print("  No wgpu criterion data found (run `cargo bench` in sonar_wgpu/ first)")

    if sycl_path:
        print(f"  Loading SYCL data from: {sycl_path}")
        sycl_data = load_sycl_json(sycl_path)
        if sycl_data:
            all_data["sycl"] = sycl_data
            print(f"    → {len(sycl_data)} configs loaded")
    else:
        print("  No SYCL benchmark data found")
        print("    → Build and run: cd dave/sonar_sycl/bench && ./sonar_sycl_bench > bench_results.json")

    if cuda_path:
        print(f"  Loading CUDA data from: {cuda_path}")
        cuda_data = load_cuda_csv(cuda_path)
        if cuda_data:
            all_data["cuda"] = cuda_data
            print(f"    → {len(cuda_data)} configs loaded")
    else:
        print("  No CUDA timing data found")

    print()

    if not all_data:
        print("  No benchmark data available. Run benchmarks first:")
        print("    wgpu:  cd sonar_wgpu && cargo bench")
        print("    SYCL:  cd dave/sonar_sycl/bench && ./sonar_sycl_bench > bench_results.json")
        sys.exit(1)

    # Collect all configs across all backends
    all_configs = set()
    for backend_data in all_data.values():
        all_configs.update(backend_data.keys())

    # Sort configs by problem size (small → large)
    def config_sort_key(c: str) -> int:
        try:
            parts = c.replace("b_", " ").replace("r_", " ").replace("f", "").split()
            return int(parts[0]) * int(parts[1]) * int(parts[2])
        except (IndexError, ValueError):
            return 0

    configs = sorted(all_configs, key=config_sort_key)

    # Terminal table
    print_terminal_table(all_data, configs)

    # Chart
    if not args.terminal_only:
        generate_chart(all_data, configs, args.output)

    # Save combined data as JSON for reproducibility
    combined_path = Path(args.output).with_suffix(".json")
    with open(combined_path, "w") as f:
        json.dump({
            "backends": all_data,
            "configs": configs,
        }, f, indent=2)
    print(f"  Data saved to: {combined_path}")


if __name__ == "__main__":
    main()
