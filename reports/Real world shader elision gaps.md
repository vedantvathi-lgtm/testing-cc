# Real world shader elision gaps

**Question:** What do real-world shaders actually look like at the points where bounds checks
are inserted? Why does the Naga elision analysis miss most of them? Which improvements are
worth building?

**Date:** 2026-09-27.
**Method:** A per-access census tool (`tools/access-census.rs`). For every runtime (non-constant)
array access it records the source line, what is being indexed, the index-expression tree, and
whether the analysis proved it safe. It uses the same "runtime access" definition as the
project's corpus harness, and it reproduces that harness exactly on the shared corpora:
- webgpu-samples: 66 files, 92 accesses, 1 elided;
- wgpu: 76 files, 79 accesses, 2 elided.

Candidate fixes were **prototyped and measured**, not estimated. Your repositories were not
modified: prototypes live in an exported scratch copy, and the diff is in `tools/`.
All data is under `research_notes/Real world shader elision gaps/`. Evidence labels follow
AGENTS.md: everything below is VERIFIED this session unless marked otherwise.

---

## Summary

1. **The real-world corpus is 729 runtime accesses across 9 projects**, dominated by
   **vello** (440 accesses, 20 production compute shaders). The committed analysis elides **78
   (10.7%)**.
2. **The biggest miss was a bug, not a missing feature.** One conditional comparing two
   variables makes the analysis discard everything for the rest of the function. Fixing it (F1)
   takes vello from 68 to 94 elisions (+38%), and real-world elisions from 78 to 105.
3. **The same bug is a soundness hole on non-WGSL input.** GLSL `if (a < b && …) { i = 100u; }`
   → `t[i]` is marked safe and its SPIR-V check is deleted. WGSL is unaffected. F1 fixes this.
4. **Cheap operator support (F3: `%`, `/`, `>>`, `&`, `min`/`max`/`clamp`, `select`,
   int conversion)** adds 11 more real-world elisions. All three prototypes together (F1–F4)
   give **117 of 729 (16.0%)**, and the full naga test suite still passes.
5. **The ceiling is structural.**
   - 46% of real-world accesses index runtime-sized storage buffers bounded only by a host-side
     count, which a shader compiler cannot prove.
   - 7% use vertex/instance indices, which are also unprovable.
   - 3% hit binding arrays, which wgpu never checks.

   So **at most ~44% (320 of 729) of real-world accesses are provable in principle**, and the prototypes
   now reach 16%.

---

## 1. Corpus

| Corpus | What it is | Shaders analyzed | Runtime accesses |
|---|---|---|---|
| vello (`a9f11bbe`) | GPU 2D renderer; all-compute pipeline | 20 (after inlining `#import`) | **440** |
| bevy (`0f38358f`) | Game engine. **Migrated to WESL** (211 `.wesl`, 0 `.wgsl`) | 10 of 39 compute (see limits) | 93 |
| webgpu-samples (`4944c705`) | Official WebGPU samples | 66 | 92 |
| wgpu upstream (`688e4761`) | examples, benches, tests | 76 | 79 |
| rerun (`8f6c4edb`) | Viewer renderer (`#import <…>` inlined) | 40 | 11 |
| maplibre-rs, egui, iced, wgpu-native | render shaders | 31 | 14 |
| **Total real-world** | | | **729** |
| *(naga test suite, project corpus, for contrast)* | | 153 | 587 |

Could not be analyzed (recorded, not substituted):
- **wonnx:** Tera templates (718 template tags).
- **ratchet:** 0 WGSL files at HEAD.
- **zed, ruffle, pixijs, iced:** parts are render shaders concatenated with a runtime prelude.
- **29 of 39 bevy compute shaders:** see §6.
- **Runtime-generated ML shaders** (TF.js, ONNX Runtime Web): these never exist as files. This is a known blind spot, because gpuweb#1202 shows ML kernels are where clamping hurt most.

## 2. What real-world accesses look like

### 2.1 By the thing being indexed (committed analysis)

| Indexed object | Accesses | Elided |
|---|---|---|
| `array<T>` runtime-sized, `storage` (read or read-write) | **364 (50%)** | 0 |
| fixed array, `workgroup` | 215 (29%) | 44 |
| fixed array, `function` (local) | 76 (10%) | 27 |
| fixed array, value / private / uniform | 41 | 3 |
| binding arrays | 23 | 0 |
| vectors | 8 | 4 |

By stage: compute 413 (68 elided), helper functions 196 (10), vertex 66 (0), fragment 50 (0).

### 2.2 Why the rest are missed (after the F1 fix), by root cause

| Share | Root cause | Provable by a shader compiler? |
|---|---|---|
| **46.2%** (337) | Runtime-sized storage array, index bounded only by a host-side quantity (e.g. `if gid.x < config.n`) | **No.** The buffer length is not tied to `config.n` inside the shader |
| 10.7% (78) | Index is a **helper-function parameter** | Yes, with interprocedural ranges. **But 78 of 79 fixed-array cases are one bevy file** (§4.3) |
| 8.2% (60) | Locals/loops not proven: pointer-chasing (`link = sh_link[link]`), data-dependent indices, compound `&&` loop conditions, bounds that are themselves variables | Partly |
| 6.7% (49) | `vertex_index` / `instance_index` | **No** (draw-call dependent), unless reduced by `%` |
| 3.2% (23) | Binding arrays | Irrelevant: every wgpu backend uses `Unchecked` for them |
| 3.4% (25) | Shifts, `/`, `%`, `&`, calls | Yes: this is F3 |
| 2.7% (20) | Runtime array in a file that uses `arrayLength` | Some. Only accesses to the *same* buffer the guard names |
| 2.1% (15) | Builtin/parameter arithmetic, user varyings | Mostly no (interpolated inputs) |

Source: `census/categorized_F1.json`.

### 2.3 Where Naga's checks actually matter in production (from `wgpu-hal` source at `83a90e0`)

| Backend | Arrays (`index` policy) | Storage buffers (`buffer` policy) |
|---|---|---|
| Vulkan | `Restrict` | `Unchecked` if the device has `robustBufferAccess2`, else `Restrict` (`vulkan/adapter.rs:2813-2820`) |
| Metal | `Restrict` | **`Restrict`**: Metal has no driver robustness (`metal/device.rs:169-202`) |
| DX12 | HLSL `restrict_indexing` | (not examined) |
| GLES | `Unchecked` | `Unchecked` |

**Consequence:**
- Workgroup and local array elision pays off on every backend.
- Storage-buffer elision pays off mainly on **Metal** (wgpu on macOS/iOS; Firefox on macOS) and on Vulkan devices without `robustBufferAccess2`.
- The paper's measurements use `ReadZeroSkipWrite` on Vulkan, which is not wgpu's production configuration. The paper should say so.

## 3. Finding: the "two-variable comparison" bug (F1)

**Where:** `refine()` in `naga/src/proc/bounds_check_elision.rs`.
`self.expression_range(right, values)?.exact()?` returns `None` whenever the other side of a
comparison has no exact constant value. In this analysis `None` means **"this branch is
infeasible"**.

**Effect 1: precision (WGSL).**
- For `if a < b { … }` with non-constant `a` and `b`, both branches are "infeasible". The `If` handler then treats the rest of the function as unreachable and proves nothing more.
- In vello this silently disabled elision after the first such `if`. Bisection found `if x0 == x1 { y1 = y0; }` in `binning.wgsl:296`.
- A minimal repro keeps the check on an unrelated `sh[local_id.x]` later in the function: `if a < b {}` gives 1 → 1 `OpULessThan`, while `if a < 5 {}` gives 0 → 1.

**Effect 2: soundness (GLSL, and probably SPIR-V input).**
- In the `(LogicalAnd, true)` arm, the `?` makes only the *true* branch "infeasible". The analysis then ignores assignments in that branch.
- The WGSL front end lowers `&&` to control flow (`front/wgsl/lower/mod.rs:2564-2591`), so WGSL never reaches this arm. That's why every test and fixture passed.
- The GLSL front end emits `Binary LogicalAnd`. Repro (`logs/audit-20260927-UNSOUND-logicaland-glsl.log`):
  ```glsl
  shared float t[8];  uint i = 0u;
  if (a < b && b > 0) { i = 100u; }
  t[i] = 1.0;      // marked SAFE; SPIR-V OpULessThan 1 → 0 with elision on
  ```
- The analysis cannot see `i = 100` on this path, so the out-of-bounds workgroup write goes unchecked.
- I didn't execute this on the GPU; out-of-bounds workgroup writes are undefined behavior.
- The SPIR-V input path (`OpLogicalAnd` → naga `LogicalAnd`) is **likely** affected, but that is UNKNOWN (not tested).

**Fix (F1):**
- `None` is returned only when a refined range is genuinely empty. "Cannot model" now returns the facts unchanged.
- Verified: the GLSL repro keeps its check; naga suite 142 + 272 + 6 pass; project corpus unchanged at 435; real-world 78 → 105.

This is a `DEVIATION` against the ledger claim "no unsound elision found". Soundness holds for
WGSL input only until F1 lands.

## 4. Measured improvements (prototypes on an exported copy; `tools/prototype-F1-F4.diff`)

| Config | webgpu-samples (92) | wgpu (79) | vello (440) | bevy (93) | rerun (11) | misc (14) | **Real-world total (729)** | naga tests (587) |
|---|---|---|---|---|---|---|---|---|
| committed (BASE) | 1 | 2 | 68 | 4 | 3 | 0 | **78 (10.7%)** | 435 |
| + F1 fix | 2 | 2 | 94 | 4 | 3 | 0 | **105 (14.4%)** | 435 |
| + F1 + F2 (range-valued bounds) | 2 | 2 | 94 | 4 | 3 | 0 | 105 | 435 |
| + F1 + F3 (operators) | 6 | 2 | 97 | 4 | 6 | 1 | 116 | 435 |
| + F1 + F4 (`!=` endpoint trim) | 2 | 2 | 94 | 4 | 3 | 0 | 105 | 435 |
| **+ F1 + F2 + F3 + F4** | **6** | **3** | **97** | **4** | **6** | **1** | **117 (16.0%)** | 436 |

Soundness gates on the ALL build:
- Naga suite 142 + 272 + 6 pass, including 58 stress tests (must-retain cases).
- GLSL repro retains its check.
- Each of the 12 newly elided accesses was checked by hand, for example:
  - `colors[instance % 6u]` on a 6-element array;
  - `tile[r][4*lid.x + u32(c)]` ≤ 127 on a 128-element array;
  - `sh_bitmaps[local_id.x / 32u]` ≤ 7 on an 8-element array;
  - `u_lights[i]` with `i < min(n, 10)` on a 10-element array;
  - `corners[vertex_idx % 6u]`.

  All are in range.

What the numbers say:
- **F1 is the single most valuable change: +27 elisions, and it removes a soundness hole.**
- **F3 is worth doing:** +11, all small local rules, and each mirrors what Tint IRA already supports (div, mod, shifts, min/max).
- **F2 and F4 add ~nothing on real code** (F2 helps only with `min`). Do them only because they're cheap and principled, not for yield.

### 4.1 Not prototyped: estimated from census data (labelled as estimates)

| Improvement | Upper bound on real-world gain | Caveat |
|---|---|---|
| **Check deduplication** (second access to the same buffer with the same index value) | ≤ 139 accesses (95 on runtime storage), from identical `(function, base, index)` expression trees | Dominance and "index unchanged in between" not verified. Driver compilers may already fold some. Applies to the 46% host-bounded bucket that elision can't touch. **Best candidate for Metal**, where buffer checks are emitted |
| **Symbolic `arrayLength` facts** (`i < arrayLength(&a)` ⇒ `a[i]` safe) | ≤ 20 | Only accesses to the *same* buffer the guard names. In Boids, `particlesB[index]` stays unprovable. Genuine novelty vs Tint IRA, which is numeric-only per its header |
| **Interprocedural ranges** (call-site argument joins + helper return ranges) | ≤ 79 | **78 of the 79 are one bevy file** (`downsample_depth`). High effort, narrow benefit |
| Guards on `let`/builtin expressions, compound `&&` conditions | a subset of the 60 "locals/loops" | Needs relational facts in several cases (e.g. `i < numLayers` where `numLayers < 12`, which F2 partly covers) |
| Loop versioning / IRCE-style hoisting | would not remove checks; would move them out of loops | Targets host-bounded storage loops. No production WGSL compiler does this per its public docs. Needs runtime-cost measurement to justify |

## 5. Recommended order (for code; the poster is separate)

1. **F1: land it now** as a bug fix, with the GLSL regression test and the vello-style
   `if a < b {}` precision test. This is small, and it fixes a real soundness hole.
2. **F3 operators**, with stress tests per operator (must-elide and must-retain variants).
3. **Re-run the real-world census** and report the per-corpus table. Stop using the naga-test
   corpus as the headline: it is inflated by 3 matrix tests and blind to these real patterns.
4. **Then choose one of:**
   - **Check deduplication.** Broadest reach, especially on Metal. Measure dominance properly first.
   - **Symbolic `arrayLength`.** A novelty story vs Tint, but narrow yield.
5. Deprioritise interprocedural analysis. It's dominated by one file.

**Branches for step 1:**
- success: repro retains, suite green, real-world ≥ 105;
- partial: suite green but a stress snapshot changes, so review it;
- bad: any diff-exec divergence, so revert and investigate;
- disaster: a WGSL case becomes unsound, so stop and add a regression test first.

## 6. Limits of this study

- **Bevy is partial.** `wesl-cli 0.5` needs rustc ≥ 1.97.1, but the pinned toolchain is 1.93.1 (the system rust was removed ~Sep 1). `wesl-cli 0.4` resolves `package::` imports relative to the file and **exits 0 on errors**, which produced 29 empty outputs. Only the 10 shaders that linked were analyzed.
- **Vello `#ifdef`s** were evaluated with no defines (the msaa16 variant gives 481 accesses / 66 elided; not included in totals).
- **Runtime-generated shaders** (TF.js, ONNX Runtime Web, TVM/WebLLM) are not in any corpus. That's the most important missing segment for compute.
- **Index-shape categories** come from heuristic rules over expression trees. The per-access rows are in `census/` for audit.
- **Driver-level optimisation** (whether NVIDIA/Apple compilers already remove some checks) was not measured here. The project's own paired run (105 / 111 ns retained-check overhead on one kernel) shows they did *not* remove checks in that kernel.
