# Bounds-check elision analysis: soundness gaps and precision gaps (code-level inventory)

> Provenance: findings returned as text by the analysis/soundness researcher on 2026-10-01 (its own file write
> was refused by the harness); saved verbatim-in-substance by the coordinator. Read-only session: nothing was
> built, run or tested. Items marked HYPOTHESIS were derived by hand-tracing the code and have NOT been executed.
> Scope: wgpu fork `bce/sound-branch-refinement` @ `19ffb4e`, `naga/src/proc/bounds_check_elision.rs` (1979 lines).
> Labels: VERIFIED (read this session), RECORDED (project doc), HYPOTHESIS (trace, not executed).

## Headline

- **Three flow-layer soundness bugs reachable from WGSL (HYPOTHESIS by trace)**, none covered by the 89 stress tests:
  - **S1 stale-Load identity**: a comparison of a value loaded *before* a store refines the variable's *current*
    value. Also idiomatic in GLSL (`if (i++ < N)`).
  - **S2 dropped `continue` edges**: the state at every `continue` is discarded when analysing `continuing`
    (independently found by the paper/poster researcher).
  - **S3 in-body "initializer" rule**: lets the header range ignore an in-body re-initialisation.
- "Syntactic-match fragility" (R2) is **not** closed, contrary to `IICT_CONTEXT/03-REVIEW-RESPONSE-STATUS.md` row 8:
  loop recognition is still shape-matched and S1 is a shape-match hole.
- Override workgroup size: production-safe by re-validation, but the first ModuleInfo *does* hold proofs from the
  placeholder size 1; row 7's wording ("override can't produce bogus product") is wrong.
- Restrict: proofs are policy-independent (VERIFIED).
- Best precision investments: call-aware invalidation (≤22 accesses), value numbering (≤20), loop fixpoint with
  widening (~10–15).

## Q1. R2 "overridable workgroup size"

**Takeaway**: latent hazard, not a production bug. Pre-override ModuleInfo contains proofs from placeholder 1.
wgpu-hal backends, naga-cli and snapshot tests all call `process_overrides`, which for any override module discards
the stale info and re-validates (re-running elision with real sizes). Direct naga API: SPIR-V rejects unprocessed
override modules; HLSL emits `numthreads(1,1,1)` (self-consistent); MSL: no rejection found.
**New**: re-validation silently re-enables elision even if the caller disabled it.

Findings (VERIFIED):
- Lowerer placeholder: `front/wgsl/lower/mod.rs:1641-1674` (`workgroup_size_out = [1; 3]`; override dims stay 1,
  expression goes to `workgroup_size_overrides_out[i]`).
- Analysis reads `entry_point.workgroup_size`, never `workgroup_size_overrides` (`bounds_check_elision.rs:230-242`);
  `argument_range` l.798-810 / `builtin_component_range` l.812-827 give [0,0] for an overridden dim → `arr[lid.x]`
  proved for any length ≥ 1 (HYPOTHESIS by trace).
- Override-sized arrays are not proved (needs `IndexableLength::Known`, l.619-623).
- `process_overrides` returns the original info only if `(entry_point.is_none() || entry_points.len() <= 1) &&
  module.overrides.is_empty()` (`back/pipeline_constants.rs:93-104`); workgroup-size overrides always name an
  `override`, so such modules go through `process_workgroup_size_override` (l.296-328) and `revalidate` (l.285, 288-294).
- `revalidate` → `Validator::new` → `validate_resolved_overrides` → same `validate_impl` (`valid/mod.rs:743-764`) →
  elision runs (`valid/function.rs:1942-1944`, default true `valid/mod.rs:623`).
- All production paths call `process_overrides`: wgpu-hal `vulkan/device.rs:770`, `metal/device.rs:157`,
  `dx12/device.rs:349`, `gles/device.rs:316`; naga-cli `naga.rs:873,899,953,988`; snapshots `snapshots.rs:327,402,445,476`.
- No back end reads `workgroup_size_overrides`.
- SPIR-V direct path rejects (`Override` expression → `Err(Error::Override)`, `spv/writer.rs:2723-2724`).
- HLSL `[numthreads]` from `ep.workgroup_size` (`hlsl/writer.rs:548`). MSL emits only `module.constants`
  initialisers (`msl/writer.rs:4740`); no rejection found.
- Precondition documented: `proc/index.rs:310-314`.
- **Elision toggle lost on re-validation**: `set_bounds_check_elision(false)` (`valid/mod.rs:645-647`) is not carried
  into `revalidate`. No current project harness is affected (none calls `process_overrides`); wgpu-core never calls the setter.
- Tint makes the opposite choice: asserts constant workgroup size (`integer_range_analysis.cc` l.810-833).

Fixes: in `elide_bounds_checks`, treat `workgroup_size_overrides.is_some()` as unknown size (≈1 line; Tint's
contract). Toggle: let `process_overrides` take the caller's validator/flag.
Tests: (a) `override WG: u32 = 64; var<workgroup> a: array<u32,1>; @compute @workgroup_size(WG) fn main(@builtin(local_invocation_id) l: vec3u) { a[l.x] = 1u; }`
— assert pre-override `safe_accesses` empty (expected FAIL today); after `process_overrides({"WG":64})` no proof
for length 1, proof for length 64. (b) elision-off, 2 entry points, `process_overrides(Some(ep))` → assert empty
(expected FAIL today). Real-world exposure: 0 in census.

## Q2. R2 "syntactic-match fragility"

Expression domain is semantic (intervals, checked arithmetic); **loop recognition and refined-operand identity are
still syntactic**. S1 is a soundness consequence; handle identity also costs ~20 real-world accesses.

Shape assumptions (VERIFIED):
- Loop variables: scalar 32-bit locals only (l.829-834); `loaded_local` needs `Load{LocalVariable}` (l.885-893).
- Update: top-level `Store(x, Load(x) ± const)`; `1u + i` fails (l.1221-1223); variable steps fail (l.1224-1228);
  nested updates not found; exactly one mutation, or two with an in-body constant "initializer" (l.1059-1065);
  a `Call` counts as a mutation of every local (l.1201); geometric/shrinking must be the only mutation (l.1068-1142).
- Guard: first top-level `If` with condition `Binary(Load(x),_)` and a `Break` arm (l.1258-1298). Not recognised:
  `Unary(LogicalNot, cond)` (GLSL `while` lowering, `front/glsl/parser/functions.rs` ~l.340-358), `u32(i) < n`,
  `return` exits, guards nested in `{}`.
- Bound must be an exact constant (l.1300-1329); start must be exact (l.428-438); `break_if` only an exit rule (l.466-468).
- Operands: `target` (l.780-795); relations keyed by handle or local (l.758-760); no structural equivalence.
  `local_id.x` is a fresh `AccessIndex` each use (stress file notes this at l.316-320).
- WGSL `&&`/`||` lower to temp-bool if/else (`lower/mod.rs:2564-2595`); `refine` handles only `Binary` and
  `LogicalNot` (l.642-651); GLSL `LogicalAnd` refines (l.653-656).

**S1 stale-Load (HYPOTHESIS by trace)**. At `Emit`, a load gets the local's range at emission (l.497-510; the
8757d7d fix). The reverse direction is unfixed: `refine`→`comparison_fact`→`target` maps any `Load(x)` to
`Target::Local(x)` with x's *current* range (l.786-791) and writes the refined range to `ranges[x]` (l.673-676);
`relation`/`key` keyed on x (l.742-759); `loop_guard`/`comparison` accept any `Binary(Load(x), c)`; `break_if`
assumes its condition tests the post-update value (l.466-468). No check for an intervening store.
- Trace 1 (WGSL let-snapshot): `var i = xs[0]; let small = i < 4u; i = xs[1]; if (small) { t[i] = 1u; }` with
  `t: array<u32,4>` workgroup → `t[i]` proved, runtime unbounded. Expected after fix: `[false]`.
- Trace 2 (relation key): `var n=min(xs[0],10u); let a=n; n=min(xs[1],10u); let i=min(xs[2],10u); if (n > i) { t16[a - i] = 1u; }`
  → `key(a)=Local(n)`, `related_difference` gives [1,10], proved; runtime a=0,i=5 wraps.
- Trace 3 (loop guard on snapshot): `var i=0u; loop { t9[i]=1u; let done = i >= 9u; i = i + 1u; if (done) { break; } }`
  → header [0,8], proved; header reaches 9. Same with `continuing { let d = i >= 9u; i = i + 1u; break if d; }`.
- Trace 4 (GLSL postfix): GLSL lowers `i++` as `Load(i)`, `Store(i, left+1)`, returns `left`
  (`front/glsl/context.rs:1232-1313`). `shared uint t[7]; uint i = xs[0]; if (i++ < 7u) { t[i] = 1u; }` → proved,
  runtime i ∈ [1,7]. `while (i++ < N)` same.
- No let-snapshot or postfix test exists (stress helpers at l.1344-1360).
- Tint avoids this by requiring the loop-variable load as the first body instruction immediately followed by the
  compare (`integer_range_analysis.h:166-181`).

Precision cost of handle identity (VERIFIED, census F006 without nagatests: 729 rows, 118 safe):
"Unsupported op: shift" 22/23 still missed; 16 are vello `sh[local_id.x ± (1u << i)]` whose guard is a separately
emitted copy (backdrop 110-111, pathtag_reduce 173-174, coarse 513/576, draw_leaf 285/313, clip_leaf 264/295,
tile_alloc 257, pathtag_scan1 180/193, clip_reduce 84, draw_reduce 192, pathtag_reduce2 172); 4 guarded
`local_id.x ± 1` (draw_leaf 322-323, pathtag_scan 189-190, pathtag_scan1 204, clip_reduce 97-98); must-retain:
tile_alloc 278 `select(0u, sh_tile_count[local_id.x - 1u], local_id.x > 0u)` (select evaluates both operands).

Fixes: S1 — per-local store generation bumped on Store/reset_local/loop-entry reset/Call; record at Load emission;
map `Load(x)`→`Local(x)` in target/key/comparison/loop_guard/break_if only if the generation matches, else
`Value(handle)`; loop matcher requires no store to x between guard load and guard `If` (≈1–2 days).
Red tests: the four traces + GLSL `while (i++ < 8u) { t8[i] }`, each with a must-elide neighbour.
P2 value numbering (2–4 days, ≤20). P4 `&&`/`||` temp-bool predicate recovery. P5 shape generalisation
(`LogicalNot` guards, commutative updates, range bounds, non-exact starts).
Gaps: SPIR-V front end (phi→local) stale-load exposure for spirv-opt output UNKNOWN; how many current real-world
proofs sit behind snapshot/postfix patterns UNKNOWN.

## Q3. R2 "Restrict interaction"

Proofs are policy-independent (VERIFIED): `prove_access` only inserts into `safe_accesses` (l.611-627); non-local
loads and `*Result` get no range (l.503-507, 523); SPIR-V consumer returns before `choose_policy`
(`spv/index.rs:518-530`); chain walker passes each Access handle (`spv/block.rs:2536-2543, 2703-2711`); MSL/HLSL via
`access_needs_check` (`proc/index.rs:321-351`, HLSL `hlsl/writer.rs:3961-3968`); GLSL never consults proofs.
Real interaction is **severity**: under shipping Restrict, a wrong proof turns a clamped access into UB — S1–S3 are
memory-safety regressions.
Latent hazard: **`safe_indices`** is consulted by both consumers (spv/index.rs:521; proc/index.rs:343) but never
populated (`valid/analyzer.rs:252,1269,1407`); being index-keyed it would transfer proofs across arrays if ever
populated — remove it.
Suggested R2 answer: "Proofs are about index values only, computed before and independent of the policy, never from
memory contents; under Restrict a wrong proof becomes UB." Test to add: policy sweep of every must-retain test.

## Q4. Other soundness assumptions

- **S2 `continue` edges (HYPOTHESIS)**: `continuing` analysed from the body's fall-through state or header (l.402-415);
  `Continue` doesn't fall through (l.363-371); `If` join keeps only falling-through arms (l.296-316).
  Trace 1: `var<workgroup> t: array<u32,8>; var i=8u; loop { if (i==0u){break;} if (xs[0]==1u){continue;} i=i-1u; continuing { t[i]=1u; } }`
  → header [0,8], fall-through [0,7] → proved; with xs[0]==1 `continuing` runs with i=8.
  Trace 2: `loop { i=i+1u; continue; continuing { t4[i]; break if i>=4u; } }` → i reaches 4.
  Fix (1–2 days): join states at every `Continue` targeting the loop (excluding nested loops, like `may_continue`
  l.1243-1256) with the fall-through state before analysing `continuing`.
- **S3 in-body initializer (HYPOTHESIS)**: `single_update` accepts two mutations when one is a constant store
  (l.1059-1065); `loop_range` takes start from the incoming value or the in-body store (l.428-438) without joining.
  Trace: `var<workgroup> t: array<u32,9>; var i=8u; loop { t[i]=1u; i=100u; if (i==0u){break;} i=i-1u; }` → header
  [0,8] proved; second iteration header = 99. Added in 8757d7d; no motivating front-end shape found. Fix: drop the
  rule or restrict it; re-run the suite.
- Calls reset every local (l.362, 846-850) and count as mutation of every local (l.1018-1022, 1201): sound, over-conservative.
- Atomics/barriers/workgroup data: non-local loads and `*Result` unranged (l.503-507, 523, 597).
- Switch: mutated locals reset (l.318-334); no selector refinement.
- Casts: `convert_range` only when representable (l.1520-1536).
- Overflow: WGSL wraps (archived spec); analysis never assumes no-wrap (checked add/sub/mul l.85-141; shift_left fit
  l.1447-1467; related_difference bounded l.764-776; divide/modulo guards l.1376-1421; shift amount < 32 l.1423-1429);
  property sweeps l.1704-1940.
- i32 negatives: `fits` needs `min >= 0` (l.171-176). 64-bit/abstract: only 32-bit ranged (l.853-865).
- Builtins: only local_invocation_index/id (l.798-827); struct-wrapped local_invocation_id unknown (l.813-821).
- **Kill/discard (latent platform-semantics hazard)**: analysis treats `Kill` as terminator (l.363-371), matching IR
  doc; WGSL §9.4.11 `discard` demotes to helper invocation and statements after it may execute. SPIR-V emits `OpKill`
  (safe); MSL `discard_fragment();` (`msl/writer.rs:3950`) and HLSL `discard;` (`hlsl/writer.rs:2275`) without
  `return`. Metal/D3D semantics UNKNOWN; if execution continues, `if (i>=4){discard;} a4[i]` loses its clamp there.
- `sdata_manual` hack: `spv/writer.rs:2824`, Polyfill-mode only (l.1674-1688), introduced 86eb81c, violates WGSL
  zero-init for that name; no shader in repo uses it. Remove.
Priority: S1, S2, S3, then override guard + toggle + `safe_indices` removal, then discard.

## Q5. Real-world yield per effort; gap vs Tint

Census F006 (VERIFIED, excl. nagatests): fixed arrays + vectors = 342 accesses, 118 proved. Remaining 224 dominated
by helper-function params (78, one bevy file), locals/loops (59), vertex/instance (36, unprovable), shift-guarded
reductions (22).
Kind × space: runtime storage read 205 missed / 0; runtime storage rw 159 / 0; fixed workgroup 141 missed / 74 safe;
fixed function 45 / 31; fixed value 20 / 7; binding 23 / 0; fixed private 9 / 0; fixed uniform 4 / 1; vector 3 / 5;
fixed storage 2 / 0.
Locals/loops (59): 22 are vello `fine.wgsl` `rgba[i]`/`area[i]` in `for (var i=0u; i<PIXELS_PER_THREAD; i+=1u)`
with calls before the access (e.g. l.1031, 1086) — HYPOTHESIS: call-resets-everything kills these.
Tint IRA (local copy, newest commit c43a057, 2026-03-02): Add/Sub/Mul/Div/Shl/Shr/Mod, min/max; builtins under
constant-size assertion; rigid loop shape (const init, ±1 update, load immediately followed by compare with const);
`If` only for loop exit; no general branch refinement, no relational facts.

Ranked plan:
| # | Improvement | Census yield | Effort |
|---|---|---|---|
| P1 | Call-aware invalidation (reset only locals passed by pointer) | ≤22 (HYPOTHESIS) | ½–1 day |
| P2 | Value numbering of pure expressions (reuses S1 generation) | ≤20 | 2–4 days |
| P3 | Loop fixpoint with widening/narrowing | ~10–15 (estimate) | ~1 week |
| P4 | `&&`/`||` temp-bool predicate | subset of 59 | 2–3 days |
| P5 | Range-valued bounds, non-exact starts, `LogicalNot` guards, commutative updates | small | 1–2 days |
| P6 | Struct-wrapped builtins, subgroup_invocation_id | 0–3 | small |
| P7 | Interprocedural | ≤78–79 (one file) | high |
| P8 | Check dedup / symbolic arrayLength (mainly Metal) | ≤139 / ≤20 | medium–high |
Do S1–S3 before P2/P4.

## Q6. Missing testing

Arithmetic layer swept exhaustively; flow layer only 89 hand-written shapes (0 hits for "override"; no
let-snapshot, postfix or `continuing`-access tests). CTS: `cts_runner/test.lst` has no
`webgpu:shader,execution,robust_access:*` selector (only `robust_access_vertex` l.391); no record of a CTS run of
the elision build. MSL/HLSL structural only.
Proposed: (1) red tests for S1–S3, override, toggle (hours); (2) IR-level differential oracle: interpreter for the
integer/control subset + random structured programs biased to risky shapes, asserting every proven access stays in
bounds (1–2 weeks, highest value); (3) "checked-elision" debug policy that counts violations of proven accesses at
runtime on all GPU/real-world shaders; (4) CTS `webgpu:shader,execution,robust_access:*` + shader,execution with
elision on; (5) front-end fuzzing (GLSL, spirv-opt output); (6) policy sweep of must-retain tests.

Branches for (1): success = red at 19ffb4e, green after fix, suite green → land; partial = a trace was wrong
(lowering differs) → record, keep as pin; bad = fix loses real-world elisions → re-run census vs 118; disaster = a red
test hits a measured benchmark kernel → stop, downgrade the ledger's soundness wording.
