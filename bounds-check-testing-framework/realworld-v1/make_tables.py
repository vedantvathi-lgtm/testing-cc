#!/usr/bin/env python3
"""Emit the markdown tables of reports/Real-world benchmark results.md from the campaign summaries
and the race-location probe (numbers copied by script, never by hand; AGENTS.md Rule 0.6)."""
import json
import os
import sys

root = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(__file__), "..", "runs-realworld")
nv = json.load(open(os.path.join(root, "summary-nvidia-19ffb4e.json")))
it = json.load(open(os.path.join(root, "summary-intel-19ffb4e.json")))


def pct(x):
    return f"{100 * (x - 1):+.2f}%"


def short(v):
    v = v["verdict"]
    return {"RESOLVED": "RESOLVED", "NO_PROOF": "no proof"}.get(v, "oracle-flagged" if v.startswith("INVALID") else "not resolvable")


print("### Per workload, both GPUs\n")
print("| workload | NVIDIA gain no_bce/bce | NVIDIA ctl dev | NVIDIA verdict | Intel gain no_bce/bce | Intel ctl dev | Intel all checks no_bce/unchecked | Intel bce/unchecked | Intel verdict |")
print("|---|---|---|---|---|---|---|---|---|")
keys = [k for k in nv if not k.startswith("_")]
for k in sorted(keys, key=lambda n: (not n.startswith("blur"), n)):
    a, b = nv[k], it.get(k)
    row = f"| {k} | {a['effect_geomean']:.4f} ({pct(a['effect_geomean'])}) | {100*a['control_max_dev']:.2f}% | {short(a)} | "
    if b:
        row += f"{b['effect_geomean']:.4f} ({pct(b['effect_geomean'])}) | {100*b['control_max_dev']:.2f}% | {b['all_checks_geomean']:.4f} | {b['residual_geomean']:.4f} | {short(b)} |"
    else:
        row += "– | – | – | – | – |"
    print(row)

print("\n### vello suite (34 scenes, each weighted equally)\n")
print("| GPU | elision gain (geomean) | cost of all checks (geomean) | bce/unchecked (geomean) | RESOLVED gains | RESOLVED losses | not resolvable | oracle-flagged |")
print("|---|---|---|---|---|---|---|---|")
for name, s in (("NVIDIA RTX 4060 Laptop", nv["_vello_suite"]), ("Intel RPL-S iGPU", it["_vello_suite"])):
    print(f"| {name} | {s['effect_geomean_over_scenes']:.4f} ({pct(s['effect_geomean_over_scenes'])}) | {s['all_checks_geomean_over_scenes']:.4f} ({pct(s['all_checks_geomean_over_scenes'])}) | {s['residual_geomean_over_scenes']:.4f} | {len(s['resolved_gain'])} | {len(s['resolved_loss'])} | {len(s['not_resolvable'])} | {len(s['invalid'])} |")

locs = [os.path.join(root, "_diag-20260930", f) for f in ("race-locations.jsonl", "race-locations-inmemory.jsonl")]
if all(os.path.exists(f) for f in locs):
    print("\n### Race-location test of oracle-flagged scenes (200 outputs per slot, slots interleaved)\n")
    print("| GPU | scene | distinct outputs no_bce / bce / bce_dup / unchecked | runs off the modal image (same order) | pixels varying without elision | pixels varying with elision | elided-only pixels | no-elision-only pixels |")
    print("|---|---|---|---|---|---|---|---|")
    tot = [0, 0]
    for f, method in zip(locs, ("images", "in-memory")):
        for line in open(f):
            r = json.loads(line)
            sc = r["dir"].split("vello:")[1].rsplit("-", 1)[0]
            d, o = r["distinct_outputs"], r["runs_off_modal_image"]
            order = ["no_bce", "bce", "bce_dup", "unchecked"]
            if r["pixels_varying_no_elision"] > 1_000_000:
                print(f"| {r['adapter'][:6]} | {sc} ({method}) | – | – | – | – | – | INVALID: every pixel counted as varying (V0 = whole image), consistent with image files truncated while the disk was full; redone in-memory below |")
                continue
            tot[0] += r["elided_only_pixels"]
            tot[1] += r["no_elision_only_pixels"]
            print(f"| {r['adapter'][:6]} | {sc} ({method}) | {' / '.join(str(d[s]) for s in order)} | {' / '.join(str(o[s]) for s in order)} | {r['pixels_varying_no_elision']} | {r['pixels_varying_elided']} | {r['elided_only_pixels']} | {r['no_elision_only_pixels']} |")
    print(f"| **total** | | | | | | **{tot[0]}** | **{tot[1]}** |")
