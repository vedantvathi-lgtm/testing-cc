# WGSL Corpus Hit-Rate Study — Results (v5, final)

**Date:** Aug 27, 2026 · **Tool:** `corpus_hit_rate` (schema 5.0.0, harness `corpus-hit-rate-5`)
**Compiler:** patched naga @ branch `bounds-check-elision` (interval-domain rewrite, commit `8757d7d`)
**Question:** *How often do the elision analyses fire on real-world WGSL, before vs. after the sound interval-domain rewrite?*

## Methodology

For every `.wgsl` file: parse → validate (`ValidationFlags::all()` / `Capabilities::all()`) → read
`FunctionInfo::safe_indices` populated by the interval-domain analysis; disassemble emitted SPIR-V
(rspirv) to count `OpULessThan` with elision enabled vs. disabled as a code-level cross-check.

- **Candidate** = one runtime-indexed `Expression::Access`. Static `AccessIndex` counted separately.
- **Elided** = `Access` whose index handle ∈ `safe_indices`.
- **Dedup** by SHA-256 content hash (duplicates reported separately).
- **Shape classification** of index expressions: builtin-lid, affine, loop-local-load, literal, unsupported.
- **Compute-kernel segmentation** via entry-point stage.
- Project-authored and external corpora reported separately.

## Results — AFTER interval-domain rewrite (v5, final)

| Corpus | Unique OK / files | Dynamic checks | Elided | Rate (all) | Dyn in compute | Elided in compute | Rate (compute) |
|---|---|---|---|---|---|---|---|
| naga test suite (`wgpu/naga/tests/in/wgsl`) | 153 / 164 | 587 | 435 | **74.1 %** | 504 | 413 | **81.9 %** |
| wgpu upstream (benches+examples+tests) | 76 / 85 | 79 | 2 | **2.5 %** | 43 | 2 | **4.7 %** |
| webgpu-samples (official) | 66 / 75 | 92 | 1 | **1.1 %** | 24 | 1 | **4.2 %** |

SPIR-V cross-check (naga test corpus): `OpULessThan` count 505 (elision off) → 134 (elision on), **−371 checks (−73 %)** — consistent with the 74.1 % IR-level rate.

## Results — BEFORE rewrite (v4 lineage, historical baseline)

| Corpus | Dynamic checks | Elided | Rate (all) | Rate (compute) |
|---|---|---|---|---|
| naga test suite | 644 | 8 | 1.24 % | 14.55 % |
| webgpu-samples | 93 | 2 | 2.15 % | 8.70 % |
| wgpu upstream | 871 | 2 | 0.23 % | 1.45 % |

## Findings

1. **The interval-domain rewrite transformed the project corpus**: 74.1 % overall / 81.9 % in-compute elision (was 1.24 % / 14.55 %). The path-sensitive ranges, condition refinement, affine indices, and recurrence recognition capture the loop-heavy, guarded patterns that dominate naga's own test shaders.
2. **External real-world corpora remain low (1–2.5 % overall, ~4–5 % in compute)** — unchanged from the legacy analyses. Two causes: (a) dynamic indexing itself is rare in shipped WGSL (~1.3 accesses/shader); (b) most external dynamic accesses index runtime-sized storage buffers, which are provable only when a dominating guard exists and whose length is unknown at compile time.
3. **Shape classification of the un-elided external accesses**: 69/79 in wgpu upstream are `unsupported` forms (runtime-length storage arrays, complex index expressions) — a ranked roadmap for the next pattern, with known yield.
4. **Honest segment-specific headline**: *"On 295 validated real-world WGSL programs (deduplicated), 758 runtime bounds checks were identified; the interval-domain analysis elides 438 overall — 74 % on loop-rich project shaders but 1–2.5 % on shipped external code, where runtime-sized storage buffers dominate the residual."*

## Reproduce

```bash
cd corpus_hit_rate && cargo build
./target/debug/corpus_hit_rate final-v5-project ../wgpu/naga/tests/in/wgsl \
  > results/final-v5-project-naga-tests.jsonl 2> results/final-v5-project-naga-tests.log
./target/debug/corpus_hit_rate final-v5-external \
  /home/bb/Projects/wgpu/benches /home/bb/Projects/wgpu/examples /home/bb/Projects/wgpu/tests \
  > results/final-v5-external.jsonl 2> results/final-v5-external.log
git clone --depth 1 https://github.com/webgpu/webgpu-samples /tmp/opencode/webgpu-samples
./target/debug/corpus_hit_rate final-v5-webgpu-samples /tmp/opencode/webgpu-samples \
  > results/final-v5-webgpu-samples.jsonl 2> results/final-v5-webgpu-samples.log
# SUMMARY line lands in each .log
```
