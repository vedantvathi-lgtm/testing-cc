#!/usr/bin/env python3
"""Poster numbers and charts for IICT 2026 paper #55, generated from committed artifacts only
(AGENTS.md Rule 0.6: numbers are copied by script, never typed).

Writes poster/numbers.json (every value with its source path) and poster/assets/*.png|svg
at A0 print size (300 dpi). Style: the IndiaFOSS deck (Arial ~ Liberation Sans, text #2F2F2F,
titles #0033A0); data marks use the validated pair NVIDIA #2457C5 / Intel #E15E32
(dataviz validate_palette.js: all six checks pass, light mode)."""
import glob
import json
import math
import os

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
FW = os.path.join(ROOT, "bounds-check-testing-framework")
OUT = os.path.join(ROOT, "poster", "assets")
os.makedirs(OUT, exist_ok=True)

INK, MUTED, GRID, TITLE = "#2F2F2F", "#6B6B6B", "#E3E3E3", "#0033A0"
NV, IN = "#2457C5", "#E15E32"
MM = 1 / 25.4
plt.rcParams.update({
    "font.family": "Liberation Sans", "font.size": 22, "text.color": INK, "axes.labelcolor": INK,
    "xtick.color": MUTED, "ytick.color": MUTED, "axes.edgecolor": GRID, "axes.linewidth": 1.2,
    "svg.fonttype": "none",
})
numbers = {}


def note(key, value, source):
    numbers[key] = {"value": value, "source": source}
    return value


def save(fig, name):
    fig.savefig(os.path.join(OUT, name + ".png"), dpi=300, bbox_inches="tight", facecolor="white")
    fig.savefig(os.path.join(OUT, name + ".svg"), bbox_inches="tight", facecolor="white")
    plt.close(fig)


# ---------- 1. real-world vello, confirmatory campaign (rule v2) ----------
def load_v2(gpu):
    p = f"runs-realworld/summary-v2-{gpu}-realworld-2-19ffb4e.json"
    return json.load(open(os.path.join(FW, p))), "bounds-check-testing-framework/" + p


fig, axes = plt.subplots(2, 1, figsize=(250 * MM, 210 * MM), sharex=True, gridspec_kw={"hspace": 0.55})
for ax, gpu, color, label in [(axes[0], "nvidia", NV, "NVIDIA RTX 4060 Laptop"), (axes[1], "intel", IN, "Intel RPL-S iGPU")]:
    s, src = load_v2(gpu)
    rows = sorted([(v["effect"], v["ci_lo"], v["ci_hi"], v["verdict"]) for k, v in s.items()
                   if k.startswith("vello:") and v["effect"]], key=lambda r: r[0])
    suite = s["_vello_suite"]
    note(f"vello_{gpu}_suite_pct", round(100 * (suite["effect_geomean"] - 1), 2), src)
    note(f"vello_{gpu}_verdicts", suite["verdicts"], src)
    xs = range(1, len(rows) + 1)
    ax.axhspan(-0.5, 0.5, color="#F2F2F2", zorder=0)
    ax.axhline(0, color=MUTED, linewidth=1.2, zorder=1)
    for x, (e, lo, hi, v) in zip(xs, rows):
        ax.vlines(x, 100 * (math.exp(lo) - 1), 100 * (math.exp(hi) - 1), color=color, linewidth=2, zorder=2)
    ax.scatter(xs, [100 * (r[0] - 1) for r in rows], s=70, color=color, edgecolor="white", linewidth=1.5, zorder=3)
    ax.set_ylabel("speedup (%)")
    ax.grid(axis="y", color=GRID, linewidth=1)
    ax.set_axisbelow(True)
    for side in ("top", "right"):
        ax.spines[side].set_visible(False)
    g = 100 * (suite["effect_geomean"] - 1)
    ax.set_title(f"{label}: suite {g:+.2f}%\n{suite['verdicts'].get('GAIN', 0)} faster · "
                 f"{suite['verdicts'].get('LOSS', 0)} slower · {suite['verdicts'].get('EQUIVALENT', 0)} within ±0.5%",
                 loc="left", fontsize=22, color=INK, pad=12, linespacing=1.4)
axes[1].set_xlabel("vello test scenes, sorted by effect  ·  bars = 95% CI across runs  ·  grey band = ±0.5%", labelpad=14)
axes[1].set_xticks([])
save(fig, "vello_realworld")

# ---------- 2. synthetic v4 kernels (19ffb4e) ----------
order = ["scan_shared", "matmul_tiled", "vello_binning", "reduce_tree", "prefix_scan_hs", "private_array"]
v4 = {g: json.load(open(os.path.join(FW, f"runs-v4/summary-{g}-19ffb4e.json"))) for g in ("nvidia", "intel")}
fig, ax = plt.subplots(figsize=(250 * MM, 120 * MM))
h = 0.38
for i, k in enumerate(order):
    for j, (g, c) in enumerate((("nvidia", NV), ("intel", IN))):
        val = 100 * (v4[g][k]["effect_geomean"] - 1)
        note(f"v4_{g}_{k}_pct", round(val, 2), f"bounds-check-testing-framework/runs-v4/summary-{g}-19ffb4e.json")
        y = i + (j - 0.5) * h
        ax.barh(y, val, height=h - 0.06, color=c)
        txt = "0.0%" if abs(val) < 0.05 else f"{val:+.1f}%"
        ax.text(max(val, 0) + 0.3, y, txt, va="center", fontsize=19, color=INK)
ax.set_yticks(range(len(order)), [k.replace("_", " ") for k in order])
ax.invert_yaxis()
ax.axvline(0, color=MUTED, linewidth=1.2)
ax.set_xlabel("speedup from elision (%), geomean of 10 (NVIDIA) / 5 (Intel) runs")
for side in ("top", "right"):
    ax.spines[side].set_visible(False)
ax.grid(axis="x", color=GRID)
ax.set_axisbelow(True)
from matplotlib.patches import Patch  # noqa: E402
ax.legend(handles=[Patch(color=NV, label="NVIDIA"), Patch(color=IN, label="Intel")], frameon=False, loc="lower right")
save(fig, "v4_kernels")

# ---------- 3. coverage funnel (census, current code) ----------
census = []
for f in glob.glob(os.path.join(ROOT, "research_notes/Real world shader elision gaps/census/F006_*.jsonl")):
    if "nagatests" in f:
        continue
    census += [json.loads(l) for l in open(f)]
src = "research_notes/Real world shader elision gaps/census/F006_*.jsonl (excl. nagatests)"
total = note("census_total", len(census), src)
fixed = note("census_fixed_or_vector", sum(1 for r in census if r["base_kind"].startswith(("fixed_array", "vector"))), src)
safe = note("census_proven", sum(r["safe"] for r in census), src)
projects = note("census_projects", sorted({r["file"].split("/")[0] if r["corpus"] == "misc" else r["corpus"] for r in census}), src)
runtime = note("census_runtime_sized", sum(1 for r in census if r["base_kind"].startswith("runtime")), src)
fig, ax = plt.subplots(figsize=(250 * MM, 95 * MM))
bars = [(f"runtime-indexed accesses in {len(projects)} open-source projects", total, "#C9C9C9"),
        ("into fixed-size arrays / vectors (provable in principle)", fixed, "#8FA8E0"),
        ("proven in bounds by the analysis", safe, NV)]
for i, (lab, v, c) in enumerate(bars):
    y = 1.6 * i
    ax.barh(y, v, color=c, height=0.62)
    ax.text(v + 8, y, f"{v}", va="center", fontsize=24, fontweight="bold", color=INK)
    ax.text(0, y - 0.42, lab, va="bottom", fontsize=19, color=INK)
ax.set_ylim(3.7, -1.1)
ax.set_xlim(0, total * 1.12)
ax.axis("off")
save(fig, "coverage")
note("census_runtime_sized_pct", round(100 * runtime / total, 1), src)

# ---------- 4. clamps in production binaries ----------
rw = sorted(glob.glob(os.path.join(FW, "runs-realworld/2*-vello:13-blend_grid/result.json")))[-1]
u = json.load(open(rw))["umin_clamps_per_slot"]
note("vello_umin_no_bce_bce_unchecked", u, os.path.relpath(rw, ROOT))
note("vello_bounds_clamps_before_after", [u[0] - u[3], u[1] - u[3]], "UMin minus shader-authored min() (unchecked slot)")

# ---------- 5. synthetic kernels: clamps and binary identity (latest 19ffb4e run per kernel) ----------
import hashlib  # noqa: E402
import subprocess  # noqa: E402


def umin(spv):
    return subprocess.run(["spirv-dis", spv], capture_output=True, text=True, check=True).stdout.count(" UMin ")


ident = {}
for k in order:
    runs = [d for d in sorted(glob.glob(os.path.join(FW, f"runs-v4/2*-{k}")))
            if json.load(open(os.path.join(d, "meta.json")))["sources"]["wgpu"]["rev"].startswith("19ffb4e")]
    a = os.path.join(runs[-1], "artifacts")
    h = lambda f: hashlib.sha256(open(os.path.join(a, f), "rb").read()).hexdigest()
    ident[k] = {"umin_no_bce": umin(os.path.join(a, "no_bce.spv")), "umin_bce": umin(os.path.join(a, "bce.spv")),
                "bce_equals_unchecked": h("bce.spv") == h("unchecked.spv"), "run": os.path.relpath(runs[-1], ROOT)}
note("v4_binary_identity", ident, "runs-v4/<latest 19ffb4e run>/artifacts/*.spv (spirv-dis, sha256)")

json.dump(numbers, open(os.path.join(ROOT, "poster", "numbers.json"), "w"), indent=1)
print(json.dumps({k: v["value"] for k, v in numbers.items() if not k.startswith("v4_")}, indent=1))
