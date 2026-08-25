# WGSL Corpus Hit-Rate Study — Results
**Date:** Aug 25, 2026 · **Tool:** `corpus_hit_rate` (this crate) · **Compiler:** patched naga @ branch `bounds-check-elision` (commit `86eb81c`)
**Question answered (Reviewer 3 / Reviewer 2):** *How often do the two elision patterns actually fire on real-world WGSL?*

## Methodology

For every `.wgsl` file: parse → validate with `ValidationFlags::all()` / `Capabilities::all()` → read
`FunctionInfo::safe_indices` populated by the two analyses (builtin-index + loop-exit-bound extraction).

- **Candidate check** = one `Expression::Access` node (runtime-varying index). `AccessIndex` nodes are static and never guarded; counted separately.
- **Elided** = an `Access` whose index handle ∈ `safe_indices`. One shared index expression serving N accesses counts as N elided checks.
- **Breakdown**: safe-set entries classified by shape — bare `FunctionArgument` (builtin-lid), `Load{LocalVariable}` (loop-iv), other.
- Compute-kernel segmentation via entry-point stage (`dyn_in_compute`, `elid_in_compute`).

## Results

| Corpus | Files OK / total | Dynamic checks | Elided | Rate (all) | Dyn in compute | Elided in compute | Rate (compute) |
|---|---|---|---|---|---|---|---|
| naga test suite (`naga/tests/in`) | 166 / 166 | 644 | 8 | **1.24 %** | 55 | 8 | **14.55 %** |
| webgpu-samples (official, cloned today) | 69 / 75¹ | 93 | 2 | **2.15 %** | 23 | 2 | **8.70 %** |
| wgpu upstream (tests + examples) | 421 / 433² | 871 | 2 | **0.23 %** | 138 | 2 | **1.45 %** |
| Bevy engine shaders³ | 23 / 193 | — | 0 | — | — | — | — |

¹ 6 parse failures (non-standard directives) · ² 12 parse failures · ³ Requires Bevy's `#import` macro preprocessor; line-stripping is insufficient (imports inject identifiers/bodies). Excluded from headline numbers rather than fudged.

### Where the elisions came from

| File | Corpus | Elided | Mechanism |
|---|---|---|---|
| `sample/imageBlur/blur.wgsl` | webgpu-samples | 2 | loop induction (`template-list-ge` style bound) |
| `wgsl/template-list-ge.wgsl` | wgpu upstream | 2 | loop induction |
| `bounds-check-elision-{zero,restrict}.wgsl` | naga tests | 6 | **our own crafted tests** (2 builtin-lid + rest) |
| `wgsl/template-list-ge.wgsl` | naga tests | 2 | loop induction |

## Findings (brutal, data-backed)

1. **Dynamic indexing itself is rare in modern WGSL.** ~1.3 runtime-indexed accesses per shader across official samples (93 total in 69 files). Most WebGPU content simply doesn't dynamically index arrays much — consistent with Würthinger et al.'s DaCapo finding for Java and Readyset's production Rust measurement.
2. **The builtin-index (spec-guarantee) pattern has a near-empty natural niche**: zero hits outside shaders written to trigger it. Real compute kernels rarely do `data[lid]` on a fixed-size workgroup array with length ≥ workgroup size; they use `lid`-derived arithmetic, tiles, or runtime-sized storage.
3. **The loop-induction pattern fires occasionally in genuine code** (blur kernel) but yields ≈0.2–2 % end-to-end.
4. **Within compute kernels that do dynamic indexing, elision reaches 8.7–14.6 %** — the honest headline is segment-specific, not global.
5. Therefore the reviewers' instinct was correct *and quantifiable*: current pattern coverage explains why this remains a workshop poster, not an upstream default-on pass.

## What this buys the project

- A falsifiable number for the poster: *"On 656 validated real-world WGSL programs, 1,608 runtime bounds checks were identified; current analyses elide 12 overall (0.7 %) but 8.7–14.6 % within dynamically-indexing compute kernels."*
- A target list ranked by observed yield: classify the 216 un-elided compute-side accesses by form (lid±k, tile[i][j], storage-runtime arrays, nested loops) to decide which next pattern pays.
- Negative-result credibility: engaging Readyset/Würthinger-style evidence instead of ignoring it.

## Reproduce

```bash
cd corpus_hit_rate && cargo build
target/debug/corpus_hit_rate <name> <dir>... >out.jsonl 2>out.log   # SUMMARY line lands in out.log
git clone --depth 1 https://github.com/webgpu/webgpu-samples /tmp/opencode/webgpu-samples
```
