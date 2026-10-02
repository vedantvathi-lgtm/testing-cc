# Feature Specification: Real-world shader and kernel benchmark

**Created**: 2026-09-29 · **Status**: In progress
**Inputs** (binding):
1. IICT reviews (`../../../IICT_CONTEXT/02-REVIEWS.md`): Experimental Evaluation = 0 from all four
   reviewers; R1 "larger, more diverse benchmark suite"; R2 "thin evaluation (one minimal shader +
   one matmul; no real corpus)"; R3 "without extensive experiments, it is unclear if the selected
   patterns cover enough optimisation opportunities".
2. The v4 results (`../../../reports/Benchmark v4 results.md`) are on *synthetic* kernels that were
   also used to find the gaps features 005/006 close. That is a selection bias the addendum states
   explicitly. This spec answers it with shaders the project did not write.
3. The benchmarking notes and the v4 method (`../004-benchmark-harness-v4/spec.md`), unchanged:
   passthrough SPIR-V, production Vulkan policy, 4 rotating slots, per-round ratios, noise +
   provenance gates, telemetry, ≥ 5 processes per workload.

## Step 0 — production-binary screen (done 2026-09-29, before any timing)

Question: under wgpu-hal's production Vulkan options (index Restrict, buffer Unchecked [RBA2],
image_load Unchecked, force_loop_bounding, native zero-init, SPIR-V 1.6), which real-world entry
points get a *different binary* when elision is on? Only those can show any runtime effect.

Tool: `bce-prod-screen` (compiles every entry point of the 98 census files twice; counts
`OpExtInst GLSL.std.450 UMin`, id 38; hashes the words). Validated on the v4 `reduce_tree` kernel:
4 → 0, matching the harness's own statics.

Result (wgpu `19ffb4e`, artifact `screen-19ffb4e.jsonl` in this directory; tool source `tools/prod-screen.rs`;
corpus files were the census's sparse clones and preprocessed copies, paths as recorded in the jsonl):
- 147 entry points compiled, **28 change their binary**.
- 5 compile failures: 2 need override processing, 3 hit an unimplemented SPIR-V writer path.
- By corpus: vello compute 14/18 entry points change, UMin **193 → 93**; webgpu-samples 4 change
  (compute `blur` 4 → 1); rerun 5 change (fragment `rectangle_fs` 10 → 1 over 3 entries); bevy 2
  change (`downsample_depth_first` 60 → 58); wgpu-upstream 2 (test shaders).

## Workloads (chosen from the screen, not from the analysis's strengths)

| ID | Workload | Why | Inputs |
|---|---|---|---|
| RW1 | **vello end-to-end render** of real scenes (all pipeline stages) | largest real-world binary change (100 clamps); real 2D renderer used by Xilem/Linebender | vello's own test scenes (SVG, text); output = rendered image |
| RW2 | **webgpu-samples imageBlur** (`blur.wgsl`), unmodified | canonical WebGPU sample; hot workgroup-tile access in the filter loop (4 → 1) | the sample's own image asset; the sample's parameter defaults |
| RW3 | bevy `downsample_depth_first` (60 → 58) | real engine kernel with a tiny change: expected ~null, a real-world near-control | synthetic depth pyramid input |
| RW4 | rerun `rectangle_fs` (fragment, 10 → 1) | a real fragment workload | textured quad; stretch goal |

## Expected outcomes, per workload, written before running (AGENTS.md Rule 3)

- **Success**: the effect (no_bce/bce) is resolved (control within ±0.5%, all runs same sign, effect
  > max(2× control deviation, 0.5%)), outputs identical across no_bce/bce/bce_dup, and bce/unchecked
  bounds the remaining gap. Next: report it per GPU.
- **Normal/partial**: effect not resolvable (< 0.5%), or resolved on one GPU only. This is the most
  likely outcome for RW3. It is a real answer: "the elision changes the binary, but not the time,
  on this workload". Next: report it as such and do not drop the workload.
- **Bad**: outputs differ between no_bce and bce. That would be a miscompile or unsoundness. Next:
  stop all timing, bisect the differing stage, treat it as a soundness incident (0.8 protocol).
- **Disaster**: the workload cannot be built on the fork (dependency or API incompatibility), or
  the GPU cannot run it. Next: record the error text verbatim, fall back to the stage-level harness
  with inputs captured from the real renderer, and state the downgrade in the report.

## Requirements
- **FR-1** Shader sources are unmodified upstream files at pinned commits; any preprocessing is the
  project's own (vello `#import`/`#ifdef`) and is recorded.
- **FR-2** Same slots, policy, statistics, gates and meta as v4.
- **FR-3** Correctness oracle: byte-identical outputs between no_bce, bce and bce_dup; unchecked
  compared and reported.
- **FR-4** Report per stage where the workload has stages (vello), plus end-to-end.
- **FR-5** Numbers are copied by script from result files into the report.

## Bring-up findings (2026-09-29/30, before any campaign number; evidence in `bounds-check-testing-framework/runs-realworld/_smoke-20260929/`)

Each is VERIFIED unless labelled otherwise.

1. **wgpu-hal Vulkan renumbers descriptor bindings.** The k-th entry of a bind group layout (entries
   sorted by binding, `wgpu-core/src/device/bgl.rs` `from_entries`) becomes Vulkan binding k
   (`wgpu-hal/src/vulkan/device.rs:1417-1456`), and naga receives that map when it emits SPIR-V
   (`device.rs:1611-1631`). Passthrough SPIR-V with raw WGSL binding numbers therefore reads the wrong
   descriptors when a group's bindings are not 0..n-1 (blur group 1: 1,2,3 → output all zero, a vacuous
   "identical outputs" pass caught by the non-zero check). `realworld-v1/src/compile.rs` reproduces the map.
   The v4 kernels (bindings 0,1) were unaffected.
2. **webgpu-samples imageBlur ships a race and an out-of-bounds read.** `main.ts` passes
   `filterDim = filterSize + 1` (16) and `blockDim = 128 - filterSize` (113). The shader (`blur.wgsl`)
   then reads `tile[r][center + f - filterOffset]` up to index 128 (array length 128) and each workgroup
   writes 114 columns while advancing 113, so neighbours write one shared column with different values.
   Evidence: 20/20 distinct outputs per slot; all 9,168 differing pixel instances across 28 output pairs
   lie within 8 px of a multiple of 113 (0 outside). With the shader's documented contract
   (`blur.wgsl:26-28`: filterDim 15, blockDim 114) outputs are deterministic (1 distinct in 20 runs, every
   slot). Upstream report: not filed (outward action; for the user to decide).
3. **The elision analysis did not prove that out-of-bounds access.** On `blur.wgsl` it proves `tile[r]`
   (twice) and `tile[r][4*lid+c]` (max index 127) and leaves `tile[r][i]` unproven; the index really does
   reach 128. A real-world soundness datapoint (no false proof on a shader with a genuine OOB).
4. **Native vello disables all runtime checks.** vello (`research/vello_research/src/wgpu_engine.rs:848-853`
   at a9f11bb) creates every module with `create_shader_module_trusted(..,
   ShaderRuntimeChecks::unchecked())` and pipelines with `zero_initialize_workgroup_memory: false`. Native
   vello has no bounds checks for elision to remove. Checks exist for vello only as web content: the
   WebGPU `GPUShaderModuleDescriptor` has no way to disable them (`references/raw/specs/WebGPU.html`,
   members `code`, `compilationHints`), and wgpu's own browser backend ignores the argument
   (`wgpu/src/backend/webgpu.rs:1829`, `_shader_runtime_checks`). Slot mapping: `no_bce` = vello in a
   wgpu-based browser today (e.g. Firefox; its exact runtime-check settings are UNKNOWN from this repo),
   `bce` = the same with this project's elision, `unchecked` ≈ vello native.
5. **vello's own renderer is not bit-deterministic on every scene.** Over 40 renders per scene
   (`captures/vello-a9f11bb-r40/*.refs*.json`): NVIDIA 28/34 scenes one image, 6 scenes 2–40 images
   (longpathdash: 40 and 39); Intel 25/33 one image, 8 scenes 4–39 images. blend_grid: vello produced
   exactly the two images the replay produces (sha256 e6e06ee4…, f449b8ec…), so the replay is byte-faithful.
   The output oracle for vello is therefore a reference-set oracle with a leave-one-out threshold
   (`realworld-v1/src/main.rs` `ref_oracle`), references per adapter.
6. **Replay fidelity.** Recordings regenerate identically across captures for 33/34 scenes (payload bytes
   and id-normalised commands); mmark generates a random instance each time (its capture is fixed).
7. **Vello production-binary clamps.** All 20 vello shaders together: UMin 197 (no_bce) → 95 (bce); the
   `unchecked` build still has 11 UMin, which are shader-authored `min()` calls. Bounds clamps: 186 → 84.

## Pre-registration 2 (written 2026-09-30, BEFORE the confirmatory campaign; nothing of it has been run yet)

**Why.** Diagnosis of the first campaign's NVIDIA data (no new timing):
- per-round timings are right-skewed: ~1 in 5 samples of any slot is 20-40% slower;
- the per-round standard deviation of log(bce_dup/bce) for identical binaries is ~15%;
- telemetry in one run shows the graphics clock at 495-2610 MHz and utilisation at 48% median.

Cause: the harness submits one slot's sample, waits for it, then submits the next. The GPU idles in every gap and
changes clock, so the 4 slots of a round do not see the same clock state. That breaks the premise of per-round
ratios. The old verdict rule then judged effects against the worst run's control deviation, with a fixed 0.5% floor.

**Harness change (realworld-2).** All 4 slots of a round are recorded back to back in one command encoder, with
8 timestamps, and sent in one submission with one wait. Slot order still rotates every round. No other change:
same binaries, policy, oracle, gates and reps target.

**Verdict rule v2** (primary; v4 rules still reported for continuity). Per run *i*:
- e_i = median over rounds of ln(no_bce/bce);
- c_i = median over rounds of ln(bce_dup/bce) (identical binaries);
- d_i = e_i − c_i, the control-corrected effect.

Across the N runs of a workload per GPU: mean(d) with a two-sided 95% t-interval (N−1 degrees of freedom).
- **GAIN**: the whole interval is above 0.
- **LOSS**: the whole interval is below 0.
- **EQUIVALENT**: the whole interval is inside ±0.5% (ln(1.005)); the equivalence bound is the v4 margin, now used
  only for "no meaningful effect" claims (TOST-style).
- **INCONCLUSIVE**: otherwise.
- A run with a failed output oracle is excluded and counted; if more than one run is excluded, the verdict is INVALID.
- Reported effect: exp(mean d) with its interval. Suite: geometric mean of per-scene exp(mean d).

**Expected outcomes.**
- *Success*: NVIDIA per-round control spread falls by at least 5x (from ~15% to under 3%), most vello scenes reach
  GAIN / LOSS / EQUIVALENT. Next: report both GPUs under rule v2.
- *Partial*: the spread falls but many scenes stay INCONCLUSIVE. Next: report as is; consider more runs.
- *Bad*: the spread does not fall. Then the noise is not caused by idle gaps. Next: keep the rule-v2 analysis,
  record the hypothesis as refuted, investigate clocks (locking needs root: UNKNOWN whether allowed).
- *Disaster*: the batched submission changes outputs or breaks the oracle. Next: stop, revert to realworld-1.

**Scope of the rerun**: the same 38 workloads, NVIDIA × 10, then Intel × 5, same campaign script. Rule v2 is also
applied post hoc to the first campaign, and that analysis is labelled post hoc.
