# Tasks: bring the benchmark suite to spec 003 (harness v4)

Paths relative to `../bounds-check-testing-framework/`. Nothing here has been executed yet.

## Phase 1: Fix what is measured (BR-01, BR-02) — blocks all timing claims
- [ ] T001 Add a replay/capture step: hash the SPIR-V that wgpu-hal hands to Vulkan for each variant; store in `runs/<id>/artifacts/executed-*.spv` (reuse `scratchpad/wgpu-replay` logic, or `WGPU_*` debug dumping if available)
- [ ] T002 Switch submission to `ShaderSource::Naga` with per-variant validation (elision on/off), so production policies (Restrict, Vulkan) are what ran; keep RZSW as a secondary, labelled configuration
- [ ] T003 Test: executed-binary check counts must differ between variants and the "on" variant must contain no clamp on proven accesses; fail the run otherwise

## Phase 2: Environment record (BR-11–14)
- [ ] T004 Write `meta.json` per standard §3: sources (+ rustc/cargo versions), machine (CPU model, governor, memory), AC/battery state, display attachment, `nvidia-smi --query-compute-apps`
- [ ] T005 Sample `nvidia-smi` (pstate, clocks, temp) every 250 ms in a background thread during the run; store the series; flag runs with clock changes inside the headline phase
- [ ] T006 Pre-run noise gate: refuse to run if other GPU compute apps are present; record load average

## Phase 3: Statistics (BR-05–10)
- [ ] T007 Report order-split deltas and an order-effect estimate
- [ ] T008 Runner script: ≥5 independent process runs per kernel; aggregate with geometric mean of per-run (disabled/enabled) ratios and report min–max between-run spread
- [ ] T009 Instrumentation overhead: empty-kernel dispatch baseline + timestamp-only pass
- [ ] T010 Warm-up convergence test (median of rolling windows stabilises within 0.1%) recorded in results

## Phase 4: Coverage (BR-04, BR-15)
- [ ] T011 Kernel set ≥5: fixed-array scan (current), matmul tile, reduction, prefix scan, one vello-derived kernel (binning-like workgroup bitmap), each at full-occupancy and 1-workgroup sizes
- [ ] T012 Results per kernel; summary geometric mean only as a secondary line

## Phase 5: Record
- [ ] T013 Re-label the 2026-09-13 numbers in the ledger with the C1 caveat; update poster wording
