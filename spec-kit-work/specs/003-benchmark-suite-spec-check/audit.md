# Audit: current benchmarking suite vs spec 003

**Auditor**: Claude (acting reviewer). **Date**: 2026-09-28.
**Scope**: `paired-vulkan-v3` (the only harness whose results are used: runs
`20260913T173908Z`, `20260913T174017Z`). Legacy harnesses (`bounds-check-bench`,
`matmul-paired-bench`, `modal-bench`) are already frozen as INVALID (`../../TEST_BENCHMARK_AUDIT.md`)
and are not re-audited.
**Labels**: VERIFIED = checked this session.

## Verdict

**The suite is statistically careful within a run, but it is not yet a valid basis for a
performance claim.** It fails the most important requirement: it does not measure the binaries
it reports. It also fails the notes' own variance rule, because the effect (~1.2%) sits inside
the 1–2% "natural variance" band with only 2 runs, on 1 kernel. Score: **5 PASS / 4 PARTIAL /
10 FAIL of 19.**

## Critical finding C1 — the reported binaries are not what ran (BR-01, BR-02)

VERIFIED by code reading and replay:

1. The harness builds `enabled.spv` / `disabled.spv` with naga (RZSW policy), then passes them to
   wgpu as `wgpu::ShaderSource::SpirV` (`paired-vulkan-v3/src/main.rs:287-290`).
2. wgpu does **not** pass SPIR-V through. `wgpu/src/backend/wgpu_core.rs:1048-1055` sends it to
   `wgpu-core/src/device/resource.rs:2388-2398`. There it is **re-parsed by naga's SPIR-V
   front end** and **re-validated** with the default validator, where elision is ON
   (`naga/src/valid/mod.rs:623`). The Vulkan backend then **re-emits SPIR-V** with
   `index: Restrict`, `buffer: Unchecked` (if `robustBufferAccess2`) and
   `force_loop_bounding: true` (`wgpu-hal/src/vulkan/adapter.rs` spv::Options).
3. Replay of that pipeline on the run's own artifacts (`runs/20260913T173908Z…/artifacts/*.spv`,
   tool `scratchpad/wgpu-replay`, naga @ `83a90e0`):

| | Recorded in report | Executed (replay) |
|---|---|---|
| BCE on | 422 words, 1 `OpULessThan`, 3 proofs | **566 words**, 1 `OpULessThan`, **1 `UMin` clamp**, 1 proof |
| BCE off | 486 words, 4 `OpULessThan` | **641 words**, 4 `OpULessThan` (explicit RZSW guards kept as ordinary code), **1 `UMin` clamp** |

4. wgpu adds the clamp to `tile[lid] = f32(lid + 1u)` in **both** variants. After the SPIR-V
   round trip the builtin is a global input load, so the spec-guarantee proof does not fire.

**Consequences:**
- The recorded `spv_sha256` / word counts describe intermediates, not executed code (fails SC-003).
- What was measured is "3 explicit RZSW branch-guards (one inside the 256-iteration loop) vs.
  none, on top of a shared clamp". That is close to the intended contrast, but the "elided"
  variant is **not check-free**.
- The measured policy (RZSW branches) is **not** what production wgpu emits on Vulkan (Restrict
  clamps, or nothing for buffers). The 105/111 ns therefore do not measure "what users of wgpu
  gain" (fails BR-02).
- **Fix:** submit the module as `ShaderSource::Naga` with a validator whose elision flag
  differs, so wgpu itself compiles each variant with its production policy. Alternatively,
  capture the SPIR-V wgpu-hal hands to Vulkan and hash that.

## Requirement-by-requirement

| ID | Requirement | Verdict | Evidence (VERIFIED 2026-09-28 unless noted) |
|---|---|---|---|
| BR-01 | Measures intended variable; executed binary recorded | **FAIL** | C1 above |
| BR-02 | Production policy measured | **FAIL** | Only RZSW (`main.rs:14,711`); wgpu Vulkan uses Restrict for arrays |
| BR-03 | Correctness before/after timing | **PASS** | `validate_output` before (`main.rs:587-595`) and after (`672-681`); exact 0.0 error both runs |
| BR-04 | Workload not dominated by other costs | **FAIL** (UNKNOWN share) | One workgroup per dispatch (`dispatch_workgroups(1,1,1)`, `main.rs:364`): 1 of the GPU's SMs busy. ~9.16 µs per dispatch. The effect is 1.15% of that; how much is dispatch/barrier latency is unmeasured |
| BR-05 | Within-run paired stats + ≥5 runs aggregated | **PARTIAL** | Within-run: interleaved AB/BA, 300 rounds, median + bootstrap B=10 000, fixed seeds (`main.rs:441-459`) ✔. Across runs: only **2** runs, not aggregated ✘ |
| BR-06 | Effect vs variance threshold | **FAIL** | Effect = **1.15% (run 1), 1.21% (run 2)** of per-dispatch time (computed from raw samples). Inside the notes' 1–2% band. The two runs' CIs do not overlap ([101.5,106.6] vs [108.4,112.4]); between-run spread 6 ns ≈ 5.7% of the effect |
| BR-07 | Negative control CI ⊇ 0, adjacent | **PASS** | Control CI [−5.45, 5.36] / [−5.16, 5.93]. It runs in the same process just before the headline phase (`main.rs:611-638`) |
| BR-08 | Order effects reported | **FAIL** | Delta by order: run 1 **99.7 vs 107.3 ns** (on-first vs off-first), run 2 107.7 vs 112.0. Not in the report |
| BR-09 | Instrumentation overhead quantified | **PARTIAL** | Timestamps only at pass begin/end, amortised over ~379 dispatches (`dispatches_per_sample`); overhead never measured |
| BR-10 | Warm-up adequacy shown; cold/warm recorded | **PARTIAL** | Steady state achieved in practice (1st vs 2nd half median drift 0.01% / 0.00%). Not tested in the harness. GPU went P3→P0 between before/after snapshots; state during the run unrecorded |
| BR-11 | Full system state | **FAIL** | `machine` has adapter/backend/driver/kernel/os/arch + nvidia-smi before/after only. Missing: CPU model & governor, memory, power source (AC/battery), GPU compute apps, display attachment, background load |
| BR-12 | Background noise minimised + verified | **FAIL** | No check or record of other processes; desktop session present (laptop) |
| BR-13 | Clock/power fixed or continuously recorded | **FAIL** | No clock lock, no continuous sampling. Power telemetry unreliable (590.01 W reading in run 2 before-sample) |
| BR-14 | `meta.json` per run (S3) | **FAIL** | Run dirs contain only `fixed-array-scan.json` + `artifacts/`; no `meta.json`, no `cpu_governor` (violates own standard §3) |
| BR-15 | ≥5 kernel shapes incl. real-world | **FAIL** | One hard-coded shader (`main.rs:20-35`). Multi-kernel sweep (T2 Step 4) never run |
| BR-16 | One variable at a time; toolchain recorded | **PARTIAL** | The BCE flag is the only toggled input ✔. rustc/cargo versions not recorded ✘. The executed difference is still one variable after replay ✔ |
| BR-17 | Cross-platform only with per-platform runs | **PASS** (by abstention) | No cross-platform claim in the ledger (anti-claims list) |
| BR-18 | Written plan before runs | **PASS** | `IICT_CONTEXT/TASKS/T2-GPU-RERUN.md` with outcome branches |
| BR-19 | Raw ordered samples persisted | **PASS** | `paired_samples`/`control_samples` with round + order in JSON |

N/A (CPU-specific notes items): NUMA/CCD layout, memory allocator, huge pages, compiler vendor
flags for C code, SPEC-style 20–30 sub-benchmarks at core count.

## What can be said honestly today (for the poster)

"On one synthetic kernel (1 workgroup, 256-iteration shared-memory scan), RTX 4060 Laptop,
Vulkan, the three ReadZeroSkipWrite-style guards cost 105 / 111 ns per ~9.2 µs dispatch
(1.15% / 1.21%, 2 runs; identical-binary control CI contains 0). This is **not** a measurement
of wgpu's production path, and not a general speed-up claim."

## Remediation (see `tasks.md`)

In priority order:
1. **Fix C1:** submit `ShaderSource::Naga` modules validated with elision on/off, so wgpu
   applies production policies. Record and hash what wgpu-hal passes to Vulkan.
2. Record system state (BR-11/13/14): `meta.json`, CPU governor, AC state, `nvidia-smi` sampled
   continuously, compute-apps list.
3. **≥5 runs × ≥5 kernels** (incl. vello-derived kernels, since vello is where elision fires in
   real code), with order effects and between-run spread reported.
4. Occupancy: dispatch enough workgroups to fill the GPU, plus a latency-bound variant; report both.
