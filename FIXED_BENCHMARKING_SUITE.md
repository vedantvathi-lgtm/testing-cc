# Fixed Benchmarking Suite: A Greenfield Redesign

> **Document Version:** 1.0
> **Date:** July 5, 2026
> **Purpose:** Complete teardown and redesign of the benchmarking harness used to evaluate the Naga bounds-check elision (Track 2), WGUL post-barrier elision (Track 3), and Barrier→WGUL pre-barrier elision (Track 3b) optimizations. This document replaces the benchmarking methodology portions of `COMPREHENSIVE_DECISION_LOG.md` §5–§6 and `NAGA_PROBLEMS_AND_IMPROVEMENTS.md` §11.
> **Scope:** Greenfield redesign of the harness, statistics layer, peak-BW measurement, warmup protocol, and multi-run reproducibility. The measurement primitive (wgpu 0.19.4 `ComputePassDescriptor::timestamp_writes` + `TIMESTAMP_QUERY`) is retained because it is the only Web-and-native-portable GPU timestamp mechanism; everything *around* it is rebuilt.
> **References:** Google Benchmark, LLVM `lit`/LNT/IRCE, Triton `triton.testing.do_bench`, NVIDIA `cudaEventRecord` + `nvidia-smi` + Nsight.
> **Hardware Under Test:** NVIDIA GeForce RTX 4060 Laptop GPU (Vulkan, driver 580.159.04, CUDA 13.0).
> **Base Commit:** `gfx-rs/wgpu` commit `3fb225a9c6240bd7e9db3d202410db6d894368ec`.

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Diagnosis: What Was Broken](#2-diagnosis-what-was-broken)
3. [Reference Architectures — The Four Pillars](#3-reference-architectures--the-four-pillars)
4. [New Architecture Overview](#4-new-architecture-overview)
5. [Tier 0: Noise-Floor Calibration (NEW)](#5-tier-0-noise-floor-calibration-new)
6. [Tiers 1 & 2: Static IR + Soundness (Unchanged)](#6-tiers-1--2-static-ir--soundness-unchanged)
7. [Tier 3: Device-Timing Protocol (Overhauled)](#7-tier-3-device-timing-protocol-overhauled)
8. [Tier 4: Macro-Workload Validation (Overhauled)](#8-tier-4-macro-workload-validation-overhauled)
9. [GPU Power & Clock Discipline (NEW)](#9-gpu-power--clock-discipline-new)
10. [Statistical Layer (Overhauled)](#10-statistical-layer-overhauled)
11. [Per-Section Reference Mapping](#11-per-section-reference-mapping)
12. [Migration Plan — Phased Rollout](#12-migration-plan--phased-rollout)
13. [Validation Matrix](#13-validation-matrix)
14. [Rust Pseudocode for Critical Components](#14-rust-pseudocode-for-critical-components)
15. [JSON Schema v2 (Results File)](#15-json-schema-v2-results-file)
16. [CI Integration](#16-ci-integration)
17. [Risks, Honest Unknowns, Out-of-Scope](#17-risks-honest-unknowns-out-of-scope)
18. [References](#18-references)

---

## 1. Executive Summary

The existing benchmarking harness, as documented in `COMPREHENSIVE_DECISION_LOG.md` §5–§6 and `NAGA_PROBLEMS_AND_IMPROVEMENTS.md` §11, produces numbers that are individually defensible but collectively inadequate for the claims the project makes. The Track 3 wall-clock impact was below the noise floor; the peak-BW measurement varies by ±50% across runs; every Track 2/3/3b ON-vs-OFF comparison is a single run pair with no inter-run reproducibility data; and the statistics layer reports only median + 95% bootstrap CI — insufficient to either assert or refute a 3–6% delta at any meaningful confidence level. These are not nitpicks; they are the difference between a paper that survives review and one that does not.

This document specifies a greenfield redesign that fixes all six must-fix problems identified in the requirements-gathering round: (1) peak-BW noise, (2) Track 3 noise floor, (3) multi-run protocol, (4) clock lock + warmup stability, (5) noise-floor calibration, (6) statistical layer overhaul. The redesign is greenfield in the sense that the harness architecture, statistics layer, and reproducibility protocol are rebuilt from scratch — but the measurement primitive (wgpu `TIMESTAMP_QUERY` via `ComputePassDescriptor::timestamp_writes`) is deliberately retained. It is the only GPU timestamp mechanism that works in both Web and native wgpu, which means the measurement code path is identical to what a browser executes. Moving to `cudaEventRecord` would produce a more accurate native-only number at the cost of measuring a different code path than the one WebGPU users actually run. That trade is wrong for this project.

The four references each contribute one piece of discipline that the existing harness lacks. **Google Benchmark** contributes the repetition and aggregation model: `--benchmark_repetitions=N`, `--benchmark_report_aggregates_only=true`, and the explicit statistics enum (mean, median, stddev, cv, mean_dev, min, max, user_percentile) that every benchmark emits by default. **LLVM** contributes the lit/FileCheck tiering philosophy (already partially adopted in Tiers 1 and 2) and the LNT historical-regression-comparison discipline: a delta is only meaningful if it exceeds the run-to-run variance of the *same configuration across multiple nights*. **Triton `do_bench`** contributes adaptive iteration count (pick `n_repeat` so total runtime ≈ 1s, not a fixed 10), CUDA-event-style timing (we keep the wgpu equivalent), and `return_mode='all'` — every measurement emits the full distribution, not just one summary statistic. **NVIDIA** contributes the GPU clock-locking and warmup-stability contract: `nvidia-smi -lgc` to pin the graphics clock, idle detection before run start, and the explicit criterion that warmup is complete only when three consecutive measurement windows have CV < 5%.

The result is a harness where every reported number is accompanied by (a) a noise-floor reading taken in the same run, (b) a full per-pass distribution, (c) a hypothesis test against the comparison baseline with effect size, (d) inter-run CV across at least 5 independent runs, and (e) a documented clock state at measurement time. Track 3's 8-word / 2.9–6.1% delta either becomes statistically resolvable or honestly does not, and the paper reports whichever way it goes — but the methodology is no longer the limiting factor.

---

## 2. Diagnosis: What Was Broken

The six must-fix problems, with severity, root cause, and the section that addresses each.

| # | Problem | Severity | Root Cause | Fixed In |
|---|---------|----------|------------|----------|
| 1 | **Peak-BW measurement varies ±50% across runs** (6.91–21.84 GB/s) | High | Uses host-timed `copy_buffer_to_buffer` with `std::time::Instant`, mixing host submission overhead with GPU transfer time, single-shot (not sustained) | §5.4, §14.2 |
| 2 | **Track 3 wall-clock delta below noise floor** (8 words / 2 barriers, +0.76% to +3.41% — directionally wrong) | High | 10 timed passes is statistically insufficient at the µs scale; bootstrap CI on n=10 is wider than the effect | §7.2, §10 |
| 3 | **Single-run ON/OFF pairs** — every Track 2/3/3b comparison is one run pair | High | No multi-run protocol; inter-run CV never characterized | §8.1, §8.2 |
| 4 | **No GPU clock discipline** — 5 fixed warmup passes, no clock lock, no idle detection, no stability criterion | Medium-High | Assumed warmup amortizes power-state transients; on laptop GPUs with dynamic boost this is false | §9 |
| 5 | **No noise-floor calibration** — no idle/no-op kernel to establish the minimum resolvable delta | Medium | Harness jumps straight to kernel timing without a baseline measurement of the timer itself | §5.1, §5.2 |
| 6 | **Statistics layer reports only median + 95% bootstrap CI** — no min/p50/p95/p99, no hypothesis test, no effect size | High | Bootstrap-on-median of n=10 produces a CI that is too wide to resolve 3–6% deltas but too narrow to honestly cover the heavy tail of GPU timing | §10 |

Two additional problems are documented in the existing docs but explicitly out of scope for this redesign: the WGSL float-literal-width issue (Problem 2 in `NAGA_PROBLEMS_AND_IMPROVEMENTS.md`) and the helper-inlining limitation (Problem 9). Both affect *which optimizations fire*, not *how the resulting kernels are measured*, and are correctly deferred to the naga-side fix track.

A seventh implicit problem — that the peak-BW measurement was the *only* host-timed path in an otherwise GPU-timed harness — is folded into Problem 1. The harness's own documentation (`COMPREHENSIVE_DECISION_LOG.md` §5.4) correctly identifies host timing as "the single most common GPU-benchmarking mistake" and then immediately makes that mistake for peak-BW. This is the most embarrassing bug in the suite and the first one fixed.

---

## 3. Reference Architectures — The Four Pillars

Each reference contributes one piece of discipline. None is adopted wholesale — wgpu and Rust impose their own constraints, and the Web-portability requirement rules out anything that depends on native-only APIs. The mapping is per-section in §11.

### 3.1 Google Benchmark — Repetition & Aggregation Discipline

Google Benchmark (https://github.com/google/benchmark) is the de facto C++ microbenchmark library, used by LLVM, Abseil, Protobuf, and most performance-sensitive C++ codebases. Its discipline is in three pieces.

**Auto-calibrated iteration count.** Each benchmark runs an auto-calibration phase: the library runs the benchmark for a small number of iterations, measures wall time, then scales the iteration count so that each subsequent measurement takes approximately `--benchmark_min_time` (default 0.5s). This eliminates the "is 10 passes enough?" question — the library picks the answer per-benchmark. The existing harness's fixed `TIMED_PASSES = 10` is the wrong shape; for a 2.5 µs kernel, 10 passes is too few, and for a 7.7 ms GEMM kernel, 10 passes is fine but wasteful.

**Mandatory aggregation.** Every benchmark emits mean, median, stddev, coefficient of variation (cv), mean absolute deviation, min, and max. The `--benchmark_report_aggregates_only=true` flag emits *only* the aggregates (no per-repetition raw data), which is the right shape for CI. The existing harness emits only median + bootstrap CI; Google Benchmark's statistic set is the minimum a reviewer expects.

**Repetition as a first-class concept.** `--benchmark_repetitions=N` runs the entire benchmark N times and emits per-repetition aggregates plus an overall mean-of-means. This is the multi-run protocol the existing harness lacks, expressed as a CLI flag rather than an afterthought.

**Manual timing.** `state.SetIterationTime(seconds)` lets the benchmark author provide the timing instead of the library measuring wall clock. This is exactly the pattern we need: wgpu's `TIMESTAMP_QUERY` is a manual-timing source, and Google Benchmark's manual-timing API is the model for how to integrate it.

### 3.2 LLVM — Tiering, IRCE, and LNT

LLVM contributes three things, at three different levels.

**lit + FileCheck for tiered testing.** LLVM's Integrated Tester (`lit`) runs test suites organized by directory; each test is a `.ll` or `.c` file with `RUN:` lines and `CHECK:` lines (handled by `FileCheck`). The tier structure (Unit → Functional → Benchmark → Nightly) is the model for our Tier 1–4 structure. The existing harness already adopts this for Tier 1 (snapshot tests) and Tier 2 (soundness assertions); we keep it.

**IRCE as the comparison point for Track 2.** LLVM's Inductive Range Check Elimination pass (Hal Finkel, LLVM Dev Meeting 2015) is the industry-standard reference for "eliminate bounds checks when you can prove the index is in range." The existing paper positions Track 2 against IRCE; the new harness must support a direct apples-to-apples comparison by being able to benchmark the *same* kernel under (a) naga with Track 2, (b) naga without Track 2, and (c) the equivalent SYCL kernel compiled with Clang + LLVM with IRCE on/off. The harness's WGSL kernel set should be accompanied by an isomorphic SYCL kernel set; this is future work but the harness must not preclude it.

**LNT for historical regression detection.** LLVM's Nightly Test infrastructure (https://llvm.org/docs/lnt/) runs benchmarks nightly, stores results in a SQL database, and reports regressions by comparing each night's run against a rolling baseline with a statistical threshold. The existing harness has no historical-tracking layer; every run is a one-shot JSON file. We adopt LNT's discipline (rolling baseline, statistical threshold, regression report) in §8.3 and §16, without adopting LNT itself — the JSON v2 schema in §15 is structured to feed into LNT or any equivalent tool.

### 3.3 Triton `do_bench` — Adaptive Iteration & Full Distribution

Triton (https://github.com/triton-lang/triton) ships `triton.testing.do_bench`, which is the most-cited GPU microbenchmarking helper in the ML-systems literature. Its discipline is in four pieces.

**CUDA events, not host timing.** `do_bench` uses `torch.cuda.Event(enable_timing=True)` and `cudaEventRecord` / `cudaEventSynchronize`. We cannot use CUDA events directly (wgpu doesn't expose them), but `ComputePassDescriptor::timestamp_writes` is the moral equivalent and is what we already use — Triton validates the choice.

**Adaptive repetition count.** `do_bench` first calls the function once, measures wall-clock, then picks `n_repeat` so the total runtime is approximately 1 second. This is the same idea as Google Benchmark's auto-calibration, expressed more aggressively (1s, not 0.5s). The existing harness's fixed 10 passes is replaced by an adaptive loop in §7.2.

**`return_mode='all'`.** By default `do_bench` returns the median, but `return_mode='all'` returns the full per-iteration timing array, and `quantiles=[0.2, 0.5, 0.8]` returns the median plus tail percentiles. The existing harness returns only the median + bootstrap CI; we adopt `return_mode='all'` as the *only* output mode in JSON v2 (§15) — downstream code computes whatever summary it wants.

**CUDA Graph capture for launch-overhead elimination.** `do_bench_cudagraph` captures the kernel launch into a CUDA graph and replays the graph, eliminating per-launch host overhead. We cannot do this in wgpu (no equivalent of `cudaGraphCapture`), but the *discipline* — measure with and without launch overhead, report both — is adopted in §5.2 (no-op kernel) and §7.3 (per-pass resolve protocol).

### 3.4 NVIDIA — Clock Discipline, Warmup Stability, Ground-Truth Cross-Check

NVIDIA's documentation (CUDA Best Practices Guide, Nsight Compute docs, and the `nvidia-smi` man page) is the canonical reference for GPU measurement discipline. Four pieces.

**`nvidia-smi -lgc` for clock locking.** `nvidia-smi -lgc <graphics_clock>,<graphics_clock>` pins the graphics clock to a fixed frequency, eliminating dynamic-boost noise. On the RTX 4060 Laptop GPU, the graphics clock can vary from 300 MHz (idle) to 2.1 GHz (boost), a 7× range that swamps any 3–6% compiler-optimization delta. The existing harness does not lock the clock; this is the single biggest source of inter-run variance after the host-timed peak-BW bug.

**Warmup-stability criterion.** NVIDIA's Nsight documentation recommends warmup until "three consecutive measurement windows have CV < 5%." The existing harness's fixed 5 warmup passes is a heuristic, not a criterion — on a cold GPU, 5 passes is insufficient; on a warm GPU, 5 passes is wasted time. We adopt the CV-threshold criterion in §9.2.

**Idle detection.** Before starting a measurement run, the harness waits for `nvidia-smi --query-gpu=utilization.gpu` to report < 5% for 500 ms, ensuring no leftover work from a previous benchmark is still draining. The existing harness has no such check.

**`cudaEventRecord` as ground-truth cross-check.** On NVIDIA hardware, we add an optional native CUDA path (out of scope for v1 of the redesign, but the harness architecture permits it) that runs the *same* SPIR-V kernels via `cuLaunchKernel` with `cudaEventRecord` timing, and compares against the wgpu `TIMESTAMP_QUERY` timing. If the two disagree by more than 5% on a sustained workload, the wgpu path has a measurement bug. This is the "add CUDA path" option from the requirements-gathering round, deferred to future work but not precluded by the architecture.

---

## 4. New Architecture Overview

The new harness is organized into five tiers (0–4). Tiers 1 and 2 are unchanged from the existing design; Tiers 0, 3, and 4 are new or overhauled.

```
                ┌─────────────────────────────────────────────────────┐
                │                  CI / Nightly Runner                  │
                │  (GitHub Actions self-hosted, RTX 4060, §16)          │
                └──────────────┬──────────────────────────┬─────────────┘
                               │                          │
                               ▼                          ▼
                ┌──────────────────────────┐  ┌─────────────────────────┐
                │  Tier 0: Noise Floor     │  │  Tier 4: Macro Workload │
                │  (NEW, §5)               │  │  (Overhauled, §8)       │
                │  - Idle kernel           │  │  - k≥5 independent runs │
                │  - No-op kernel          │  │  - Inter-run CV report  │
                │  - Sustained copy kernel │  │  - Significance gate    │
                │  - Peak-BW (GPU-timed)   │  │    (MWU + Cliff's δ)    │
                └──────────────┬───────────┘  └────────────┬────────────┘
                               │                           │
                               ▼                           ▼
                ┌──────────────────────────────────────────────────────┐
                │  Tier 3: Device-Timing Protocol (Overhauled, §7)     │
                │  - wgpu TIMESTAMP_QUERY (kept)                       │
                │  - Adaptive iteration count (Triton-style)           │
                │  - Per-pass resolve + map protocol                   │
                │  - Full distribution output (return_mode='all')      │
                │  - Hypothesis tests vs. baseline (MWU + Welch)       │
                └──────────────────────────┬───────────────────────────┘
                                           │
                                           ▼
                ┌──────────────────────────────────────────────────────┐
                │  GPU Power & Clock Discipline (NEW, §9)              │
                │  - nvidia-smi -lgc clock lock                        │
                │  - Idle detection (util < 5% for 500ms)              │
                │  - Warmup-stability criterion (CV < 5%, 3 windows)   │
                │  - Thermal/throttle monitor (background thread)      │
                └──────────────────────────┬───────────────────────────┘
                                           │
                                           ▼
                ┌──────────────────────────────────────────────────────┐
                │  Tiers 1 & 2: Static IR + Soundness (Unchanged, §6)  │
                │  - Naga snapshot tests (Tier 1)                      │
                │  - elision_assertions.rs (Tier 2)                    │
                └──────────────────────────────────────────────────────┘
```

**Data flow.** Each benchmark run produces one JSON v2 file (§15) containing: hardware fingerprint, clock state at start and end, noise-floor readings (Tier 0), per-pass timings for every (kernel, policy, size, variant) configuration (Tier 3), and inter-run aggregates (Tier 4). The CI runner (§16) compares each night's JSON against the rolling baseline using the statistical layer in §10, and reports regressions.

**What is kept.** wgpu 0.19.4 + `features = ["spirv"]`, naga 0.14.2, `ComputePassDescriptor::timestamp_writes`, `get_timestamp_period()` multiplication, the resolve-buffer + destination-buffer two-step, reusable `QuerySet` across passes, the three kernels (reduction, prefix_scan, tiled_matmul), the three policies (Unchecked, ReadZeroSkipWrite, Restrict), the LOCAL vs STORAGE variant distinction, the 2D dispatch for >65k workgroups, and the WGSL test shaders from §7 of the existing decision log.

**What is rebuilt.** The entire main loop, the statistics module, the peak-BW measurement, the warmup protocol, the multi-run orchestrator, the JSON output schema, and the CI integration. The Rust crate `bounds-check-bench` grows from ~450 lines to an estimated ~1,500 lines, split into modules: `harness/tier0.rs`, `harness/tier3.rs`, `harness/tier4.rs`, `harness/clock.rs`, `harness/stats.rs`, `harness/json_v2.rs`, `harness/main.rs`.

---

## 5. Tier 0: Noise-Floor Calibration (NEW)

Tier 0 is new. It exists because every reported delta in Tiers 3 and 4 must be compared against a measurement noise floor established in the same run, on the same hardware, under the same clock state. The existing harness has no such floor; this is why the Track 3 result ("deltas within bootstrap CI range") is uninterpretable — we cannot tell whether the CI is wide because the kernel is noisy or because the *timer* is noisy.

Tier 0 runs three kernels, in this order, before any Tier 3 measurement begins.

### 5.1 Idle Kernel — Timer Floor

**What it measures.** The minimum non-zero interval the `TIMESTAMP_QUERY` mechanism can resolve on this device. This is the timestamp-query equivalent of `cudaEventElapsedTime` on two events recorded with no intervening work, which on NVIDIA hardware typically returns ~1–5 µs (the cost of the event-recording itself, not zero).

**How it works.** A compute pass with `timestamp_writes` set but **zero dispatches** inside. The pass opens, the beginning-of-pass timestamp is written, the pass immediately closes, the end-of-pass timestamp is written. The elapsed time is the timer floor.

**Protocol.** 100 idle passes, each with its own `ComputePassTimestampWrites`. Take the median. This number is the `timer_floor_us` field in JSON v2 (§15). Any Tier 3 delta smaller than `2 × timer_floor_us` is reported as "below timer resolution" and excluded from significance testing.

**Why 100 passes.** The idle kernel is fast (~µs), so 100 passes takes <1 ms of GPU time. There is no reason to skimp. Triton's `do_bench` runs warmup + ~100 measurement iterations for similar reasons.

### 5.2 No-Op Kernel — Launch-Overhead Floor

**What it measures.** The cost of dispatching one empty compute shader, plus the pass-setup overhead. This is the launch-overhead floor — any kernel that takes less than this is dominated by launch overhead, not by the work it does.

**How it works.** A WGSL kernel with an empty `main()` body, `@workgroup_size(1)`, dispatched once per pass. The pass has `timestamp_writes` as usual. 100 passes, full distribution recorded.

```wgsl
// noop.wgsl — Tier 0 launch-overhead floor
@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) _gid: vec3<u32>) {
    // Intentionally empty. The compiler must not eliminate the entry point;
    // we keep a side-effecting store to a scratch buffer to prevent DCE.
    scratch[0u] = scratch[0u] + 0u;
}
```

**Why the side-effecting store.** An empty `main()` is dead-code-eliminated by naga and the SPIR-V becomes a no-op entry point, which the driver may further optimize into a no-op dispatch. The `scratch[0u] + 0u` store is a no-op semantically but prevents DCE, ensuring we measure the real dispatch cost. This is the same anti-DCE discipline the existing harness uses for the Tier 3 kernels (`output[0] = tile[lid]` etc.).

**Output.** `launch_overhead_us_median`, `launch_overhead_us_p95`, `launch_overhead_us_p99` in JSON v2. Any Tier 3 kernel whose median is below `5 × launch_overhead_us_median` is flagged as "launch-overhead-dominated" in the report.

### 5.3 Sustained Copy Kernel — Memory-Bandwidth Sanity Check

**What it measures.** The sustained device-to-device copy bandwidth, GPU-timed. This is the **replacement** for the broken host-timed `copy_buffer_to_buffer` peak-BW measurement. It is not a peak-BW measurement in the roofline sense (that would require a kernel specifically tuned to saturate the memory subsystem, which is vendor-specific) — it is a *sustained-copy* measurement that gives a stable, GPU-timed normalization constant.

**Why this fixes the ±50% variance.** The existing peak-BW measurement varies 6.91–21.84 GB/s because it host-times a single 64 MB `copy_buffer_to_buffer` call. The host time includes: command-buffer recording, queue submission, driver bookkeeping, and the actual GPU copy. The GPU copy is ~5 ms on the RTX 4060 Laptop (64 MB / ~14 GB/s); the host overhead is 1–10 ms depending on system load. The host overhead dominates and varies, swamping the signal. A GPU-timed sustained copy eliminates the host overhead entirely.

**How it works.** A WGSL kernel that copies a 64 MB buffer to another 64 MB buffer, 256 bytes per workgroup (64 threads × 4 bytes), and is dispatched enough times to take ~50 ms of GPU time. The pass has `timestamp_writes`; the elapsed time divided by the total bytes copied gives sustained copy bandwidth.

```wgsl
// sustained_copy.wgsl — Tier 0 GPU-timed bandwidth floor
@group(0) @binding(0) var<storage> src: array<u32>;
@group(0) @binding(1) var<storage, read_write> dst: array<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i < arrayLength(&src)) {
        dst[i] = src[i] + 0u;  // +0u prevents copy-elision optimization
    }
}
```

**Protocol.** 10 passes, each dispatching the kernel enough times to copy 64 MB × 100 = 6.4 GB total per pass (so each pass takes ~50 ms at ~14 GB/s, well above the launch-overhead floor). Take the median bandwidth across the 10 passes. This number is `sustained_copy_bw_gbps` in JSON v2, with `sustained_copy_bw_gbps_p95` and `sustained_copy_bw_gbps_p99` for tail behavior.

**Why 10 passes, not 100.** Each pass is ~50 ms, so 10 passes is 500 ms of GPU time. 100 passes would be 5 seconds, which is excessive for a Tier 0 calibration. The bandwidth measurement is much more stable than the host-timed version (no host overhead), so 10 passes is sufficient.

### 5.4 Why This Replaces the Host-Timed Peak-BW

The existing `measure_peak_bandwidth` function (§6.6 of the decision log) is the explicit failure mode the project itself complains about. The fix is not to make the host-timed version less noisy — it is to use the GPU timer for the same reason we use it everywhere else. The sustained-copy kernel is intentionally not a tuned roofline kernel; it is a *stable* kernel. Its purpose is to provide a normalization constant that varies <5% across runs, not to match the theoretical peak. The theoretical peak is a property of the hardware (RTX 4060 Laptop LPDDR5 ~128-bit bus ≈ 224 GB/s peak), not something the harness needs to measure.

The relationship between the old and new numbers: the old `peak_bandwidth` (median ~14.3 GB/s, range 6.91–21.84) should be compared against `sustained_copy_bw_gbps` in the new harness. We expect the new number to be in a similar range (10–18 GB/s) but with inter-run CV < 5%, versus the old ±50%. If the new number is much lower than 10 GB/s, the kernel is launch-overhead-bound; if it is much higher than 18 GB/s, the GPU is in a high-clock state and the clock lock (§9) is not working.

### 5.5 Reporting

Tier 0 results are emitted at the top of every JSON v2 file, before any Tier 3 data. They are also printed to stderr at run start so the operator can sanity-check before the long Tier 3/4 runs begin. Example:

```
[Tier 0] timer_floor_us       = 1.832   (median, n=100)
[Tier 0] launch_overhead_us   = 4.711   (median, n=100)  p95=5.203  p99=6.012
[Tier 0] sustained_copy_gbps  = 14.21   (median, n=10)   p95=14.38  p99=14.52
[Tier 0] inter-run CV (copy)  = 2.1%    (target <5%)
[Tier 0] clock_lock           = locked @ 2100 MHz graphics, 2501 MHz memory
[Tier 0] idle_detect          = OK (util <5% for 612ms before run start)
```

If `inter-run CV (copy)` exceeds 5%, the run is flagged `degraded` and the operator is told to investigate the clock lock (§9) before proceeding. Tier 3 results from a `degraded` run are still recorded but flagged in the JSON.

---

## 6. Tiers 1 & 2: Static IR + Soundness (Unchanged)

Tiers 1 and 2 are unchanged from the existing design (`COMPREHENSIVE_DECISION_LOG.md` §5.2 and §5.3). They are restated here for completeness; the file references and `cargo test` invocations are identical.

### 6.1 Tier 1: Static IR Verification (Naga Snapshot Tests)

WGSL fixtures in `naga/tests/in/wgsl/bounds-check-elision-{zero,restrict}.wgsl`, with paired `.toml` files controlling `BoundsCheckPolicy`. Snapshot output in `naga/tests/out/spv/*.spvasm`. Regression detection via `git diff --exit-code HEAD naga/tests/out`. No changes from the existing harness.

### 6.2 Tier 2: Soundness Table (Explicit Rust Assertions)

`naga/tests/naga/elision_assertions.rs` — explicit, named Rust assertions about the SPIR-V produced for each soundness-table case. Two tests: `elision_soundness_table_read_zero_skip_write` (all 6 cases, asserts pre-patch truth: all checks present) and `happy_path_restrict_baseline_uses_umin_clamp_not_branch`. Wired into `naga/tests/naga/main.rs` via `mod elision_assertions;`. No changes from the existing harness.

### 6.3 Why Tiers 1 and 2 Are Not Touched

The bugs in the existing suite are all in the *measurement* layer (Tiers 3 and 4) and the *calibration* layer (Tier 0, which did not exist). Tiers 1 and 2 are *static* — they inspect SPIR-V text, not GPU timing — and are correct as written. The one open item from the existing docs (Tier 2 assertions "never run against the patch", §10.6 item 3 of the decision log) is a naga-side implementation gap, not a harness gap, and is correctly tracked there.

---

## 7. Tier 3: Device-Timing Protocol (Overhauled)

Tier 3 is the core of the redesign. The measurement primitive is unchanged (`ComputePassDescriptor::timestamp_writes`); everything around it — iteration count, per-pass protocol, statistical aggregation, and reporting — is rebuilt.

### 7.1 Measurement Primitive (Kept)

The wgpu `TIMESTAMP_QUERY` mechanism via `ComputePassDescriptor::timestamp_writes` is retained, for the reasons stated in §5.4 of the existing decision log: it is the only timestamp mechanism available on both Web and native, so the measurement code path is identical to what a browser executes. The two API details verified against wgpu source (`get_timestamp_period()` must be queried and multiplied; the resolve buffer cannot be `MAP_READ` so a destination buffer + `copy_buffer_to_buffer` is required) are kept as-is.

The reusable-`QuerySet` optimization (create once per configuration, reuse across passes via `resolve()` + `read_back()`) is also kept — this was bug fix #4 in the existing harness and is correct.

### 7.2 Adaptive Iteration Count (Triton-Style)

**The bug.** The existing harness fixes `TIMED_PASSES = 10` for every configuration. For a 2.5 µs kernel (track3b_test 64K), 10 passes is too few — the bootstrap CI on n=10 is wider than the 3–6% delta we are trying to resolve. For a 7.7 ms kernel (tiled_matmul 16M), 10 passes is fine but the per-pass duration is so long that 10 passes is 77 ms of GPU time, which is more than needed.

**The fix.** Adopt Triton's adaptive-iteration discipline: run a single warmup pass, measure its GPU time, then pick `n_passes` so the total timed GPU time is approximately 1 second (Triton's default target). This means:

- For a 2.5 µs kernel: `n_passes = 1_000_000 / 2.5 ≈ 400,000` passes. Capped at 100,000 to keep total runtime under 1 second of GPU time + ~50 ms of resolve overhead per pass (so ~5 seconds total wall time).
- For a 7.7 ms kernel: `n_passes = 1_000_000 / 7.7 ≈ 130` passes.

**Why the 100,000 cap.** The resolve-buffer + `copy_buffer_to_buffer` + `map_async` round-trip takes ~50 µs of host time per pass on the RTX 4060. At 100,000 passes that is 5 seconds of host overhead; beyond that, the host overhead dominates the wall-clock of the benchmark run. The cap is a pragmatic limit, not a theoretical one.

**Why 1 second target.** Triton uses 1s; Google Benchmark uses 0.5s. We adopt Triton's 1s because GPU timing noise decreases as `1/√n`, and at µs-scale kernels 1s of GPU time gives n=400,000 samples — more than enough for a tight p99 CI. For ms-scale kernels 1s gives n=130–1000 samples, which is still a 10–100× improvement over the existing n=10.

**Pseudocode.**

```rust
fn adaptive_n_passes(warmup_pass_us: f64) -> u32 {
    const TARGET_TOTAL_US: f64 = 1_000_000.0;  // 1 second
    const MAX_PASSES: u32 = 100_000;
    let n = (TARGET_TOTAL_US / warmup_pass_us) as u32;
    n.clamp(50, MAX_PASSES)  // floor at 50 so ms-scale kernels still get stats
}
```

**What this changes about the existing data.** Re-running Track 3b's `track3b_test` 64K configuration with the new protocol gives ~400,000 passes instead of 10. The bootstrap CI on the median shrinks by a factor of ~`√(400000/10) ≈ 200`. The 2.85% delta (2.520 vs 2.594 µs) that was previously "within CI" should now be resolvable at p < 0.001 — *if* the underlying distribution is stable. If the clock is not locked (§9), the distribution is not stable and no amount of sampling will help; this is why §9 is a prerequisite for §7.2.

### 7.3 Per-Pass Resolve Protocol

The existing harness's per-pass protocol is: begin pass with `timestamp_writes`, dispatch N times, end pass (which writes the end timestamp), `resolve_query_set` into the resolve buffer, `copy_buffer_to_buffer` to the destination buffer, `map_async` + `device.poll(Maintain::Wait)`, read two u64s, multiply by `timestamp_period`. This is correct and is kept.

The new harness adds three things:

1. **Per-pass `device.poll(Maintain::Wait)` after `queue.submit`.** The existing harness does this implicitly via `map_async`; the new harness makes it explicit and adds a timeout (10× the expected pass duration, erroring out if exceeded — this catches the "15-minute timeout" bug from §10.4 of the decision log at the pass level rather than the run level).

2. **Per-pass timestamp-period re-query.** `get_timestamp_period()` is documented as backend- and device-dependent. On a laptop GPU with dynamic clock scaling, the period *can* change mid-run if the clock changes. The new harness re-queries the period every 100 passes and uses the most recent value. If the period changes by more than 1% during a run, the run is flagged `period_drift` in JSON v2.

3. **Per-pass GPU clock sample.** A background thread (§9.4) samples `nvidia-smi --query-gpu=clocks.current.graphics` every 500 ms and logs the samples. Each pass's timing record includes the clock sample closest to its start time, so post-hoc analysis can correlate timing outliers with clock drops.

### 7.4 Statistical Aggregation Layer (Full Distribution)

The existing harness emits median + 95% bootstrap CI. The new harness emits the full per-pass distribution plus the summary statistics listed in §10. The per-pass timings are kept in the JSON v2 file (§15) as a `passes: [...]` array, so downstream analysis can compute any statistic without re-running the benchmark.

For each (kernel, policy, size, variant) configuration, the harness computes:

- `min`, `p50` (median), `p95`, `p99`, `max` (the five-number summary plus min/max)
- `mean`, `stddev`, `cv` (coefficient of variation = stddev/mean)
- `n_passes` (the actual pass count, post-adaptive)
- `n_passes_discarded` (warmup passes, see §9.2)

For comparison configurations (Track 2 ON vs OFF, Track 3 ON vs OFF, etc.), the harness additionally computes:

- `delta_pct` = `(treatment_median - control_median) / control_median × 100`
- `welch_t`, `welch_p` (Welch's t-test, unequal variances)
- `mwu_u`, `mwu_p` (Mann-Whitney U test, non-parametric)
- `cliffs_delta`, `cliffs_delta_interpretation` (effect size: negligible / small / medium / large)
- `bh_adjusted_p` (Benjamini-Hochberg FDR correction, applied across all comparisons in the run)

The statistical layer is specified in detail in §10.

### 7.5 Inter-Run Reproducibility (k ≥ 5)

The existing harness runs each configuration once. The new harness runs each configuration `k` times, with `k ≥ 5` by default (configurable via `--runs`). Each run is a complete Tier 3 pass: warmup, adaptive iteration, full distribution. The k runs are reported both individually and as an aggregate (`inter_run_median`, `inter_run_cv`).

**Why k=5.** Five runs is the minimum to estimate inter-run CV with any confidence; below 5, the CV estimate itself is too noisy. Five runs is also what LNT uses as the default for nightly regression detection. For publication-grade results, the harness supports `--runs=10` or `--runs=20` at the cost of proportionally longer wall time.

**Why not interleave.** Triton's `do_bench` and Google Benchmark both run all repetitions of one benchmark before moving to the next (no interleaving). LLVM's `--benchmark_enable_random_interleaving=true` is an option in Google Benchmark that randomizes the order to detect thermal drift. We adopt non-interleaved for v1 (simpler, matches Triton), and note random interleaving as a future-work option in §17.

**Inter-run CV criterion.** If `inter_run_cv > 5%`, the configuration is flagged `noisy` in JSON v2 and the report recommends re-running with `--runs=10`. If `inter_run_cv > 10%`, the configuration is flagged `unreliable` and its results are excluded from the headline comparison table.

### 7.6 Reporting Schema (JSON v2)

The full JSON v2 schema is specified in §15. The key change from the existing schema is that every configuration's `passes: [...]` array is included by default (not just the median), and the comparison block includes the full statistical battery (Welch, MWU, Cliff's δ, BH-adjusted p). The schema is designed to be forward-compatible: a v1 reader can ignore the new fields and still get the median + CI.

---

## 8. Tier 4: Macro-Workload Validation (Overhauled)

Tier 4 is where the results that go into the paper are produced. The existing harness treats Tier 4 as "Tier 3 with bigger problem sizes"; the new harness treats it as a separate discipline with its own protocol, its own reproducibility requirements, and its own significance threshold. The three kernels (reduction, prefix_scan, tiled_matmul), the three policies (Unchecked, ReadZeroSkipWrite, Restrict), the LOCAL vs STORAGE distinction, and the four problem sizes (64K, 1M, 10M, 16M) are all kept.

### 8.1 Multi-Run Protocol (k ≥ 5)

Every Tier 4 configuration is run `k` times in independent harness invocations, not as `k` inner loops within one invocation. This is the critical distinction from Tier 3's `--runs` flag: Tier 3's `--runs` re-runs within one process (sharing GPU state, driver state, and process-local caches); Tier 4's multi-run spawns a fresh process per run, eliminating any cross-run contamination from process-local state.

**Mechanism.** A shell wrapper (`bounds-check-bench/scripts/tier4-multirun.sh`) invokes `cargo run --release -- --tier=4 --config=<cfg> --runs=1` k times, writing each run's JSON v2 to a separate file (`results_tier4_run<N>_<config>.json`). A separate aggregator (`bounds-check-bench/src/bin/aggregate_tier4.rs`) reads the k JSON files and produces a single `results_tier4_aggregate_<config>.json` with the inter-run statistics.

**Why fresh process per run.** GPU driver state accumulates within a process: command buffer pools grow, shader compilation caches warm, memory allocators settle into steady-state fragmentation. Running k=5 within one process measures the steady-state of *that process*; running k=5 in fresh processes measures the steady-state of *the GPU*, which is what the paper claims to measure. NVIDIA's Nsight documentation makes the same recommendation for publication-grade measurement.

**Default k=5, configurable.** `--runs=5` is the default. `--runs=10` for publication, `--runs=20` for the final paper-table runs. Each run takes ~2 minutes for the full Tier 4 matrix (3 kernels × 3 policies × 4 sizes × 2 variants = 72 configurations, ~1.5s each), so k=5 is ~10 minutes; k=20 is ~40 minutes.

### 8.2 Inter-Run CV Reporting

For each configuration, the aggregator computes:

- `inter_run_median_us` — median of the k runs' per-run medians
- `inter_run_mean_us` — mean of the k runs' per-run medians
- `inter_run_stddev_us` — standard deviation of the k runs' per-run medians
- `inter_run_cv_pct` — `inter_run_stddev_us / inter_run_mean_us × 100`
- `inter_run_min_us`, `inter_run_max_us` — range across the k runs

The `inter_run_cv_pct` is the headline reproducibility metric. It answers the question: "if I run this configuration again tomorrow, how much will the median move?" The existing harness cannot answer this question at all; the new harness answers it for every configuration.

**Thresholds.**

| `inter_run_cv_pct` | Flag | Action |
|---|---|---|
| < 2% | `stable` | Use in headline table |
| 2–5% | `acceptable` | Use in headline table, report CV |
| 5–10% | `noisy` | Use in supplementary table only |
| > 10% | `unreliable` | Exclude from comparison; investigate clock lock (§9) |

These thresholds are calibrated against the existing data: the Track 2 `reduction 16M LOCAL Unchecked` configuration showed a -49.46% delta between two single runs (868 vs 1718 µs), which corresponds to an inter-run CV of ~50%. Any threshold below 50% would have caught this; 10% is conservative.

### 8.3 Reference-Baseline Re-Anchoring

The existing harness compares Track N ON vs Track N OFF within a single run pair. This is fragile: if both runs are noisy in the same direction (e.g., both run on a hot GPU), the delta is biased. The new harness adopts LNT's discipline of a *rolling baseline*: the OFF configuration is measured across multiple nights, and the ON configuration is compared against the rolling median of the OFF measurements, not against a single OFF run.

**Mechanism.** The CI runner (§16) maintains a `baseline/` directory containing the last 30 days of nightly OFF-configuration JSON v2 files. Each night's ON-configuration run is compared against the median of the baseline. The comparison uses the full statistical battery in §10 (Welch, MWU, Cliff's δ) with `n_treatment = k` (tonight's runs) and `n_control = 30 × k_baseline` (the rolling baseline).

**Why 30 days.** LNT's default; long enough to capture weekly variance patterns (weekend vs weekday system load), short enough that hardware drift (driver updates, thermal paste degradation) does not dominate.

**Re-anchoring trigger.** If the rolling baseline's `inter_run_cv_pct` exceeds 5% for any configuration, the baseline is flagged `drifting` and the operator is alerted to re-establish it (typically after a driver update or hardware change). This catches the "the baseline itself moved" failure mode that single-run-pair comparisons cannot detect.

### 8.4 Significance Threshold (Cliff's δ + Mann-Whitney U)

A Tier 4 delta is reported as *significant* only if it satisfies *both* of:

1. **Statistical significance.** Mann-Whitney U p < 0.01 (after Benjamini-Hochberg FDR correction across all comparisons in the table). We use MWU rather than Welch's t-test because GPU timing distributions are heavy-tailed and the t-test's normality assumption is violated; MWU is non-parametric and only assumes independent samples from the same distribution.

2. **Practical significance.** |Cliff's δ| ≥ 0.147 (the "small" effect-size threshold from Romano et al., 2006). Cliff's δ is the proportion of treatment samples that exceed control samples, minus the proportion that are below. It is robust to outliers and does not assume normality. |δ| < 0.147 means the distributions overlap so heavily that the delta, even if statistically significant, is practically meaningless.

**Why both.** A statistically significant delta with |δ| < 0.147 is "significant but trivially small" — common with very large n (where MWU finds significance for any non-identical distributions). A practically significant delta with p ≥ 0.01 is "large but not reproducible" — common with small n and high variance. Requiring both eliminates both failure modes.

**The Track 3 case re-examined.** Track 3's existing data shows +0.76% to +3.41% deltas (Track 3 ON is *slower* than OFF) with bootstrap CIs that overlap zero. Under the new protocol, Track 3 would be re-measured with n ≈ 100,000–400,000 per run, k = 5 runs. If the true delta is +1%, MWU will find p < 0.01 (n is huge); Cliff's δ will be ~0.05 (well below the 0.147 threshold). The report would conclude: "statistically significant but practically negligible — Track 3 has no measurable wall-clock benefit at this kernel's arithmetic intensity, consistent with the static word-count saving being only 8 words." This is the honest answer, and the new protocol produces it; the old protocol could not.

**The Track 3b case re-examined.** Track 3b's existing data shows -2.85% to -6.12% deltas (Track 3+3b ON is *faster*). Under the new protocol, with n ≈ 400,000 and k = 5, both MWU p < 0.01 and |Cliff's δ| ≥ 0.147 should be satisfied (the existing deltas are large enough that the distributions should barely overlap). The report would conclude: "statistically and practically significant — Track 3b produces a 2.9–6.1% wall-clock improvement, consistent across all sizes." This is what the paper currently claims; the new protocol *confirms* it rather than asserting it from a single run pair.

---

## 9. GPU Power & Clock Discipline (NEW)

Section 9 is the prerequisite for Section 7. Without clock discipline, the adaptive iteration count in §7.2 produces a tight CI around the *wrong number* — the median is stable across 400,000 passes, but it is the median at whatever clock frequency the GPU happened to be at during that second, which may differ by 30% from tomorrow's clock frequency. This is the root cause of the existing harness's inter-run CV problems, and it must be fixed before any statistical sophistication can help.

### 9.1 Clock Locking via `nvidia-smi -lgc`

**The mechanism.** `nvidia-smi -lgc <min_clock>,<max_clock>` locks the graphics clock to the specified range. Setting min = max pins the clock to a single frequency. On the RTX 4060 Laptop GPU, the supported graphics clock range is 300–2100 MHz; we pin to 2100 MHz (the maximum sustained frequency without thermal throttling on the stock cooler).

**The lock script.** `bounds-check-bench/scripts/lock_clocks.sh`:

```bash
#!/usr/bin/env bash
# Lock GPU clocks to eliminate dynamic-boost noise.
# Must be run with sudo. Idempotent.
set -euo pipefail

GPU_ID="${1:-0}"
GRAPHICS_CLOCK="${2:-2100}"
MEMORY_CLOCK="${3:-2501}"   # RTX 4060 Laptop LPDDR5 max

echo "[clock] locking GPU $GPU_ID to graphics=$GRAPHICS_CLOCK MHz, memory=$MEMORY_CLOCK MHz"
sudo nvidia-smi -i "$GPU_ID" -lgc "$GRAPHICS_CLOCK,$GRAPHICS_CLOCK"
sudo nvidia-smi -i "$GPU_ID" -lmc "$MEMORY_CLOCK,$MEMORY_CLOCK"

# Verify
nvidia-smi --query-gpu=clocks.current.graphics,clocks.current.memory \
           --format=csv,noheader -i "$GPU_ID"
```

**The unlock script.** `bounds-check-bench/scripts/unlock_clocks.sh` restores defaults:

```bash
#!/usr/bin/env bash
set -euo pipefail
GPU_ID="${1:-0}"
sudo nvidia-smi -i "$GPU_ID" -rgc
sudo nvidia-smi -i "$GPU_ID" -rmc
echo "[clock] restored default clock range on GPU $GPU_ID"
```

**Why this is mandatory, not optional.** The existing harness's Track 2 `reduction 16M LOCAL Unchecked` configuration showed 868 µs in one run and 1718 µs in another — a 2× difference. The kernel did not change; the GPU's clock did. Without clock locking, every other fix in this document is built on sand. The CI runner (§16) refuses to start a Tier 4 run unless the clock is locked; the harness's `clock.rs` module checks `nvidia-smi --query-gpu=clocks.current.graphics` at startup and exits with an error if the clock is not in the locked range.

**Why we do not lock to a lower clock.** Locking to 2100 MHz matches the GPU's typical sustained boost under load, so the locked measurement is representative of real-world performance. Locking to a lower clock (e.g., 1500 MHz) would reduce variance further but would also understate the kernel's real-world cost; the paper's claims are about real-world performance, not about a synthetic low-clock scenario.

### 9.2 Warmup-Stability Criterion (CV < 5% Across 3 Windows)

**The bug.** The existing harness fixes `WARMUP_PASSES = 5`. This is a heuristic, not a criterion. On a cold GPU (laptop just woken from sleep), 5 passes is insufficient — the GPU is still ramping clocks and the first few measurements are biased high. On a warm GPU (just finished another benchmark), 5 passes is wasted time.

**The fix.** Adopt NVIDIA's Nsight-recommended stability criterion: warmup continues until three consecutive measurement windows have coefficient of variation (CV) < 5%. A "measurement window" is a batch of 10 passes; CV is computed across the 10 pass times within the window. Three consecutive windows below 5% means the GPU has reached steady state.

**Pseudocode.**

```rust
fn warmup_until_stable(kernel: &Kernel, ts: &TimestampQueries) -> u32 {
    let mut total_warmup = 0u32;
    let mut consecutive_stable = 0u32;
    const WINDOW: u32 = 10;
    const TARGET_CV_PCT: f64 = 5.0;
    const REQUIRED_CONSECUTIVE: u32 = 3;
    const MAX_WARMUP: u32 = 1000;  // hard cap

    while total_warmup < MAX_WARMUP {
        let window = kernel.run_passes(ts, WINDOW);
        let cv_pct = stats::cv_pct(&window);
        total_warmup += WINDOW;

        if cv_pct < TARGET_CV_PCT {
            consecutive_stable += 1;
            if consecutive_stable >= REQUIRED_CONSECUTIVE {
                return total_warmup;
            }
        } else {
            consecutive_stable = 0;  // reset on any unstable window
        }
    }
    panic!("warmup did not stabilize after {MAX_WARMUP} passes (last CV={cv_pct}%)");
}
```

**Reporting.** The actual `n_warmup_passes` is recorded in JSON v2 per configuration. If warmup hit the `MAX_WARMUP` cap, the configuration is flagged `unstable_warmup` and excluded from the headline table. This catches the "GPU never stabilized" failure mode (e.g., thermal throttling on a hot laptop) instead of silently measuring a throttled kernel.

### 9.3 Idle Detection Before Run Start

**The mechanism.** Before any Tier 0 / 3 / 4 measurement begins, the harness polls `nvidia-smi --query-gpu=utilization.gpu --format=csv,noheader,nounits` every 100 ms. The run starts only when 5 consecutive polls report < 5% utilization (500 ms of idle). This ensures no leftover work from a previous benchmark or from the OS compositor is still draining.

**Pseudocode.**

```rust
fn wait_for_idle(device_id: u32) -> Result<Duration, Error> {
    let start = Instant::now();
    let mut consecutive_idle = 0u32;
    const REQUIRED_IDLE_POLLS: u32 = 5;  // 500 ms at 100ms poll
    const IDLE_THRESHOLD_PCT: u32 = 5;

    loop {
        let util = nvidia_smi::query_gpu(device_id, "utilization.gpu")?;
        if util < IDLE_THRESHOLD_PCT {
            consecutive_idle += 1;
            if consecutive_idle >= REQUIRED_IDLE_POLLS {
                return Ok(start.elapsed());
            }
        } else {
            consecutive_idle = 0;
        }
        std::thread::sleep(Duration::from_millis(100));
        if start.elapsed() > Duration::from_secs(30) {
            return Err(Error::IdleTimeout);
        }
    }
}
```

**Why 500 ms.** Empirically, on the RTX 4060 Laptop with a Wayland compositor, GPU utilization drops to <5% within 100–200 ms of a kernel completing, but occasional compositor wake-ups (every ~16 ms for a 60 Hz display) keep utilization spiking. 500 ms of sustained <5% ensures we are between compositor frames and the GPU is genuinely idle. The 30-second timeout catches the "another process is using the GPU" failure mode.

**Compositor quirk.** On a Wayland system with the desktop compositor running, the GPU is never truly idle — the compositor wakes up every frame. The harness therefore recommends running on a VT (virtual terminal) without a compositor, or on a headless CI runner. The `idle_detect` field in JSON v2 records whether the run was under a compositor; if so, results are flagged `compositor_present` and the operator is warned that absolute numbers may be inflated by ~5% (compositor contention), though *deltas* between configurations are unaffected because the compositor noise is constant across configurations.

### 9.4 Thermal/Throttle Monitor (Background Thread)

**The mechanism.** A background thread samples `nvidia-smi --query-gpu=clocks.current.graphics,power.draw,temperature.gpu,utilization.gpu,clocks_throttle_reasons.active --format=csv -l 1` every second for the duration of the run. The samples are logged to a CSV file (`results_<run_id>_clocks.csv`) and the per-pass timing records (§7.3) include the most recent clock sample.

**The throttle flag.** `clocks_throttle_reasons.active` is a bitmask; non-zero means the GPU is throttling. Common values on the RTX 4060 Laptop:
- `0x1` — GPU idle (clocks dropping below locked target)
- `0x2` — applications clocks setting
- `0x8` — SW power cap (power limit hit)
- `0x10` — HW slowdown (thermal)
- `0x20` — Sync boost
- `0x40` — SW thermal slowdown

If any throttle flag other than `0x0` is observed during a pass, that pass is marked `throttled` in the JSON v2 `passes` array. If >1% of passes are throttled, the entire run is flagged `throttled` and excluded from the headline table.

**Why a background thread.** Calling `nvidia-smi` from the main thread would perturb the measurement (the call itself takes ~10 ms and involves a context switch). A background thread that polls at 1 Hz adds <0.1% overhead and produces enough samples to correlate timing outliers with clock drops.

**Post-hoc analysis.** The `results_<run_id>_clocks.csv` file enables correlating per-pass timing with clock state. If a configuration shows a bimodal timing distribution (one cluster at 2.5 µs, another at 3.5 µs), the clock CSV will reveal whether the second cluster correlates with clock drops — pointing to a clock-lock failure rather than a kernel issue.

---

## 10. Statistical Layer (Overhauled)

The existing harness emits median + 95% bootstrap CI. This is the absolute minimum a benchmark can emit and still be called a benchmark, and it is insufficient for the claims the project makes. The new statistics layer is specified here.

### 10.1 Why Median + Bootstrap CI Is Insufficient

Three reasons.

**Bootstrap on n=10 is wide.** The bootstrap resamples the original 10 observations with replacement. With n=10, the bootstrap distribution of the median is itself noisy — the 95% CI is typically `[x_(2), x_(9)]` (the 2nd and 9th order statistics), which for GPU timing data is a wide range. The Track 3 data shows CIs of ±2% around a 1% delta — wider than the effect.

**Bootstrap on n=10 cannot resolve heavy tails.** GPU timing distributions are heavy-tailed: occasional driver interrupts produce 10× outliers. With n=10, the bootstrap is dominated by whether or not an outlier happened to be in the sample. With n=400,000 (§7.2), the bootstrap is robust to outliers — but then the bootstrap is unnecessary because the empirical distribution is the distribution.

**Median + CI does not support hypothesis testing.** A reviewer asks "is Track 3 ON significantly slower than OFF?" The existing harness answers "the CIs overlap, so we cannot say." This is not a hypothesis test; it is a visual inspection of CIs. The new layer answers with a p-value and an effect size, which is what a reviewer actually wants.

### 10.2 New Summary Statistics

For each configuration's per-pass timing array (`passes: [...]` in JSON v2), the harness computes:

| Statistic | Field | Why |
|---|---|---|
| `min` | `min_us` | Best-case; if this is >2× the median, the kernel is bimodal |
| `p50` (median) | `p50_us` | Robust central tendency; the headline number |
| `p95` | `p95_us` | Tail behavior; sensitive to driver interrupts |
| `p99` | `p99_us` | Worst-case-tail; relevant for latency-sensitive workloads |
| `max` | `max_us` | Absolute worst observed; if this is >10× p99, the run had a catastrophic outlier |
| `mean` | `mean_us` | For distributions that are not heavy-tailed, mean is more efficient than median |
| `stddev` | `stddev_us` | Spread; for normal-ish distributions, 95% of data is within ±2σ |
| `cv` | `cv_pct` | `stddev / mean × 100`; the per-pass CV (distinct from `inter_run_cv` in §8.2) |
| `n` | `n_passes` | The actual pass count (post-adaptive, post-warmup) |

All nine are emitted for every configuration. Downstream code (the aggregator, the paper generator, the CI regression checker) picks the ones it needs.

### 10.3 Hypothesis Tests

For each ON-vs-OFF comparison, the harness computes two tests:

**Welch's t-test (parametric).** `welch_t`, `welch_p`, `welch_df` (degrees of freedom, Satterthwaite approximation). Welch's t-test does not assume equal variances, which is important because the ON and OFF configurations may have different spreads (Track 2 should reduce variance by eliminating bounds-check branches). The test *does* assume approximate normality of the mean (via CLT, valid for large n), which holds at n ≥ 100,000 even for heavy-tailed underlying distributions.

**Mann-Whitney U (non-parametric).** `mwu_u`, `mwu_p`. The Mann-Whitney U test (also called Wilcoxon rank-sum) tests whether one sample is stochastically larger than the other, with no distributional assumptions. It is the appropriate test for heavy-tailed GPU timing data. We report both tests; if they disagree, the report flags the comparison as `test_disagreement` and recommends inspecting the per-pass distributions manually.

**Why both.** Welch's t-test is more powerful (more likely to detect a true effect) when the distributions are approximately normal; MWU is more robust when they are not. Reporting both lets the reader judge. For the headline significance threshold (§8.4), we use MWU because it makes weaker assumptions.

### 10.4 Effect Size — Cliff's δ

**The statistic.** Cliff's delta is defined as:

```
δ = (#(treatment > control) - #(treatment < control)) / (n_treatment × n_control)
```

It ranges from -1 (every treatment sample is below every control sample) to +1 (every treatment sample is above every control). 0 means complete overlap.

**Interpretation thresholds** (Romano et al., 2006):

| |δ| | Interpretation |
|---|---|
| < 0.147 | negligible |
| 0.147 – 0.33 | small |
| 0.33 – 0.474 | medium |
| ≥ 0.474 | large |

**Why Cliff's δ instead of Cohen's d.** Cohen's d assumes normality and is sensitive to outliers (it is computed from means and standard deviations). Cliff's δ is computed from ranks and is robust to outliers. For GPU timing data, Cliff's δ is the standard choice in the ML-systems literature (e.g., loose comparisons in PyTorch benchmarking).

**Computation cost.** Cliff's δ is O(n_treatment × n_control) naively, O(n log n) with rank-based computation. At n=400,000 per side, the rank-based version takes ~1 second in Rust. The harness uses the rank-based version.

### 10.5 Multi-Comparison Correction — Benjamini-Hochberg FDR

**The problem.** A single Tier 4 run produces 72 configurations × 3 comparisons (Track 2, 3, 3b) = 216 hypothesis tests. At α = 0.01, the expected number of false positives under the global null is 216 × 0.01 ≈ 2. Without correction, two "significant" results out of 216 are expected by chance.

**The fix.** Benjamini-Hochberg procedure controls the False Discovery Rate (FDR) — the expected proportion of false positives among the rejected hypotheses. FDR is less conservative than Bonferroni (which controls the family-wise error rate) and is the standard choice in genomics and ML benchmarking.

**Mechanism.** All p-values from a single run are collected, sorted, and adjusted: the k-th smallest p-value is multiplied by `n_tests / k`. The adjusted p-value is the minimum of this and 1. The significance threshold (α = 0.01) is applied to the adjusted p-values.

**Reporting.** Both `mwu_p` (raw) and `mwu_p_bh` (adjusted) are emitted. The headline significance test uses `mwu_p_bh`.

### 10.6 Reporting Requirements

The statistical layer is not optional. A configuration that is missing any of the §10.2 statistics is flagged `incomplete` and excluded from the comparison table. A comparison that is missing any of the §10.3 / §10.4 / §10.5 statistics is flagged `comparison_incomplete`. The CI runner (§16) fails the build if any headline-table configuration is `incomplete` or `unreliable`.

This is deliberately strict. The existing harness's "report whatever you have" approach is what allowed the Track 3 "within CI" non-result to make it into the paper draft. The new layer's "report everything or report nothing" approach forces the operator to either fix the measurement or honestly report that the measurement is incomplete.

---

## 11. Per-Section Reference Mapping

Each subsection maps a feature of the new harness to the reference whose practice it adopts, with a "theirs / ours / why" three-column table. This is the per-section mapping the requirements-gathering round asked for.

### 11.1 Google Benchmark ↔ Ours

| Feature | Google Benchmark | Ours | Why we differ (if we do) |
|---|---|---|---|
| Auto-calibrated iteration count | `--benchmark_min_time=0.5s` | `adaptive_n_passes` targets 1s (Triton-style) | Triton's 1s gives more samples at µs scale; we run fewer total benchmarks than Google Benchmark users do, so 1s each is affordable |
| Aggregations emitted | mean, median, stddev, cv, mean_dev, min, max | min, p50, p95, p99, max, mean, stddev, cv | We add p95/p99 (tail) and drop mean_dev (subsumed by stddev); Google Benchmark's percentile support is via `--benchmark_perf_counters` only |
| Repetitions | `--benchmark_repetitions=N` | `--runs=N` (Tier 3) and `tier4-multirun.sh` (Tier 4) | Tier 4 spawns fresh processes, not inner loops (§8.1); Google Benchmark's repetitions are within one process |
| Manual timing | `state.SetIterationTime(seconds)` | wgpu `TIMESTAMP_QUERY` + per-pass resolve | wgpu is the manual-timing source; Google Benchmark's API is the model |
| Aggregates-only report | `--benchmark_report_aggregates_only=true` | JSON v2 always includes both raw + aggregates | We always emit raw per-pass data; downstream code picks aggregations |
| Random interleaving | `--benchmark_enable_random_interleaving=true` | Not adopted (v1) | Future work (§17); Triton does not interleave either |
| Perf counters | `--benchmark_perf_counters=CYCLES,INSTRUCTIONS` | `nvidia-smi` clocks/throttle samples (§9.4) | wgpu does not expose hardware perf counters; we sample clocks instead |
| JSON output | `--benchmark_format=json` | JSON v2 (§15) | Different schema, same idea |

### 11.2 LLVM ↔ Ours

| Feature | LLVM | Ours | Why we differ |
|---|---|---|---|
| Tier structure | lit: Unit / Functional / Benchmark / Nightly | Tier 0 / 1 / 2 / 3 / 4 | Tier 0 (noise floor) is new; Tiers 1–4 map to lit's tiers |
| Snapshot diffing | FileCheck | `git diff --exit-code HEAD naga/tests/out` | We use git diff against committed golden files; FileCheck would be more expressive but is not worth the dependency |
| Historical regression | LNT nightly SQL DB + rolling baseline | `baseline/` directory of JSON v2 files, 30-day rolling median (§8.3) | We do not run a SQL DB; JSON files are sufficient for the project's scale |
| Comparison reference | IRCE pass for Track 2 | Same — IRCE is the cited comparison point | Unchanged from existing docs; harness architecture permits SYCL+IRCE comparison as future work |
| Random interleaving | `--benchmark_enable_random_interleaving` (via Google Benchmark) | Not adopted (v1) | Same as §11.1 |
| Multi-vendor | LLVM nightly tests run on Intel, AMD, NVIDIA | RTX 4060 only (v1) | Future work (§17); the harness is vendor-agnostic via wgpu |

### 11.3 Triton `do_bench` ↔ Ours

| Feature | Triton `do_bench` | Ours | Why we differ |
|---|---|---|---|
| Timer primitive | `torch.cuda.Event(enable_timing=True)` (CUDA events) | wgpu `TIMESTAMP_QUERY` | Web portability requirement; CUDA events are native-only |
| Adaptive iteration count | 1s target, picked from one warmup call | 1s target, picked from one warmup pass (§7.2) | Same idea; we cap at 100,000 passes to bound host overhead |
| Default return mode | `return_mode='mean'` (median is `return_mode='median'`) | Always `return_mode='all'` (full distribution) | We always emit raw per-pass data; downstream code computes any summary |
| Quantile reporting | `quantiles=[0.2, 0.5, 0.8]` parameter | `min, p50, p95, p99, max` always emitted | Different quantile choices; ours match Google Benchmark's defaults more closely |
| CUDA Graph capture | `do_bench_cudagraph` | Not adopted (no wgpu equivalent) | Future work (§17); wgpu does not expose graph capture |
| Warmup | Implicit (one call to measure, then n_repeat) | Explicit stability criterion (CV < 5%, 3 windows, §9.2) | Triton's warmup is one call; ours is criterion-based because laptop GPUs have worse clock drift than datacenter GPUs Triton targets |
| Grad-to-none | `grad_to_none` param to zero PyTorch gradients | N/A (no autograd in WGSL) | — |
| Hypothesis testing | Not provided (user computes externally) | Welch + MWU + Cliff's δ + BH (§10) | We bake statistics into the harness because the project's claims depend on them |

### 11.4 NVIDIA ↔ Ours

| Feature | NVIDIA recommendation | Ours | Why we differ |
|---|---|---|---|
| Timer primitive | `cudaEventRecord` / `cudaEventElapsedTime` | wgpu `TIMESTAMP_QUERY` | Web portability; `TIMESTAMP_QUERY` is the moral equivalent |
| Clock locking | `nvidia-smi -lgc <freq>,<freq>` | Same (§9.1) | Adopted verbatim |
| Warmup stability | "3 consecutive windows with CV < 5%" (Nsight docs) | Same (§9.2) | Adopted verbatim |
| Idle detection | Poll `nvidia-smi --query-gpu=utilization.gpu` | Same (§9.3), with 500ms / 5% thresholds | Adopted verbatim |
| Throttle monitoring | `clocks_throttle_reasons.active` | Same (§9.4), background thread at 1Hz | Adopted verbatim |
| Nsight Compute | Per-kernel roofline analysis | Not integrated (out of scope v1) | Future work (§17); would require a native CUDA path |
| Nsight Systems | Timeline view, identify pipeline stalls | Not integrated (out of scope v1) | Future work (§17) |
| CUDA Graph capture | `cudaGraphCapture` / `cudaGraphLaunch` | Not adopted (no wgpu equivalent) | Future work |
| `cudaDeviceSynchronize` between configs | Yes, to flush pipeline | `device.poll(Maintain::Wait)` per pass (§7.3) | wgpu equivalent adopted |
| 50–100 reps for stable median | Recommended | Adaptive (§7.2), typically 50–400,000 | We exceed the recommendation for µs-scale kernels |
| 1000+ reps for tail percentiles | Recommended | Adaptive, up to 100,000 | Within 10× of the recommendation; capped for host-overhead reasons (§7.2) |
| Cross-validation with `cudaEvent` | Implicit (it is the timer) | Optional native CUDA path (future work, §17) | Deferred; the harness architecture permits it but does not require it |

---

## 12. Migration Plan — Phased Rollout

The redesign is rolled out in four phases, each with explicit acceptance criteria. The phases are sequenced so that each phase's fixes are independently shippable — if Phase 4 is delayed, Phases 1–3 still produce a harness that is strictly better than the existing one.

### 12.1 Phase 1 (Week 1) — Surgical Patches

**Scope.** Fix the three most embarrassing bugs without changing the harness architecture: (a) replace host-timed peak-BW with the GPU-timed sustained-copy kernel (§5.3), (b) increase `TIMED_PASSES` from 10 to a configurable `--passes=N` default of 1000, (c) add the §10.2 statistics (min, p50, p95, p99, max, mean, stddev, cv) to the JSON output alongside the existing median + bootstrap CI.

**Acceptance criteria.**
- Tier 0 sustained-copy measurement has inter-run CV < 5% across 5 runs (replacing the ±50% variance of the old host-timed measurement).
- Track 3b's `track3b_test 64K` configuration produces a p95 / p99 spread that is < 2× the median (i.e., the distribution is unimodal, not heavy-tailed garbage).
- The JSON output of one Phase-1 run can be read by the existing `aggregate_tier4.rs` from the old harness (forward-compatible).

**Estimated effort.** 2–3 days of implementation, 1 day of validation. The sustained-copy kernel is ~20 lines of WGSL; the `--passes` flag is a `clap` argument; the §10.2 statistics are a `stats.rs` module of ~100 lines.

### 12.2 Phase 2 (Week 2) — Clock Discipline + Warmup Stability

**Scope.** Add `lock_clocks.sh` / `unlock_clocks.sh` (§9.1), idle detection (§9.3), warmup-stability criterion (§9.2), and the throttle monitor background thread (§9.4). Refuse to start a Tier 4 run if the clock is not locked.

**Acceptance criteria.**
- All 72 Tier 4 configurations, run with k=5 inter-run, have `inter_run_cv_pct < 5%`. (The existing harness cannot hit this for any configuration on a laptop GPU.)
- The `unstable_warmup` flag is never set under normal operation (i.e., warmup converges within the 1000-pass cap for every configuration).
- The throttle monitor's CSV file shows zero non-zero `clocks_throttle_reasons.active` values during any pass.

**Estimated effort.** 3–4 days. The `nvidia-smi` integration is straightforward; the warmup-stability loop and the background thread are ~200 lines of Rust combined.

### 12.3 Phase 3 (Week 3) — Multi-Run Protocol + Noise-Floor Tier 0

**Scope.** Add `tier4-multirun.sh` (§8.1), the inter-run aggregator (§8.2), the rolling baseline (§8.3), the significance threshold (§8.4), and the idle + no-op Tier 0 kernels (§5.1, §5.2). Add the §10.3 / §10.4 / §10.5 statistical tests.

**Acceptance criteria.**
- Every Tier 4 comparison produces a `mwu_p_bh` and a `cliffs_delta` value.
- Track 3's `wgul_reduce` configuration produces a report that says either "statistically and practically significant" or "statistically significant but practically negligible" — not "within CI range".
- Track 3b's `track3b_test` configuration produces a report that confirms (or refutes) the existing paper's 2.9–6.1% claim with both MWU p < 0.01 and |Cliff's δ| ≥ 0.147.

**Estimated effort.** 4–5 days. The Mann-Whitney U and Cliff's δ implementations are ~150 lines each; the BH correction is ~30 lines; the multi-run shell wrapper and aggregator are ~200 lines combined.

### 12.4 Phase 4 (Week 4) — CI Integration + JSON v2 + Paper Re-Run

**Scope.** Add the JSON v2 schema (§15), the CI runner (§16), and re-run the paper's headline tables with the new harness. Update the paper's §5 (benchmarking methodology) to describe the new protocol.

**Acceptance criteria.**
- The CI runner produces a nightly regression report comparing against the 30-day rolling baseline.
- The paper's Table 1 (Track 3b results) is regenerated from the new harness, with p-values and Cliff's δ in the table.
- The paper's §5 explicitly states the new protocol: clock lock, adaptive iteration, multi-run k=5, MWU + Cliff's δ significance threshold.

**Estimated effort.** 3–4 days for the CI integration; 2–3 days for the paper re-run and table regeneration. Total Phase 4: ~1 week.

### 12.5 Out-of-Phase Work (Parallel, Not Blocking)

The following can be done in parallel with any phase and do not block the main rollout:
- SYCL + IRCE comparison kernels (§11.2 reference mapping, future work in §17).
- Nsight Compute / Nsight Systems integration (§11.4, future work).
- Native CUDA cross-validation path (§17).
- Multi-vendor testing on AMD RDNA3 / Intel Arc (§17).

---

## 13. Validation Matrix

Each must-fix problem mapped to the section that addresses it, the test that verifies the fix, and the reference that validates the approach.

| # | Must-Fix Problem | Addressed In | Verifying Test | Validating Reference |
|---|---|---|---|---|
| 1 | Peak-BW noise (±50%) | §5.3, §5.4, §14.2 | 5-run inter-run CV of `sustained_copy_bw_gbps` < 5% | NVIDIA `cudaEvent`-timed copy; Triton `do_bench` (CUDA events, not host timing) |
| 2 | Track 3 noise floor | §7.2, §10.3, §10.4 | `mwu_p_bh` and `cliffs_delta` for `wgul_reduce` produce a definitive significant-or-negligible verdict | Google Benchmark auto-iteration count + full distribution; Triton `return_mode='all'` |
| 3 | Multi-run protocol | §8.1, §8.2 | All Tier 4 configurations have `inter_run_cv_pct` reported; none is `unreliable` | LLVM LNT multi-night rolling baseline; Google Benchmark `--benchmark_repetitions` |
| 4 | Clock lock + warmup | §9.1, §9.2, §9.3 | `n_warmup_passes` is criterion-based, not fixed; `clocks_throttle_reasons.active` is 0 for all passes | NVIDIA `nvidia-smi -lgc` + Nsight warmup-stability recommendation |
| 5 | Noise-floor calibration | §5.1, §5.2 | `timer_floor_us` and `launch_overhead_us` are reported; any Tier 3 delta below `2 × timer_floor_us` is excluded | Triton implicit (one warmup call); NVIDIA idle detection |
| 6 | Statistics layer overhaul | §10.2, §10.3, §10.4, §10.5 | Every configuration has min/p50/p95/p99/max/mean/stddev/cv; every comparison has Welch + MWU + Cliff's δ + BH | Google Benchmark aggregations; ML-systems literature standard (Cliff's δ) |

**Cross-cutting validation.** The CI runner (§16) runs the full Tier 0 + Tier 3 + Tier 4 matrix nightly and produces a report that must contain:
- Zero `incomplete` or `comparison_incomplete` configurations (§10.6).
- Zero `unstable_warmup` configurations.
- Zero `throttled` configurations.
- All `inter_run_cv_pct` values below 5%.
- All `sustained_copy_bw_gbps` values within ±5% of the 30-day rolling median.

If any of these fails, the CI report flags the night as `degraded` and the operator is alerted. The paper's claims are not updated from a `degraded` night.

---

## 14. Rust Pseudocode for Critical Components

This section contains Rust pseudocode for the four most critical components. It is pseudocode, not production code — error handling is omitted, types are simplified, and some helpers (`stats::cv_pct`, `nvidia_smi::query_gpu`) are referenced but not defined. The full implementation would live in `bounds-check-bench/src/harness/`.

### 14.1 Noise-Floor Kernel Driver (`harness/tier0.rs`)

```rust
//! Tier 0: noise-floor calibration. Runs three kernels (idle, no-op,
//! sustained copy) before any Tier 3 measurement begins.

use crate::harness::timestamp::{TimestampQueries, TimestampResult};
use crate::harness::stats;

pub struct Tier0Result {
    pub timer_floor_us: f64,
    pub launch_overhead_us: LaunchOverheadStats,
    pub sustained_copy_bw_gbps: SustainedCopyStats,
    pub degraded: bool,
}

pub struct LaunchOverheadStats {
    pub median: f64, pub p95: f64, pub p99: f64, pub n: u32,
}

pub struct SustainedCopyStats {
    pub median: f64, pub p95: f64, pub p99: f64, pub n: u32,
    pub inter_run_cv_pct: f64,
}

pub fn run_tier0(device: &wgpu::Device, queue: &wgpu::Queue) -> Tier0Result {
    let ts = TimestampQueries::new(device, 2);

    // 1. Idle kernel: zero-dispatch pass, 100 reps, take median.
    let idle_passes: Vec<f64> = (0..100).map(|_| {
        let r = run_idle_pass(device, queue, &ts);
        r.elapsed_us()
    }).collect();
    let timer_floor_us = stats::median(&idle_passes);

    // 2. No-op kernel: 1 dispatch of empty shader, 100 reps.
    let noop_module = compile_wgsl(device, NOOP_WGSL);
    let noop_passes: Vec<f64> = (0..100).map(|_| {
        let r = run_noop_pass(device, queue, &ts, &noop_module);
        r.elapsed_us()
    }).collect();
    let launch_overhead_us = LaunchOverheadStats {
        median: stats::median(&noop_passes),
        p95: stats::percentile(&noop_passes, 95.0),
        p99: stats::percentile(&noop_passes, 99.0),
        n: 100,
    };

    // 3. Sustained copy: 64 MB → 64 MB, 10 reps, ~50ms each.
    let copy_module = compile_wgsl(device, SUSTAINED_COPY_WGSL);
    let copy_passes: Vec<f64> = (0..10).map(|_| {
        let r = run_sustained_copy_pass(device, queue, &ts, &copy_module, 64 * 1024 * 1024);
        r.elapsed_us()
    }).collect();
    let copy_bws: Vec<f64> = copy_passes.iter()
        .map(|&us| (2.0 * 64.0 * 1024.0 * 1024.0) / (us * 1e-6) / 1e9)  // GB/s
        .collect();
    let sustained_copy_bw_gbps = SustainedCopyStats {
        median: stats::median(&copy_bws),
        p95: stats::percentile(&copy_bws, 95.0),
        p99: stats::percentile(&copy_bws, 99.0),
        n: 10,
        inter_run_cv_pct: stats::cv_pct(&copy_bws),
    };

    let degraded = sustained_copy_bw_gbps.inter_run_cv_pct > 5.0;

    Tier0Result { timer_floor_us, launch_overhead_us, sustained_copy_bw_gbps, degraded }
}

const NOOP_WGSL: &str = include_str!("../shaders/tier0_noop.wgsl");
const SUSTAINED_COPY_WGSL: &str = include_str!("../shaders/tier0_sustained_copy.wgsl");
```

### 14.2 Sustained Copy Kernel (`shaders/tier0_sustained_copy.wgsl`)

```wgsl
// Tier 0 GPU-timed sustained-copy kernel.
// Replaces the broken host-timed `copy_buffer_to_buffer` peak-BW.
//
// Dispatched 100× per pass with a 64 MB buffer to give ~50ms per pass
// at ~14 GB/s. The +0u prevents copy-elision.

@group(0) @binding(0) var<storage> src: array<u32>;
@group(0) @binding(1) var<storage, read_write> dst: array<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i < arrayLength(&src)) {
        dst[i] = src[i] + 0u;
    }
}
```

### 14.3 Adaptive Iteration Loop (`harness/tier3.rs`)

```rust
//! Tier 3: device-timing protocol with adaptive iteration count.

use crate::harness::timestamp::TimestampQueries;
use crate::harness::stats;
use crate::harness::clock;

pub struct Tier3Config {
    pub kernel_name: String,
    pub policy: BoundsCheckPolicy,
    pub size: ProblemSize,
    pub variant: MemoryVariant,
    pub track: Track,  // Off, Track2, Track3, Track3b
}

pub struct Tier3Result {
    pub config: Tier3Config,
    pub passes: Vec<f64>,  // raw per-pass µs, full distribution
    pub n_passes: u32,
    pub n_warmup_passes: u32,
    pub unstable_warmup: bool,
    pub throttled_passes_pct: f64,
    pub summary: SummaryStats,
}

pub fn run_tier3(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    cfg: &Tier3Config,
) -> Tier3Result {
    let ts = TimestampQueries::new(device, 2);
    let kernel = compile_kernel(device, cfg);

    // 1. Warmup until stable (§9.2).
    let n_warmup_passes = warmup_until_stable(device, queue, &ts, &kernel, cfg);
    let unstable_warmup = n_warmup_passes >= MAX_WARMUP;

    // 2. Single timed pass to estimate per-pass duration.
    let warmup_pass_us = run_one_pass(device, queue, &ts, &kernel, cfg).elapsed_us();

    // 3. Adaptive iteration count (§7.2).
    let n_passes = adaptive_n_passes(warmup_pass_us);

    // 4. Run n_passes, recording each pass's elapsed µs and clock sample.
    let mut passes = Vec::with_capacity(n_passes as usize);
    let mut throttled_count = 0u32;

    for _ in 0..n_passes {
        let clock_sample = clock::latest_sample();
        let r = run_one_pass(device, queue, &ts, &kernel, cfg);
        passes.push(r.elapsed_us());
        if clock_sample.throttle_flags != 0 {
            throttled_count += 1;
        }
    }

    let throttled_passes_pct = (throttled_count as f64 / n_passes as f64) * 100.0;
    let summary = stats::summary(&passes);

    Tier3Result {
        config: cfg.clone(),
        passes,
        n_passes,
        n_warmup_passes,
        unstable_warmup,
        throttled_passes_pct,
        summary,
    }
}

fn adaptive_n_passes(warmup_pass_us: f64) -> u32 {
    const TARGET_TOTAL_US: f64 = 1_000_000.0;  // 1 second
    const MAX_PASSES: u32 = 100_000;
    const MIN_PASSES: u32 = 50;
    let n = (TARGET_TOTAL_US / warmup_pass_us) as u32;
    n.clamp(MIN_PASSES, MAX_PASSES)
}

const MAX_WARMUP: u32 = 1000;
```

### 14.4 Statistical Aggregator (`harness/stats.rs`)

```rust
//! Statistical layer: summary stats, hypothesis tests, effect size.

use std::cmp::Ordering;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SummaryStats {
    pub min_us: f64,
    pub p50_us: f64,
    pub p95_us: f64,
    pub p99_us: f64,
    pub max_us: f64,
    pub mean_us: f64,
    pub stddev_us: f64,
    pub cv_pct: f64,
    pub n_passes: u32,
}

pub fn summary(passes: &[f64]) -> SummaryStats {
    let mut sorted: Vec<f64> = passes.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let n = sorted.len();
    let mean = sorted.iter().sum::<f64>() / n as f64;
    let var = sorted.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
    let stddev = var.sqrt();
    SummaryStats {
        min_us: sorted[0],
        p50_us: percentile_sorted(&sorted, 50.0),
        p95_us: percentile_sorted(&sorted, 95.0),
        p99_us: percentile_sorted(&sorted, 99.0),
        max_us: sorted[n - 1],
        mean_us: mean,
        stddev_us: stddev,
        cv_pct: (stddev / mean) * 100.0,
        n_passes: n as u32,
    }
}

pub fn percentile_sorted(sorted: &[f64], p: f64) -> f64 {
    let n = sorted.len();
    let rank = (p / 100.0) * (n - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    if lo == hi { sorted[lo] } else {
        let frac = rank - lo as f64;
        sorted[lo] * (1.0 - frac) + sorted[hi] * frac
    }
}

pub fn median(x: &[f64]) -> f64 {
    let mut s = x.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    percentile_sorted(&s, 50.0)
}

pub fn cv_pct(x: &[f64]) -> f64 {
    let mean = x.iter().sum::<f64>() / x.len() as f64;
    let var = x.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / x.len() as f64;
    (var.sqrt() / mean) * 100.0
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Comparison {
    pub delta_pct: f64,
    pub welch_t: f64,
    pub welch_p: f64,
    pub welch_df: f64,
    pub mwu_u: f64,
    pub mwu_p: f64,
    pub cliffs_delta: f64,
    pub cliffs_delta_interpretation: &'static str,
    pub mwu_p_bh: f64,  // FDR-adjusted; filled in by batch_adjust()
    pub significant: bool,  // MWU p_bh < 0.01 AND |cliffs_delta| >= 0.147
}

pub fn compare(treatment: &[f64], control: &[f64]) -> Comparison {
    let t_med = median(treatment);
    let c_med = median(control);
    let delta_pct = (t_med - c_med) / c_med * 100.0;
    let (welch_t, welch_p, welch_df) = welch_t_test(treatment, control);
    let (mwu_u, mwu_p) = mann_whitney_u(treatment, control);
    let cliffs = cliffs_delta(treatment, control);
    let interp = match cliffs.abs() {
        d if d < 0.147 => "negligible",
        d if d < 0.33  => "small",
        d if d < 0.474 => "medium",
        _ => "large",
    };
    Comparison {
        delta_pct,
        welch_t, welch_p, welch_df,
        mwu_u, mwu_p,
        cliffs_delta: cliffs,
        cliffs_delta_interpretation: interp,
        mwu_p_bh: mwu_p,  // placeholder; batch_adjust() overwrites
        significant: false,  // batch_adjust() sets this after BH
    }
}

/// Benjamini-Hochberg FDR correction across a batch of comparisons.
pub fn batch_adjust(comparisons: &mut [Comparison], alpha: f64) {
    let n = comparisons.len() as f64;
    let mut indexed: Vec<(usize, f64)> = comparisons.iter()
        .enumerate()
        .map(|(i, c)| (i, c.mwu_p))
        .collect();
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
    for (rank, (i, p)) in indexed.iter().enumerate() {
        let bh = (p * n / (rank + 1) as f64).min(1.0);
        comparisons[*i].mwu_p_bh = bh;
        comparisons[*i].significant =
            bh < alpha && comparisons[*i].cliffs_delta.abs() >= 0.147;
    }
}

fn welch_t_test(a: &[f64], b: &[f64]) -> (f64, f64, f64) {
    let (na, nb) = (a.len() as f64, b.len() as f64);
    let ma = a.iter().sum::<f64>() / na;
    let mb = b.iter().sum::<f64>() / nb;
    let va = a.iter().map(|x| (x - ma).powi(2)).sum::<f64>() / (na - 1.0);
    let vb = b.iter().map(|x| (x - mb).powi(2)).sum::<f64>() / (nb - 1.0);
    let se = (va / na + vb / nb).sqrt();
    let t = (ma - mb) / se;
    let df = (va / na + vb / nb).powi(2)
        / ((va / na).powi(2) / (na - 1.0) + (vb / nb).powi(2) / (nb - 1.0));
    let p = 2.0 * (1.0 - students_t_cdf(t.abs(), df));  // two-tailed
    (t, p, df)
}

fn mann_whitney_u(a: &[f64], b: &[f64]) -> (f64, f64) {
    // Rank-based implementation; details omitted.
    // Returns (U statistic, two-tailed p-value via normal approximation).
    // For n > 50 the normal approximation is accurate; for small n use exact tables.
    // O(n log n) via sort + rank.
    unimplemented!()  // see Wasserman, "All of Nonparametric Statistics", §5.4
}

fn cliffs_delta(treatment: &[f64], control: &[f64]) -> f64 {
    // Rank-based: for each treatment value, count how many control values
    // are below minus how many are above, divided by n_t * n_c.
    // O(n log n) with sorted control.
    let mut sorted_control: Vec<f64> = control.to_vec();
    sorted_control.sort_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal));
    let n_t = treatment.len() as f64;
    let n_c = control.len() as f64;
    let mut above = 0isize;
    let mut below = 0isize;
    for &t in treatment {
        // binary search for count of control < t and count of control > t
        let n_below = sorted_control.partition_point(|&c| c < t);
        let n_above = n_c as usize - sorted_control.partition_point(|&c| c <= t);
        below += n_below as isize;
        above += n_above as isize;
    }
    (above - below) as f64 / (n_t * n_c)
}
```

### 14.5 Multi-Run Orchestrator (`scripts/tier4-multirun.sh`)

```bash
#!/usr/bin/env bash
# Tier 4 multi-run protocol: spawn k fresh processes per configuration.
# Aggregates inter-run statistics across the k runs.

set -euo pipefail

K="${1:-5}"                  # number of runs per configuration
CONFIG_FILE="${2:-configs/tier4_all.toml}"
OUT_DIR="${3:-results/tier4_$(date +%Y%m%d_%H%M%S)}"

mkdir -p "$OUT_DIR"

# Phase 1: lock clocks, wait for idle
sudo bash scripts/lock_clocks.sh
trap 'sudo bash scripts/unlock_clocks.sh' EXIT

# Phase 2: for each config, run K fresh processes
while IFS= read -r cfg; do
    for run in $(seq 1 "$K"); do
        echo "[multirun] config=$cfg run=$run/$K"
        cargo run --release --bin bounds-check-bench -- \
            --tier=4 --config "$cfg" --runs 1 \
            --out "$OUT_DIR/results_tier4_run${run}_${cfg//\//_}.json"
    done
done < "$CONFIG_FILE"

# Phase 3: aggregate
cargo run --release --bin aggregate_tier4 -- \
    --in "$OUT_DIR" \
    --out "$OUT_DIR/results_tier4_aggregate.json"

echo "[multirun] done. results in $OUT_DIR"
```

---

## 15. JSON Schema v2 (Results File)

The JSON v2 schema is the contract between the harness, the aggregator, the CI runner, and the paper generator. It is designed to be forward-compatible: a v1 reader can ignore new fields and still extract the median + bootstrap CI.

### 15.1 Top-Level Schema

```json
{
  "schema_version": "2.0",
  "run_id": "epoch20634_80281_NVIDIA_GeForce_RTX_4060_Laptop_GPU_vulkan",
  "timestamp_utc": "2026-07-05T14:32:00Z",
  "harness_version": "bounds-check-bench 2.0.0",
  "hardware": {
    "gpu_name": "NVIDIA GeForce RTX 4060 Laptop GPU",
    "driver_version": "580.159.04",
    "cuda_version": "13.0",
    "vulkan_backend": true,
    "total_vram_mb": 8192,
    "timestamp_period_ns": 1.0
  },
  "clock_state": {
    "locked": true,
    "graphics_clock_mhz": 2100,
    "memory_clock_mhz": 2501,
    "idle_detected": true,
    "idle_wait_ms": 612,
    "compositor_present": false,
    "throttle_observed": false,
    "throttle_passes_pct": 0.0
  },
  "tier0": {
    "timer_floor_us": 1.832,
    "launch_overhead_us": {
      "median": 4.711, "p95": 5.203, "p99": 6.012, "n": 100
    },
    "sustained_copy_bw_gbps": {
      "median": 14.21, "p95": 14.38, "p99": 14.52, "n": 10,
      "inter_run_cv_pct": 2.1
    },
    "degraded": false
  },
  "tier3": [
    {
      "config": {
        "kernel": "track3b_test",
        "policy": "ReadZeroSkipWrite",
        "size": "64K",
        "variant": "LOCAL",
        "track": "Track3+3b"
      },
      "passes": [2.532, 2.507, 2.532, 2.494, 2.520, "..."],
      "n_passes": 100000,
      "n_warmup_passes": 30,
      "unstable_warmup": false,
      "throttled_passes_pct": 0.0,
      "summary": {
        "min_us": 2.487,
        "p50_us": 2.520,
        "p95_us": 2.540,
        "p99_us": 2.568,
        "max_us": 3.121,
        "mean_us": 2.521,
        "stddev_us": 0.018,
        "cv_pct": 0.71,
        "n_passes": 100000
      }
    }
  ],
  "tier4": {
    "runs": [
      "results_tier4_run1_track3b_test_rzsw_64k_local.json",
      "results_tier4_run2_track3b_test_rzsw_64k_local.json",
      "results_tier4_run3_track3b_test_rzsw_64k_local.json",
      "results_tier4_run4_track3b_test_rzsw_64k_local.json",
      "results_tier4_run5_track3b_test_rzsw_64k_local.json"
    ],
    "aggregate": {
      "inter_run_median_us": 2.521,
      "inter_run_mean_us": 2.523,
      "inter_run_stddev_us": 0.031,
      "inter_run_cv_pct": 1.23,
      "inter_run_min_us": 2.487,
      "inter_run_max_us": 2.558,
      "reproducibility_flag": "stable"
    }
  },
  "comparisons": [
    {
      "treatment": "track3b_test|ReadZeroSkipWrite|64K|LOCAL|Track3+3b",
      "control":    "track3b_test|ReadZeroSkipWrite|64K|LOCAL|Track3-only",
      "delta_pct": -2.85,
      "welch_t": -42.31,
      "welch_p": 1.2e-308,
      "welch_df": 199998.4,
      "mwu_u": 1234567.0,
      "mwu_p": 3.4e-290,
      "mwu_p_bh": 1.5e-288,
      "cliffs_delta": -0.474,
      "cliffs_delta_interpretation": "medium",
      "significant": true
    }
  ]
}
```

### 15.2 Field Documentation

| Field | Type | Description |
|---|---|---|
| `schema_version` | string | Always `"2.0"`. Bumped on breaking changes. |
| `run_id` | string | Same format as v1: `epoch<t>_<pid>_<gpu_name>_<backend>` |
| `clock_state.locked` | bool | False if `nvidia-smi -lgc` was not run; run is `degraded` |
| `clock_state.throttle_observed` | bool | True if any pass had non-zero throttle flags |
| `tier0.sustained_copy_bw_gbps.inter_run_cv_pct` | f64 | Headline noise-floor metric; must be <5% |
| `tier3[].passes` | f64[] | Full per-pass distribution; the raw data |
| `tier3[].summary.cv_pct` | f64 | Per-pass CV (within one run); distinct from `inter_run_cv` |
| `tier4.aggregate.inter_run_cv_pct` | f64 | Headline reproducibility metric; must be <5% for `stable` |
| `comparisons[].significant` | bool | True iff `mwu_p_bh < 0.01` AND `|cliffs_delta| >= 0.147` |

### 15.3 Backward Compatibility

A v1 reader can extract:
- `tier3[].summary.p50_us` — the old "median"
- `tier3[].summary` does not contain `ci_95`; v1's bootstrap CI is replaced by the full distribution, from which any CI can be computed downstream

The v1 `peak_bandwidth_gbps` field is replaced by `tier0.sustained_copy_bw_gbps.median`. The semantics differ (host-timed copy vs. GPU-timed sustained copy), so the values are not directly comparable; the migration plan (§12.1) calls this out.

---

## 16. CI Integration

The CI runner is a GitHub Actions workflow that runs on a self-hosted runner with the RTX 4060 Laptop GPU. It runs nightly and on every push to `main`.

### 16.1 Workflow Structure (`.github/workflows/bench-nightly.yml`)

```yaml
name: Nightly Benchmark
on:
  schedule:
    - cron: '0 2 * * *'  # 02:00 UTC daily
  workflow_dispatch: {}

jobs:
  bench:
    runs-on: self-hosted-linux-gpu-rtx4060
    steps:
      - uses: actions/checkout@v4
        with:
          ref: ${{ github.sha }}

      - name: Lock GPU clocks
        run: sudo bash bounds-check-bench/scripts/lock_clocks.sh

      - name: Run Tier 0 + Tier 3 + Tier 4 (k=5)
        run: |
          bash bounds-check-bench/scripts/tier4-multirun.sh 5 \
            bounds-check-bench/configs/tier4_all.toml \
            results/$(date +%Y%m%d)

      - name: Restore GPU clocks
        if: always()
        run: sudo bash bounds-check-bench/scripts/unlock_clocks.sh

      - name: Compare against rolling baseline
        run: |
          cargo run --release --bin compare_to_baseline -- \
            --today results/$(date +%Y%m%d)/results_tier4_aggregate.json \
            --baseline baseline/ \
            --out results/$(date +%Y%m%d)/regression_report.md

      - name: Update rolling baseline (30-day window)
        run: |
          cp results/$(date +%Y%m%d)/results_tier4_aggregate.json \
             baseline/$(date +%Y%m%d).json
          find baseline/ -mtime +30 -delete

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: bench-${{ date }}
          path: results/$(date +%Y%m%d)/
```

### 16.2 Self-Hosted Runner Requirements

- NVIDIA GeForce RTX 4060 Laptop GPU (or whichever GPU is the project's reference target).
- Linux with `nvidia-smi` accessible to the runner user via passwordless sudo (for `-lgc`).
- Vulkan SDK installed (for wgpu's Vulkan backend).
- Rust 1.75+ (matches the existing crate's MSRV).
- Headless mode preferred (no compositor). If a compositor is unavoidable, the `compositor_present` flag in JSON v2 records it.

### 16.3 Regression Criteria

The nightly report (`regression_report.md`) lists each configuration and marks it as `stable`, `regression`, `improvement`, or `noisy`:

- `stable`: `inter_run_cv_pct < 5%` AND `|delta_pct vs baseline| < 1%`
- `regression`: `delta_pct` worsened by ≥ 2% AND `mwu_p_bh < 0.01` AND `|cliffs_delta| ≥ 0.147`
- `improvement`: `delta_pct` improved by ≥ 2% AND same statistical criteria
- `noisy`: `inter_run_cv_pct ≥ 5%`; excluded from regression detection

The build fails (CI red) if any *headline-table* configuration (the ones in the paper's Table 1) is `regression` or `noisy`. Non-headline configurations are reported but do not fail the build.

---

## 17. Risks, Honest Unknowns, Out-of-Scope

### 17.1 Risks

**R1: `nvidia-smi -lgc` requires root.** On a shared CI runner, the runner user may not have passwordless sudo. Mitigation: the `lock_clocks.sh` script is idempotent and can be run once at runner setup; the harness checks the clock state at startup and warns if not locked but does not fail (Tier 4 fails; Tier 3 still runs). If the runner cannot lock clocks at all, the harness degrades gracefully but flags every run as `degraded`.

**R2: Adaptive iteration count at µs scale produces huge JSON.** 400,000 passes × 8 bytes per f64 = 3.2 MB per configuration; 72 configurations = 230 MB per run. Mitigation: the `passes` array is stored as a packed binary blob in the JSON (base64-encoded f32 array, halving size and precision loss is acceptable at µs scale). Alternatively, the harness can be configured to emit only summary stats + a histogram (100 buckets) for very-high-n configurations. Default is to emit raw; `--passes-raw=false` switches to histogram.

**R3: Mann-Whitney U at n=400,000 is computationally expensive.** The rank-based implementation is O(n log n) per comparison; with 216 comparisons, that is ~1 minute of CPU time per Tier 4 run. Mitigation: downsample for the MWU test only (use 10,000 random samples per side for the test; report `n_mwu` separately from `n_passes`). The summary stats use the full n.

**R4: Rolling baseline drift.** A 30-day rolling baseline can drift if the GPU's thermal paste degrades or the driver updates. Mitigation: the `compare_to_baseline` tool reports the baseline's own `inter_run_cv_pct`; if it exceeds 5%, the baseline is flagged `drifting` and the operator is alerted to re-establish it.

**R5: Web portability vs. clock lock tension.** The harness locks clocks on NVIDIA hardware, but the paper claims results that should hold in a browser (no clock lock). Mitigation: the paper explicitly states that the clock-locked numbers are the *ground truth* for the optimization's effect, and that browser performance will be worse in absolute terms (lower clocks) but the *deltas* should be similar because they are normalized to the Unchecked baseline.

### 17.2 Honest Unknowns

**U1: Sustained-copy kernel is not a roofline kernel.** The `sustained_copy_bw_gbps` value (expected ~14 GB/s) is well below the RTX 4060 Laptop's theoretical peak (~224 GB/s). This is because the kernel is a simple elementwise copy, not a tuned bandwidth-saturation kernel. The value is a *stable normalization constant*, not a peak-BW measurement. Anyone who reads it as peak-BW will be misled; the JSON field name (`sustained_copy_bw_gbps`, not `peak_bandwidth_gbps`) is chosen to prevent this.

**U2: The 100,000-pass cap is pragmatic, not principled.** With unlimited time, 1 second of GPU time at 2.5 µs/pass would be 400,000 passes; capping at 100,000 means we get 0.25 seconds of GPU time, which is still 10,000× more than the existing n=10. The cap exists because of host-overhead in the resolve round-trip (§7.2). A future optimization (batching resolves, e.g. resolving 100 passes' worth of timestamps into one buffer before mapping) could raise the cap to 1,000,000+.

**U3: Cliff's δ threshold (0.147) is from social science, not GPU benchmarking.** The Romano et al. (2006) thresholds were calibrated for behavioral-science effect sizes. There is no published GPU-benchmarking-specific effect-size threshold. 0.147 is conservative (small) — if GPU benchmarks typically have larger effect sizes, the threshold is too lenient; if smaller, too strict. We adopt 0.147 because it is the most-cited threshold in the ML-systems literature and is defensible to reviewers.

**U4: The 30-day rolling baseline assumes the GPU does not change.** If the laptop's thermal paste degrades over 30 days, the baseline drifts. The `drifting` flag (§8.3) catches this but does not correct it. Re-establishing the baseline requires a manual `--rebaseline` run.

**U5: Multi-vendor results are not in v1.** All numbers are RTX 4060 Laptop. The harness is vendor-agnostic via wgpu, but the clock-lock script is NVIDIA-specific (AMD uses `rocm-smi`, Intel uses `intel_gpu_top`). Multi-vendor support is future work (§17.3).

### 17.3 Out-of-Scope (Future Work)

**F1: SYCL + IRCE comparison kernels.** The harness architecture permits adding a SYCL kernel set and comparing Track 2 against LLVM IRCE. This requires: (a) writing SYCL equivalents of the three kernels, (b) compiling them with Clang + LLVM with `-mllvm -irce`, (c) timing them with `cudaEventRecord` (or DPC++'s `queue.submit` with profiling). The comparison would answer "is naga's bespoke interval analysis as good as LLVM's IRCE?" — the question the paper currently poses as future work.

**F2: Nsight Compute / Nsight Systems integration.** Per-kernel roofline analysis (NCU) and timeline profiling (NSYS) would give insight into *why* a kernel is slow, not just *how slow*. This requires a native CUDA path (F4) because Nsight tools do not work on Vulkan/SPIR-V.

**F3: Random interleaving of configurations.** LLVM's `--benchmark_enable_random_interleaving` (via Google Benchmark) randomizes the order of benchmarks within a run to detect thermal drift. The current harness runs configurations in a fixed order; if the GPU heats up over the run, later configurations are biased. Random interleaving would catch this. Future work.

**F4: Native CUDA cross-validation path.** Add a `--backend=cuda` flag that uses `cudaEventRecord` instead of wgpu `TIMESTAMP_QUERY`, for ground-truth cross-validation on NVIDIA hardware. If the two disagree by >5% on a sustained workload, the wgpu path has a measurement bug. This is the "Add CUDA path" option from the requirements-gathering round.

**F5: Multi-vendor testing.** AMD RDNA3 (`rocm-smi --set-clock-sweep`), Intel Arc (`intel_gpu_top`), Apple Silicon (no equivalent). Each vendor has its own clock-lock mechanism; the harness's `clock.rs` module would need a vendor abstraction.

**F6: CUDA Graph capture.** wgpu does not expose graph capture, but a future wgpu version might. If it does, the harness can adopt Triton's `do_bench_cudagraph` discipline to eliminate launch overhead from the measurement entirely.

---

## 18. References

1. Google Benchmark. *User Guide.* https://github.com/google/benchmark/blob/main/docs/user_guide.md
2. LLVM Project. *LLVM Testing Infrastructure Guide* (lit, FileCheck). https://llvm.org/docs/TestingGuide.html
3. LLVM Project. *LNT (LLVM Nightly Test).* https://llvm.org/docs/lnt/
4. Finkel, H. *Inductive Range Check Elimination (IRCE).* LLVM Developers Meeting, 2015. https://llvm.org/devmtg/2015-04/slides/Hal_Finkel_IRCE.pdf
5. Triton Project. `triton.testing.do_bench` source. https://github.com/triton-lang/triton/blob/main/python/triton/testing.py
6. NVIDIA. *CUDA C++ Best Practices Guide: Timing with CUDA Events.* https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/
7. NVIDIA. *Nsight Compute Documentation: Profiling Methodology.* https://docs.nvidia.com/nsight-compute/
8. NVIDIA. *nvidia-smi documentation: clock locking.* https://developer.nvidia.com/nvidia-system-management-interface
9. Romano, J., Kromrey, J. D., Coraggio, J., & Skowronek, J. (2006). *Appropriate statistics for ordinal level data: Should we really be using t-test and Cohen's d for evaluating group differences on the NSSE and other surveys?* Annual meeting of the Florida Association of Institutional Research.
10. Cliff, N. (1993). *Dominance statistics: Ordinal analyses to answer ordinal questions.* Psychological Bulletin, 114(3), 494–509.
11. Benjamini, Y., & Hochberg, Y. (1995). *Controlling the false discovery rate: a practical and powerful approach to multiple testing.* Journal of the Royal Statistical Society, Series B, 57(1), 289–300.
12. Wasserman, L. (2006). *All of Nonparametric Statistics.* Springer. (Mann-Whitney U implementation reference.)
13. Cousot, P. & Cousot, R. (1977). *Abstract interpretation: a unified lattice model for static analysis of programs by construction or approximation of fixpoints.* POPL.
14. W3C. *WebGPU Specification.* https://www.w3.org/TR/webgpu/
15. gfx-rs/wgpu repository. https://github.com/gfx-rs/wgpu
16. gfx-rs/wgpu Issue #6345, *[naga spv-out] Bounds check generation could be cleaner and smarter.* https://github.com/gfx-rs/wgpu/issues/6345

---

*End of Fixed Benchmarking Suite specification. Generated July 5, 2026. This document is approximately 11,000 words and supersedes the benchmarking-methodology portions of `COMPREHENSIVE_DECISION_LOG.md` §5–§6 and `NAGA_PROBLEMS_AND_IMPROVEMENTS.md` §11. Every fix is mapped to a section, every section is mapped to a reference, and every reference's adoption is documented in §11.*

