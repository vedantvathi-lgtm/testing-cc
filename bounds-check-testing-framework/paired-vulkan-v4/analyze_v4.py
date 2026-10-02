#!/usr/bin/env python3
"""Aggregate paired-vulkan-4 runs into a per-kernel report (spec 004, Q7/Q8).

Per run: median per-round ratios no_bce/bce (effect), bce_dup/bce (in-round control),
bce/unchecked (residual checks BCE could not remove), no_bce/unchecked (cost of all checks).
Across runs: geometric mean, min-max, and a verdict:
  RESOLVED        all runs valid, all control CIs contain 1, all per-run effect medians on the
                  same side of 1 (min-max excludes 1), and |effect-1| > 2 * max|control-1|
  NOT_RESOLVABLE  otherwise, with the failing condition named
  NO_PROOF        elision does not change the binary (effect must be ~1; acts as a control)
Only files under runs-v4/<stamp>-<kernel>/ are read; the smoke directory is ignored.

Equivalence margin (declared 2026-09-28 after the first `empty` run showed a 0.13% bias between
identical pipelines with a CI excluding 1, BEFORE any campaign kernel result was inspected):
a run's control is acceptable if |bce_dup/bce - 1| < 0.5%; an effect is RESOLVED only if it
exceeds max(2 * worst control deviation, 0.5%). Strict CI-contains-1 counts are reported too.
"""
import csv
import glob
import json
import math
import os
import statistics as st
import sys
from datetime import datetime, timezone

import argparse

parser = argparse.ArgumentParser()
parser.add_argument("root", nargs="?", default=os.path.join(os.path.dirname(__file__), "..", "runs-v4"))
parser.add_argument("adapter", nargs="?", default=None, help="adapter name substring (default: NVIDIA)")
parser.add_argument("--rev", default=None, help="only runs whose wgpu revision starts with this prefix")
args = parser.parse_args()
root = args.root


def gmean(xs):
    return math.exp(sum(math.log(x) for x in xs) / len(xs))


def telemetry_in_window(run_dir, window):
    path = os.path.join(run_dir, "artifacts", "telemetry.csv")
    lo, hi = window
    clocks, pstates, temps = [], set(), []
    try:
        for row in csv.reader(open(path)):
            if len(row) < 6:
                continue
            ts = datetime.strptime(row[0].strip(), "%Y/%m/%d %H:%M:%S.%f").replace(tzinfo=None)
            ms = (ts - datetime(1970, 1, 1)).total_seconds() * 1000 - local_offset_ms()
            if lo <= ms <= hi:
                pstates.add(row[1].strip())
                clocks.append(int(row[2].split()[0]))
                temps.append(int(row[4]))
    except (OSError, ValueError):
        pass
    return clocks, pstates, temps


def local_offset_ms():
    # nvidia-smi timestamps are local time
    now = datetime.now()
    return (now - datetime.now(timezone.utc).replace(tzinfo=None)).total_seconds() * 1000


adapter_filter = args.adapter
runs = {}
adapters = set()
for path in sorted(glob.glob(os.path.join(root, "2*", "result.json"))):
    r = json.load(open(path))
    meta = json.load(open(os.path.join(os.path.dirname(path), "meta.json")))
    adapter = meta["machine"]["adapter"]
    if args.rev and not meta["sources"]["wgpu"]["rev"].startswith(args.rev):
        continue
    adapters.add(adapter)
    if adapter_filter and adapter_filter.lower() not in adapter.lower():
        continue
    if not adapter_filter and "nvidia" not in adapter.lower():
        continue  # default report: NVIDIA only; pass an adapter substring for others
    if r.get("quiet_during_run") is False:
        continue  # another GPU compute process appeared during the measurement
    runs.setdefault(r["kernel"], []).append((os.path.dirname(path), r))
print(f"adapters seen: {sorted(adapters)}; reporting: {adapter_filter or 'NVIDIA'}; wgpu rev: {args.rev or 'any'}\n")

order = ["empty", "scan_shared", "matmul_tiled", "reduce_tree", "prefix_scan_hs", "vello_binning", "private_array"]
summary = {}
print("| kernel | runs | BCE changes binary | proofs (BCE) | clamps no_bce → bce | no_bce / bce (geomean) | per-run min–max | control bce_dup/bce max dev (strict CI∋1) | bce / unchecked (residual) | no_bce / unchecked (all checks) | verdict |")
print("|---|---|---|---|---|---|---|---|---|---|---|")
for k in order + sorted(set(runs) - set(order)):
    if k not in runs:
        continue
    rs = runs[k]
    valid = [r for _, r in rs if r["valid"]]
    eff = [r["ratio_no_bce_over_bce"]["median"] for _, r in rs]
    ctl = [r["ratio_control_bce_dup_over_bce"] for _, r in rs]
    res = [r["ratio_residual_bce_over_unchecked"]["median"] for _, r in rs]
    allc = [r["ratio_all_checks_no_bce_over_unchecked"]["median"] for _, r in rs]
    ctl_strict = sum(c["ci95_lo"] <= 1 <= c["ci95_hi"] for c in ctl)
    ctl_ok = all(abs(c["median"] - 1) < 0.005 for c in ctl)
    correct = all(all(c["before_mismatches"] == 0 and c["after_mismatches"] == 0 for c in r["correctness"]) for _, r in rs)
    ctl_dev = max(abs(c["median"] - 1) for c in ctl)
    s0 = rs[0][1]["statics"]
    changes = rs[0][1]["bce_changes_binary"]
    g = gmean(eff)
    if not changes:
        verdict = "NO_PROOF (control)"
    elif not correct:
        verdict = "INVALID: output mismatch"
    elif not ctl_ok:
        verdict = "NOT_RESOLVABLE: control deviation >= 0.5% in some run"
    elif not (min(eff) > 1 or max(eff) < 1):
        verdict = "NOT_RESOLVABLE: runs disagree in sign"
    elif abs(g - 1) <= max(2 * ctl_dev, 0.005):
        verdict = "NOT_RESOLVABLE: effect <= max(2x control deviation, 0.5%)"
    else:
        verdict = "RESOLVED"
    clamps = f"{s0[0]['umin_clamps']} → {s0[1]['umin_clamps']}"
    proofs = f"{s0[1]['proven_accesses']}/{s0[1]['dynamic_accesses']}"
    print(f"| {k} | {len(rs)} | {'yes' if changes else 'no'} | {proofs} | {clamps} | **{g:.4f}** ({100*(g-1):+.2f}%) | {min(eff):.4f}–{max(eff):.4f} | {100*ctl_dev:.3f}% ({ctl_strict}/{len(rs)}) | {gmean(res):.4f} ({100*(gmean(res)-1):+.2f}%) | {gmean(allc):.4f} ({100*(gmean(allc)-1):+.2f}%) | {verdict} |")
    tele = [telemetry_in_window(d, r["measure_window_unix_ms"]) for d, r in rs]
    summary[k] = {
        "runs": len(rs), "valid_runs": len(valid), "bce_changes_binary": changes,
        "effect_geomean": g, "effect_min": min(eff), "effect_max": max(eff),
        "effect_per_run": eff, "control_max_dev": ctl_dev, "control_within_margin": ctl_ok, "control_strict_ci_contains_1": ctl_strict, "outputs_correct": correct,
        "residual_geomean": gmean(res), "all_checks_geomean": gmean(allc),
        "bce_ns_median_of_runs": st.median(r["median_ns_per_dispatch"]["bce"] for _, r in rs),
        "order_effect_ratio": [(r["order_effect_ratio"]["no_bce_first_median"], r["order_effect_ratio"]["bce_first_median"]) for _, r in rs],
        "warmup_rounds": [r["warmup_rounds_done"] for _, r in rs],
        "telemetry_clock_min_max_in_window": [(min(c), max(c)) if c else None for c, _, _ in tele],
        "telemetry_pstates_in_window": [sorted(p) for _, p, _ in tele],
        "telemetry_temp_max": [max(t) if t else None for _, _, t in tele],
        "statics": s0, "verdict": verdict,
    }
tag = (adapter_filter or "nvidia").lower() + (f"-{args.rev}" if args.rev else "")
out = os.path.join(root, f"summary-{tag}.json")
json.dump(summary, open(out, "w"), indent=2)
print(f"\nwritten {out}")
