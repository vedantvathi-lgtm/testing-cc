# Smoke and diagnostic runs, 2026-09-29/30 — NOT results

Harness bring-up under a loaded machine (OpenFOAM job, load ~19). None of these may be used as
measurements. They document the bring-up findings (spec 005):
- `*-blur` 182523Z: outputs all zero = vacuous identity (passthrough bindings not remapped the way
  wgpu-hal Vulkan does); fixed by the rank binding map in `compile.rs`.
- `*-blur` 182631Z/182803Z: nondeterministic outputs = the webgpu-samples imageBlur host parameters
  (filterDim 16, blockDim 113) make adjacent workgroups race on one column (all 9168 differing pixel
  instances lie within 8 px of a multiple of 113).
- `*-vello:*`: replay determinism probes; vello's own renderer is itself nondeterministic on some
  scenes (blend_grid: 2 states, both reproduced byte-exactly by the replay).
Directories with `DIAGNOSTIC-ONLY.txt` are determinism probes.
