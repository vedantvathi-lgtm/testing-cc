#!/usr/bin/env python3
"""EXPLORATORY (not in pre-registration 2): total cost of all bounds checks (no_bce/unchecked) and the share of it
that elision recovers (ln(no_bce/bce) / ln(no_bce/unchecked)), per suite and GPU, from realworld-2 runs at 19ffb4e
and the runs-v4 summaries. Raw per-run medians, not control-corrected. Writes poster/check_cost.json."""
import collections, glob, json, math, os, statistics as st
FW = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "bounds-check-testing-framework")
gm = lambda xs: math.exp(st.mean(math.log(x) for x in xs))
per = collections.defaultdict(list)
for rp in glob.glob(os.path.join(FW, "runs-realworld/2*/result.json")):
    r = json.load(open(rp)); m = json.load(open(rp.replace("result.json", "meta.json")))
    if r.get("harness_version") != "realworld-2" or not m["sources"]["wgpu"]["rev"].startswith("19ffb4e"): continue
    if r.get("quiet_during_run") is False or not r.get("outputs_ok"): continue
    gpu = "nvidia" if "nvidia" in m["machine"]["adapter"].lower() else "intel"
    per[(gpu, "vello" if r["kernel"].startswith("vello:") else r["kernel"], r["kernel"])].append(r)
agg = collections.defaultdict(lambda: {"all": [], "elid": []})
for (gpu, suite, _), rs in per.items():
    agg[(gpu, suite)]["all"].append(gm([r["ratio_all_checks_no_bce_over_unchecked"]["median"] for r in rs]))
    agg[(gpu, suite)]["elid"].append(gm([r["ratio_no_bce_over_bce"]["median"] for r in rs]))
out = {}
for (gpu, suite), d in sorted(agg.items()):
    a, e = gm(d["all"]), gm(d["elid"])
    out[f"realworld:{gpu}:{suite}"] = {"items": len(d["all"]), "all_checks_pct": round(100 * (a - 1), 2),
        "elision_raw_pct": round(100 * (e - 1), 2), "share_recovered": round(math.log(e) / math.log(a), 3) if a > 1.0005 else None}
for g in ("nvidia", "intel"):
    for k, v in json.load(open(os.path.join(FW, f"runs-v4/summary-{g}-19ffb4e.json"))).items():
        if isinstance(v, dict) and "all_checks_geomean" in v and k != "empty":
            a, e = v["all_checks_geomean"], v["effect_geomean"]
            out[f"v4:{g}:{k}"] = {"all_checks_pct": round(100 * (a - 1), 2), "elision_pct": round(100 * (e - 1), 2),
                "share_recovered": round(math.log(e) / math.log(a), 3) if a > 1.0005 else None, "verdict": v["verdict"]}
json.dump(out, open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "check_cost.json"), "w"), indent=1)
print(json.dumps(out, indent=1))
