# [EPOCH 0 — SECTION 0.2]: INSIDE NAGA — FRONT ENDS, THE IR, THE VALIDATOR, AND THE BACK ENDS

> **Evidence rule for this file** (AGENTS.md Rule 0). Labels: **VERIFIED** (read or run on
> 2026-09-29 against wgpu fork `19ffb4e`), **RECORDED** (project artifact, path cited),
> **REPORTED**, **UNKNOWN**. Every IR listing below is real output of
> `IICT_CONTEXT/epoch/tools/bce-epoch-tools` (built against `wgpu/naga` at `19ffb4e`), not hand-written.

⛔ **DO NOT SKIP.** The elision analysis (0.6) is ~2,000 lines of Rust that walk *Naga IR*. You cannot read one line of it without knowing what these are:
- a `Handle<Expression>`;
- why a variable read is `Load { pointer: LocalVariable }` and not "a variable";
- what `Emit` does;
- why a WGSL `for` loop has no condition in the IR.

In particular:
- **0.6** keeps two maps, `Ranges` (per `LocalVariable`) and `Values` (per `Expression` handle). That split mirrors the IR's split between *mutable locals behind pointers* and *immutable SSA expressions*.
- **0.8's first soundness bug** exists *because* the WGSL front end and the GLSL/SPIR-V front ends build different IR for the same `&&`. Tests written only in WGSL could never see it.
- **0.3 and 0.6** both hinge on one fact: a proof is attached to an `Access` **expression handle**. The back end asks "is handle 26 in `safe_accesses`?" If you don't know what handle 26 is, the proof mechanism is opaque.

---

## Prerequisites

- Section 0.1: the safety contract, and the wgpu → wgpu-core → wgpu-hal path.
- Rust: `enum`, `match`, `Vec`, and the idea of an index into a `Vec`.
- The phrase "compiler front end / back end" at the level of "front = reads a language, back = writes a language".

---

## THE WHY

### WHY 1 (Existence): what would be broken without an IR?

wgpu accepts three input languages (WGSL, GLSL, SPIR-V; VERIFIED, `wgpu-core/src/device/resource.rs:2369-2414`). It must emit four output languages (SPIR-V for Vulkan, MSL for Metal, HLSL for D3D12, GLSL for GL; VERIFIED, the four `naga::back::*` calls in wgpu-hal, Section 0.1 F.4). Without a shared middle, that is 3 × 4 = 12 translators, and **12 copies of every bounds-check rule and every validation rule**. The elision analysis would have to be written 12 times, or once per input language and trusted to agree with 4 emitters.

With an IR, the elision pass is written **once**, on Naga IR, and runs regardless of input or output. `naga/src/back/` has `spv`, `msl`, `hlsl`, `glsl`, `wgsl`, `dot` (VERIFIED, `ls`). `naga/src/front/` has `wgsl`, `glsl`, `spv` (VERIFIED). The IR is the only reason a ~2,000-line analysis can protect Firefox on all four GPU APIs.

### WHY 2 (Mechanical): what forces the IR's shape?

Three machine facts:
1. **GPUs are SSA-friendly and pointer-shy.** SPIR-V, the dominant target, is in *Static Single Assignment* form. Every value is defined once; mutable storage is reached only through explicit pointers (`OpVariable`, `OpLoad`, `OpStore`). Naga mirrors this: "This is a Single Static Assignment (SSA) scheme similar to SPIR-V" (VERIFIED, `naga/src/ir/mod.rs:1560`).
2. **Shading languages need structured control flow.** SPIR-V requires structured merges (`OpSelectionMerge`, `OpLoopMerge`); MSL/HLSL/GLSL are C-like. So Naga keeps `If` / `Loop` / `Switch` as a **tree**, not a free-form control-flow graph (VERIFIED, `ir/mod.rs:2007-2110`).
3. **Evaluation time matters for loads.** A `Load` must happen after the `Store`s before it. So expressions that read memory must be pinned to a point in the statement tree. That is `Emit` (explained below).

### WHY 3 (Design decision): why arenas and handles?

The alternative is a pointer tree (`Box<Expression>` children). Naga chose **arenas** instead: "most structures are stored in an `Arena`. An `Arena<T>` stores a series of `T` values, indexed by `Handle<T>` values, which are just wrappers around integer indexes" (VERIFIED, `naga/src/arena/mod.rs:3-8`). The reasons:
- **Sharing**: an expression used twice (e.g. `lid` in `s[lid]` and `s[lid + stride]`) is one node referenced by two handles. That makes it a DAG, not a tree.
- **Cheap side tables**: an analysis stores facts in `HashSet<Handle<Expression>>` or a `Vec` indexed by handle. **That is exactly what `safe_accesses` is**: `FastHashSet<Handle<crate::Expression>>` (VERIFIED, `naga/src/valid/analyzer.rs:254`).
- **Type safety**: `Handle<Expression>` and `Handle<LocalVariable>` are different Rust types (`PhantomData<T>`, VERIFIED, `arena/handle.rs:44-49`), so you can't index the wrong arena.

The cost is that handles are *positional*. A handle means "the 26th expression of *this function*". That is fine for a single compile, and it is why proofs are per-function.

### WHY 4 (Historical)

Naga began as the shader translator of the gfx-rs project, replacing the C++ SPIRV-Cross + glslang toolchain with pure Rust. It now lives inside the wgpu repository (VERIFIED: `wgpu/naga/` is a workspace member at `19ffb4e`). Its design copies SPIR-V's ideas (SSA, pointers, structured control flow) because SPIR-V was the lingua franca it had to round-trip. Chrome's Tint made the same choice independently. (Historical claims beyond the repo layout are REPORTED; this repo has no primary source on Naga's founding.)

---

## What Is This? (analogy first)

Think of a **recipe card system in a restaurant kitchen**.
- Customers order in three languages (WGSL, GLSL, SPIR-V). A **front-end** clerk translates each order onto a standard card.
- The card has two parts:
  1. **The ingredient list, numbered** (the `Expression` arena): "[0] the customer's seat number; [3] seat number + 128; [9] the plate at position [3]". Each line is written once and never changed. Later lines refer to earlier ones by number.
  2. **The step list** (the `Statement` tree): "Emit [3]: now prepare line 3. If [4], then: Store into plate [9]…". Steps happen in order, can branch and loop, and are the only place anything *changes*.
- A **health inspector** (the validator) reads every card before it reaches the kitchen: types match, nothing used before it exists, and so on. In this fork, the inspector also stamps some plate lines "proven in range" (`safe_accesses`).
- Four **back-end** cooks (SPIR-V, MSL, HLSL, GLSL) read the same card and cook for four kitchens. When a cook sees a plate line stamped "proven", they skip the "check the shelf number" step.

---

## How It Actually Works — the 8-Layer Trace (layers B and C)

Section 0.1 covered A (source), F (wgpu), G (driver). This section covers **B (front end)** and **C (validator/analyzer)**. It also previews how E (back ends) consume the result.

### The IR's two halves: `Expression` and `Statement`

From the IR's module docs (VERIFIED, `naga/src/ir/mod.rs:16-27`):

> "An `Expression` produces a value, but has no side effects or control flow. … A `Statement` can have side effects and structured control flow. `Statement`s do not produce a value, other than by storing one in some designated place. … `Statement`s form a tree, with pointers into the DAG of `Expression`s. Restricting side effects to statements simplifies analysis and code generation."

That last sentence is the licence for the elision analysis. Because expressions are pure, the analysis can compute "the range of expression 25" once and reuse it. Only statements (`Store`, `Call`, loops) can invalidate facts, and those are the exact places 0.6's code calls `reset_local` / `forget_local`.

The variants this project touches (VERIFIED, line numbers in `naga/src/ir/mod.rs`):

| Variant | Line | Meaning | Why the analysis cares |
|---|---|---|---|
| `Expression::Access { base, index }` | 1634 | index with a *computed* index | **the thing being proven**; proofs are keyed by this handle |
| `Expression::AccessIndex { base, index: u32 }` | 1642 | index with a *constant* | constant, so checked statically; never needs a proof |
| `Expression::FunctionArgument(u32)` | 1661 | a parameter; for entry points, a builtin like `local_invocation_index` | builtins have ranges from `@workgroup_size` |
| `Expression::GlobalVariable(h)` | 1677 | *pointer* to a global (buffer or workgroup array) | the base of most accesses |
| `Expression::LocalVariable(h)` | 1684 | *pointer* to a `var` | reads go through `Load` |
| `Expression::Load { pointer }` | 1690 | read through a pointer | a local's value at *this* program point |
| `Expression::Binary { op, left, right }` | 1787 | `+ - * / % << >> & < == && …` | interval arithmetic and guard conditions |
| `Expression::Select` / `Math` / `As` | 1796 / 1814 / 1822 | `select`, `min`/`max`/`clamp`, casts | feature 005 added ranges for these (0.12) |
| `Expression::ArrayLength(e)` | 1868 | runtime length of a runtime-sized array | known only at dispatch, so buffer accesses stay unproven |
| `Statement::Emit(Range<Expression>)` | 2013 | "evaluate these expressions now" | fixes *when* a `Load` observes a variable |
| `Statement::If { condition, accept, reject }` | 2021 | structured `if` | **path sensitivity**: refine on each edge |
| `Statement::Loop { body, continuing, break_if }` | 2087 | the *only* loop form | induction variables live here |
| `Statement::Break` / `Continue` | 2101 / 2110 | exits | `Continue` caused soundness bug 2 (0.8) |
| `Statement::Store { pointer, value }` | 2147 | write through a pointer | invalidates facts about that local |

### Handles: what "[26]" means

`Handle<T>` is "a strongly typed reference to an arena item … used as an index into an `Arena`" (VERIFIED, `arena/handle.rs:30-49`). The tool prints `handle.index()`: [0] is the first expression appended to this function's arena, [26] the 27th. Expressions must come before their users: "An `Expression` must occur before all other `Expression`s that use its value" (VERIFIED, `ir/mod.rs` `Function::expressions` doc, near line 2424).

### `Emit`: when does an expression "happen"?

The IR docs give the rules (VERIFIED, `ir/mod.rs:52-100`):
- `Literal`, `Constant`, `ZeroValue` are "implicitly evaluated before execution begins".
- `FunctionArgument` and `LocalVariable` are "implicitly evaluated upon entry to the function … `LocalVariable` expressions produce a *pointer to* the variable's value (for use with `Load` and `Store`)".
- "All other expressions are evaluated when the (unique) `Statement::Emit` statement that covers them is executed."
- "`Load`, `ImageSample`, and `ImageLoad` expressions are influenced by stores to the variables or images they access, and must execute at the proper time relative to them."

**Consequence for the analysis.** A `Load { pointer: LocalVariable(stride) }` at handle [16] and another at [24] are *different values*: the variable may have been stored to in between. The analysis must track "what is `stride`'s range *here*" per program point. That is `Ranges` (per local, flowing through statements). The value of *this particular* `Load` is `Values` (per expression handle). **This two-map design in `bounds_check_elision.rs` is dictated by the IR, not chosen freely.**

---

### Layer B — the WGSL front end, observed

Naga's WGSL front end is two stages: **parse** (text to AST, `front/wgsl/parse/`) and **lower** (AST to IR, `front/wgsl/lower/`). Three lowering facts drive this whole project.

#### B.1 — `var` becomes a pointer; every read is a `Load`

Input (`IICT_CONTEXT/epoch/tools/bce-epoch-tools/shaders/six.wgsl`):

```wgsl
var<workgroup> s: array<u32, 256>;
@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32) {
    var i = lid;
    if (i < 128u) { s[i + 128u] = s[i]; }
}
```

Real output of `bce-epoch-tools ir six.wgsl` (VERIFIED 2026-09-29), with commentary on the right:

```
local [0] Some("i") init=None                 ← `var i = lid;` is a LocalVariable with NO initializer.
                                                 lid is a runtime value, so the front end emits a Store instead.
[ 0] FunctionArgument(0)                      ← lid (the builtin); range [0, 255] from @workgroup_size(256)
[ 1] LocalVariable([0])                       ← POINTER to i
[ 2] Load { pointer: [1] }                    ← i, read for the condition
[ 3] Literal(U32(128))
[ 4] Binary { op: Less, left: [2], right: [3] }        ← i < 128u
[ 5] GlobalVariable([0])                      ← POINTER to s
[ 6] Load { pointer: [1] }                    ← i, read AGAIN (a new value handle, same variable)
[ 7] Literal(U32(128))                        ← a second literal 128: expressions are not hash-consed
[ 8] Binary { op: Add, left: [6], right: [7] }         ← i + 128u
[ 9] Access { base: [5], index: [8] }   <= in safe_accesses   ← &s[i + 128u]  (a pointer)
[10] GlobalVariable([0])
[11] Load { pointer: [1] }                    ← i, third read
[12] Access { base: [10], index: [11] }   <= in safe_accesses ← &s[i]
[13] Load { pointer: [12] }                   ← the value s[i]
-- body (statement tree)
Store { pointer: [1], value: [0] }            ← i = lid
Emit([2])
Emit([4])
If { condition: [4] }
  accept:                                     ← on this edge the analysis learns i ∈ [0, 127]
    Emit([6])
    Emit([8, 9])
    Emit([11, 12, 13])
    Store { pointer: [9], value: [13] }       ← s[i + 128u] = s[i]
    Return { value: None }
  reject:
    Return { value: None }                    ← Returns were pushed INTO both branches (see B.4)
-- safe_accesses = [9, 12]
```

Three things to notice:
1. **`Access` produces a pointer.** The store target is `[9]` (a pointer to `s[i+128]`); the value read is `Load [13]` of pointer `[12]`. The proof is on the `Access` ([9], [12]), not the `Load`.
2. **Each read of `i` is a new handle** ([2], [6], [11]). The analysis does not know `[6] == [2]` from the IR shape. It knows it because it tracks `i`'s range per program point and nothing stores to `i` in between.
3. **Both accesses are proven.** `i + 128 ∈ [128, 255]` on the accept edge, and `i ∈ [0, 127]`. Both fit `array<u32, 256>`.

#### B.2 — `for` has no condition in the IR

A WGSL `for (init; cond; update) { body }` is desugared in the **parser** (VERIFIED, `front/wgsl/parse/mod.rs:1457-1504`). The condition becomes `if cond {} else { break; }` at the top of the body. The update becomes the `continuing` block. The body becomes a nested `Block`. The result is `Loop { body, continuing, break_if: None }`.

The running example's IR (VERIFIED, `bce-epoch-tools ir shaders/reduce_tree.wgsl`; source copied from the recorded benchmark artifact):

```
local [0] Some("stride") init=Some(11)        ← `var stride = 128u` — constant init IS kept as init
…
[16] Load { pointer: [12] }                   ← stride (read inside the guard)
[17] Binary { op: Less, left: [0], right: [16] }        ← lid < stride
[19] Access { base: [18], index: [0] }   <= in safe_accesses   ← &s[lid]  (store target)
[21] Access { base: [20], index: [0] }   <= in safe_accesses   ← &s[lid]  (read)
[24] Load { pointer: [12] }                   ← stride again
[25] Binary { op: Add, left: [0], right: [24] }         ← lid + stride
[26] Access { base: [23], index: [25] }  <= in safe_accesses   ← &s[lid + stride]   THE running example
[31] Binary { op: ShiftRight, left: [29], right: [30] } ← stride >> 1u
…
Loop { break_if: None }
  body:
    Emit([13])
    Emit([15])                                ← stride > 0u
    If { condition: [15] }
      accept:                                 ← (empty)
      reject:
        Break                                 ← the for-condition, desugared
    Block:                                    ← the user's loop body
        Emit([16, 17])
        If { condition: [17] }                ← if (lid < stride)
          accept:
            …
            Store { pointer: [19], value: [28] }
          reject:
        ControlBarrier(Barrier(WORK_GROUP))
  continuing:
    Emit([29])
    Emit([31])
    Store { pointer: [12], value: [31] }      ← stride = stride >> 1u   (the "update")
…
-- safe_accesses = [3, 19, 21, 26]
```

Access `[9]` (`input[wid.x * 256u + lid]`) and `[36]` (`output[wid.x]`) are **not** proven. `input` and `output` are `array<u32>` (runtime-sized), whose length is `ArrayLength`, known only at dispatch. No shader compiler can prove these (0.7 finds that 46% of real-world misses are this kind; RECORDED, plan §0.7). `s[0]` is `AccessIndex` ([38]), a constant, and needs no proof.

**Why this shape matters for the analysis.** To prove `[26]` the analysis must know three things:
1. `stride` is in `[1, 128]` inside the body. This comes from the entry guard `stride > 0` combined with the recurrence `stride = stride >> 1` from 128. That needs **shrinking-recurrence** support, added in feature 005 (`2ffe1b7`, RECORDED, plan §0.12).
2. `lid < stride` on the inner accept edge. That needs **path sensitivity** with a **range-valued** bound (`stride` is not a constant). Also feature 005.
3. Therefore `lid + stride < 2·stride ≤ 256`. That needs a **relational** fact, `lid < stride`, kept alongside the ranges. Feature 006 (`19ffb4e`).

This is why `[26]` was unproven on `83a90e0` and proven on `19ffb4e` (RECORDED, `reports/Benchmark v4 results.md` addendum: `reduce_tree` clamps 4→1 before, 4→0 after).

#### B.3 — `&&` becomes control flow in WGSL, but not in GLSL

The WGSL lowerer turns `a && b` into an `if` that stores into a temporary bool. Its own comment (VERIFIED, `front/wgsl/lower/mod.rs:2564-2591`):

```rust
/// Generate IR for the short-circuiting operators `&&` and `||`.
…
if ctx.is_runtime() {
    // To simulate short-circuiting behavior, we want to generate IR
    // like the following for `&&`. For `||`, the condition is `!_lhs`
    // and the else value is `true`.
    //
    // var _e0: bool;
    // if _lhs {
    //     _e0 = _rhs;
    // } else {
    //     _e0 = false;
    // }
```

The GLSL and SPIR-V front ends keep a single `Binary { op: LogicalAnd }`. For SPIR-V: `Op::LogicalAnd => { … parse_expr_op!(crate::BinaryOperator::LogicalAnd, BINARY)?; }` (VERIFIED, `front/spv/next_block.rs:1836-1839`). For GLSL: `BinaryOperator::LogicalAnd` (VERIFIED, `front/glsl/context.rs:712`).

**The same guard in both languages, observed** (VERIFIED 2026-09-29, `shaders/and.wgsl` and `shaders/and.comp`, array of 128, workgroup of 256, so the guard is *needed*):

```wgsl
// WGSL                                              // GLSL
var<workgroup> s: array<u32, 128>;                   shared uint s[128];
fn main(@builtin(local_invocation_index) lid: u32) { void main() { uint lid = gl_LocalInvocationIndex;
    if (lid < 128u && lid > 3u) { s[lid] = 1u; }         if (lid < 128u && lid > 3u) { s[lid] = 1u; }
}                                                    }
```

WGSL IR (abridged; full output in the tool):

```
[ 2] Binary { op: Less, left: [0], right: [1] }       ← lid < 128u
[ 4] LocalVariable([0])                               ← the hidden temporary `_e0: bool`
[ 6] Binary { op: Greater, left: [0], right: [5] }    ← lid > 3u
[ 8] Load { pointer: [7] }                            ← read the temporary
[10] Access { base: [9], index: [0] }                 ← s[lid]    — NOT proven
If { condition: [2] }  accept: Store [4] ← [6]   reject: Store [4] ← false
If { condition: [8] }  accept: Store [10] ← 1u …
-- safe_accesses = []
```

GLSL IR (abridged; the GLSL front end wraps `main` in a separate function the entry point `Call`s):

```
[ 6] Binary { op: Less, left: [4], right: [5] }       ← lid < 128u
[ 9] Binary { op: Greater, left: [7], right: [8] }    ← lid > 3u
[10] Binary { op: LogicalAnd, left: [6], right: [9] } ← ONE expression
[12] Access { base: [0], index: [11] }   <= in safe_accesses   ← s[lid]  — PROVEN
If { condition: [10] } …
-- safe_accesses = [12]
```

**Two lessons from one experiment:**
1. **Soundness (history, 0.8).** Before the fix `9c4615b`, `refine()` handled `LogicalAnd` on the *false* edge by treating "can't model" as "infeasible". Only GLSL and SPIR-V input ever reaches that code with `LogicalAnd`, and WGSL `&` (non-short-circuit) reaches it with `And`. The WGSL-only test corpus never exercised it (RECORDED, ledger lines 71 and 73). *Different front ends build different IR for the same source idea, so an analysis on the IR must be tested from every front end.*
2. **Precision (new finding, 2026-09-29).** After the fix, the *true* edge of a `LogicalAnd` is refined correctly. So **GLSL input proves `s[lid]` and WGSL input does not**. The analysis cannot see through WGSL's lowered form, `If(Load(bool temporary))`. This precision gap for WGSL `&&` guards was not in any project record before this section (VERIFIED absent: `grep` over `reports/`, `IICT_CONTEXT/`, `research_notes/`, `spec-kit-work/specs/` finds only the soundness entries). How many real-world misses it explains is **UNKNOWN**: the census records the access line, not its guard.

#### B.4 — small lowering facts that bite

- **`Return` is pushed into branches.** In `six.wgsl` the function ends in an `if`; the lowerer calls `proc::ensure_block_returns(&mut body)` (VERIFIED, `front/wgsl/lower/mod.rs:1633`). That recurses into the last `If`'s arms (`proc/terminator.rs:7-29`), which is why both arms end in `Return`. The analysis must not assume code after an `If` is reachable from both arms.
- **Literals are not deduplicated.** `128u` appears as both [3] and [7] in `six.wgsl`. Any "same expression" reasoning must compare values, not handles.
- **Constant `var` initializers are kept as `init`**: `stride` has `init=Some(11)`. A runtime initializer becomes a `Store` (`i` in `six.wgsl`). The analysis handles both: `default_ranges` reads `init` (VERIFIED, `proc/bounds_check_elision.rs:252-255`), and the `Store` path handles the rest.

---

### Layer C — the validator and analyzer

`Validator::validate` walks the module, type-checks every expression, checks handle ordering, and computes a **`ModuleInfo`** (VERIFIED, `valid/mod.rs:343-348`). A `ModuleInfo` holds one **`FunctionInfo`** per function and per entry point. Its fields (VERIFIED, `valid/analyzer.rs:242-256`):

```rust
pub struct FunctionInfo {
    flags: ValidationFlags,
    pub available_stages: ShaderStages,
    pub uniformity: Uniformity,                                    // for barriers and derivatives
    pub may_kill: bool,
    /// Index expressions whose ranges are safe at every use.
    pub safe_indices: crate::FastHashSet<Handle<crate::Expression>>,   // ← added by this fork
    /// Access expressions proven safe at their specific program points.
    pub safe_accesses: crate::FastHashSet<Handle<crate::Expression>>,  // ← added by this fork
    …
}
```

There is also a per-expression `ExpressionInfo` (VERIFIED, `analyzer.rs:156-170`) with a `uniformity` and a `ref_count`. The back ends use `ref_count` to decide which expressions to "bake" into temporaries.

**The hook.** At the end of validating each function (VERIFIED, `valid/function.rs:1942-1943`):

```rust
if self.bounds_check_elision {
    crate::proc::bounds_check_elision::elide_bounds_checks(fun, module, &mut info);
}
```

The analysis entry point (VERIFIED, `proc/bounds_check_elision.rs:230-242`):

```rust
pub fn elide_bounds_checks(function: &Function, module: &Module, info: &mut FunctionInfo) {
    let workgroup_size = module.entry_points.iter().find_map(|entry_point| {
        core::ptr::eq(&entry_point.function, function).then_some(entry_point.workgroup_size)
    });                                   // ← builtin ranges come from @workgroup_size
    let mut analysis = Analysis { function, module, info, workgroup_size };
    let initial = analysis.default_ranges();                          // locals' starting ranges
    analysis.analyze_block(&function.body, initial, Values::default()); // walk the statement tree
}
```

Design consequences:
- **The analysis runs after validation succeeds**, so it may assume well-typed IR. That is why `access_needs_check` can `unwrap()` the indexable length (`proc/index.rs:331-334`: "validation should have caught those problems").
- **It writes only to `info`**, never to the module. The IR is untouched; the proof is a side table. A back end that ignores `safe_accesses` (the GLSL back end, for example) is still correct.
- **It is per function.** Helper-function parameters start as "unknown" ranges. That is why 0.7 finds interprocedural misses (RECORDED, plan §0.13: "interprocedural ranges (78 of 79 cases in one bevy file)").
- **It is on by default** in the fork (`valid/mod.rs:623`). Callers opt out with `set_bounds_check_elision(false)` (`valid/mod.rs:645-646`). That is how the benchmark builds its `no_bce` slot (`paired-vulkan-v4/src/main.rs:71`).

`safe_indices` is the older, coarser set ("safe at every use") from the first, syntactic implementation. `safe_accesses` is the per-program-point set of the interval rewrite `8757d7d` (RECORDED, plan §0.6). The back ends check both (VERIFIED, `proc/index.rs:342-345`, `back/spv/index.rs:520-525`).

---

### Layer E preview — the back ends consume one set

Each checking back end asks the same question before emitting a check.
- SPIR-V: `back/spv/index.rs:519-526` (VERIFIED):
  ```rust
  // If our bounds check elision pass proved this index is safe, skip the check entirely.
  if let GuardedIndex::Expression(expr) = index {
      if self.fun_info.safe_indices.contains(&expr)
          || access.is_some_and(|handle| self.fun_info.safe_accesses.contains(&handle))
      {
          return Ok(BoundsCheckResult::Computed(self.cached[expr]));   // raw index, no clamp
      }
  }
  ```
- MSL: `back/msl/writer.rs:832-838` → `index::access_needs_check(…)`.
- HLSL: `back/hlsl/writer.rs:3961` → `index::access_needs_check(…)`.

The fork added the `access: Option<Handle<Expression>>` parameter (VERIFIED: upstream `access_needs_check` at `688e476` has no `access` parameter, `wgpu-upstream/naga/src/proc/index.rs:322-328`). **That one parameter is the whole interface between the analysis and code generation.** Section 0.3 shows what gets skipped.

---

## The Causation Chain

```
WGSL text
  → parser desugars `for` into Loop{ If(cond){}else{Break}; Block(body) } + continuing(update)   parse/mod.rs:1457
  → lowerer turns `var` into LocalVariable + Store, every read into Load, `&&` into If + temp bool  lower/mod.rs:2564
  → validator type-checks, builds FunctionInfo                                                     valid/function.rs
  → elide_bounds_checks walks Statements, tracking Ranges (per local) + Values (per expression)   bounds_check_elision.rs:230
  → proven Access handles go into info.safe_accesses                                               analyzer.rs:254
  → back end: `if safe_accesses.contains(access) { emit raw index } else { emit policy check }`   back/spv/index.rs:522
```

**Correctness argument for the interface.** A handle identifies exactly one `Access` expression at exactly one `Emit` point. Expressions cannot be re-emitted: "the (unique) `Statement::Emit`" (`ir/mod.rs` docs). So a proof about handle `h` is a proof about one program point. The back end consults it for that one access and no other. There is no way for a proof to "leak" to a different access with the same source text; `s[i]` at [12] and `s[i]` elsewhere are different handles.

---

## ASCII diagram — Naga's pipeline, with where each fact lives

```
            ┌──────── front/wgsl ────────┐   ┌─ front/glsl ─┐   ┌─ front/spv ─┐
 text ────► │ parse → AST → lower → IR   │   │  → IR        │   │  → IR       │
            │ (for→Loop, &&→If+temp,     │   │ (&& stays    │   │ (OpLogicalAnd│
            │  var→Local+Store/Load)     │   │  LogicalAnd) │   │  stays)      │
            └─────────────┬──────────────┘   └──────┬───────┘   └──────┬──────┘
                          └─────────────┬───────────┴──────────────────┘
                                        ▼
                  naga::Module { types, global_variables, functions, entry_points }
                     each Function { local_variables: Arena<LocalVariable>,
                                     expressions:     Arena<Expression>,   ← handles [0],[1],…
                                     body:            Block (Statement tree) }
                                        │
                                        ▼  valid::Validator::validate
                  ModuleInfo { functions: Vec<FunctionInfo>, entry_points: Vec<FunctionInfo> }
                     FunctionInfo { uniformity, …, safe_indices, safe_accesses }  ← side tables by handle
                                        │
               ┌──────────────┬─────────┴────┬──────────────┬──────────────┐
               ▼              ▼              ▼              ▼              ▼
          back/spv       back/msl       back/hlsl      back/glsl      back/wgsl, dot
         (consults)     (consults)     (consults)     (no checks)    (debug/output)
```

---

## Code / IR example — mini-project output reproduced

See Layer B.1–B.3 above. All four listings are verbatim tool output. The tool source is `IICT_CONTEXT/epoch/tools/bce-epoch-tools/src/main.rs`. It uses only public Naga API: `front::wgsl::parse_str`, `front::glsl::Frontend`, `Validator::set_bounds_check_elision`, `FunctionInfo::safe_accesses`, `Handle::index()`.

---

## Surface Detection Self-Audit (FM1–FM6)

| # | Test |
|---|---|
| FM1 | Point at the exact IR node a proof is attached to in `s[lid + stride]`. (Handle [26], an `Access`, not the `Load` [27] and not the `Binary` [25].) |
| FM2 | Name the two stages in which a WGSL `for` loses its condition, and where each lives. (Parser desugars it: `parse/mod.rs:1457`. Lowerer builds `Statement::Loop`.) |
| FM3 | Why must the analysis keep *two* maps? (Locals change via `Store`; each `Load` is a new immutable value.) |
| FM4 | Why can the analysis prove GLSL's `s[lid]` but not WGSL's in `and.*`? (WGSL's `&&` became `If(Load(temp bool))`, which carries no range fact.) |
| FM5 | What would break if Naga hash-consed identical expressions so the two `Load`s of `i` shared a handle? (They are different *values* when a `Store` intervenes. Merging them would be a miscompile, and a proof about one would wrongly cover the other.) |
| FM6 | Why is a proof keyed by *access* handle rather than *index* handle safer? (The same index expression can feed two accesses into arrays of different lengths. `safe_indices` "safe at every use" is the coarse, older form.) |

---

## Correctness and Testing Dimension

- **Front-end coverage is a soundness requirement, not a nicety.** B.3 shows the three front ends produce different IR for one idea. The stress suite now includes GLSL (`glsl_access_proofs`) and SPIR-V round-trip (`spv_roundtrip_access_proofs`) helpers (RECORDED, session summary of the F001 work; helper names VERIFIED by `grep` below). Run:
  `grep -n 'fn glsl_access_proofs\|fn spv_roundtrip_access_proofs' wgpu/naga/tests/naga/bounds_check_stress.rs`
- **Snapshot tests** (`naga/tests/out/analysis/*.info.ron`) serialise `FunctionInfo`, including `safe_accesses`. Every added proof shows up as a diff that must be reviewed. Feature 005 added 9 to `spv-shadow.info.ron`, all constant indices 0..2 into length-4 arrays (RECORDED, session summary).
- **What IR-level tests cannot show:** that a back end *uses* the proof. That needs back-end tests: `msl_uses_access_specific_proof` and `hlsl_uses_access_specific_proof` (RECORDED, ledger CORRECTION 2026-09-28).

---

## Common Bugs and Misconceptions

1. **"A WGSL variable is an SSA value."** A `var` is a `LocalVariable` *pointer*; each read is a fresh `Load`. Only `let` bindings are plain expressions.
2. **"The loop condition is on the `Loop` node."** Not for `for`/`while`. It is an `If … Break` inside the body. `break_if` is only for `loop { … continuing { break if … } }`.
3. **"`&&` is an operator in Naga IR, so the analysis sees it."** Only for GLSL and SPIR-V input. WGSL runtime `&&` is control flow (`lower/mod.rs:2564`).
4. **"Handles are global IDs."** They are per-arena, per-function indices. [26] in `main` and [26] in a helper are unrelated.
5. **"The analysis modifies the shader."** It only fills `FunctionInfo` side tables; the IR is unchanged.
6. **"`AccessIndex` needs elision too."** It has a constant index. `access_needs_check` resolves it statically (`proc/index.rs:336-341`), and struct fields are skipped entirely (`proc/index.rs:392-400`).
7. **"If the IR validates, the proof is right."** Validation checks types and structure, not ranges. The two soundness bugs (0.8) produced perfectly valid IR with wrong `safe_accesses`.

---

## Integrated Research

- Naga IR documentation: the module docs at `naga/src/ir/mod.rs:1-230` are the primary source. They are unusually good; read them whole.
- SPIR-V specification (archived: `references/raw/specs/SPIR-V.html`, 2.2 MB; VERIFIED present). §2.11 "Structured Control Flow" explains why Naga keeps a statement tree (VERIFIED: heading found in the archived HTML).
- SSA background: Cytron et al. 1991 is **not** in `references/` (UNKNOWN locally). Do not cite it in the poster without archiving it.

---

## Connection to the Real Project

| Concept | File (fork `19ffb4e`) |
|---|---|
| IR definitions | `wgpu/naga/src/ir/mod.rs` (Expression 1574, Statement 2007, Function 2399, Module 2799) |
| Arenas and handles | `wgpu/naga/src/arena/{mod.rs,handle.rs}` |
| WGSL `for` desugaring | `wgpu/naga/src/front/wgsl/parse/mod.rs:1457-1504` |
| WGSL `&&` lowering | `wgpu/naga/src/front/wgsl/lower/mod.rs:2564-2591` |
| SPIR-V/GLSL `LogicalAnd` | `front/spv/next_block.rs:1836-1839`; `front/glsl/context.rs:712` |
| Validator hook | `wgpu/naga/src/valid/function.rs:1942-1943`; flag `valid/mod.rs:375,623,645` |
| Proof sets | `wgpu/naga/src/valid/analyzer.rs:252-254` |
| Analysis entry | `wgpu/naga/src/proc/bounds_check_elision.rs:230-242` |
| Teaching tool + shaders | `IICT_CONTEXT/epoch/tools/bce-epoch-tools/` |

---

## Resources

- `naga/src/ir/mod.rs` module docs (lines 1-230): the IR spec, written by its authors.
- `naga/src/arena/mod.rs` docs: arenas and handles in 20 lines.
- The tool in this repo: run it on your own shaders. It is the fastest way to build intuition.

---

## Note-Taking Instructions

1. Copy the `six.wgsl` IR listing by hand. Next to each handle, write which source characters produced it.
2. Draw the reduce_tree `Loop` as nested boxes (body / Block / continuing). Mark where `stride`'s range is *learned* (entry guard) and where it *changes* (continuing).
3. Write one sentence: "A proof is attached to ____ and consumed by ____ at file:line ____."

---

## Mini-Project — dump the IR of a 6-line shader and label every handle

```bash
cd IICT_CONTEXT/epoch/tools/bce-epoch-tools
cargo build --release              # ~20 s first time; builds naga from ../../../../wgpu/naga
./target/release/bce-epoch-tools ir shaders/six.wgsl
./target/release/bce-epoch-tools ir shaders/and.wgsl
./target/release/bce-epoch-tools ir shaders/and.comp
./target/release/bce-epoch-tools ir shaders/reduce_tree.wgsl
```

**Expected** (VERIFIED 2026-09-29 on `19ffb4e`):
- `six.wgsl` → `safe_accesses = [9, 12]`
- `and.wgsl` → `safe_accesses = []` (WGSL `&&` precision gap)
- `and.comp` → function `main`: `safe_accesses = [12]`; entry point wrapper: `[]`
- `reduce_tree.wgsl` → `safe_accesses = [3, 19, 21, 26]`

**Branches:**
- *success*: all four match.
- *partial*: same sets, different numbers. The front end changed how many expressions it emits; relabel by hand.
- *bad*: `reduce_tree` lacks 26. You are on a revision before `2ffe1b7`/`19ffb4e`; run `git -C wgpu log -1`.
- *disaster*: build fails. `naga` path wrong; the manifest uses the relative path `../../../../wgpu/naga`.

**Extension:** rewrite `and.wgsl` with nested `if (lid < 128u) { if (lid > 3u) { … } }`. Predict whether `s[lid]` is proven, then run. (VERIFIED 2026-09-29, `shaders/and_nested.wgsl`: `safe_accesses = [6]`, so the nested form is proven. The gap is specific to the lowered `&&`.)

---

## Mastery Checklist

**WHY**
- [ ] Why does Naga have an IR at all? (3 inputs × 4 outputs; one analysis.)
- [ ] Why arenas + handles rather than boxed trees?

**Trace**
- [ ] For `s[lid + stride]`, list the handles from `FunctionArgument` to `Access`.
- [ ] Where, in which file, does WGSL's `for` condition become an `If … Break`?

**Causation**
- [ ] Why does each `Load` of `stride` need its own range?
- [ ] Why could WGSL-only tests never find the `LogicalAnd` soundness bug?

**Counterfactual**
- [ ] If the WGSL front end emitted `Binary LogicalAnd` for runtime `&&`, what would change in `and.wgsl`'s proofs? (It would match GLSL: `s[lid]` proven.)
- [ ] If `elide_bounds_checks` ran *before* validation, what could go wrong? (Ill-typed IR; `unwrap`s in `access_needs_check` could panic.)

---

## Teach-Back

Explain to a classmate, using only the `six.wgsl` listing:
1. what `[9]` and `[12]` are;
2. why `i` is read three times;
3. what `Emit` does;
4. where the proof is stored;
5. how the SPIR-V back end uses it.

---

## Interview Fireproofing

- **Q: "Why not run your analysis on SPIR-V directly, like spirv-opt would?"**
  A: wgpu never sees SPIR-V for WGSL input until *after* Naga's back end has already inserted the checks. The analysis must run before emission, on the IR every back end shares. Running on Naga IR also covers MSL and HLSL with one implementation.
- **Q: "Is your analysis front-end independent?"**
  A: It runs on front-end-independent IR, but front ends produce different IR shapes for the same source. We found a soundness bug reachable only from GLSL/SPIR-V `LogicalAnd` and WGSL `&`, and a precision gap reachable only from WGSL `&&`. Both are now tested from multiple front ends.
- **Q: "How does the back end know which check to drop?"**
  A: By access handle. `FunctionInfo.safe_accesses` is a `HashSet<Handle<Expression>>`. The SPIR-V writer checks membership at `back/spv/index.rs:522` and emits the raw index.
