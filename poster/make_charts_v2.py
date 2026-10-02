#!/usr/bin/env python3
"""Charts for poster v2, drawn at their printed size (one A0 column = 380 mm wide) so that point sizes
are true on paper. Data come from the same artifacts as make_assets.py (run that first for numbers.json)."""
import json
import math
import os

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
FW = os.path.join(ROOT, "bounds-check-testing-framework")
OUT = os.path.join(ROOT, "poster", "assets_v2")
os.makedirs(OUT, exist_ok=True)
INK, MUTED, GRID = "#2F2F2F", "#6B6B6B", "#E3E3E3"
NV, IN = "#2457C5", "#E15E32"
MM = 1 / 25.4
plt.rcParams.update({"font.family": "Liberation Sans", "font.size": 22, "text.color": INK,
                     "axes.labelcolor": INK, "xtick.color": MUTED, "ytick.color": MUTED,
                     "axes.edgecolor": "#BBBBBB", "axes.linewidth": 1.2, "svg.fonttype": "none"})


def save(fig, name):
    fig.savefig(os.path.join(OUT, name + ".svg"), facecolor="white")
    fig.savefig(os.path.join(OUT, name + ".png"), dpi=200, facecolor="white")
    plt.close(fig)


def clean(ax):
    for s in ("top", "right"):
        ax.spines[s].set_visible(False)


# ---- synthetic kernels (runs-v4, 19ffb4e) ----
order = ["scan_shared", "matmul_tiled", "vello_binning", "reduce_tree", "prefix_scan_hs", "private_array"]
v4 = {g: json.load(open(os.path.join(FW, f"runs-v4/summary-{g}-19ffb4e.json"))) for g in ("nvidia", "intel")}
fig, ax = plt.subplots(figsize=(372 * MM, 128 * MM))
fig.subplots_adjust(left=0.2, right=0.97, top=0.97, bottom=0.2)
h = 0.38
for i, k in enumerate(order):
    for j, (g, c) in enumerate((("nvidia", NV), ("intel", IN))):
        r = v4[g][k]
        val = 100 * (r["effect_geomean"] - 1)
        y = i + (j - 0.5) * h
        if r["verdict"] == "RESOLVED":
            ax.barh(y, val, height=h - 0.05, color=c)
            ax.text(max(val, 0) + 0.25, y, f"{val:+.1f}%", va="center", fontsize=20, color=INK)
        else:  # summary verdict NOT_RESOLVABLE: no effect distinguishable from noise
            ax.text(0.25, y, "not resolvable", va="center", fontsize=18, color=c, style="italic")
ax.set_yticks(range(len(order)), [k.replace("_", " ") for k in order], fontsize=22, color=INK)
ax.tick_params(axis="y", length=0)
ax.set_ylim(5.55, -0.55)
ax.axvline(0, color=MUTED, linewidth=1.2)
ax.set_xlim(0, 19)
ax.set_xlabel("speedup from elision (%), geomean of valid runs", fontsize=21)
ax.grid(axis="x", color=GRID)
ax.set_axisbelow(True)
clean(ax)
ax.text(18.8, 3.0, "NVIDIA", color=NV, fontsize=24, fontweight="bold", ha="right")
ax.text(18.8, 3.7, "Intel", color=IN, fontsize=24, fontweight="bold", ha="right")
save(fig, "v4_kernels")

# ---- vello scenes (realworld-2, rule v2) ----
fig, axes = plt.subplots(2, 1, figsize=(372 * MM, 140 * MM), gridspec_kw={"hspace": 0.5})
fig.subplots_adjust(left=0.11, right=0.985, top=0.92, bottom=0.04)
for ax, gpu, color, label in [(axes[0], "nvidia", NV, "NVIDIA RTX 4060 Laptop"), (axes[1], "intel", IN, "Intel RPL-S iGPU")]:
    s = json.load(open(os.path.join(FW, f"runs-realworld/summary-v2-{gpu}-realworld-2-19ffb4e.json")))
    rows = sorted([(v["effect"], v["ci_lo"], v["ci_hi"]) for k, v in s.items()
                   if k.startswith("vello:") and v["effect"]], key=lambda r: r[0])
    suite = 100 * (s["_vello_suite"]["effect_geomean"] - 1)
    xs = range(1, len(rows) + 1)
    ax.axhspan(-0.5, 0.5, color="#EDEDED", zorder=0)
    ax.axhline(0, color=MUTED, linewidth=1.2, zorder=1)
    ax.axhline(suite, color=color, linewidth=1.6, linestyle=(0, (5, 4)), zorder=1)
    for x, (e, lo, hi) in zip(xs, rows):
        ax.vlines(x, 100 * (math.exp(lo) - 1), 100 * (math.exp(hi) - 1), color=color, linewidth=2.2, zorder=2)
    ax.scatter(xs, [100 * (r[0] - 1) for r in rows], s=60, color=color, edgecolor="white", linewidth=1.2, zorder=3)
    ax.text(1, suite, f"suite geomean {suite:+.2f}%", color=color, fontsize=20, va="bottom", fontweight="bold")
    ax.set_title(label, loc="left", fontsize=23, color=INK, pad=8, fontweight="bold")
    ax.set_ylabel("speedup (%)", fontsize=20)
    ax.set_xticks([])
    ax.set_xlim(0, len(rows) + 1)
    ax.grid(axis="y", color=GRID)
    ax.set_axisbelow(True)
    clean(ax)
save(fig, "vello_realworld")
print("written", OUT)
