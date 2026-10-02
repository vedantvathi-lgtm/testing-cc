# wg-oob — workgroup-array OOB policy test (written BEFORE running, 2026-09-27)

Kernel: wg_size 8; var<workgroup> a,b: array<u32,8>; a[lid]=100+lid, b[lid]=200+lid; barrier;
off = params[0] (=8, runtime so the compiler cannot fold it); lid==0 writes a[off]=999 (OOB);
barrier; out[lid]=a[lid+off] (OOB reads a[8..15]); out[8+lid]=a[lid]; out[16+lid]=b[lid].
Workgroup memory is NOT covered by Vulkan robustBufferAccess → only Naga's check can protect it.
All arms loaded with create_shader_module_trusted(ShaderRuntimeChecks::unchecked()) so wgpu-hal
adds NO clamps of its own (it re-parses SPIR-V and would otherwise re-emit with Restrict).

Arms: A Unchecked | B RZSW, elision off | C Restrict, elision off | D RZSW, elision on |
      E Unchecked SPIR-V via plain create_shader_module (the path diff-exec/paired-v3 use)

Predictions:
 B: out[0..8]=0; a=100..107; b=200..207   (read→0, write dropped)
 C: a[7]=999 (write clamped to last); out[0..8]=999 (reads clamped to a[7]); b intact
 D: == B exactly (a[off], a[lid+off] must be retained; only lid-indexed accesses elide)
 A: undefined — plausibly reads alias b (out[0..8]=b) and write lands in b[0]
 E: == C (wgpu-hal re-emits with index: Restrict)

Branches:
 SUCCESS : A shows an OOB effect (≠ B and ≠ C); B, C, D match; E == C → the protection is Naga's own check; harness re-lowering confirmed.
 NORMAL  : A shows no visible OOB effect → attribution inconclusive; record; try larger arrays/another layout.
 BAD     : B or C deviate from the policy semantics → investigate naga policy lowering before any claim.
 DISASTER: D ≠ B → elision soundness bug → stop, minimise, record in ledger.
