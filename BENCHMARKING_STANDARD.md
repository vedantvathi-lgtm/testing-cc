# Benchmarking Standard — Bounds-Check Elision Project
**Version 1.0 · Aug 25 2026 · Supersedes ad-hoc practice; implements the unimplemented parts of FIXED_BENCHMARKING_SUITE.md**

## 0. Non-negotiable rules

1. **Provenance or it didn't happen.** Every result file MUST record the exact source revision of every component under test (`naga_git_rev`, `wgpu_git_rev`, dirty flag), driver version, GPU name, OS kernel, timestamp UTC, and harness version. `wgpu_version: "0.1.0"`-style junk fields are forbidden.
2. **No metric lives only on stderr.** SPIR-V word counts, instruction counts, disassembly hashes: persisted as JSON alongside timings, same run id.
3. **One file, one hypothesis.** Result filenames follow the grammar below; `_v2`-style suffixes are forbidden — a changed methodology is a new methodology name, not a silent overwrite.
4. **Statistics are part of the schema**, not prose: interleaved rounds, warmup policy, per-config sample count, bootstrap CI on medians, and a negative control run logged in the same file family.
5. **Comparability rule:** cross-harness comparisons (wgpu vs CUDA) must state per-side methodology deltas explicitly in the file, else the comparison is void.

## 1. Directory layout

```
bounds-check-testing-framework/
  BENCHMARKING_STANDARD.md        <- this file
  runs/
    <UTCYYYYMMDDTHHMMSSZ>-<slug>/
      meta.json                   <- provenance block (§3)
      <bench-name>.json           <- measurements, schema v3
      <bench-name>.words.json     <- SPIR-V/instruction counts (if applicable)
      artifacts/                  <- disassembly, shaders used, plots
  harnesses/
    paired-vulkan/                <- merged single harness (absorbs bounds-check-bench + matmul-paired-bench)
    cuda/
  tools/
    index_results.py              <- builds INDEX.md from runs/** (schema v1/v2/v3 aware)
```

## 2. Filename grammar

`<UTC stamp>-<kernel>-<variant>-<policy-set>-<harnessver>.json`
Example: `20260825T141530Z-matmul-elided-vs-retained-paired-h3.json`
Legacy files stay in place until migrated; the indexer tags them `legacy-schema`.

## 3. Mandatory meta.json (provenance block)

```json
{
  "schema": "bce-meta-1",
  "utc_start": "...", "utc_end": "...",
  "sources": {
    "naga":   {"path": "...", "rev": "<sha>", "dirty": true, "branch": "..."},
    "wgpu":   {"path": "...", "rev": "<sha>", "dirty": false, "branch": "..."},
    "harness": {"version": "paired-vulkan-3"}
  },
  "machine": {"gpu": "NVIDIA GeForce RTX 4060 Laptop", "driver": "580.159.04",
               "vulkan_api_version": "...", "kernel": "...", "cpu_governor": "..."},
  "power_thermal": {"note": "laptop GPU; plug state + thermals recorded if available"},
  "negative_control": {"file": "...-negctl.json", "ci_includes_zero": true}
}
```

## 4. Measurement protocol (fixed)

| Item | Value |
|---|---|
| Timing primitive | GPU timestamp queries (`timestampPeriod` verified = 1 ns) |
| Structure | Round-robin interleave, one trial of each config per round |
| Warmup | ≥ N_warmup recorded in file (default 50 rounds discarded) |
| Samples | ≥ 200 retained rounds default; record actual |
| Statistic | Median + bootstrap 95% CI (B=10 000, fixed seed recorded) |
| Outlier policy | None post-hoc; AD normality recorded, no filtering |
| Negative control | Identical-config A/B must yield CI ⊇ 0; run once per harness change, stored adjacent |

## 5. Metrics dictionary (canonical names)

- `spv_words_total`, `spv_words_by_op{OpULessThan,...}` — static size signal
- `median_us`, `ci95_lo_us`, `ci95_hi_us`, `iqr_us`, `n_rounds`, `n_warmup`
- `delta_vs_baseline_ns` + its CI (paired, same-round differencing)
- `elision_hits`, `elision_candidates` (from corpus_hit_rate definitions)

## 6. Migration duty

`tools/index_results.py` indexes legacy v1/v2 files read-only, emitting per-file provenance gaps.
New work writes ONLY into `runs/` with schema v3. Old files are frozen evidence — never edited.
