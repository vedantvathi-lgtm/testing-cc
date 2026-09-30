#!/usr/bin/env python3
"""Race-location test for oracle-flagged vello scenes (spec 005).

Input: a determinism probe directory (bce-realworld-v1 with BCE_DETERMINISM=n): every distinct
output of every slot, saved as artifacts/determinism.<slot>.<hash>.bin, counts in determinism.json.
Question: do elided binaries (bce, bce_dup) deviate at pixels where binaries WITHOUT elision
(no_bce, unchecked) never deviate? V0 = pixels varying among no-elision outputs, V1 = among elided
outputs (both relative to the modal no_bce image). A miscompile shows up as elided-only pixels that
persist; a shared race shows comparable, overlapping sets in both directions."""
import collections, glob, json, os, sys

d = sys.argv[1].rstrip("/") + "/artifacts/"
meta = json.load(open(d + "determinism.json"))
imgs = collections.defaultdict(dict)
for f in glob.glob(d + "determinism.*.*.bin"):
    _, slot, h, _ = os.path.basename(f).split(".")
    imgs[slot][h] = open(f, "rb").read()
counts = dict(zip(meta["slots"], meta["counts"]))
mode_hash = max(counts["no_bce"], key=counts["no_bce"].get)
modal = imgs["no_bce"][mode_hash]

def px(a):
    return {i // 4 for i in range(0, len(a), 4) if a[i:i + 4] != modal[i:i + 4]}

def varying(slots):
    out = set()
    for s in slots:
        for img in imgs[s].values():
            out |= px(img)
    return out

def off_mode_runs(slot):
    return sum(c for h, c in counts[slot].items() if h != mode_hash)

v0, v1 = varying(["no_bce", "unchecked"]), varying(["bce", "bce_dup"])
res = {
    "dir": sys.argv[1], "adapter": meta["adapter"], "runs_per_slot": meta["runs_per_slot"],
    "distinct_outputs": {s: len(counts[s]) for s in meta["slots"]},
    "runs_off_modal_image": {s: off_mode_runs(s) for s in meta["slots"]},
    "pixels_varying_no_elision": len(v0), "pixels_varying_elided": len(v1),
    "elided_only_pixels": len(v1 - v0), "no_elision_only_pixels": len(v0 - v1),
}
print(json.dumps(res))
