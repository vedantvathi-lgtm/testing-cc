# wg-oob outcome — 2026-09-27T14:37:56Z, RTX 4060 Laptop, NVIDIA 580.178.04, wgpu fork 83a90e0 (+ uncommitted stress-file fix)

Branch called: SUCCESS (per PLAN.md). Raw output: run1.log. All arms deterministic over 5 runs.

- A Unchecked (trusted load): OOB read out[0] = 999 — the OOB write a[8]=999 landed in real
  workgroup memory outside `a` and was read back. out[1..8] = 0. b intact (did NOT alias b:
  the PLAN's "plausibly aliases b" guess was wrong; noted, not a branch deviation).
- B RZSW, elision off: out[0..8] all 0, a and b intact → read→0, write dropped. Matches prediction.
- C Restrict: a[7] = 999, out[0..8] all 999 → write and reads clamped to last element. Matches.
- D RZSW, elision on: byte-identical output to B; safe_accesses=7; the two OOB-capable accesses
  (a[off], a[lid+off]) kept their OpULessThan (2 retained). Matches → no soundness issue.
- E Unchecked SPIR-V via plain create_shader_module: output == C → wgpu re-parsed the SPIR-V and
  re-emitted it with index: Restrict.

Conclusion: on workgroup memory (not covered by robustBufferAccess), A ≠ B and A ≠ C, while
hal added no clamps on the trusted path (replay: UMin 0 for A/B/D) → the protection seen in
B/C/D is Naga's own emitted check. 

DEVIATION (harness methodology): expected diff-exec / paired-vulkan-v3 to execute the SPIR-V they
hash; observed both load plain ShaderSource::SpirV, which wgpu-core re-parses
(wgpu-core/src/device/resource.rs:2388) and wgpu-hal re-emits with index: Restrict
(wgpu-hal/src/vulkan/adapter.rs ~2815) — arm E proves this on device. Replay of the T2 run's saved
SPIR-V (replay-t2.log): enabled 1 ULessThan/0 UMin → after re-lowering 1/1; disabled 4/0 → 4/1.
Consequence: in T2 the "elision enabled" arm was not check-free at execution (1 Restrict clamp in
both arms); the 105/111 ns delta is the cost of the 3 extra RZSW checks on top of that shared clamp,
not "checked vs Unchecked". Replay replicates wgpu's pipeline via naga spv-in → validate → spv-out;
the actual hal-emitted SPIR-V was not captured. Next: load via create_shader_module_trusted(unchecked)
(or passthrough) in both harnesses, re-run T2 and T3, compare.
