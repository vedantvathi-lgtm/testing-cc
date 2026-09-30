#!/usr/bin/env python3
"""Aggregate realworld-1 runs (spec 005) per workload and adapter, with the v4 verdict rules.

Per run: median of per-round ratios no_bce/bce (effect), bce_dup/bce (in-round control),
bce/unchecked (residual), no_bce/unchecked (all checks). Across runs: geometric mean, min-max.
Verdicts (identical to analyze_v4.py, margin declared 2026-09-28 before any v4 kernel result):
  NO_PROOF        elision does not change the binary
  INVALID         some run failed the output oracle
  NOT_RESOLVABLE  control deviation >= 0.5% in a run / runs disagree in sign /
                  effect <= max(2 x worst control deviation, 0.5%)
  RESOLVED        otherwise
Runs are read from runs-realworld/<stamp>-<workload>[-adapter]/; directories starting with "_"
(smoke, sizing) are ignored. Suite line: geometric mean over vello scenes of per-scene geomeans
(each scene weighted equally), and counts of RESOLVED gains / losses.
UMin counts include shader-authored min() calls: the `unchecked` slot's count is that floor.
"""
import argparse
import glob
import json
import math
import os

parser = argparse.ArgumentParser()
parser.add_argument("root", nargs="?", default=os.path.join(os.path.dirname(__file__), "..", "runs-realworld"))
parser.add_argument("adapter", nargs="?", default="nvidia", help="adapter name substring")
parser.add_argument("--rev", default=None, help="only runs whose wgpu revision starts with this prefix")
args = parser.parse_args()


def gmean(xs):
    return math.exp(sum(math.log(x) for x in xs) / len(xs))


runs = {}
skipped = {"not_quiet": 0, "other_adapter": 0, "other_rev": 0, "blocked": 0}
for d in sorted(glob.glob(os.path.join(args.root, "2*"))):
    rp, mp = os.path.join(d, "result.json"), os.path.join(d, "meta.json")
    if not os.path.exists(rp):
        skipped["blocked"] += 1
        continue
    r, m = json.load(open(rp)), json.load(open(mp))
    if args.rev and not m["sources"]["wgpu"]["rev"].startswith(args.rev):
        skipped["other_rev"] += 1
        continue
    if args.adapter.lower() not in m["machine"]["adapter"].lower():
        skipped["other_adapter"] += 1
        continue
    if r.get("quiet_during_run") is False:
        skipped["not_quiet"] += 1
        continue
    runs.setdefault(r["kernel"], []).append(r)

print(f"adapter filter: {args.adapter}; wgpu rev: {args.rev or 'any'}; skipped: {skipped}\n")
print("| workload | runs | clamps no_bce→bce (unchecked floor) | no_bce/bce geomean | per-run min–max | control max dev | bce/unchecked | no_bce/unchecked | oracle | verdict |")
print("|---|---|---|---|---|---|---|---|---|---|")
summary = {}
for k in sorted(runs, key=lambda n: (not n.startswith("blur"), n)):
    rs = runs[k]
    eff = [r["ratio_no_bce_over_bce"]["median"] for r in rs]
    ctl = [r["ratio_control_bce_dup_over_bce"]["median"] for r in rs]
    res = [r["ratio_residual_bce_over_unchecked"]["median"] for r in rs]
    allc = [r["ratio_all_checks_no_bce_over_unchecked"]["median"] for r in rs]
    ok = all(r["outputs_ok"] for r in rs)
    ctl_dev = max(abs(c - 1) for c in ctl)
    g = gmean(eff)
    umin = rs[0]["umin_clamps_per_slot"]
    if not rs[0]["bce_changes_binary"]:
        verdict = "NO_PROOF"
    elif not ok:
        verdict = "INVALID: output oracle failed in some run"
    elif ctl_dev >= 0.005:
        verdict = "NOT_RESOLVABLE: control deviation >= 0.5% in some run"
    elif not (min(eff) > 1 or max(eff) < 1):
        verdict = "NOT_RESOLVABLE: runs disagree in sign"
    elif abs(g - 1) <= max(2 * ctl_dev, 0.005):
        verdict = "NOT_RESOLVABLE: effect <= max(2x control deviation, 0.5%)"
    else:
        verdict = "RESOLVED"
    oracle = rs[0].get("reference_oracle")
    oracle_kind = "exact (deterministic)" if oracle is None else f"reference set ({oracle['reference_states']} vello states)"
    print(f"| {k} | {len(rs)} | {umin[0]} → {umin[1]} ({umin[3]}) | **{g:.4f}** ({100*(g-1):+.2f}%) | {min(eff):.4f}–{max(eff):.4f} | {100*ctl_dev:.3f}% | {gmean(res):.4f} | {gmean(allc):.4f} | {'pass' if ok else 'FAIL'}: {oracle_kind} | {verdict} |")
    summary[k] = {
        "runs": len(rs), "outputs_ok_all": ok, "bce_changes_binary": rs[0]["bce_changes_binary"], "umin_clamps_per_slot": umin,
        "effect_geomean": g, "effect_min": min(eff), "effect_max": max(eff), "effect_per_run": eff,
        "control_max_dev": ctl_dev, "residual_geomean": gmean(res), "all_checks_geomean": gmean(allc),
        "bce_ns_per_rep_median_of_runs": sorted(r["median_ns_per_rep"]["bce"] for r in rs)[len(rs) // 2],
        "order_effect_ratio": [(r["order_effect_ratio"]["no_bce_first_median"], r["order_effect_ratio"]["bce_first_median"]) for r in rs],
        "provenance": rs[0]["provenance"], "verdict": verdict,
    }

vello = {k: v for k, v in summary.items() if k.startswith("vello:")}
if vello:
    resolved = {k: v for k, v in vello.items() if v["verdict"] == "RESOLVED"}
    suite = {
        "scenes": len(vello),
        "effect_geomean_over_scenes": gmean([v["effect_geomean"] for v in vello.values()]),
        "all_checks_geomean_over_scenes": gmean([v["all_checks_geomean"] for v in vello.values()]),
        "residual_geomean_over_scenes": gmean([v["residual_geomean"] for v in vello.values()]),
        "resolved_gain": sorted(k for k, v in resolved.items() if v["effect_geomean"] > 1),
        "resolved_loss": sorted(k for k, v in resolved.items() if v["effect_geomean"] < 1),
        "not_resolvable": sorted(k for k, v in vello.items() if v["verdict"].startswith("NOT_RESOLVABLE")),
        "invalid": sorted(k for k, v in vello.items() if v["verdict"].startswith("INVALID")),
    }
    summary["_vello_suite"] = suite
    e = suite["effect_geomean_over_scenes"]
    a = suite["all_checks_geomean_over_scenes"]
    print(f"\nvello suite ({suite['scenes']} scenes, equal weight): elision gain geomean {e:.4f} ({100*(e-1):+.2f}%), "
          f"cost of all checks geomean {a:.4f} ({100*(a-1):+.2f}%); RESOLVED gains {len(suite['resolved_gain'])}, "
          f"RESOLVED losses {len(suite['resolved_loss'])}, not resolvable {len(suite['not_resolvable'])}, invalid {len(suite['invalid'])}")
tag = args.adapter.lower() + (f"-{args.rev}" if args.rev else "")
out = os.path.join(args.root, f"summary-{tag}.json")
json.dump(summary, open(out, "w"), indent=2)
print(f"\nwritten {out}")
