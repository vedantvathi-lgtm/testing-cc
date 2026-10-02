# Feature Specification: Benchmark suite that meets a rigorous benchmarking spec

**Feature Branch**: `003-benchmark-suite-spec-check`
**Created**: 2026-09-28
**Status**: Draft (requirements) + audit of the current suite (see `audit.md`)
**Input**: "spec check the benchmarking suite to check if it is good or not; here are notes about
benchmarking @benchmakring.pdf". Source notes copied to `source-benchmarking-notes.{pdf,txt}`
(title "CPU_Performance_Benchmarking_Notes.md", 6 pp, created 2026-07-26).

## Requirement sources

- **N#** = benchmarking notes (section 8–11 and the 8-point Q&A list; quoted text in
  `source-benchmarking-notes.txt`).
- **S#** = the project's own `../bounds-check-testing-framework/BENCHMARKING_STANDARD.md` v1.0
  (rules §0.1–0.8, §3 meta.json, §4 protocol).
- **G#** = GPU/WebGPU-specific requirements derived from how wgpu actually compiles shaders
  (VERIFIED 2026-09-28 in code; see audit).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A claim a reviewer cannot knock down (Priority: P1)

The author states "retained bounds checks cost X% on kernel K, elision removes it" in the IICT
poster/paper. A reviewer (all four gave Experimental Evaluation = 0) checks whether the number
measures the intended variable, survives run-to-run variance, and generalises beyond one shape.

**Independent Test**: every requirement below has a PASS verdict with an artifact path.

**Acceptance Scenarios**:
1. **Given** a result file, **When** a reader asks "what binary actually ran on the GPU?", **Then**
   the file contains the hash of the executed shader, not only of an intermediate.
2. **Given** a claimed effect of ~1%, **When** compared with the notes' 1–2% natural variance,
   **Then** the claim is backed by ≥ N independent process runs aggregated with a stated rule.
3. **Given** the suite, **When** asked "does this generalise?", **Then** ≥ 5 distinct kernel
   shapes (incl. at least one real-world kernel) are reported per shape, never pooled.

### User Story 2 - Reproducible by someone else (Priority: P2)

**Independent Test**: a second person re-runs with the recorded state and obtains per-shape
effects within the stated tolerance.

## Requirements *(mandatory)*

### Measurement validity
- **BR-01 (N10, S0.6, G1)**: The benchmark MUST measure the intended variable: the executed GPU
  binaries MUST differ only by the elision decision, and the executed binaries (as seen by the
  driver) MUST be recorded and hashed.
- **BR-02 (G2)**: The measured bounds-check policy MUST be stated, and at least one configuration
  MUST use the policy production wgpu applies on that backend (Vulkan arrays: `Restrict`).
- **BR-03 (S0.7–0.8)**: Output correctness MUST be validated before and after timing.
- **BR-04 (N10)**: The workload MUST not be dominated by something other than the variable under
  test (dispatch/barrier latency, one-workgroup occupancy) — or that domination MUST be reported.

### Statistics and variance
- **BR-05 (N8, N11.6, S4)**: Multiple iterations with a robust aggregate; within-run: paired,
  interleaved, median + bootstrap CI; across runs: ≥ 5 independent process runs aggregated
  (geometric mean of per-run ratios) with a between-run spread.
- **BR-06 (N8)**: The effect MUST be compared with an explicit variance threshold; effects below
  2% MUST show between-run spread smaller than the effect.
- **BR-07 (S4)**: A negative control (identical binaries) MUST have CI ⊇ 0, run adjacent to the
  headline measurement.
- **BR-08**: Order effects (A-first vs B-first) MUST be reported, not only averaged away.
- **BR-09 (N11.8)**: Timing-instrumentation overhead MUST be quantified.
- **BR-10 (N7)**: Warm-up adequacy MUST be demonstrated (steady state reached), and cold vs warm
  state recorded.

### Environment control
- **BR-11 (N11.1, N11.5, S3)**: Full system state per run: GPU name/driver/clocks/pstate/temps
  *during* the run, CPU model + governor, memory, kernel, power source (AC/battery), other GPU
  processes, display attached to GPU, background load.
- **BR-12 (N11 "overhead and noise")**: Background activity MUST be minimised and verified (no
  other GPU compute apps; no monitoring daemons during runs).
- **BR-13 (N9 power/perf)**: Clock/power policy MUST be fixed or recorded continuously (laptop GPU).
- **BR-14 (S3)**: `meta.json` provenance block per run (sources incl. dirty diff, machine incl.
  `cpu_governor`, harness version).

### Coverage and generality
- **BR-15 (N9, N11.7, R1 review)**: ≥ 5 kernel shapes incl. real-world-derived kernels (e.g. from
  vello/webgpu-samples), multiple input sizes; results per shape, geometric mean across shapes
  only as a summary.
- **BR-16 (N11.2–3)**: Change one variable at a time; same compiler/toolchain/build flags; the
  toolchain and naga revision recorded.
- **BR-17 (S0.5)**: Cross-platform claims (other GPUs/backends) only with per-platform runs.

### Process
- **BR-18 (N11 "planning")**: A written run plan with expected outcomes exists before runs.
- **BR-19 (S0.2)**: Raw ordered samples persisted with round id and order.

## Success Criteria *(mandatory)*
- **SC-001**: 19/19 requirements PASS with artifact evidence before any timing number is used as
  a headline.
- **SC-002**: For each kernel shape, the between-run spread (min–max of ≥ 5 runs) is smaller than
  the reported effect, or the shape is reported as "not resolvable".
- **SC-003**: The executed-binary hash in each result matches a binary captured from the driver
  path (not the harness's pre-wgpu intermediate).

## Assumptions
- Hardware is the single RTX 4060 Laptop; root-only clock locking (`nvidia-smi -lgc`) may be
  unavailable, so BR-13 may be met by continuous recording instead of locking.
- The notes are CPU-oriented; items specific to CPUs (NUMA, CCDs, allocators, huge pages) are
  out of scope for a GPU shader benchmark and are listed as N/A in the audit.
