# Experiments & methods — what ran, what it proved, what is disqualified

> Purpose: any future agent must be able to (a) know which numbers may be cited,
> (b) understand exactly why legacy numbers may not be, and (c) run new experiments
> under the standard without re-deriving anything.

## 1. Measurement primitives (keep using these)

- **GPU timing**: wgpu `ComputePassDescriptor::timestamp_writes` (`Features::TIMESTAMP_QUERY`)
  — the only Web-and-native-portable GPU timestamp path; deliberately NOT the
  native-only alternatives (`TIMESTAMP_QUERY_INSIDE_ENCODERS/PASSES`).
  Multiply by `Queue::get_timestamp_period()` (verified non-1.0 historically;
  1 ns/tick with 64 valid bits in the paired harness).
- **Statistics**: paired per-round deltas between configs; round-robin interleave with
  rotating start order; bootstrap 95% CI (≥2,000 resamples) on the median; report full
  per-round distribution. Paired estimator is the scientific question — do not replace
  with unpaired permutation tests (audit finding #11).
- **Pipeline self-check**: identical-binary negative control through the same
  interleave+bootstrap machinery; PASS = CI contains zero.
- Source of record for the redesigned protocol: `FIXED_BENCHMARKING_SUITE.md`
  (greenfield spec, Jul 5) and `bounds-check-testing-framework/BENCHMARKING_STANDARD.md`.

## 2. Why legacy timing JSON is disqualified (the audit's verdict, condensed)

From `TEST_BENCHMARK_AUDIT.md` (Aug 27) — the load-bearing items:
1. No compiler provenance in 20/20 files (cannot tie a number to a source revision).
2. n=10 samples in 18/20 — CIs wider than any claimed effect.
3. `bounds-check-bench` depends on the wrong naga checkout (`~/Projects/naga`) vs the
   paper's implementation (`Dave-Sonar-WGPU--master/wgpu/naga`) — wrong-lineage risk.
4. "Interleaved" claim false: samples collected blocked per policy (drift-prone).
5. `bounds_checks_emitted` field actually stored `spv.len()` (word count ≠ check count).
6. Output buffer too small for prefix_scan/track3b writes → the three policies executed
   materially different workloads.
7. Policy comparison (Unchecked vs RZSW vs Restrict) ≠ analysis on/off comparison —
   the BCE experiment requires same-policy enabled/disabled builds.
8. Paired-matmul retained arm had a genuinely OOB access (`B[k+1]`, k=1023, len 1024)
   → not semantically equivalent; workload had no externally observable output.
Consequence: **all legacy timing = SUPERSEDED/INVALID for headline claims.**
Static word counts (176/213/399/432/404) remain valid (reproduced twice, structural).

## 3. The one rigorously measured result that stands (frozen, awaiting re-run)

Paired matmul, RTX 4060 Laptop, Vulkan, interleaved 500 rounds × 4 configs, bootstrap
2000-resample (`a.md` §10b):
- Elided (checked vs unchecked): median delta **0.0 ns**, CI [0.0, 0.0].
- Retained: median delta **7,696 ns**, CI [7,536, 7,808].
- Derived bounds-check overhead: **7,568 ns** CI [7,456, 7,744] ≈ 28.9 ps/check ≈ ~69
  cycles @2.4 GHz (262,144 evaluations). Old 2,400 ns / 5–6 cycles figures: dead.
- Negative control (§11b): A/B identical binaries → CI [0.0, 0.0] PASS;
  timestampPeriod 1 ns/tick, 64 valid bits.
Status: VERIFIED-RECORDED; must be **re-run under `paired-vulkan-v3`** (provenance-
complete) before appearing in the final paper — driver now permits it (P1).

## 4. Corpus study (the current headline evidence)

`corpus_hit_rate` v5 (commit `6ff062b`, schema 5.0.0) — method + numbers in
`corpus_hit_rate/results/RESULTS.md`. Key properties: dedup by SHA-256; validation-
gated; shape classification (builtin-lid / affine / loop-local-load / literal /
unsupported); separate project vs external reporting; `OpULessThan` disassembly
cross-check (505→134). Reproduce commands in that file; external corpus
(`webgpu-samples`) re-cloned 2026-09-13 at `references/raw/webgpu-samples`.

## 5. Standing experiment backlog (with expected-result branching summary)

Each item is expanded with full outcome branches in `07-MASTER-PLAN.md`; listed here
for experiment-map completeness:

| ID | Experiment | Answers |
|---|---|---|
| E1 | schema-v3 paired GPU rerun + negative control (matmul + 2–3 kernel shapes) | R2 timing rigor; R1 diversity |
| E2 | Differential execution: run elided vs checked policies on device buffers, diff outputs (incl. boundary values) | the "structural-only testing" gap |
| E3 | Overridable-workgroup-size pinning test | R2 corner case |
| E4 | MSL/HLSL behavioral verification of access-specific proofs | cross-backend claim |
| E5 | Interval-operation fuzz + adversarial WGSL shapes | TODO §4 |
| E6 | Corpus re-run on current branch + external corpora; publish per-shape table | R3 coverage |
| E7 | Go-style inline-annotated expectation tests (`test/prove.go` pattern) | maintainable proof regressions |
| E8 | Fix standalone-naga Track 3 back-edge bug + regression test | hygiene; follow-up paper |

## 6. Evidence gates (from `AUDIT_INVENTORY.md` — binding)

1. Full CPU/static test suite + formatter pass.
2. Differential enabled/disabled codegen with exact structural check classification.
3. Deduplicated corpus records regenerated from the current branch.
4. Correctness-validated, provenance-complete GPU runs with balanced paired samples.
5. Identical-binary negative control CI containing zero.
6. Paper tables/figures/prose regenerated from those artifacts only.
