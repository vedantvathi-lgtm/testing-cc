# Benchmark v4 results

**Date**: 2026-09-28 · **Harness**: `bounds-check-testing-framework/paired-vulkan-v4` (paired-vulkan-4)
**Compiler under test**: naga from `wgpu` branch `bce/sound-branch-refinement` @ `83a90e07` plus
the uncommitted `refine()` soundness fix (recorded per run in `meta.json`); rustc 1.93.1.
**Spec**: `spec-kit-work/specs/004-benchmark-harness-v4/spec.md`. Numbers below are copied from
`runs-v4/summary-nvidia.json` and `runs-v4/summary-intel.json`, which the aggregator
`paired-vulkan-v4/analyze_v4.py` produces from the per-run `result.json` files.

## What changed vs. v3 and why

| v3 problem (audit 003 / reviewers) | v4 answer |
|---|---|
| Timed binaries differ from the recorded ones: wgpu re-compiled `ShaderSource::SpirV` (audit C1) | `create_shader_module_passthrough`: the driver receives exactly the words that are hashed and saved (`artifacts/*.spv`) |
| Measured ReadZeroSkipWrite, not what wgpu uses (R2 "Restrict interaction") | wgpu-hal's Vulkan options: arrays `Restrict`, buffers `Unchecked`, `force_loop_bounding`, native zero-init, SPIR-V 1.6 |
| 1 kernel, 1 workgroup, latency-bound (R1, R2 "thin evaluation", R3 coverage) | 7 kernels at full occupancy (1024–2048 workgroups), modelled on real patterns (vello binning/fine, tiled matmul, reductions, scans) |
| "0.0 ns reads as within noise" (R2) | 4 slots per round: `no_bce`, `bce`, `bce_dup` (identical words, separate pipeline = in-round control), `unchecked` (no checks = upper bound). Where elision removes every clamp, `bce` and `unchecked` are **byte-identical** (same sha256) |
| 2 runs, effect inside 1–2% variance band (notes §8) | 10 independent processes per kernel (NVIDIA), 5 (Intel); kernel order rotated with a per-cycle random offset; geometric mean of per-run ratios + min–max. **CORRECTION 2026-10-01:** this cell previously read "kernel order reshuffled each cycle". That was false. The campaign script used `shuf --random-source=<(yes "$cycle")`, which is a deterministic rotation, not a randomisation. Retrospective audit (`paired-vulkan-v4/audit_order.py` on `runs-v4/campaign-20260928T162710Z.log`) shows 6 of 7 kernels occupied only 3 of 7 possible positions. Consequence measured at +0.0057%/cycle drift, i.e. small for this campaign; the script now uses a CSPRNG-seeded randomised Latin square and verifies balance. |
| ns differences confounded by clock changes | Per-round ratios. Verified: a mid-run clock step (−17% on all slots) left the ratio unchanged (1.158 before and after) |
| No environment record (audit BR-11–14) | `meta.json` per run (CPU, governor, AC, display, load, sources, rustc) + `nvidia-smi` sampled every 200 ms + noise gate (no other GPU compute apps) |
| Correctness assumed | Exact u32 CPU reference; every slot checked before **and** after timing; 105/105 runs exact |

## Results — NVIDIA GeForce RTX 4060 Laptop GPU (Vulkan, driver 580.178.04), 10 runs/kernel

| kernel | proofs with BCE | clamps no_bce → bce | **gain from elision** no_bce/bce | per-run min–max | in-round control max dev | cost of checks BCE could not remove (bce/unchecked) | verdict |
|---|---|---|---|---|---|---|---|
| empty (control) | 0/1 | 0 → 0 | 1.0013 | 0.9997–1.0024 | 0.290% | 1.0003 | no proof (control) |
| scan_shared | 2/4 | 2 → 0 | **1.1579 (+15.79%)** | 1.1578–1.1579 | 0.007% | 1.0000 | **RESOLVED** |
| matmul_tiled | 8/11 | 8 → 0 | **1.1080 (+10.80%)** | 1.1079–1.1081 | 0.005% | 1.0000 | **RESOLVED** |
| vello_binning | 4/9 | 6 → 2 | **1.1388 (+13.88%)** | 1.1376–1.1398 | 0.054% | 1.0108 (+1.08%) | **RESOLVED** |
| reduce_tree | 3/6 | 4 → 1 | 1.0009 | 1.0007–1.0011 | 0.027% | **1.0120 (+1.20%)** | not resolvable (< 0.5% margin) |
| prefix_scan_hs | 4/7 | 5 → 1 | 1.0017 | 1.0010–1.0030 | 0.252% | **1.0071 (+0.71%)** | not resolvable |
| private_array | 4/6 | 4 → 0 | 0.9997 | 0.9991–1.0005 | 0.103% | 0.9997 | not resolvable (driver removes clamps itself) |

## Results — Intel Graphics RPL-S iGPU (Vulkan, Mesa 26.2.2), 5 runs/kernel

| kernel | **gain from elision** no_bce/bce | per-run min–max | control max dev | cost of remaining checks (bce/unchecked) | verdict |
|---|---|---|---|---|---|
| empty (control) | 1.0000 | 0.9999–1.0002 | 0.020% | 1.0000 | no proof (control) |
| scan_shared | **1.1162 (+11.62%)** | 1.1161–1.1163 | 0.006% | 1.0000 | **RESOLVED** |
| matmul_tiled | 1.0000 | 0.9994–1.0003 | 0.012% | 1.0001 | not resolvable (no effect) |
| vello_binning | 0.9999 | 0.9995–1.0002 | 0.016% | 0.9999 | not resolvable (no effect) |
| reduce_tree | 1.0001 | 0.9999–1.0007 | 0.028% | **1.0730 (+7.30%)** | not resolvable (no effect) |
| prefix_scan_hs | 1.0000 | 0.9996–1.0005 | 0.041% | **1.0365 (+3.65%)** | not resolvable (no effect) |
| private_array | 1.0000 | 1.0000–1.0000 | 0.001% | 0.9995 | not resolvable (no effect) |

## What the numbers say

1. **When elision removes a clamp from a hot loop, the gain is large and very reproducible.**
   It is +10.8% to +15.8% on NVIDIA for 3 of 6 kernels, with a per-run spread of at most 0.22
   percentage points and controls within 0.3%. That is well beyond the 1–2% variance band in the
   benchmarking notes.
2. **The gain depends on hardware and driver.** Intel's driver hides the clamp cost for matmul and
   vello binning (0%) but not for the shared-memory scan (+11.6%).
3. **Where elision cannot prove an access, the leftover clamp can cost a lot:** +7.30% (Intel) and
   +1.20% (NVIDIA) on the tree reduction, and +3.65% / +0.71% on the Hillis–Steele scan. These
   are the concrete targets for the next analysis improvements (answers R3 "coverage").
4. **Drivers remove some clamps themselves.** On the per-thread private-array kernel, clamps
   cost nothing on either GPU.
5. **The effect is invariant to clock state.** NVIDIA clocks ranged 1545–2625 MHz across runs
   (P0, up to 87–88 °C, thermally limited laptop), and the scan ratio stayed 1.1578–1.1579.

## Honest limits

- Two GPUs in one laptop; no desktop, AMD or Apple GPU. Metal/D3D12 are not measured.
- Synthetic kernels modelled on real shaders, not the real vello pipeline.
- **Order effect:** when `bce` runs first in a round, the no_bce/bce ratio is up to ~0.8% higher
  (e.g. matmul 1.1080 vs 1.1163 in some runs). The rotating order balances it; per-run order
  splits are in `result.json` (`order_effect_ratio`).
- **Validity margin:** the strict "control CI contains 1" rule fails on the empty kernel (a 0.13%
  bias between identical pipelines is statistically detectable). A ±0.5% practical-equivalence
  margin was declared after that first run and before any kernel result was inspected; strict
  counts are reported (e.g. empty 6/10, scan 9/10).
- **SPIR-V validation:** current `spirv-val` rejects naga's `ArrayStride` on workgroup arrays
  (VUID-10684) in *all* variants alike. The driver accepts them; this does not bias the
  comparison.
- CPU governor `powersave`; GPU clocks not lockable without root. Clock telemetry is recorded
  instead.

---

## Addendum (2026-09-29): after features 005 + 006 and the two soundness fixes — wgpu `19ffb4e`

**Campaign**: `runs-v4/wait-and-campaign-20260929-d.log`. NVIDIA × 10 per kernel (12:40:35Z–12:54:05Z),
Intel × 5 per kernel (12:54:07Z–13:01:56Z). 105/105 runs `quiet_during_run: true`. Every `meta.json`
records `sources.wgpu.rev = 19ffb4ee1a7e…` with an empty naga diff (`e3b0c442…` = sha256 of
empty input), so the provenance gate held. Aggregated with
`analyze_v4.py runs-v4 --rev 19ffb4e` and `analyze_v4.py runs-v4 intel --rev 19ffb4e` →
`runs-v4/summary-nvidia-19ffb4e.json`, `runs-v4/summary-intel-19ffb4e.json`. The earlier tables above
are `summary-{nvidia,intel}-83a90e0.json`. All numbers below are copied by script from those four files.

| GPU | kernel | proofs 83a90e0 → 19ffb4e | clamps no_bce→bce (before / after) | gain from elision no_bce/bce, before → **after** | leftover-check cost bce/unchecked, before → **after** | control max dev (after) | verdict (after) |
|---|---|---|---|---|---|---|---|
| NVIDIA | empty | 0/1 → 0/1 | 0→0 / 0→0 | 1.0013 → **1.0010** | 1.0003 → **1.0004** | 0.233% | no proof (control) |
| NVIDIA | scan_shared | 2/4 → 2/4 | 2→0 / 2→0 | 1.1579 → **1.1579** | 1.0000 → **1.0000** | 0.010% | RESOLVED |
| NVIDIA | matmul_tiled | 8/11 → 8/11 | 8→0 / 8→0 | 1.1080 → **1.1080** | 1.0000 → **1.0000** | 0.008% | RESOLVED |
| NVIDIA | reduce_tree | 3/6 → 4/6 | 4→1 / 4→0 | 1.0009 → **1.0131** | 1.0120 → **1.0001** | 0.055% | **RESOLVED** (was not resolvable) |
| NVIDIA | prefix_scan_hs | 4/7 → 5/7 | 5→1 / 5→0 | 1.0017 → **1.0086** | 1.0071 → **0.9998** | 0.115% | **RESOLVED** (was not resolvable) |
| NVIDIA | vello_binning | 4/9 → 6/9 | 6→2 / 6→0 | 1.1388 → **1.1522** | 1.0108 → **0.9999** | 0.032% | RESOLVED |
| NVIDIA | private_array | 4/6 → 4/6 | 4→0 / 4→0 | 0.9997 → **0.9997** | 0.9997 → **0.9997** | 0.259% | not resolvable (driver removes clamps itself) |
| Intel | empty | 0/1 → 0/1 | 0→0 / 0→0 | 1.0000 → **1.0000** | 1.0000 → **1.0000** | 0.011% | no proof (control) |
| Intel | scan_shared | 2/4 → 2/4 | 2→0 / 2→0 | 1.1162 → **1.1161** | 1.0000 → **0.9999** | 0.084% | RESOLVED |
| Intel | matmul_tiled | 8/11 → 8/11 | 8→0 / 8→0 | 1.0000 → **0.9999** | 1.0001 → **1.0001** | 0.995% ⚠ | not resolvable (see deviation) |
| Intel | reduce_tree | 3/6 → 4/6 | 4→1 / 4→0 | 1.0001 → **1.0733** | 1.0730 → **1.0001** | 0.038% | **RESOLVED** (was not resolvable) |
| Intel | prefix_scan_hs | 4/7 → 5/7 | 5→1 / 5→0 | 1.0000 → **1.0365** | 1.0365 → **0.9999** | 0.068% | **RESOLVED** (was not resolvable) |
| Intel | vello_binning | 4/9 → 6/9 | 6→2 / 6→0 | 0.9999 → **1.0001** | 0.9999 → **0.9999** | 0.037% | not resolvable (no effect on this driver) |
| Intel | private_array | 4/6 → 4/6 | 4→0 / 4→0 | 1.0000 → **1.0001** | 0.9995 → **0.9998** | 0.057% | not resolvable (no effect) |

### What the addendum shows

1. **The analysis now removes every clamp in all six kernels that have dynamic accesses**
   (clamps `no_bce → bce` end at 0 everywhere). The leftover-check cost is 0.9998–1.0004 on every kernel and GPU.
2. **The residual measured on `83a90e0` predicted the gain on `19ffb4e`.** Each pair is the old leftover cost, then the new gain:
   - Intel `reduce_tree`: 1.0730, then 1.0733;
   - Intel `prefix_scan_hs`: 1.0365, then 1.0365;
   - NVIDIA `reduce_tree`: 1.0120, then 1.0131;
   - NVIDIA `prefix_scan_hs`: 1.0071, then 1.0086.

   NVIDIA `vello_binning` rose from 1.1388 to 1.1522, closing its 1.0108 leftover.
   The v4 design had to deliver this internal consistency: the `unchecked` slot bounds what elision can still win, and features 005/006 won it.
3. **Two kernels move from "not resolvable" to RESOLVED on both GPUs:** `reduce_tree` and `prefix_scan_hs`. The largest new gain is **+7.33% on Intel `reduce_tree`**, the running example `s[lid + stride]`.
4. **Unchanged, as expected:** `scan_shared` and `matmul_tiled` were already fully proven. Kernels where the driver hides clamp cost stay at ~1.0: `private_array` on both GPUs, and `matmul_tiled` / `vello_binning` on Intel.

### DEVIATION

`DEVIATION: expected every Intel control within the ±0.5% margin (it was 0.012% max on 83a90e0);
observed matmul_tiled control 1.0100 (CI 1.0004–1.0176) in run 20260929T125417Z-matmul_tiled-intel, the
second Intel run of the campaign, started 12 s after the NVIDIA campaign ended; its order split was
1.0258 (no_bce first) vs 0.9979 (bce first); the other 4 Intel matmul runs have controls ≤ 1.0002 and
effects 0.9996–1.0001; evidence: runs-v4/20260929T125417Z-matmul_tiled-intel/result.json; consequence:
the verdict for Intel matmul_tiled is NOT_RESOLVABLE by rule, but no run shows an elision effect in
either direction, so the substantive conclusion (Intel's driver hides these 8 clamps) is unchanged;
next: add a cool-down/settling period between the NVIDIA and Intel campaigns in wait_idle_then_campaign.sh.`

### Limits unchanged

The limits in "Honest limits" above still apply: two GPUs, synthetic kernels, no Metal or D3D12. These
kernels were also the ones used to *find* the gaps that 005/006 close. The gains are real measurements of real
binaries, but they are not an unbiased sample of real-world shaders. The census (`research_notes/Real world shader
elision gaps/`) is the coverage measure: 118 / 729 real-world runtime accesses proven on `19ffb4e` (recomputed 2026-09-29 from `census/F006_*.jsonl`, excluding `F006_nagatests`; root commit `9a53103`). That is **the same 118 as F005 on `0418863`** (`census/F005_summary.txt`). Feature 006 (relational facts, geometric loops) closed the benchmark kernels' last clamps but proved no additional access in the real-world corpus.
