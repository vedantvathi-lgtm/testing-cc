# Naga Bounds-Check Elision: Incremental Improvement Plan

> **Status:** Draft — Phase 0 (profiling) pending
> **Companion to:** `contextual.md` (implementation log), `README.md` (design doc v1)

---

## 1. Problem Statement

Naga's current `access_needs_check` function (`naga/src/proc/index.rs:322`) **only elides bounds checks for compile-time literal indices** (e.g. `array[5]`). It does nothing for builtin-derived indices like `tile[lid]` even though the proof is trivial: `lid ∈ [0, 64)`, array size is 64.

**The paper's §4.2 patch** adds recognition of `@builtin(local_invocation_index)` — a check that elides when the index expression is the builtin itself (or a simple offset from it) and the array size ≥ workgroup size.

### Where the current heuristic breaks down

| Pattern | Example | Why it fails | Fix |
|---------|---------|-------------|-----|
| Arithmetic composition | `tile[lid * 2 + 1]` | Only recognizes atomic builtins | Interval arithmetic on expression trees |
| Control-flow merge | `if (x) { idx = lid; } else { idx = 0; }` — unifies at phi node | Needs interval union at merges | Join at block merge points |
| Function call boundary | `helper(lid)` with `fn helper(i: u32)` | Builtin binding not on helper's param | Per-function bound summaries |
| 2D dispatch grid | `tile[wid.x * 256 + lid]` | Only checks workgroup dims, not dispatch dims | Extend to multi-dimensional builtins |

---

## 2. Proposed Approach: Incremental Local Extensions

The key insight: **full abstract interpretation (global fixpoint over interprocedural lattice) is overkill.** LLVM only uses abstract-interpretation-derived techniques (e.g. the Attributor) in select passes, not as a blanket approach. The four extensions below are all *local* (single-function, single-merge-join, cached-summary) — each stays O(n) in the function size with no fixpoint iteration.

### Extension 1: Interval arithmetic over expression trees

**What it solves:** `tile[lid + 1]`, `tile[lid * 2 + offset]`

**How:** Define transfer functions for standard interval arithmetic on expression trees:
- `[a,b] + [c,d] = [a+c, b+d]`
- `[a,b] - [c,d] = [a-d, b-c]`
- `[a,b] * [c,d] = [min(...), max(...)]`
- `min([a,b], [c,d])`, `max([a,b], [c,d])`

Single bottom-up pass over the expression DAG. No fixpoint needed — this is O(1) per access instruction.

**Compile-time cost:** Microseconds per access. Pattern-matching, not dataflow analysis.

### Extension 2: Join at control-flow merge points

**What it solves:** Index bounded differently on two branches (phi nodes)

**How:** At each block merge in the CFG, compute interval union of the incoming values. Cap: if a value flows through >N merges without stabilizing, bail out to inserting the runtime check. Naga IR is already SSA-ish per function, so this maps cleanly to block argument resolution.

**Compile-time cost:** Linear in merge points per function. The cap prevents pathological cases from exploding.

### Extension 3: Local value numbering (LVN)

**What it solves:** Redundant re-derivation of the same bound (e.g. `x*4` computed twice)

**How:** Hash-cons the expression trees so the interval lookup is O(1) memoized hit instead of recompute. This is primarily a compile-time *win*, not a precision win — it keeps everything fast as more expressions are added.

**Compile-time cost:** Negative (saves time by avoiding recomputation).

### Extension 4: Per-function bound summaries

**What it solves:** Values crossing function call boundaries — `helper(lid)` where `fn helper(i: u32)`

**How:** For each function, compute a lazy, memoized "bound signature" mapping parameter indices → return value intervals. At call sites, inline the summary (not the function body). If a call site doesn't fit the cached summary's assumptions, fall back to a runtime check.

No inlining pass needed. No whole-program analysis needed. The summary is computed once per function (lazily) and cached.

**Compile-time cost:** O(1) per call site after cache warm-up.

---

## 3. Implementation Plan

### Phase 0: Profile the Baseline

**Goal:** Determine which extension yields the largest gain first.

**Actions:**
1. Run the existing Tier 2 assertion tests to confirm every case retains its check (pre-patch baseline) — **DONE** (see `elision_assertions.rs`)
2. Check that the pre-patch SPIR-V golden files show `OpSelectionMerge`/`OpBranchConditional` on all 6 cases — **DONE**
3. Profile the benchmark kernels (reduction, prefix_scan, tiled_matmul) to identify which access patterns dominate:
   - Measure compile time per kernel
   - Classify each access by pattern type (bare builtin, builtin+offset, builtin*const, composite, etc.)
   - Count how many accesses each extension would capture

### Phase 1: Implement Extensions (priority-ordered by Phase 0 results)

Likely order (assuming bare builtin is the most common pattern):

1. **v1: Direct builtin recognition** (~20 lines in `access_needs_check`)
   - Recognize `@builtin(local_invocation_index)` → range `[0, workgroup_size)`
   - Recognize `@builtin(local_invocation_id.x)` (for 2D workgroup_size)
   - Check: `array_size >= upper_bound` → elide

2. **v2: Builtin + constant offset** (~30 lines)
   - Recognize `builtin ± K` where K is a compile-time constant
   - Check: `0 + K <= lid` and `lid + K < workgroup_size` (handles overflow/underflow)

3. **v3: 2D dispatch indexing** (~50 lines)
   - Recognize `wid.x * stride + lid` patterns
   - Need: `@builtin(workgroup_id)` bounded by dispatch dimensions

4. **v4: Full interval arithmetic** (~100 lines)
   - Expression tree walk with interval transfer functions

5. **v5: Merge joins** (~150 lines)
   - Block-level interval union at phi nodes

6. **v6: LVN** (~80 lines)
   - Hash-cons for bound expressions

7. **v7: Function summaries** (~200 lines)
   - Lazy per-function bound signatures

### Phase 2: Validation

#### Correctness Axis
- **Regression baseline:** Run all existing Tier 2 assertions — they must pass unchanged for out-of-scope cases
- **Adversarial test cases:** Add new WGSL fixtures targeting each gap:
  - `bounds-check-elision-arithmetic.wgsl` — `lid * 2 + 1`, `lid + 5 - 3`
  - `bounds-check-elision-merge.wgsl` — index through if/else with join
  - `bounds-check-elision-crossfn.wgsl` — index through helper function
  - `bounds-check-elision-2d.wgsl` — `wid.x * N + lid`
- **Soundness check:** For each adversarial case, confirm the check is RETAINED when the index CAN go out of bounds, and ELIDED when it CANNOT

#### Compile-Time Cost Axis
- `cargo test` before/after timing
- Per-kernel compile time (Naga parsing + validation + SPIR-V emit)
- Target: <5% regression on total compile time for the full benchmark suite

#### Elision Rate Axis
- Count total access instructions in each kernel
- Count elided vs retained checks per extension
- Report per-extension improvement fraction
- Target: 100% elision for in-scope patterns (bare builtin, builtin+offset)

---

## 4. Verification Structure

```
Phase 0: Profiling
├── Run Tier 2 assertion tests           (baseline dump)
├── Classify access patterns per kernel  (which extension helps most?)
└── Profile compile time                 (baseline cost)

Phase 1: Implementation (in priority order)
├── v1: Direct builtin recognition       (~20 lines)
├── v2: Builtin + offset                 (~30 lines)
├── v3: 2D dispatch indexing             (~50 lines)
├── v4: Full interval arithmetic         (~100 lines)
├── v5: Merge joins                      (~150 lines)
├── v6: Local value numbering            (~80 lines)
└── v7: Function summaries               (~200 lines)

Phase 2: Validation per version
├── Correctness: adversarial test suite  (no false elisions)
│   ├── Compile each adversarial case
│   ├── Assert elision only where safe
│   └── Fuzz with out-of-range inputs
├── Compile-time: benchmark timing       (<5% regression)
│   ├── Time full test suite
│   ├── Time per-kernel compilation
│   └── Check linear growth
└── Elision rate: fraction of checks removed
    ├── Count total accesses
    ├── Count elided per extension
    └── Report per-extension improvement
```

---

## 5. Success Metrics

| Metric | Target | How to measure |
|--------|--------|----------------|
| Compile-time regression | <5% | `cargo test` timing before/after |
| Elision rate (bare builtin) | 100% | SPIR-V disassembly inspection |
| Elision rate (builtin + offset) | 100% | SPIR-V disassembly inspection |
| Elision rate (2D dispatch) | 100% | SPIR-V disassembly inspection |
| False elision rate | 0% | Adversarial test suite + fuzzing |
| Benchmark overhead reduction | Proportional to elision rate | GPU benchmark timing |

---

## 6. References

- `naga/src/proc/index.rs:322` — `access_needs_check` function
- `naga/src/proc/bounds.rs` — `BoundsCheckPolicy` definitions
- `bounds-check-testing-framework/naga/tests/naga/elision_assertions.rs` — Tier 2 assertion tests
- `bounds-check-testing-framework/contextual.md` — Implementation log
- `bounds-check-testing-framework/README.md` — Design doc v1
