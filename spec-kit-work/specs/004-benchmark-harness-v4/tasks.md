# Tasks: benchmark harness v4 (paired-vulkan-4)

Paths relative to `../bounds-check-testing-framework/`.

## Build
- [x] T001 New crate `paired-vulkan-v4/` (v3 untouched, frozen evidence)
- [x] T002 Passthrough submission (`create_shader_module_passthrough`); per-slot sha256 of the exact words (FR-1, audit C1)
- [x] T003 wgpu-hal Vulkan production options: index Restrict, buffer Unchecked, force_loop_bounding, native zero-init, SPIR-V 1.6 (FR-2)
- [x] T004 7 u32 kernels with exact CPU references (`src/kernels.rs`) (FR-3)
- [x] T005 4-slot rotating interleave; correctness before/after for every slot (FR-4, FR-3)
- [x] T006 Noise gate (no other GPU compute apps), `meta.json`, continuous `nvidia-smi` telemetry at 200 ms (FR-6)

## Question → answer iterations during execution
- [x] T007 Smoke run (4 warm-up / 8 rounds): all kernels exact; found a mid-run clock step (all slots −17% at round 5) → Q: are ns differences valid? A: no — switched headline to per-round ratios (clock shifts cancel within a round; verified 404.9/349.7 = 335.6/289.9 = 1.158)
- [x] T008 Warm-up changed to steady-state rule (≥ 30 rounds, ≥ 5 s, last two 10-round windows within 0.5%, cap 60 s); measured window timestamps recorded for telemetry alignment
- [x] T009 First campaign run of `empty`: identical pipelines differ by 0.13% with CI excluding 1 → Q: is the strict CI rule usable at this precision? A: no — equivalence margin ±0.5% declared before inspecting campaign kernel results; strict counts still reported
- [x] T010 Mechanism check: SPIR-V diff no_bce vs bce = only `UMin` clamps (+ ID renumbering)

## Campaign & report
- [ ] T011 `N=10 run_campaign.sh`: 10 independent processes × 7 kernels, order reshuffled per cycle
- [ ] T012 `analyze_v4.py` → per-kernel table + `runs-v4/summary.json` (verdict rule Q8 with margin)
- [ ] T013 Telemetry check: clocks/pstates/temperatures inside each measured window
- [ ] T014 Report `reports/Benchmark v4 results.md`; ledger + session log; poster numbers only from `summary.json`
