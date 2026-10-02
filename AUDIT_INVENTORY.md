# Bounds-check-elision audit inventory

Updated 2026-08-27. This is an evidence index, not a claim that every green test
is a valid experiment.

## Repository state

| Component | State | Interpretation |
|---|---|---|
| `wgpu` | branch `bounds-check-elision`, uncommitted implementation/test changes | Never commit this repository from the agent; preserve user work. |
| `wgpu` history | commits `86eb81c`, `f105ff1`, `500dd82`; eight unreachable checkpoint commits; one trunk stash | Checkpoints are historical evidence and must be reviewed before deleting anything. |
| `corpus_hit_rate` | standalone repository; harness rewrite uncommitted; old v2/v3 outputs present | Old outputs are stale and must not be cited. |
| `bounds-check-testing-framework` | standalone repository; schema standard committed; v3 runner and runs uncommitted | Fresh runs remain pending GPU availability. |

## Verified implementation state

- Access-specific proof membership exists in `FunctionInfo.safe_accesses`.
- The interval domain supports signed/unsigned 32-bit ranges and rejects checked arithmetic overflow.
- Branch facts, joins, local mutation invalidation, monotone recurrence, affine expressions,
  equality exits, and builtin invocation ranges have focused tests.
- SPIR-V, MSL, and HLSL consume access-specific proof handles.
- Full Naga integration tests pass: 265/265 on this audit run.
- The local-array induction case remains unproved; this is a documented coverage gap, not a
  benchmark failure.

## Invalid or incomplete evidence

- Legacy performance JSON: no exact source revisions; 18/20 have n=10; one field mislabels
  SPIR-V words as emitted bounds checks.
- Legacy policy comparisons change `Unchecked`/`ReadZeroSkipWrite`/`Restrict`, so they do not
  isolate the BCE analysis.
- Legacy prefix-scan and `track3b_test` output extents are too small for their writes.
- Legacy matmul has no externally observable output and contains an out-of-bounds retained arm.
- Legacy blocked-order loops are not paired interleaving despite comments claiming otherwise.
- Corrected corpus JSON is now schema-valid, but no full rerun is headline-ready until aggregate
  records are regenerated and checked against enabled/disabled codegen.
- GPU timing is blocked in this environment by Vulkan/NVIDIA adapter initialization failure.

## Required evidence gates before paper claims

1. Full CPU/static test suite and formatter pass.
2. Differential enabled/disabled codegen with exact structural check classification.
3. Deduplicated corpus records and shape totals regenerated from the current branch.
4. Correctness-validated, provenance-complete GPU runs with balanced paired samples.
5. Identical-binary negative control whose confidence interval contains zero.
6. Paper tables, figures, and prose regenerated from those artifacts only.
