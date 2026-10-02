# Feature Specification: Benchmark harness v4 — real numbers that survive review

**Created**: 2026-09-28 · **Status**: In progress
**Inputs** (all three binding):
1. IICT reviews (`../../IICT_CONTEXT/02-REVIEWS.md`): Experimental Evaluation = 0 from all four;
   R1 "larger, more diverse benchmark suite"; R2 "timing methodology under-specified (sample count,
   variance); exact 0.0 ns reads as within-noise; thin evaluation (one minimal shader + one matmul);
   Restrict interaction undiscussed"; R3 "unclear if the selected patterns cover enough
   optimisation opportunities".
2. Benchmarking notes (`../003-benchmark-suite-spec-check/source-benchmarking-notes.txt`).
3. Audit of v3 (`../003-benchmark-suite-spec-check/audit.md`): 5 PASS / 4 PARTIAL / 10 FAIL.

## Question → answer cycle (Claude as questioner and answerer; each answer cites its source)

| # | Question | Answer | Source |
|---|---|---|---|
| Q1 | How do we guarantee the GPU runs exactly the binary we record? | `create_shader_module_passthrough` with SPIR-V the harness emits; hash those exact words | Audit C1; `wgpu/src/api/device.rs:197`; Vulkan advertises `PASSTHROUGH_SHADERS` (`wgpu-hal/src/vulkan/adapter.rs:674`) |
| Q2 | Which policy do we measure? | wgpu's Vulkan production options: `index: Restrict`, `buffer: Unchecked` (device has robustBufferAccess2), `force_loop_bounding: true`, SPIR-V 1.6 — copied from `wgpu-hal/src/vulkan/adapter.rs` | Audit BR-02; R2 "Restrict interaction" |
| Q3 | What are the configurations? | 4 slots per round: `noBCE` (production today), `BCE` (with elision), `BCE-dup` (identical words, separate pipeline object = in-round negative control), `unchecked` (no checks: upper bound on any possible gain) | Notes §11.2 "one variable"; R2 "0.0 ns within noise" → control in the same rounds; R1 comparison baseline |
| Q4 | How many kernels, which? | 7: empty (overhead floor), shared-memory scan, tiled matmul, tree reduction, Hillis–Steele scan, vello-style bitmap/binning, per-thread private-array loop | R1/R2/R3; notes §9 "no single benchmark is sufficient"; real-world census (vello patterns) |
| Q5 | How do we avoid a latency-only measurement? | Every kernel dispatches enough workgroups to occupy the whole GPU (≥ 1024 workgroups) | Audit BR-04 |
| Q6 | Correctness? | Integer (u32) kernels with exact CPU reference; every config must match exactly before and after timing, else the run is invalid | Notes §10 "measure the intended variable"; v3 legacy files had broken shaders |
| Q7 | Statistics? | Rounds interleave all 4 slots in a rotating order (4 cyclic orders); per-slot medians + paired-difference bootstrap CI; order effect reported; ≥ 5 independent **processes** per kernel; across runs: geometric mean of per-run ratios with min–max | Notes §8/§11.6 (multiple iterations, geomean, 1–2% threshold); audit BR-05/06/08 |
| Q8 | Is an effect real? | Only if (a) in-round control CI ⊇ 0, (b) all 5 runs agree in sign, (c) min–max of per-run effect excludes 0, (d) effect > 2× the between-run spread of the control; otherwise "not resolvable" | Notes §8 tolerance; R2 |
| Q9 | Environment? | `meta.json`: rustc/cargo, CPU model + governor, memory, AC state, GPU compute apps (must be empty), display state, `nvidia-smi` sampled every 200 ms for the whole run | Notes §11.5, "overhead and noise"; audit BR-11–14 |
| Q10 | Instrumentation overhead? | Measured by the empty kernel slot set: per-dispatch floor with identical timing code | Notes §11.8 |
| Q11 | What if BCE does not change a kernel's binary? | Then BCE and noBCE hashes are identical — the kernel is reported as "no proof ⇒ no effect possible" and doubles as a negative control | R3 coverage honesty |

## Requirements
- **FR-1** Executed SPIR-V = recorded SPIR-V (passthrough; sha256 per slot).
- **FR-2** Production Vulkan policy; policy recorded in every result.
- **FR-3** 7 kernels, full occupancy, exact CPU-reference correctness before and after timing.
- **FR-4** 4-slot rotating interleave; ≥ 100 retained rounds; ≥ 30 warm-up rounds with convergence check.
- **FR-5** ≥ 5 independent processes per kernel; aggregate report with geomean, min–max, verdict rule Q8.
- **FR-6** `meta.json` + continuous GPU telemetry per run; refuse to run if other GPU compute apps exist.

## Success criteria
- **SC-1** Every kernel yields a verdict (resolvable effect with numbers, or "not resolvable", or
  "no proof") — no unexplained failures.
- **SC-2** In-round control CI ⊇ 0 in ≥ 95% of runs.
- **SC-3** Report table regenerable from `runs/` artifacts only.
