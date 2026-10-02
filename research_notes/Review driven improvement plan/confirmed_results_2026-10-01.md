# Confirmed results, 2026-10-01 (coordinator, executed this session)

## 1. Soundness: S1, S2, S3 and the override case are CONFIRMED (VERIFIED, executed)

Tool: `IICT_CONTEXT/epoch/tools/bce-epoch-tools` built against `wgpu/naga` @ `19ffb4e` (`ir` mode prints
`safe_accesses`; `spv` mode emits SPIR-V under Restrict with elision off/on). Probe shaders saved in
`research_notes/Review driven improvement plan/probes/`. Every probe contains an access that goes out of bounds
at runtime on some execution; every such access is in `safe_accesses`:

| probe | bug | OOB access proved? | Restrict SPIR-V UMin, elision off → on |
|---|---|---|---|
| s1_let_snapshot.wgsl | S1 stale Load (let snapshot, then store) | yes | 1 → 0 |
| s1_loop_snapshot.wgsl | S1 (loop guard on a snapshot taken before the update) | yes | 1 → 0 |
| s1_postfix.comp (GLSL `if (i++ < 7u) t[i]`) | S1 via GLSL postfix | yes | (GLSL input; not emitted) |
| s2_continue.wgsl | S2 continue edge dropped into `continuing` | yes | 1 → 0 |
| s2_continue_paper.wgsl | S2 (probe from the paper/poster notes) | yes | 1 → 0 |
| s3_initializer.wgsl | S3 in-body initializer rule | yes | 1 → 0 |
| ov_workgroup.wgsl | override workgroup size, pre-override ModuleInfo | yes (`a[l.x]`, array length 1) | not emitted (SPIR-V rejects unresolved overrides); production path re-validates |

Consequence: the elision at `19ffb4e` is **unsound** on these shapes: under the production Restrict policy the
clamp that contains the access is deleted. The ledger's wording "two unsound cases found by audit, both fixed"
is no longer accurate: five bug shapes in three root causes (S1–S3) are open. Branch of the plan that happened
(AGENTS.md Rule 3, analysis notes Q6): "success" for the red tests (they are red at 19ffb4e). Whether any
proof used in the measured benchmarks (v4 kernels, vello's 102 removed clamps) depends on S1–S3 is UNKNOWN until
the fixes land and the production screen / census are re-run.

## 2. Confirmatory real-world campaign (pre-registration 2, harness realworld-2) — VERIFIED

Log `bounds-check-testing-framework/runs-realworld/campaign-20260930T182304Z.log`: NVIDIA ×10 (18:23–20:16Z),
Intel ×5 (20:17–22:22Z). Rule v2 (`realworld-v1/analyze_v2.py`), summaries
`runs-realworld/summary-v2-{nvidia,intel}-realworld-2-19ffb4e.json`:

| GPU | vello suite geomean (control-corrected) | GAIN | LOSS | EQUIVALENT | INVALID |
|---|---|---|---|---|---|
| NVIDIA RTX 4060 Laptop | 1.0059 (+0.59%, 32 scenes) | 22 (range 1.0007–1.0545) | 4 (emoji 0.9987, robust_paths 0.9985, base_color_test 0.9987, image_luminance_mask 0.9941) | 6 | 2 (cardioid_and_friends 2 runs excluded; longpathdash butt 4 excluded — output oracle / vello nondeterminism) |
| Intel RPL-S iGPU | 1.0701 (+7.01%, 32 scenes) | 22 (range 1.0148–1.3452) | 10 (0.9501–0.9940) | 0 | 0 |

imageBlur (contract params): NVIDIA GAIN 1.0012 (CI 1.0006–1.0019); Intel GAIN 1.0034 (1.0033–1.0035).
blur_4k: NVIDIA LOSS 0.9996; Intel GAIN 1.0007. The pre-registered "success" criterion (NVIDIA per-round control
spread down ≥5x) was met in the pilot (15% → 0.6–2.7%, `runs-realworld/_sizing-20260930`), and NVIDIA scenes
are now mostly resolvable (26 of 34 with a GAIN/LOSS/EQUIVALENT verdict, vs ~4 RESOLVED under the old rule).

Deviations:
- `DEVIATION`: Intel `vello:00-splash_with_tiger` and `vello:32-luminance_mask` panicked in all 5 runs with
  `assertion failed: ticks > 0` (zero-length timestamp interval; the two smallest scenes, 200×200 and 55×55 px)
  under the batched harness; they ran fine under realworld-1. No Intel data for them in the confirmatory campaign.
- Slot-order imbalance (evaluation notes): cyclic rotation puts no_bce before bce in 3/4 rounds but bce_dup
  before bce in 1/4, so rule v2's control correction adds a small position effect; sensitivity analysis pending.
- The measured binaries are from `19ffb4e`, which has the S1–S3 unsoundness above. The numbers measure those
  binaries; after the fixes, elision may remove fewer clamps and the benchmark must be re-run.
