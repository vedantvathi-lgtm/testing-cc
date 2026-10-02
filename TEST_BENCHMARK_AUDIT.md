# Test and Benchmark Audit

Updated: 2026-08-27

## Executive finding

The existing suites contain useful fixtures and working GPU timestamp code, but they are not yet a trustworthy publication baseline. The tests often identify a bounds check by searching the entire SPIR-V disassembly for an opcode substring. The benchmark repositories contain two incompatible harnesses and twenty legacy result files without compiler provenance. Existing paper numbers must be treated as provisional until the repairs and fresh runs listed below are complete.

## Test-suite findings

### Critical

1. `bounds_check_stress.rs` usually treats any `UMin`, `ULessThan`, or branch anywhere in a module as evidence for the access under test. An unrelated access, nested array dimension, or initialization branch can satisfy the assertion.
2. The loop implementation marks index-expression handles function-wide. Existing tests do not cover accesses before a guard, after a loop, in a sibling branch, or after mutation. Those are the highest-risk false-elision cases.
3. Several tests assert only that no `UMin` exists. That validates the `Restrict` backend shape, not the validator proof itself, and does not test `ReadZeroSkipWrite` or other backends.

### High

4. `nested_loops_2d_array` checks only that the module contains no `UMin`; it cannot identify which array dimension was proved.
5. Runtime-array retention accepts `OpSelectionMerge`, but workgroup initialization and unrelated control flow can also emit selection instructions.
6. `dual_array_lid_accesses_both_elided` compares aggregate opcode counts across policies. Equal counts do not prove both intended accesses were elided.
7. The test helpers compile directly to SPIR-V but do not first assert which exact Naga `Access` handles the validator marked safe.
8. No randomized or differential-execution test searches for false elisions over integer boundary values.

### Repair standard

- Validator tests must locate exact `Expression::Access` handles and assert access-specific proof membership.
- Backend tests must use minimal one-access shaders or structurally inspect the SPIR-V dataflow feeding the exact `OpAccessChain`.
- Every positive test needs a nearby negative boundary case.
- Every path-sensitive rule needs before-path, sibling-path, post-path, and mutation-kill tests.
- Tests must cover both `Restrict` and `ReadZeroSkipWrite`; cross-backend claims require MSL and HLSL assertions.

## Benchmark findings

### Critical

1. All twenty legacy JSON files lack exact Naga/WGPU source revisions. They cannot establish which compiler implementation produced a result.
2. Eighteen legacy files use only ten samples per configuration.
3. `bounds-check-bench` depends on `/home/bb/Projects/naga`, while the current paper implementation lives in `Dave-Sonar-WGPU--master/wgpu/naga`. Results can silently benchmark the wrong implementation lineage.
4. The paired matmul harness creates and destroys a shader module, pipeline, descriptor resources, command pool resources, query pool, and fence for every trial. GPU timestamps exclude most host setup, but repeated pipeline creation and allocation can change device state and make the workload unlike steady-state dispatch.
5. The harnesses do not perform result-buffer readback and correctness validation for every measured configuration. A compiler or shader error could be timed as a fast but incorrect result.
6. `bounds-check-bench` claims interleaved A/B measurement, but the main loop measures all 200 samples for one policy before constructing and measuring the next policy. Its permutation and bootstrap comparisons therefore operate on blocked, drift-prone samples.
7. `compile_wgsl_with_policy` stores `spv.len()` in a field named `bounds_checks_emitted`. This is SPIR-V word count, not an instruction count and not a count of emitted bounds checks.
8. `compute_buffer_bytes` allocates only one output float per workgroup for every non-matmul kernel. `prefix_scan` and `track3b_test` write one float per invocation, so most output writes are out of bounds. `Restrict`, `ReadZeroSkipWrite`, and `Unchecked` then execute materially different workloads and semantics.
9. Comparing `Unchecked`, `ReadZeroSkipWrite`, and `Restrict` does not measure the effect of the elision analysis. It measures different bounds-check policies. A valid BCE experiment needs the same policy and shader compiled with the analysis enabled and disabled.
10. The paired matmul shader writes only workgroup memory. It has no storage output or other externally observable result, so a driver is free to eliminate most or all of the workload.
11. The paired matmul `retained` shader indexes `B[k + 1]` while `k` reaches 1023 for `array<..., 1024>`. Its `Unchecked` arm has an actual out-of-bounds access and is not semantically equivalent to the elided shader.

### High

6. The paired harness has a negative control, but it is a separate binary and its result is not persisted beside the main run with shared provenance.
7. Warmup is fixed at twenty rounds in the paired harness and does not verify steady state.
8. Bootstrap confidence intervals use a simple percentile bootstrap with 2,000 resamples. This is usable for exploration but should be raised and documented; discreteness from timestamp resolution must be reported.
9. `bounds-check-bench` labels its package version as `wgpu_version`, producing the meaningless value `0.1.0`.
10. Static SPIR-V word counts and raw per-round timing samples are not consistently persisted in the legacy results.
11. Multiple-testing machinery exists in the newer monolithic harness, but the scientific question is primarily paired within-round differences. Unpaired permutation and effect-size calculations must not replace the paired estimator.
12. CUDA/SYCL comparison utilities use different timing and workload conventions and are not an apples-to-apples baseline for the current paper.
13. The monolithic harness creates separate dispatch counts through per-policy calibration, then compares per-dispatch summaries. Batch size, device state, and ordering can differ across the compared configurations.
14. Its L2 invalidation runs once before an entire configuration, not between paired observations. It does not repair the blocked-order design and adds a large, unquantified thermal perturbation.
15. The Anderson-Darling implementation uses the same ordered sample in both logarithmic terms instead of pairing the lower and upper order statistics. The fixed `0.752` threshold also ignores parameter-estimation and finite-sample corrections.
16. Median definitions differ inside the same harness: `summarize` uses a lower order statistic for even sample counts, while comparison helpers average the two central values.
17. Roofline byte counts omit repeated storage traffic in reduction/scan kernels and treat working-set size as bytes transferred for tiled matmul. The resulting achieved-bandwidth and bound classifications are not defensible roofline measurements.
18. Timestamp deltas use wrapping or saturating subtraction without validating counter width or rejecting invalid samples. A counter anomaly can become a huge value or a silent zero.
19. The paired Vulkan harness creates and destroys pipelines, query pools, command buffers, and fences for every observation. Although host setup is outside the GPU timestamps, first-use state and repeated allocation make the experiment unlike steady-state dispatch.
20. The paired and negative-control binaries duplicate nearly all Vulkan and statistics code, so a methodology fix can silently reach one and not the other.

### Repair standard

- One schema-versioned runner and one run directory per hypothesis.
- Exact git revision, dirty state, compiler flags, GPU, driver, timestamp period, and thermal/power metadata.
- Pre-create pipelines and reusable resources outside timed rounds.
- Validate output buffers before timing and after the run.
- Interleave configurations with randomized or balanced order.
- Persist raw paired samples and analyze paired differences.
- Run and persist an identical-binary negative control for every harness revision.
- Store emitted-check counts and SPIR-V words with the timing data.
- Do not cite legacy runs as headline evidence.
- Compare analysis-enabled and analysis-disabled builds under the same bounds policy.
- Require semantically identical shaders with externally observable outputs; no undefined `Unchecked` accesses.
- Unit-test kernel buffer sizes, dispatch coverage, statistics, SPIR-V check classification, and schema validation before any GPU run.

## Evidence status

- Legacy monolithic results: historical only; invalid for headline performance claims.
- Existing paired matmul results: invalid for BCE cost because the workload is unobservable and the retained/unchecked arm is out of bounds.
- Existing negative control: useful warning signal, but not publication evidence because it is not persisted with shared provenance and uses duplicated runner code.
- Fresh results may be cited only after the replacement runner passes its CPU unit tests, GPU correctness gate, identical-binary negative control, and enabled/disabled code-difference audit.
