#!/usr/bin/env python3
"""Index all legacy benchmark result files; flag provenance gaps per BENCHMARKING_STANDARD.md."""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CANDIDATES = [ROOT / "bounds-check-bench", ROOT / "matmul-paired-bench"]

rows = []
for d in CANDIDATES:
    for f in sorted(d.glob("*.json")):
        if f.name == "Cargo.lock":
            continue
        try:
            data = json.loads(f.read_text())
        except Exception as e:
            rows.append((f, "UNPARSEABLE", str(e)[:40]))
            continue
        if not isinstance(data, dict) or "measurements" not in data:
            continue
        schema = data.get("schema_version", "MISSING")
        sw = data.get("software", {})
        gaps = []
        rev = sw.get("naga_git_rev") or data.get("sources")
        if not rev:
            gaps.append("no-source-rev")
        if sw.get("wgpu_version") in ("0.1.0", None, ""):
            gaps.append("junk-wgpu-version")
        meas = data["measurements"]
        has_ci = any("ci_95_median" in m for m in meas)
        if not has_ci:
            gaps.append("no-ci")
        n_rounds = max((m.get("n_samples", 0) for m in meas), default=0)
        if n_rounds < 200:
            gaps.append(f"low-n={n_rounds}")
        kernels = sorted({m.get("kernel", "?") for m in meas})
        rows.append((f, f"schema={schema}", ",".join(gaps) or "clean"))

print(f"| {'File':70} | {'Schema':10} | Gaps |")
print("|---|---|---|")
for f, schema, gaps in rows:
    rel = f.relative_to(ROOT)
    print(f"| `{rel}` | {schema} | {gaps} |")

bad = sum(1 for _, _, g in rows if g not in ("clean", "UNPARSEABLE"))
print(f"\n{len(rows)} result files indexed; {bad} with provenance/statistics gaps.")
print("Per BENCHMARKING_STANDARD.md §6: legacy files are frozen; new runs write schema v3 under runs/.")
