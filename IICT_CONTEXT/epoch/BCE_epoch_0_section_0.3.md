# [EPOCH 0 — SECTION 0.3]: BOUNDS CHECKS — WHAT THE SPEC REQUIRES, WHAT NAGA EMITS, WHAT THEY COST

> **Evidence rule for this file** (AGENTS.md Rule 0). Labels: **VERIFIED** (read or run on
> 2026-09-29 against wgpu fork `19ffb4e`), **RECORDED** (project artifact, path cited),
> **REPORTED**, **UNKNOWN**. All SPIR-V / MSL / HLSL listings are real output of
> `IICT_CONTEXT/epoch/tools/bce-epoch-tools` on `shaders/policies.wgsl`, or of `spirv-dis` on recorded
> benchmark binaries. Cost numbers are copied by script from `runs-v4/summary-*-{83a90e0,19ffb4e}.json`.

⛔ **DO NOT SKIP.** This section defines the object that everything after it removes.
- **0.4 and 0.6** are about proving that a check "never fires". You need to know exactly what instruction "the check" is under each policy. It is a clamp (`UMin`) under Restrict, and a compare-and-branch (`OpULessThan` + `OpPhi`) under ReadZeroSkipWrite.
- **0.9** includes a checker bug, the "opcode 166" incident. It happened because counting check instructions by opcode is subtler than it looks: the user's own `if (lid < stride)` is also an `OpULessThan`.
- **0.10** found the v3 benchmark measured ReadZeroSkipWrite while production Vulkan wgpu uses Restrict. You can't see why that matters without the side-by-side listings below.
- **0.11 and 0.12** report costs per kernel and per GPU. This section explains why the *same* number of removed clamps can cost +15% on one kernel and 0% on another.

---

## Prerequisites

- **0.1**: WGSL §6.5.7's menu of out-of-bounds outcomes, and the per-backend policy table.
- **0.2**: `Access` vs `AccessIndex`, handles, `safe_accesses`, and how the SPIR-V back end looks them up (`back/spv/index.rs:519-526`).
- Reading SPIR-V disassembly at the level of `%id = OpSomething %type %operands`.

---

## THE WHY

### WHY 1 (Existence)

WGSL forbids an out-of-bounds access from touching memory outside its variable (0.1, Layer A). Hardware robustness does not cover workgroup or private arrays (0.1, Layer G). So for those arrays, *something in the compiled shader* must make an out-of-range index harmless. A **bounds check** is that something: code, emitted by Naga's back end, that runs before the memory operation. No check means no containment.

### WHY 2 (Mechanical)

To contain an index, a shader has only two options:
1. **Change the index**: map any `i` to some in-range `i'`. The cheapest version is one unsigned `min(i, len − 1)`. Negative signed indices, reinterpreted as huge unsigned values, also clamp to the last element.
2. **Guard the operation**: compute `i < len` and branch around the load or store. A skipped load must still produce a value, so it needs a merge (`OpPhi`) with a default of 0.

Option 1 adds one ALU instruction on the address path and no control flow. Option 2 adds a compare, a branch, a merge block, and a φ. On a GPU, where 32 or more threads execute one instruction stream together, a branch adds structured control flow. The driver must preserve it or prove it away.

### WHY 3 (Design decision): three policies, and why wgpu picks Restrict

Naga offers exactly three policies (VERIFIED, `naga/src/proc/index.rs:39-62`):

```rust
pub enum BoundsCheckPolicy {
    /// Replace out-of-bounds indexes with some arbitrary in-bounds index.
    ///
    /// (This does not necessarily mean clamping. For example, interpreting the
    /// index as unsigned and taking the minimum with the largest valid index
    /// would also be a valid implementation. That would map negative indices to
    /// the last element, not the first.)
    Restrict,

    /// Out-of-bounds reads return zero, and writes have no effect.
    ///
    /// When applied to a chain of accesses, like `a[i][j].b[k]`, all index
    /// expressions are evaluated, regardless of whether prior or later index
    /// expressions were in bounds. But all the accesses per se are skipped
    /// if any index is out of bounds.
    ReadZeroSkipWrite,

    /// Naga adds no checks to indexing operations. Generate the fastest code
    /// possible. This is the default for Naga, as a translator, but consumers
    /// should consider defaulting to a safer behavior.
    Unchecked,
}
```

Each maps onto an entry in the WGSL §6.5.7 menu (0.1):
- **Restrict** is "a value from any memory location(s) in the originating variable" for loads, and "store the value to any memory locations(s) in the originating variable" for stores.
- **ReadZeroSkipWrite** is "the zero value for store type" for loads, and "not be executed" for stores.
- **Unchecked** is *not* on the menu. It is legal only when something else (the driver, or a proof) guarantees containment.

The policy is chosen *per address space* (VERIFIED, `proc/index.rs:143-164`, `choose_policy`):
- binding arrays use `binding_array`;
- `Storage` and `Uniform` pointers use `buffer`;
- everything else uses `index`: workgroup, private and function memory, and vectors and matrices by value.

wgpu-hal's production choice is **Restrict** for `index` on Vulkan, Metal and D3D12 (0.1, F.4). The cost argument in WHY 2 favours it: no control flow. Naga's own documentation disagrees (`proc/index.rs:28-29` says native wgpu uses ReadZeroSkipWrite). The code wins; see 0.1's doc/code-gap note and ledger entry 2026-09-29.

### WHY 4 (Historical)

- **Before this fork**, Naga removed a check only if both index and length were compile-time constants and the index was in range. That was upstream's entire `access_needs_check` logic at `688e476` (VERIFIED, `wgpu-upstream/naga/src/proc/index.rs:322-343`). Every `s[lid]` got a clamp, although `lid < 256 = len(s)` is guaranteed by `@workgroup_size(256)`.
- **The submitted paper** measured ReadZeroSkipWrite binaries on one workgroup (RECORDED, `reports/Benchmark v4 results.md` "What changed vs. v3"). Upstream's doc comment named that policy, but production Vulkan does not use it (0.1).
- **The v4 harness** measures Restrict/Unchecked, matching production Vulkan with `robustBufferAccess2` (VERIFIED, `paired-vulkan-v4/src/main.rs:369`).

---

## What Is This? (analogy first)

A **coat check with 64 hooks**. A guest hands over ticket number `i`.
- **Restrict**: "if the number is over 63, use hook 63." The attendant always walks to *some* hook. There is one extra glance at the ticket and no decision tree.
- **ReadZeroSkipWrite**: "if the number is over 63, hand back nothing and don't take the coat." The attendant stops, decides, and takes one of two paths.
- **Unchecked**: "walk to hook `i`, wherever that is." That is fine if someone upstream guaranteed `i ≤ 63`. That guarantee is either a separate security guard (driver robustness) or a proof (this project).
- **Elision**: the manager has shown that this particular queue only ever issues tickets 0–63. For *that queue*, the attendant may skip the glance. Other queues still get it.

---

## How It Actually Works — Layers D and E of the 8-layer trace

### The experiment shader

`IICT_CONTEXT/epoch/tools/bce-epoch-tools/shaders/policies.wgsl` (VERIFIED content):

```wgsl
@group(0) @binding(0) var<storage, read_write> buf: array<u32>;   // runtime-sized STORAGE buffer
var<workgroup> s: array<u32, 64>;                                   // fixed-size WORKGROUP array
@compute @workgroup_size(64)
fn main(@builtin(local_invocation_index) lid: u32) {
    let j = buf[lid];     // (A) length known only at dispatch → never provable by a shader compiler
    s[lid] = j;           // (B) lid ∈ [0, 63], len 64          → provable
    s[j] = lid;           // (C) j is data loaded from memory   → never provable
}
```

The IR, with the proof (VERIFIED, `bce-epoch-tools ir shaders/policies.wgsl`):

```
[ 2] Access { base: [1], index: [0] }                  ← (A) buf[lid]
[ 5] Access { base: [4], index: [0] }   <= in safe_accesses   ← (B) s[lid]
[ 7] Access { base: [6], index: [3] }                  ← (C) s[j]
-- safe_accesses = [5]
```

This is *one* proof. The prediction is that under every checking policy, turning elision on removes exactly one check, (B), and nothing else.

### Layer D — the decision function

Every back end asks `access_needs_check` (MSL, HLSL) or its SPIR-V twin `write_bounds_check` (VERIFIED, `proc/index.rs:321-351`; `back/spv/index.rs:508-543`). In order:

1. **Resolve constants.** `index.try_resolve_to_constant(...)`. If index and length are both known and `index < length`, there is no check. This is the only rule upstream had.
2. **Consult the proof** (fork addition, `proc/index.rs:342-347`):
   ```rust
   if let GuardedIndex::Expression(expr) = index {
       if info.safe_indices.contains(&expr)
           || access.is_some_and(|handle| info.safe_accesses.contains(&handle))
       {
           return None;                                   // proven: no check
       }
   }
   ```
3. **Otherwise pick the policy** with `choose_policy(base, …)` and emit it:
   ```rust
   Ok(match policy {                                                     // back/spv/index.rs:533-542
       BoundsCheckPolicy::Restrict => self.write_restricted_index(base, index, block)?,
       BoundsCheckPolicy::ReadZeroSkipWrite => self.write_index_comparison(base, index, block)?,
       BoundsCheckPolicy::Unchecked => match index { … Computed(self.cached[expr]) … },
   })
   ```

The SPIR-V back end communicates the outcome through `BoundsCheckResult` (VERIFIED, `back/spv/index.rs:21-50`):
- `KnownInBounds(u32)`;
- `Computed(Word)`: "When `Restrict` is in force, this is a clamped version of the index the user supplied";
- `Conditional { condition_id, index_id }`: "returned when `ReadZeroSkipWrite` is in force".

A proven access returns `Computed(self.cached[expr])`: the raw, unclamped index.

### Layer D′ — the length: `Known` or `Dynamic`

`IndexableLength` is either `Known(u32)` ("Values of this type always have the given number of elements") or `Dynamic` ("The number of elements is determined at runtime") (VERIFIED, `proc/index.rs:635-641`).

For a `Dynamic` array the SPIR-V back end must *compute* the maximum index at runtime (VERIFIED, `back/spv/index.rs:295-323`). A known length becomes a constant (`known_length - 1`). A dynamic one becomes `OpArrayLength` followed by `OpISub … 1`. **That runtime length is exactly why access (A) can never be proven by a shader compiler**: the fact needed ("`lid < arrayLength(&buf)`") is decided by the host when it binds the buffer. Section 0.7 finds that this category is 46% of real-world misses (RECORDED, plan §0.7).

### Layer E — what each policy emits (real output)

Counts across all six variants (VERIFIED 2026-09-29, `bce-epoch-tools spv shaders/policies.wgsl <out>`, then `spirv-dis | grep -c`). The tool sets `index = buffer = policy` and `force_loop_bounding: false` to keep the listings short:

```
restrict-elision-off.spv     UMin=3 ULessThan=0 ArrayLength=1 Phi=0 words=249
restrict-elision-on.spv      UMin=2 ULessThan=0 ArrayLength=1 Phi=0 words=242
rzsw-elision-off.spv         UMin=0 ULessThan=3 ArrayLength=1 Phi=1 words=279
rzsw-elision-on.spv          UMin=0 ULessThan=2 ArrayLength=1 Phi=1 words=261
unchecked-elision-off.spv    UMin=0 ULessThan=0 ArrayLength=0 Phi=0 words=210
unchecked-elision-on.spv     UMin=0 ULessThan=0 ArrayLength=0 Phi=0 words=210
```

The prediction held: exactly one check disappears under each checking policy, and `Unchecked` is unaffected. Note the size ordering too: ReadZeroSkipWrite (279 words) > Restrict (249) > Unchecked (210).

#### E.1 — Restrict, elision OFF (function body, `spirv-dis --raw-id`, VERIFIED)

`%32 = OpConstant %3 1` and `%38 = OpConstant %3 63` (VERIFIED by grep).

```
         %12 = OpLabel
         %15 = OpLoad %3 %13                    ; lid  (from the Input builtin variable)
         %20 = OpAccessChain %18 %7 %19         ; &buf.member0 (the runtime array inside the Block struct)
               OpBranch %21
         %21 = OpLabel                          ; ── workgroup zero-init prologue (see note) ──
         %23 = OpIEqual %24 %15 %19             ;   if (lid == 0)
               OpSelectionMerge %25 None
               OpBranchConditional %23 %26 %25
         %26 = OpLabel
               OpStore %10 %22                  ;     s = null
               OpBranch %25
         %25 = OpLabel
               OpControlBarrier %27 %27 %28     ;   barrier
               OpBranch %29
         %29 = OpLabel                          ; ── user code ──
         %31 = OpArrayLength %3 %7 0            ; (A) runtime length of buf
         %33 = OpISub %3 %31 %32                ;     max index = len − 1
         %34 = OpExtInst %3 %1 UMin %15 %33     ;     CHECK (A): min(lid, len−1)
         %35 = OpAccessChain %30 %20 %34
         %36 = OpLoad %3 %35                    ;     j = buf[...]
         %39 = OpExtInst %3 %1 UMin %15 %38     ; (B) CHECK: min(lid, 63)
         %40 = OpAccessChain %37 %10 %39
               OpStore %40 %36                  ;     s[...] = j
         %41 = OpExtInst %3 %1 UMin %36 %38     ; (C) CHECK: min(j, 63)
         %42 = OpAccessChain %37 %10 %41
               OpStore %42 %15                  ;     s[...] = lid
               OpReturn
```

#### E.2 — Restrict, elision ON (the only change, VERIFIED)

```
         %38 = OpAccessChain %37 %10 %15        ; (B) raw lid — no UMin
               OpStore %38 %36
         %40 = OpExtInst %3 %1 UMin %36 %39     ; (C) still clamped
```

(A) is unchanged. It still has `OpArrayLength`, `OpISub`, `UMin`.

#### E.3 — ReadZeroSkipWrite, elision OFF (VERIFIED)

`%6 = OpConstant %3 64`, `%34 = OpConstantNull %3`.

```
         %29 = OpLabel
         %31 = OpArrayLength %3 %7 0
         %32 = OpULessThan %24 %15 %31          ; (A) CHECK: lid < len ?
               OpSelectionMerge %35 None
               OpBranchConditional %32 %36 %35
         %36 = OpLabel
         %33 = OpAccessChain %30 %20 %15
         %37 = OpLoad %3 %33                    ;     in-bounds path: load
               OpBranch %35
         %35 = OpLabel
         %38 = OpPhi %3 %34 %29 %37 %36         ;     j = (came from %29) ? 0 : loaded   ← read-zero
         %40 = OpULessThan %24 %15 %6           ; (B) CHECK: lid < 64 ?
               OpSelectionMerge %42 None
               OpBranchConditional %40 %43 %42
         %43 = OpLabel
         %41 = OpAccessChain %39 %10 %15
               OpStore %41 %38                  ;     store only if in bounds             ← skip-write
               OpBranch %42
         %42 = OpLabel
         %44 = OpULessThan %24 %38 %6           ; (C) CHECK: j < 64 ?
               OpSelectionMerge %46 None
               OpBranchConditional %44 %47 %46
         %47 = OpLabel
         %45 = OpAccessChain %39 %10 %38
               OpStore %45 %15
               OpBranch %46
         %46 = OpLabel
               OpReturn
```

The load version of this shape is `write_conditional_indexed_load` (VERIFIED, `back/spv/index.rs:446-481`). Its comment describes exactly the φ shape above: "result = zero; if in_bounds { result = do the load; } use result;". For access chains with several indices, the conditions are AND-ed together (`extend_bounds_check_condition_chain`, VERIFIED, `back/spv/block.rs:2719-2727`, `:2731+`).

**Compare E.1 and E.3.** Each ReadZeroSkipWrite check costs a compare, a structured selection, and a φ for loads, instead of a single `UMin`. That is why the v3 benchmark overstated the per-check cost relative to production: RZSW's control flow is not what production Vulkan wgpu emits. (The v3 numbers themselves are SUPERSEDED/INVALID for other reasons too; RECORDED, ledger "BENCHMARK VALIDITY" entry.)

#### E.4 — MSL (Metal), Restrict, elision OFF then ON (VERIFIED)

```cpp
    uint j = buf[metal::min(unsigned(lid), (_buffer_sizes.size0 - 0 - 4) / 4)];  // (A) runtime size from a side buffer
    s.inner[metal::min(unsigned(lid), 63u)] = j;                                // (B)
    s.inner[metal::min(unsigned(j), 63u)] = lid;                                // (C)
```
With elision on:
```cpp
    uint j = buf[metal::min(unsigned(lid), (_buffer_sizes.size0 - 0 - 4) / 4)];
    s.inner[lid] = j;                                                           // (B) proven
    s.inner[metal::min(unsigned(j), 63u)] = lid;
```
MSL has no `OpArrayLength`. The runtime-array length is passed in `_buffer_sizes`, and wgpu-hal Metal clamps *buffers too* (buffer policy Restrict, 0.1 F.4).

#### E.5 — HLSL (D3D12), elision OFF then ON (VERIFIED)

```hlsl
    uint j = asuint(buf.Load(lid*4));          // (A) ByteAddressBuffer: no clamp emitted
    s[min(uint(lid), 63u)] = j;                // (B)
    s[min(uint(j), 63u)] = lid;                // (C)
```
With elision on: `s[lid] = j;` and `s[min(uint(j), 63u)] = lid;`.

The HLSL back end emits no clamp for the raw-buffer load (A). What D3D12 does with an out-of-range `ByteAddressBuffer.Load` is **UNKNOWN** from this repo's sources. It is not documented in `references/`, so this section makes no claim about it.

#### Note — the workgroup zero-init prologue

The `if (lid == 0) { s = null } barrier` block in E.1/E.3, and the `if (lid == 0u) { s = {}; }` in MSL, are Naga's **polyfill** for WGSL's rule that workgroup memory starts zeroed. The tool uses `spv::Options::default()`. wgpu-hal Vulkan uses `ZeroInitializeWorkgroupMemoryMode::Native` when the device supports it (VERIFIED, `wgpu-hal/src/vulkan/adapter.rs:2829-2835`), and the v4 harness does the same (VERIFIED, `paired-vulkan-v4/src/main.rs:79`). So production binaries on such devices do not carry this prologue. It is unrelated to bounds checks, but it shows up in every listing, so don't mistake it for one.

---

## The running example on real, recorded binaries

`reduce_tree` under the production Vulkan policy (`index: Restrict`, `buffer: Unchecked`, `force_loop_bounding: true`, native zero-init), from the `19ffb4e` campaign (VERIFIED, `runs-v4/20260929T125306Z-reduce_tree/artifacts/`, `spirv-dis --raw-id`; `%42 = OpConstant %3 255`):

```
; no_bce.spv — inside `if (lid < stride)`:
         %76 = OpULessThan %53 %20 %75          ; ← the USER's `lid < stride`, NOT a bounds check
               OpBranchConditional %76 %78 %77
         %78 = OpLabel
         %79 = OpExtInst %3 %1 UMin %20 %42     ; clamp s[lid]           (read)
         %80 = OpAccessChain %35 %14 %79
         %81 = OpLoad %3 %80
         %82 = OpLoad %3 %32                    ; stride
         %83 = OpIAdd %3 %20 %82                ; lid + stride
         %84 = OpExtInst %3 %1 UMin %83 %42     ; clamp s[lid + stride]  ← the running example's check
         %85 = OpAccessChain %35 %14 %84
         %86 = OpLoad %3 %85
         %87 = OpIAdd %3 %81 %86
         %88 = OpExtInst %3 %1 UMin %20 %42     ; clamp s[lid]           (write)
         %89 = OpAccessChain %35 %14 %88
               OpStore %89 %87

; bce.spv (== unchecked.spv, same sha256 f97b7fe6…) — same block:
         %74 = OpULessThan %51 %20 %73          ; user's comparison, still there
         %77 = OpAccessChain %35 %14 %20        ; raw lid
         %80 = OpIAdd %3 %20 %79
         %81 = OpAccessChain %35 %14 %80        ; raw lid + stride
         %84 = OpAccessChain %35 %14 %20        ; raw lid
```

Totals: `no_bce` 4 `UMin`, `bce` 0, `unchecked` 0. Each has **1 `OpULessThan`**, the user's own comparison (VERIFIED). A tool that counts `OpULessThan` as "bounds checks" would report 1 check remaining in the fully-elided binary. That trap is related to the opcode-166 incident in 0.9. The robust method is to count per-access proofs (`safe_accesses`), then cross-check against a policy-specific opcode.

Also visible in the diff (VERIFIED): the `force_loop_bounding` counter (`OpVariable` of `vec2<u32>` initialised to `(0xFFFFFFFF, 0xFFFFFFFF)`, decremented each iteration, with `OpAll(OpIEqual(…, 0))` → break) is present in **both** binaries. It is a separate runtime check (`force_loop_bounding: true`, `adapter.rs:2836`) that this project does not touch.

---

## What they cost — measured, not assumed

The v4 harness times four variants per round: `no_bce`, `bce`, `bce_dup`, `unchecked`. Two ratios isolate the cost of checks (RECORDED, `analyze_v4.py` docstring, lines 4-6):
- `no_bce / unchecked` is the **cost of all checks** production Vulkan inserts;
- `bce / unchecked` is the **cost of the checks elision could not remove**.

**Cost of all checks** (`all_checks_geomean`, copied by script from `runs-v4/summary-{nvidia,intel}-19ffb4e.json`; clamp counts from `statics`):

| kernel | clamps (no_bce) | NVIDIA RTX 4060 Laptop | Intel RPL-S iGPU |
|---|---|---|---|
| empty (control) | 0 | 1.0008 | 1.0000 |
| scan_shared | 2 | **1.1579** | **1.1162** |
| matmul_tiled | 8 | **1.1080** | 1.0012 |
| reduce_tree | 4 | 1.0130 | **1.0732** |
| prefix_scan_hs | 5 | 1.0091 | 1.0364 |
| vello_binning | 6 | **1.1518** | 1.0000 |
| private_array | 4 | 0.9994 | 1.0006 |

**What this table proves, and what it doesn't:**
1. **Clamp count does not predict cost.** 2 clamps cost 15.8% (scan_shared, NVIDIA); 8 clamps cost 0.1% (matmul_tiled, Intel); 4 clamps cost 0% (private_array, both). VERIFIED from the table.
2. **Cost is GPU-specific.** matmul_tiled: +10.8% NVIDIA, +0.1% Intel. reduce_tree: +1.3% NVIDIA, +7.3% Intel. VERIFIED.
3. **Why?** This is **not measured** in this project. No ISA (machine-code) dumps from either driver were inspected. The candidate explanations below are hypotheses (UNKNOWN), each with the evidence that would settle it:
   - *Dependency-chain position*: the clamp sits between index computation and address generation. In a loop whose body is a short load-add-store (scan, reduction), one more dependent ALU op lengthens the critical path. **Test:** inspect the NVIDIA SASS / Mesa ISA for scan_shared and count instructions between index and `LDS`/`STS`.
   - *Blocked address folding*: `s[lid + const]` can fold the constant into the load's immediate offset; `s[min(lid + c, 255)]` cannot. **Test:** same ISA inspection.
   - *Driver removes the clamp itself*: the report's reading of private_array ("driver removes clamps itself", RECORDED, `reports/Benchmark v4 results.md`) is an **inference** from the ~1.0 ratio, not an ISA observation. It is REPORTED until an ISA dump shows it.
4. **Elision recovers exactly the measured cost.** On `19ffb4e`, `bce / unchecked` is 0.9998–1.0004 on every kernel and both GPUs. Every kernel went to 0 clamps (RECORDED, addendum in `reports/Benchmark v4 results.md`).

---

## The Causation Chain (with the legality argument)

```
access s[e] in address space A, not in safe_accesses
  → choose_policy(A) → index policy (workgroup) = Restrict on Vulkan            proc/index.rs:143-164; adapter.rs:2815
  → write_restricted_index: max = len−1 (constant, or OpArrayLength−1)        back/spv/index.rs:341-376
  → emit %c = UMin(e, max); OpAccessChain base %c
  → driver compiles; the clamp may or may not survive and may or may not be on the critical path
  → measured cost ∈ [~0%, ~16%] depending on kernel × GPU                      summary-*-19ffb4e.json
access in safe_accesses
  → write_bounds_check returns Computed(raw e)                                 back/spv/index.rs:519-526
  → OpAccessChain base %e   (identical to Unchecked output for that access)
```

**Legality.** Restrict's clamp is `c(e) = min(e, len−1)`. If the analysis proves `e ≤ len−1` on every execution reaching the access, then `c(e) = e` on every such execution. The clamp is then an identity, and deleting it preserves every observable behaviour. ReadZeroSkipWrite's guard is `e < len`. Under the same proof it is always true, so the "else" path (zero / skip) is dead, and deleting the compare, branch and φ is again an identity transformation. **One proof licenses deletion under any policy.** That is why the analysis is policy-independent, and why the tool's counts drop by exactly one under both Restrict and RZSW.

---

## ASCII diagram — one access, three policies

```
                     index e, length L (Known → constant, Dynamic → OpArrayLength)
                                          │
              ┌───────────────────────────┼───────────────────────────┐
         Restrict                 ReadZeroSkipWrite                Unchecked
              │                           │                           │
   m = L − 1                   ok = e < L                             │
   e' = UMin(e, m)             SelectionMerge / BranchConditional ok  │
   AccessChain(base, e')        ├─ true:  AccessChain(base, e); Load  │
   Load / Store                 │         (or Store)                  │
                                └─ false: (nothing)                   │
                               x = Phi(0 from false, v from true)     │
                                                                      │
   1 ALU op, no control flow   cmp + branch + merge + φ          AccessChain(base, e)
                                                                      ▲
              └──────── if access ∈ safe_accesses, every column becomes this ─┘
```

---

## Code / IR example — the mini-project, reproduced

```bash
cd IICT_CONTEXT/epoch/tools/bce-epoch-tools
cargo build --release
T=./target/release/bce-epoch-tools
$T ir  shaders/policies.wgsl | tail -1                 # -- safe_accesses = [5]
$T spv shaders/policies.wgsl /tmp/pol
for f in /tmp/pol/*.spv; do
  printf "%-28s UMin=%s ULessThan=%s Phi=%s\n" $(basename $f) \
    $(spirv-dis $f | grep -c UMin) $(spirv-dis $f | grep -c OpULessThan) $(spirv-dis $f | grep -c OpPhi)
done
diff <(spirv-dis --raw-id /tmp/pol/restrict-elision-off.spv) <(spirv-dis --raw-id /tmp/pol/restrict-elision-on.spv)
$T msl  shaders/policies.wgsl off | grep min ; $T msl  shaders/policies.wgsl | grep 's.inner'
$T hlsl shaders/policies.wgsl off | grep 's\[' ; $T hlsl shaders/policies.wgsl | grep 's\['
```

**Expected** (VERIFIED 2026-09-29, fork `19ffb4e`): the six-line count table in Layer E; the MSL/HLSL lines in E.4/E.5.

**Branches:**
- *success*: counts match. One check removed per checking policy.
- *partial*: counts match but word counts differ. Naga version drift; the per-policy *deltas* are what matter.
- *bad*: elision-on equals elision-off under Restrict. Check `safe_accesses` in the `ir` output. If it is `[]`, you are on a Naga without the analysis (e.g. upstream) or elision is disabled.
- *disaster*: `ULessThan` appears in `restrict-*`. Either the tool's policy wiring broke, or the shader gained a user comparison. Re-read the shader first.

**Extension (predict, then run):** change `s: array<u32, 64>` to `array<u32, 32>`, keeping `@workgroup_size(64)`. Predict `safe_accesses`. (Expected: `[]`. `lid ∈ [0, 63]` no longer fits length 32.) Then add `if (lid < 32u) { … }` around (B) and predict again. VERIFIED 2026-09-29: without the guard `safe_accesses = []`; with it, `[7]` (the guarded `s[lid]`).

---

## Surface Detection Self-Audit (FM1–FM6)

| # | Test |
|---|---|
| FM1 | Write, from memory, the SPIR-V for one Restrict check against a fixed length 64. (`%c = OpExtInst %u32 %glsl UMin %i %const63`, then `OpAccessChain … %c`.) |
| FM2 | Trace an access from `choose_policy` to the emitted instruction, naming both functions and files. |
| FM3 | Why does wgpu use Restrict rather than ReadZeroSkipWrite for arrays? (No control flow: one ALU op vs compare + branch + φ. Both are on the WGSL menu.) |
| FM4 | Why can't (A) `buf[lid]` ever be proven by a shader compiler? (`IndexableLength::Dynamic`: length set by the host at bind time.) |
| FM5 | If Vulkan wgpu ran on a device *without* RBA2, what changes in reduce_tree's `no_bce.spv`? (`input[wid.x*256+lid]` and `output[wid.x]` gain `OpArrayLength`/`ISub`/`UMin`. Elision cannot remove those, because they are runtime-sized.) |
| FM6 | Why does "count `OpULessThan`" not count bounds checks? (User comparisons compile to the same opcode; production uses Restrict, which emits `UMin`.) |

---

## Correctness and Testing Dimension

- **Policy-independence of proofs** is tested implicitly by the stress suite. Its helpers compile under Restrict and ReadZeroSkipWrite, and there are MSL and HLSL consumption tests (`msl_uses_access_specific_proof`, `hlsl_uses_access_specific_proof`; RECORDED, ledger CORRECTION 2026-09-28).
- **Byte-identity as an oracle.** When `bce.spv` hashes equal to `unchecked.spv` (reduce_tree above), you have a strong structural fact: the elided production binary *is* the no-checks binary. What makes that safe is the soundness of the proof, never the hash.
- **What cannot be tested by running:** that an elided access *would have been* contained. A correct proof means no execution goes out of bounds, so there is nothing to observe. Negative tests assert that the check *remains* for unprovable accesses (e.g. (C) `s[j]`), and on-GPU `diff-exec` asserts identical outputs between checked and elided binaries on in-bounds inputs (0.9).

---

## Common Bugs and Misconceptions

1. **"Restrict means clamp to 0 on negative."** It is *unsigned* `min`: a negative `i32` reinterpreted as `u32` is huge and maps to the **last** element (`proc/index.rs:44-48`).
2. **"`OpULessThan` count = number of bounds checks."** Only under ReadZeroSkipWrite, and only if the program has no comparisons of its own. See the reduce_tree listing.
3. **"More clamps removed ⇒ bigger speedup."** Refuted by the cost table: 8 removed on Intel matmul gives 0.1%, 2 removed on scan gives 11.6%.
4. **"Elision removes the buffer check on `buf[lid]` too."** Not for runtime-sized arrays (`Dynamic` length). And on Vulkan with RBA2 there is no buffer check to remove.
5. **"The v3 benchmark measured production checks."** It measured ReadZeroSkipWrite, and wgpu re-emitted the SPIR-V anyway (0.10). Both invalidate it.
6. **"The zero-init prologue is a bounds check."** It is the workgroup-zeroing polyfill, absent with native zero-init.
7. **"`force_loop_bounding` is part of bounds checking."** It is a separate runtime check that guarantees termination (`wgpu-types/src/shader.rs:26-31`). This project leaves it alone.
8. **"The driver always removes provably-dead clamps."** Sometimes it seems to (private_array, an inference), often it does not (scan, matmul on NVIDIA). ISA evidence: UNKNOWN.

---

## Integrated Research

- **WGSL §6.5.7 and WebGPU §2.1.4**: archived specs (0.1).
- **SPIR-V spec**: archived `references/raw/specs/SPIR-V.html`. See §2.11 "Structured Control Flow" (VERIFIED heading) for why RZSW needs `OpSelectionMerge`. The GLSL.std.450 extended instruction set (`UMin`) is a separate Khronos document, not archived here (UNKNOWN locally).
- **Prior art on check cost**: ABCD (PLDI 2000) and Würthinger et al. discuss JVM array-check cost. Section 0.4 treats them from the archived copies in `references/`. No GPU-specific cost model is archived; the costs above are this project's own measurements.

---

## Connection to the Real Project

| What | Where (fork `19ffb4e`) |
|---|---|
| Policy enum and docs | `wgpu/naga/src/proc/index.rs:10-62` |
| Per-address-space choice | `proc/index.rs:143-164` (`choose_policy`) |
| Decision + proof lookup | `proc/index.rs:321-351` (`access_needs_check`) |
| SPIR-V: dispatch on policy | `back/spv/index.rs:509-543` (`write_bounds_check`) |
| SPIR-V: Restrict | `back/spv/index.rs:295-376` (`write_sequence_max_index`, `write_restricted_index`) |
| SPIR-V: RZSW | `back/spv/index.rs:398-481` (`write_index_comparison` 398, `write_conditional_indexed_load` 453); `back/spv/block.rs:2711-2740` |
| MSL / HLSL consumption | `back/msl/writer.rs:832-838`; `back/hlsl/writer.rs:3961` |
| Production policies | `wgpu-hal/src/vulkan/adapter.rs:2814-2836` (and 0.1 F.4) |
| Cost artifacts | `bounds-check-testing-framework/runs-v4/summary-{nvidia,intel}-{83a90e0,19ffb4e}.json` |
| Teaching shader + tool | `IICT_CONTEXT/epoch/tools/bce-epoch-tools/shaders/policies.wgsl` |

---

## Resources

- `naga/src/proc/index.rs:1-170`: the policy design, in the authors' words.
- `naga/src/back/spv/index.rs`: short, heavily commented; read `write_restricted_index` and `write_conditional_indexed_load` whole.
- `reports/Benchmark v4 results.md`, including the 2026-09-29 addendum.

---

## Note-Taking Instructions

1. Draw the three-column diagram from memory, then check it against the one above.
2. Copy the cost table. Next to each cell, write one word: *hot* (> 5%), *mild* (1–5%), or *free* (< 1%). Circle the cells where the two GPUs disagree. Those are the reason the project reports per-GPU.
3. Write the one-line legality argument: "if `e ≤ len−1` on every execution, `min(e, len−1) = e`."

---

## Mini-Project

As in "Code / IR example" above. Then do the extension: shrink `s` to 32 and add the `lid < 32u` guard. Record which branch (success / partial / bad / disaster) you landed on.

---

## Mastery Checklist

**WHY**
- [ ] Why does WGSL permit both clamping and read-zero?
- [ ] Why is Unchecked legal for buffers on Vulkan with RBA2 but not for workgroup arrays?

**Trace**
- [ ] From `Access` handle to emitted SPIR-V under Restrict: every function called.
- [ ] Where does a runtime-array length come from in SPIR-V? In MSL?

**Causation**
- [ ] Why does one proof remove a check under *every* policy?
- [ ] Why does removing 2 clamps give +15.79% on NVIDIA scan_shared but 4 clamps give ~0% on private_array? (Answer honestly: measured, but the mechanism is UNKNOWN pending ISA inspection.)

**Counterfactual**
- [ ] If wgpu switched Vulkan `index` to RZSW, what would reduce_tree's inner loop contain? (Three compare-branch-merge regions, two with φ.)
- [ ] If a proof were wrong for `s[lid + stride]`, which instruction would be missing, and what memory could the store reach? (`UMin %83 %42`; workgroup memory beyond `s`, i.e. outside the variable, violating §6.5.7.)

---

## Teach-Back

Explain to a classmate in six sentences:
1. what a bounds check is;
2. the three policies;
3. which one Vulkan wgpu uses for workgroup arrays, and why;
4. what the SPIR-V for it looks like;
5. how elision removes it;
6. why the measured cost varies from 0% to 16%, and which part of that is still unexplained.

---

## Interview Fireproofing

- **Q: "A clamp is one instruction. How can it cost 15%?"**
  A: We measured it, but we have not explained it. On NVIDIA scan_shared the two clamps cost 1.1579× (`all_checks_geomean`, 10 runs; the elision effect's per-run min–max is 1.1579–1.1579). The same clamps cost 1.1162× on Intel. Candidate mechanisms are dependency-chain length in a short loop body and blocked immediate-offset folding. Neither is confirmed; that needs ISA dumps.
- **Q: "Why not just use ReadZeroSkipWrite everywhere? It's more predictable."**
  A: It adds control flow per access (compare, branch, merge, φ; E.3). Restrict is one ALU op. Both satisfy WGSL §6.5.7. Production wgpu picks Restrict.
- **Q: "Does elision depend on the policy?"**
  A: No. The proof says the index is always in range. Under that fact every policy's check is an identity, so the same proof deletes a `UMin` or a compare-branch-φ. Verified on the teaching shader: exactly one check removed under both Restrict and RZSW.
