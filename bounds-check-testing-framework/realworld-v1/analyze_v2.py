#!/usr/bin/env python3
"""Verdict rule v2 (spec 005, pre-registration 2, committed 98bd514 before the confirmatory campaign).

Per run i: e_i = ln(median per-round no_bce/bce), c_i = ln(median per-round bce_dup/bce),
d_i = e_i - c_i (control-corrected effect). Across N runs: mean(d), two-sided 95% t-interval.
  GAIN        interval entirely above 0
  LOSS        interval entirely below 0
  EQUIVALENT  interval inside +-ln(1.005)
  INCONCLUSIVE otherwise
Runs whose output oracle failed are excluded and counted; > 1 excluded -> INVALID.
Usage: analyze_v2.py <runs-root> <adapter-substring> <harness-version> [--rev PREFIX]
"""
import argparse
import glob
import json
import math
import os
import statistics as st

T975 = {1: 12.706, 2: 4.303, 3: 3.182, 4: 2.776, 5: 2.571, 6: 2.447, 7: 2.365, 8: 2.306, 9: 2.262, 10: 2.228, 11: 2.201, 12: 2.179}
EQ = math.log(1.005)

p = argparse.ArgumentParser()
p.add_argument("root")
p.add_argument("adapter")
p.add_argument("harness")
p.add_argument("--rev", default="19ffb4e")
a = p.parse_args()

runs = {}
for rp in sorted(glob.glob(os.path.join(a.root, "2*", "result.json"))):
    r = json.load(open(rp))
    m = json.load(open(rp.replace("result.json", "meta.json")))
    if r.get("harness_version") != a.harness or a.adapter.lower() not in m["machine"]["adapter"].lower():
        continue
    if not m["sources"]["wgpu"]["rev"].startswith(a.rev) or r.get("quiet_during_run") is False:
        continue
    runs.setdefault(r["kernel"], []).append(r)

out = {}
print(f"rule v2 | harness {a.harness} | adapter {a.adapter} | wgpu {a.rev}\n")
print("| workload | runs used (excluded) | control-corrected effect exp(mean d) | 95% CI | mean control c | verdict v2 |")
print("|---|---|---|---|---|---|")
for k in sorted(runs, key=lambda n: (not n.startswith("blur"), n)):
    rs = runs[k]
    used = [r for r in rs if r["outputs_ok"]]
    excluded = len(rs) - len(used)
    d = [math.log(r["ratio_no_bce_over_bce"]["median"]) - math.log(r["ratio_control_bce_dup_over_bce"]["median"]) for r in used]
    c = [math.log(r["ratio_control_bce_dup_over_bce"]["median"]) for r in used]
    if not rs[0]["bce_changes_binary"]:
        verdict, lo, hi, mean = "NO_PROOF", float("nan"), float("nan"), float("nan")
    elif excluded > 1 or len(d) < 2:
        verdict, lo, hi, mean = "INVALID", float("nan"), float("nan"), float("nan")
    else:
        mean = st.mean(d)
        half = T975[len(d) - 1] * st.stdev(d) / math.sqrt(len(d))
        lo, hi = mean - half, mean + half
        verdict = "GAIN" if lo > 0 else "LOSS" if hi < 0 else "EQUIVALENT" if (lo > -EQ and hi < EQ) else "INCONCLUSIVE"
    out[k] = {"runs": len(rs), "excluded": excluded, "mean_d": mean, "ci_lo": lo, "ci_hi": hi,
              "effect": math.exp(mean) if mean == mean else None, "mean_control": st.mean(c) if c else None, "verdict": verdict}
    e = lambda x: f"{math.exp(x):.4f}" if x == x else "–"
    pct = lambda x: f"{100*(math.exp(x)-1):+.2f}%" if x == x else ""
    print(f"| {k} | {len(d)} ({excluded}) | **{e(mean)}** ({pct(mean)}) | {e(lo)}–{e(hi)} | {e(st.mean(c)) if c else '–'} | {verdict} |")

vello = {k: v for k, v in out.items() if k.startswith("vello:") and v["effect"]}
if vello:
    g = math.exp(st.mean(math.log(v["effect"]) for v in vello.values()))
    counts = {s: sum(v["verdict"] == s for v in out.values() if k) for s in ()}
    tally = {}
    for k, v in out.items():
        if k.startswith("vello:"):
            tally[v["verdict"]] = tally.get(v["verdict"], 0) + 1
    out["_vello_suite"] = {"scenes_with_effect": len(vello), "effect_geomean": g, "verdicts": tally}
    print(f"\nvello suite: control-corrected effect geomean over {len(vello)} scenes {g:.4f} ({100*(g-1):+.2f}%); verdicts {tally}")
tag = f"{a.adapter.lower()}-{a.harness}-{a.rev}"
path = os.path.join(a.root, f"summary-v2-{tag}.json")
json.dump(out, open(path, "w"), indent=2)
print(f"\nwritten {path}")
