#!/usr/bin/env python3
"""Charts for poster v3 (spec: poster/POSTER_RATINGS.md §5.6). One colour meaning everywhere:
NVIDIA = blue #0033A0, Intel = teal #2E9FB0; orange is reserved for clamps / not proved.
Drawn at printed size so point sizes are true on A0."""
import json
import math
import os

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
from matplotlib.patches import Patch  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
FW = os.path.join(ROOT, "bounds-check-testing-framework")
OUT = os.path.join(ROOT, "poster", "assets_v3")
os.makedirs(OUT, exist_ok=True)
INK, MUTED, GRID, NR = "#2F2F2F", "#6B6B6B", "#E3E3E3", "#9AA3B2"
NV, IN = "#0033A0", "#2E9FB0"
MM = 1 / 25.4
plt.rcParams.update({"font.family": "Liberation Sans", "font.size": 24, "text.color": INK,
                     "axes.labelcolor": INK, "xtick.color": MUTED, "ytick.color": MUTED,
                     "axes.edgecolor": "#BBBBBB", "axes.linewidth": 1.2, "svg.fonttype": "none"})


def save(fig, name):
    fig.savefig(os.path.join(OUT, name + ".svg"), facecolor="white")
    fig.savefig(os.path.join(OUT, name + ".png"), dpi=150, facecolor="white")
    plt.close(fig)


def clean(ax):
    for s in ("top", "right"):
        ax.spines[s].set_visible(False)


# ---- synthetic kernels ----
order = ["scan_shared", "matmul_tiled", "vello_binning", "reduce_tree", "prefix_scan_hs", "private_array"]
v4 = {g: json.load(open(os.path.join(FW, f"runs-v4/summary-{g}-19ffb4e.json"))) for g in ("nvidia", "intel")}
fig, ax = plt.subplots(figsize=(388 * MM, 190 * MM))
fig.subplots_adjust(left=0.215, right=0.985, top=0.86, bottom=0.14)
h = 0.36
for i, k in enumerate(order):
    for j, (g, c) in enumerate((("nvidia", NV), ("intel", IN))):
        r = v4[g][k]
        val = 100 * (r["effect_geomean"] - 1)
        y = i + (j - 0.5) * (h + 0.04)
        if r["verdict"] == "RESOLVED":
            ax.barh(y, val, height=h, color=c)
            ax.text(val + 0.25, y, f"+{val:.1f}%", va="center", fontsize=23, color=INK)
        else:  # summary verdict NOT_RESOLVABLE
            ax.plot([0, 0.35], [y, y], color=NR, linewidth=5, solid_capstyle="butt")
            ax.text(0.55, y, "n.r.", va="center", fontsize=22, color=NR)
ax.set_yticks(range(len(order)), [k.replace("_", " ") for k in order], fontsize=25, color=INK)
ax.tick_params(axis="y", length=0)
ax.set_ylim(5.6, -0.6)
ax.set_xlim(0, 18)
ax.set_xticks([0, 5, 10, 15], ["0", "5", "10", "15 %"])
ax.grid(axis="x", color=GRID)
ax.set_axisbelow(True)
clean(ax)
ax.legend(handles=[Patch(color=NV, label="NVIDIA RTX 4060"), Patch(color=IN, label="Intel RPL-S iGPU")],
          loc="lower left", bbox_to_anchor=(-0.02, 1.0), ncol=2, frameon=False, fontsize=25, handlelength=1.2)
ax.text(0, -0.13, "speedup from elision, geomean of valid runs   ·   n.r. = no effect distinguishable from noise",
        transform=ax.transAxes, fontsize=21, color=MUTED, va="top")
save(fig, "v4_kernels")

# ---- vello: side by side, each y-range stated in its title ----
fig, axes = plt.subplots(1, 2, figsize=(388 * MM, 140 * MM), gridspec_kw={"wspace": 0.28})
fig.subplots_adjust(left=0.085, right=0.99, top=0.82, bottom=0.07)
for ax, gpu, color, label in [(axes[0], "nvidia", NV, "NVIDIA RTX 4060"), (axes[1], "intel", IN, "Intel RPL-S iGPU")]:
    s = json.load(open(os.path.join(FW, f"runs-realworld/summary-v2-{gpu}-realworld-2-19ffb4e.json")))
    rows = sorted([(v["effect"], v["ci_lo"], v["ci_hi"]) for k, v in s.items()
                   if k.startswith("vello:") and v["effect"]], key=lambda r: r[0])
    suite = 100 * (s["_vello_suite"]["effect_geomean"] - 1)
    xs = range(1, len(rows) + 1)
    lo_all = min(100 * (math.exp(r[1]) - 1) for r in rows)
    hi_all = max(100 * (math.exp(r[2]) - 1) for r in rows)
    ax.axhspan(-0.5, 0.5, color="#EDEDED", zorder=0)
    ax.axhline(0, color=MUTED, linewidth=1.2, zorder=1)
    ax.axhline(suite, color=color, linewidth=1.8, linestyle=(0, (5, 4)), zorder=1)
    for x, (e, lo, hi) in zip(xs, rows):
        ax.vlines(x, 100 * (math.exp(lo) - 1), 100 * (math.exp(hi) - 1), color=color, linewidth=2.2, zorder=2)
    ax.scatter(xs, [100 * (r[0] - 1) for r in rows], s=55, color=color, edgecolor="white", linewidth=1.0, zorder=3)
    ax.set_title(f"{label}\ny axis: {math.floor(lo_all)} … {math.ceil(hi_all)} %", loc="left", fontsize=23,
                 color=INK, pad=10, linespacing=1.3)
    ax.text(len(rows) * 0.03, suite, f"geomean {suite:+.2f}%", color=color, fontsize=21, va="bottom", fontweight="bold")
    ax.set_xticks([])
    ax.set_xlim(0, len(rows) + 1)
    ax.grid(axis="y", color=GRID)
    ax.set_axisbelow(True)
    clean(ax)
save(fig, "vello_realworld")
print("written", OUT)
