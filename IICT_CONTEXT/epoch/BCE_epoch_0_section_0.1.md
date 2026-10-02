# [EPOCH 0 — SECTION 0.1]: WHAT A WebGPU PROGRAM IS — THE SAFETY CONTRACT, AND wgpu AS A BACKEND-AGNOSTIC LAYER

> **Evidence rule for this file** (AGENTS.md Rule 0). Every factual claim carries a label:
> **VERIFIED** = read or run in the session that wrote this file (2026-09-29), against wgpu fork
> `19ffb4e` (branch `bce/sound-branch-refinement`) unless another revision is named;
> **RECORDED** = in a project artifact, path cited, not re-checked;
> **REPORTED** = claimed somewhere, never independently confirmed; **UNKNOWN** = nobody knows.
> Line numbers refer to the files as they are at `19ffb4e`. They drift upstream; re-grep before quoting.

⛔ **DO NOT SKIP.** Every later section is "how the compiler keeps *this* promise more cheaply". Without this section:
- **0.3 (bounds checks)** makes no sense. A bounds check is the *mechanism* that keeps the promise defined here. Without the promise, a check is just a wasted instruction.
- **0.6 (the analysis)** and **0.8 (soundness bugs)** make no sense. "Unsound" means "broke this promise". The two bugs in 0.8 are serious only because a web page is untrusted.
- **0.10–0.11 (benchmarking)** make no sense. The v3 benchmark measured the wrong binaries because nobody had traced the path in the Layer F trace below (WGSL → wgpu-core → wgpu-hal → driver). That path *re-compiles* SPIR-V. You have to know it exists to see the bug.
- Every number in this project is a cost *of keeping this promise*: +15.79% for `scan_shared` on NVIDIA, +7.30% for the leftover check in `reduce_tree` on Intel (RECORDED, `reports/Benchmark v4 results.md`). Without the promise those numbers are just numbers.

---

## Prerequisites

- You know roughly what a **GPU** is: a chip that runs the same small program on thousands of data items at once.
- You know an **array index** `a[i]` reads element number `i`.
- You know Rust syntax well enough to read a `match`.

Nothing else is needed. No graphics, no compilers, no GPU programming.

---

## THE WHY — Before Anything Else

### WHY 1 (Existence): what would be broken or impossible without this?

A web page is **untrusted code**. You open a link; its author is a stranger; their JavaScript runs on your machine. Browsers have spent 25 years building walls around that code: it cannot read your files, cannot read other tabs' memory, cannot touch the operating system.

WebGPU lets that same stranger ship a **shader**, a program that runs *on your GPU*. A GPU has no browser sandbox around it. A shader instruction that computes an address and loads from it goes straight to the memory controller. If the stranger can write `a[i]` with `i = 10_000_000`, and the hardware does exactly what it is told, then they read whatever lives 40 MB past the end of their buffer. That could be another tab's framebuffer, a password field rendered as pixels, or the compositor's memory.

So WebGPU is possible only if its implementation **guarantees that no shader can read or write outside the memory it was given**. Remove that guarantee and WebGPU cannot ship in a browser. That is the whole reason bounds checks exist, and so the whole reason this project exists.

### WHY 2 (Mechanical): what physical reality forces this?

A GPU load instruction takes an **address** (a number) and returns the bytes there. The hardware has no notion of "this array ends at element 255". An array is a programmer's idea; the machine only sees *base address + index × element size*. Nothing in the silicon stops `index = 1000` on a 256-element array. (Some GPUs *can* check buffer bounds in hardware; that is the "robustness" feature in Layer G. It is optional, differs per vendor, and does not cover every kind of memory.)

So the promise has to be **manufactured**. Something must sit between the stranger's `a[i]` and the hardware load and make an out-of-range `i` harmless. There are exactly two places it can be manufactured:
1. **In the driver/hardware**, by asking the GPU to check buffer accesses ("robust buffer access").
2. **In the compiled shader**, by the compiler inserting an instruction before the load that forces `i` into range (a **bounds check**).

The WebGPU spec names exactly these two options (§2.1.4, quoted below).

### WHY 3 (Design decision): what alternatives were considered, and why these?

| Alternative | Why it was rejected |
|---|---|
| **Trust the programmer** (CUDA, native Vulkan) | Fine for code you wrote yourself. Impossible for code from a stranger. Native APIs make an out-of-bounds access *undefined behaviour*: "Without robustness, out-of-bounds access results in undefined behavior, which may include program termination" (Vulkan Guide, `chapters/robustness.adoc`, retrieved via context7 on 2026-09-29, main branch, revision not pinned; VERIFIED as retrieved text). Undefined behaviour cannot be allowed on the web. |
| **Trap on every out-of-bounds access** (like Rust's panic) | GPUs have no cheap, portable way to abort one of 10,000 threads and report it. WGSL *allows* a trap but does not require it (§6.5.7 below). |
| **Only hardware robustness** | Not every device has it. It covers buffers but not arrays in workgroup or private memory. Its strength differs (`robustBufferAccess` vs `robustBufferAccess2`, Layer G). |
| **Only compiler-inserted checks** | Always available, but they cost instructions in hot loops. Measured: up to +15.79% (Section 0.3). |
| **What WebGPU chose: either, per access** | The spec defines *what may happen* on an out-of-bounds access and lets the implementation pick the mechanism. wgpu picks per backend and per address space (Layer F). |

The key design idea: WGSL does **not** say "an out-of-bounds access returns 0". It lists a *menu* of acceptable outcomes. That menu is what lets a compiler use a cheap clamp instead of an expensive branch, and what lets it *delete* a check it can prove is never needed. **Elision is legal only because the check is a mechanism, not the semantics.** If an access is provably in bounds, the check never changes the result, so removing it changes nothing observable.

### WHY 4 (Historical): why are we living with this design?

- **WebGL (2011)** exposed OpenGL ES to the web. It had no compute shaders and no general storage buffers, so bounds safety was a smaller problem, handled mostly in the browser and driver.
- **Vulkan (2016), Metal (2014), D3D12 (2015)** gave native programs low-level GPU access with *no* safety promise; out-of-bounds is undefined.
- **WebGPU** is the W3C's answer: a modern API on top of those three, with a safety promise strong enough for the web. The archived specs in this repo are Candidate Recommendation Drafts: WGSL dated 25 August 2026, WebGPU dated 20 August 2026 (VERIFIED, title blocks of `references/raw/specs/WGSL.html` and `WebGPU.html`).
- Two production implementations exist. **Firefox** uses **wgpu** with its shader compiler **Naga**: "It serves as the core of the WebGPU integration in Firefox, Servo, and Deno" (VERIFIED, `wgpu/README.md:10`). **Chrome** uses **Dawn** with the compiler **Tint** (RECORDED, `IICT_CONTEXT/04-EVIDENCE-LEDGER.md:69`: "Tint IntegerRangeAnalysis (Dawn, ©2024; Chrome 141, 2025-09-24)"). This project works on the Naga side.

---

## What Is This? (analogy first)

Think of a **library that lends out rooms by the hour**.

- Every visitor (a web page) gets a key to *their* rooms (their buffers).
- The building has no internal walls, just painted lines on the floor (GPU memory is flat addresses).
- The visitor hands the doorman (the GPU) instructions: "walk 37 steps east from the start of my room, pick up the box". Nothing stops "walk 3,700 steps", which lands in someone else's room.
- The library's promise: **no visitor can ever pick up a box outside their own painted lines.**
- Two ways to keep it. Hire **floor guards** who stop anyone crossing a line (hardware robustness; not every building has them, and they only patrol some floors). Or **rewrite every instruction** before handing it to the doorman: "walk min(37, last-step-in-your-room) steps" (a compiler-inserted bounds check).
- The rewriting clerk is **Naga**. **wgpu** is the front office. It accepts visitors' instructions in one language and hands them to four different buildings (Vulkan, Metal, D3D12, GL), each with its own doorman, each speaking a different language.
- **This project** is a smarter clerk. When the clerk can *prove* from the instructions that "37 steps" always stays inside the lines (say, the visitor wrote "if steps < 37 …"), the `min(…)` rewrite is dropped. The promise is still kept, with one fewer instruction.

---

## How It Actually Works — the 8-Layer Trace

### Running example

The tree reduction used across this epoch. It is the benchmark kernel `reduce_tree`, copied verbatim from the recorded artifact `bounds-check-testing-framework/runs-v4/20260929T125306Z-reduce_tree/artifacts/kernel.wgsl` (VERIFIED):

```wgsl
@group(0) @binding(0) var<storage, read> input: array<u32>;         // runtime-sized buffer
@group(0) @binding(1) var<storage, read_write> output: array<u32>;

var<workgroup> s: array<u32, 256>;                                  // fixed-size shared array
@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32, @builtin(workgroup_id) wid: vec3<u32>) {
    s[lid] = input[wid.x * 256u + lid];
    workgroupBarrier();
    for (var stride = 128u; stride > 0u; stride = stride >> 1u) {
        if (lid < stride) {
            s[lid] = s[lid] + s[lid + stride];                      // ← THE access: s[lid + stride]
        }
        workgroupBarrier();
    }
    if (lid == 0u) {
        output[wid.x] = s[0];
    }
}
```

**The question the whole stack answers:** a stranger wrote `s[lid + stride]`. How does the system guarantee it never touches memory outside `s`? And can it prove the guarantee holds *without* paying for a runtime check?

This section covers layers **A, F and G**, the *contract* and the *plumbing*. Layers B–E (inside Naga) are Sections 0.2 and 0.3.

---

### Layer A — WGSL source: what the stranger wrote, and what the spec says it means

The WGSL spec defines exactly what `s[lid + stride]` means when `lid + stride ≥ 256` (VERIFIED, text extracted from `references/raw/specs/WGSL.html`, §6.5.7 "Out-of-Bounds Access", CRD 25 August 2026):

> "An operation that accesses an invalid memory reference or an invalid swizzle view is an out-of-bounds access. An out-of-bounds access is a program defect, because if it were performed as written, it would typically: read or write memory locations outside of a variable, or interpret the contents of those locations as the wrong store type, or cause an unintended data race. For this reason, an implementation will not perform the access as written. Executing an out-of-bounds access generates a dynamic error."

The allowed outcomes, same section:

> **Trap**: "The shader invocation immediately terminates, and shader stage outputs are set to zero values."
>
> **Invalid Load**: "Loads from an invalid reference may return one of: when the originating variable is a uniform buffer or a storage buffer, the value from any memory location(s) of the WebGPU GPUBuffer bound to the originating variable; when the originating variable is not a uniform buffer or storage buffer, a value from any memory location(s) in the originating variable; the zero value for store type of the reference; if the loaded value is a vector, the value (0, 0, 0, x) …"
>
> **Invalid Store**: "Stores to an invalid reference may do one of: when the originating variable is a storage buffer, store the value to any memory location(s) of the WebGPU GPUBuffer bound to the originating variable; when the originating variable is not a storage buffer, store the value to any memory locations(s) in the originating variable; not be executed."

And the sentence that matters for 0.8 (soundness):

> "An out-of-bounds access invalidates the assumptions of uniformity analysis. For example, if an invocation terminates early due to an out-of-bounds access, then it can no longer particpate in collective operations. In particular, a call to workgroupBarrier may hang the shader …" (typo "particpate" is in the spec)

**Read those outcomes as a menu for the implementer:**
- "a value from any memory location(s) in the originating variable" means **clamping `i` to the last element is legal**. That is Naga's `Restrict` policy (0.3).
- "the zero value" plus "not be executed" means **return 0 / skip the write is legal**. That is Naga's `ReadZeroSkipWrite` policy (0.3).
- Nothing on the menu allows touching memory *outside the variable* (or, for buffers, outside the bound GPUBuffer). **That is the promise.**

**Where the promise stops.** For a *storage buffer* the permitted memory is "the WebGPU GPUBuffer bound to the originating variable", so a wrong-but-in-buffer answer is legal. Out-of-bounds access is a *program defect*. WebGPU guarantees containment, not correctness.

### Layer A′ — the WebGPU API spec: the two mechanisms

The WebGPU spec (not WGSL) says *how* an implementation may contain accesses (VERIFIED, `references/raw/specs/WebGPU.html`, §2.1.4 "Out-of-bounds access in shaders", CRD 20 August 2026):

> "In order to prevent the shaders from accessing GPU memory an application doesn't own, the WebGPU implementation may enable a special mode (called "robust buffer access") in the driver that guarantees that the access is limited to buffer bounds. Alternatively, an implementation may transform the shader code by inserting manual bounds checks. When this path is taken, the out-of-bound checks only apply to array indexing. They aren't needed for plain field access of shader structures due to the minBindingSize validation on the host side."

Three facts from this paragraph run through the whole project:
1. **Two mechanisms**: driver robustness *or* compiler checks. wgpu mixes them per backend (Layer F).
2. **Checks apply only to array indexing.** A struct field `p.velocity` needs no check; the host already validated that the buffer is big enough (`minBindingSize`). This is why Naga's `bounds_check_iter` skips struct members (Section 0.3, `naga/src/proc/index.rs:392-399`).
3. **The phrase "manual bounds checks"** is what Naga's `BoundsCheckPolicy` implements, and what this project elides.

---

### Layer F — wgpu: one API, four native backends

This is the "backend-agnostic" part. wgpu is three crates stacked on top of each other. The crate docs describe them directly (VERIFIED, `wgpu-hal/src/lib.rs:1-25`):

> "A cross-platform unsafe graphics abstraction. This crate defines a set of traits abstracting over modern graphics APIs, with implementations ("backends") for Vulkan, Metal, Direct3D, and GL. … Our traits' contracts are *unsafe*: implementations perform minimal validation, if any … If you need safety, the [`wgpu-core`] crate provides a safe API for driving `wgpu-hal`, implementing all necessary validation … the [`wgpu`] crate provides more idiomatic Rust bindings for `wgpu-core`."

```
┌──────────────────────────────────────────────────────────────────────────┐
│  wgpu            idiomatic Rust API  (Firefox uses wgpu-core via FFI)    │
│                  wgpu/src/api/device.rs, wgpu/src/backend/wgpu_core.rs   │
├──────────────────────────────────────────────────────────────────────────┤
│  wgpu-core       SAFE layer: validation, resource tracking.              │
│                  Runs Naga's front end + validator (+ elision analysis). │
│                  wgpu-core/src/device/resource.rs                        │
├──────────────────────────────────────────────────────────────────────────┤
│  wgpu-hal        UNSAFE, thin, per-backend. Chooses Naga back end +      │
│                  BoundsCheckPolicies, emits SPIR-V / MSL / HLSL / GLSL.  │
│   vulkan/        metal/          dx12/          gles/                    │
├──────────────────────────────────────────────────────────────────────────┤
│  driver          compiles SPIR-V / MSL / HLSL / GLSL → GPU machine code  │
│                  (+ optional robust buffer access)                       │
└──────────────────────────────────────────────────────────────────────────┘
```

"Backend-agnostic" means one WGSL shader and one API call work unchanged on all four. The *cost* of that is that the safety mechanism must be chosen **per backend**, because each backend offers different hardware help. Now the actual code, top to bottom.

#### F.1 — `wgpu`: the default is "fully checked"

`wgpu/src/api/device.rs:139-144` (VERIFIED):

```rust
pub fn create_shader_module(&self, desc: ShaderModuleDescriptor<'_>) -> ShaderModule {
    let module = self
        .inner
        .create_shader_module(desc, wgt::ShaderRuntimeChecks::checked());   // ← safe default
    ShaderModule { inner: module }
}
```

`ShaderRuntimeChecks::checked()` turns every runtime check on (VERIFIED, `wgpu-types/src/shader.rs:64-66` calls `Self::all(true)`; `:88-97` sets `bounds_checks`, `force_loop_bounding`, … all to the same value). The field's own documentation states the promise in wgpu's words (VERIFIED, `wgpu-types/src/shader.rs:7-24`):

> "Enforce bounds checks in shaders, even if the underlying driver doesn't support doing so natively. When this is `true`, `wgpu` promises that shaders can only read or write the accessible region of a bindgroup's buffer bindings. If the underlying graphics platform cannot implement these bounds checks itself, `wgpu` will inject bounds checks before presenting the shader to the platform. When this is `false`, `wgpu` only enforces such bounds checks if the underlying platform provides a way to do so itself."

There are two escape hatches. Both are `unsafe`, and the benchmark harness uses the second one:
- `create_shader_module_trusted(desc, runtime_checks)`, `wgpu/src/api/device.rs:181-188` (VERIFIED): the caller picks the checks.
- `create_shader_module_passthrough(desc)`, `wgpu/src/api/device.rs:197-203` (VERIFIED): "Creates a shader module which will bypass wgpu's shader tooling and validation and be used directly by the backend … No attempt is made to ensure that data is valid."

#### F.2 — `wgpu` → `wgpu-core`: even SPIR-V input is re-parsed

`wgpu/src/backend/wgpu_core.rs:1046-1056` (VERIFIED):

```rust
let source = match desc.source {
    #[cfg(feature = "spirv")]
    ShaderSource::SpirV(ref spv) => {
        // Parse the given shader code and store its representation.
        let options = naga::front::spv::Options {
            adjust_coordinate_space: false, // we require NDC_Y_UP feature
            strict_capabilities: true,
            block_ctx_dump_prefix: None,
        };
        wgc::pipeline::ShaderModuleSource::SpirV(Borrowed(spv), options)
    }
```

**This is the line that invalidated the v3 benchmark** (Section 0.10). Hand wgpu a SPIR-V binary through the normal API, and wgpu does *not* give it to the driver. It parses the binary back into Naga IR, validates it, and emits *new* SPIR-V with its own policies. The binary you saved and hashed is not the one that runs. The v4 harness therefore uses `create_shader_module_passthrough` (VERIFIED, `bounds-check-testing-framework/paired-vulkan-v4/src/main.rs:190`).

#### F.3 — `wgpu-core`: parse, then validate (and, in this fork, elide)

`wgpu-core/src/device/resource.rs:2362` `create_shader_module` (VERIFIED) does the following in order:
1. **Parse** with the matching Naga front end: WGSL (`:2371-2386`), SPIR-V (`:2388-2398`), GLSL (`:2400-2411`), or a ready-made Naga module (`:2413`).
2. **Check bind-group indices** against device limits (`:2416-2427`).
3. **Validate** (`:2444-2455`):
   ```rust
   let info = create_validator(
       self.features,
       self.downlevel.flags,
       naga::valid::ValidationFlags::all(),
   )
   .validate(&module)
   ```
   `create_validator` is `naga::valid::Validator::new(flags, caps)` (VERIFIED, `wgpu-naga-bridge/src/lib.rs:190-197`). In this fork, `Validator::new` sets `bounds_check_elision: true` (VERIFIED, `naga/src/valid/mod.rs:623`). At the end of each function's validation it runs `elide_bounds_checks(fun, module, &mut info)` (VERIFIED, `naga/src/valid/function.rs:1942-1943`). **So in this fork, every shader any wgpu user creates is analysed. There is no opt-in.** Upstream wgpu has neither the flag nor the pass (VERIFIED: the fork's diff against merge-base `3fb225a` adds `naga/src/proc/bounds_check_elision.rs`, +1979 lines, and 10 lines to `valid/mod.rs`).
4. **Hand the module plus its analysis to wgpu-hal** as `hal::ShaderInput::Naga(hal::NagaShader { module, info, debug_source })` (`:2459-2463`), with `runtime_checks` passed through unchanged (`:2464-2467`).

**Architectural consequence:** the analysis result (`FunctionInfo.safe_accesses`) travels *inside* `info` from wgpu-core to wgpu-hal. The proof is computed once, backend-independently, and every back end consumes it. That is the design that makes the elision "backend-agnostic".

#### F.4 — `wgpu-hal`: each backend picks its own policy

Here the backends stop being identical. Each one tells Naga's back end which `BoundsCheckPolicy` to use for ordinary arrays (`index`) and for storage/uniform buffers (`buffer`).

**Vulkan**, `wgpu-hal/src/vulkan/adapter.rs:2814-2828` (VERIFIED):

```rust
bounds_check_policies: naga::proc::BoundsCheckPolicies {
    index: naga::proc::BoundsCheckPolicy::Restrict,          // workgroup/private/function arrays: always clamp
    buffer: if self.private_caps.robust_buffer_access2 {
        naga::proc::BoundsCheckPolicy::Unchecked             // driver guarantees containment
    } else {
        naga::proc::BoundsCheckPolicy::Restrict              // no strong driver help: clamp
    },
    image_load: if self.private_caps.robust_image_access { … Unchecked } else { … Restrict },
    // TODO: support bounds checks on binding arrays
    binding_array: naga::proc::BoundsCheckPolicy::Unchecked,
},
…
force_loop_bounding: true,                                   // :2836
```

The robustness features are requested at device creation (VERIFIED, `adapter.rs:299` `.robust_buffer_access(private_caps.robust_buffer_access)`, `:404` `.robust_buffer_access2(private_caps.robust_buffer_access2)`). The shader is emitted in `wgpu-hal/src/vulkan/device.rs:1873-1898` (VERIFIED). If the user turned bounds checks off, every policy becomes `Unchecked` (`:1884-1891`). Then:

```rust
let spv = naga::back::spv::write_vec(
    &naga_shader.module,
    &naga_shader.info,          // ← carries safe_accesses from F.3
    &naga_options,
    None,
)
```

(Modules with overrides, or devices needing the `SEPARATE_ENTRY_POINTS` workaround, are compiled later, per pipeline, in `compile_stage`, `device.rs:699-782`. Same options, same `info`.)

**Metal**, `wgpu-hal/src/metal/device.rs:169-208` (VERIFIED): one policy for everything:

```rust
let bounds_check_policy = if stage.module.runtime_checks.bounds_checks {
    naga::proc::BoundsCheckPolicy::Restrict
} else {
    naga::proc::BoundsCheckPolicy::Unchecked
};
… index: bounds_check_policy, buffer: bounds_check_policy, image_load: bounds_check_policy, …
```

**D3D12** uses the HLSL back end's boolean `restrict_indexing: true` (VERIFIED, `wgpu-hal/src/dx12/device.rs:1489`), overridden per stage from `runtime_checks.bounds_checks` (`:293`, `:310`).

**GLES**, `wgpu-hal/src/gles/device.rs:333-348` (VERIFIED): "Other bounds check are either provided by glsl or not implemented yet."

```rust
index: BoundsCheckPolicy::Unchecked,
buffer: BoundsCheckPolicy::Unchecked,
image_load: image_check,          // ReadZeroSkipWrite on desktop GL ≥ 4.3, else Unchecked
```

**The production policy table** (VERIFIED from the four files above, wgpu fork `19ffb4e`; upstream `688e476` has the same policy code):

| Backend | arrays (`index`) | buffers (`buffer`) | where elision can save work |
|---|---|---|---|
| Vulkan | Restrict (`UMin` clamp) | Unchecked if `robustBufferAccess2`, else Restrict | workgroup/private arrays always; buffers only without RBA2 |
| Metal | Restrict | Restrict | everything |
| D3D12 | `restrict_indexing: true` | same flag | arrays (HLSL back end) |
| GLES | Unchecked | Unchecked | nothing, because nothing is checked |

**Read this table twice.** It decides whether the project's work matters on a given machine:
- Vulkan with RBA2 checks *no buffer accesses*. Elision can only help workgroup and private arrays. That is why every v4 benchmark kernel puts its hot accesses in `var<workgroup>` or private arrays (Section 0.11).
- Metal checks *everything*, so elision has the most to remove there. It is unmeasured in this project (UNKNOWN; no Apple hardware, `reports/Benchmark v4 results.md` "Honest limits").
- GLES relies on the platform, so elision has nothing to remove.

**A doc/code gap found while writing this section (Rule 0.7).** Naga's own documentation says (VERIFIED, `naga/src/proc/index.rs:28-29`, identical in `wgpu-upstream` at `688e476`):

> "When used by the `wgpu` crate for native development, `wgpu` selects `ReadZeroSkipWrite` as its default."

The code does not do this. No wgpu-hal backend selects `ReadZeroSkipWrite` for `index` or `buffer`. The only use is GLES `image_load` (VERIFIED: `grep -rn ReadZeroSkipWrite wgpu-upstream/wgpu-hal/src` → one hit, `gles/device.rs:337`). **Code wins over docs (AGENTS.md Rule 5).** This matters historically: the original paper's benchmark measured `ReadZeroSkipWrite` (RECORDED, `reports/Benchmark v4 results.md` "What changed vs. v3": "Measured ReadZeroSkipWrite, not what wgpu uses"). That is the policy the upstream doc comment names, and it is not what production uses.

---

### Layer G — the driver: robustness, and what "Unchecked" relies on

When wgpu-hal picks `buffer: Unchecked` on Vulkan, it is betting on the driver. The Vulkan spec's definitions (VERIFIED as retrieved text; context7 `/khronosgroup/vulkan-docs`, `chapters/shaders.adoc`, main branch, retrieved 2026-09-29; revision not pinned):

> **Robust Buffer Access**: "When active, buffer accesses are checked against the range defined for the descriptor or vertex input. This ensures that out-of-bounds reads return predictable values—such as zeros or valid memory contents—rather than causing undefined behavior …"
>
> **Robust Buffer Access 2**: "Out-of-bounds writes do not modify memory, while out-of-bounds reads return zero values … accesses wider than 32-bits may be treated as multiple independent 32-bit checks."

And from the `VK_KHR_device_address_commands` proposal (same retrieval): "For `robustBufferAccess`, OOB accesses can still return values from memory beyond the specified range as long as they are bound to the underlying buffer. For `robustBufferAccess2`, only values in the range or zeroes can be returned."

That is why wgpu only drops its own buffer checks when **RBA2** is present. Plain RBA is looser: it may read past the *binding range* while staying inside the *buffer*. WebGPU forbids that, because a binding can be a sub-range of a larger buffer.

**What the driver does not cover.** Robustness is about *buffer descriptors*. A `var<workgroup>` array such as `s` lives in on-chip shared memory, with no descriptor. The driver will not bounds-check `s[lid + stride]`. That is why `index` stays `Restrict` on Vulkan even with RBA2, and why **the running example's check cannot be delegated to hardware. Only the compiler can remove it, by proving it unnecessary.**

**This machine's drivers** (VERIFIED, `runs-v4/20260929T125306Z-reduce_tree/meta.json`): "NVIDIA GeForce RTX 4060 Laptop GPU", driver_info "580.178.04", backend "Vulkan". The Intel iGPU runs Mesa 26.2.2 (RECORDED, `reports/Benchmark v4 results.md`). Whether each reports `robustBufferAccess2` was not logged by the harness (UNKNOWN). The harness hard-codes the RBA2 branch (`buffer: Unchecked`, VERIFIED, `paired-vulkan-v4/src/main.rs:369`). It does not read the adapter's capability. That is a documented modelling choice, and it has a limit: it matches production only on devices that expose RBA2.

---

### Layer H — the hardware, briefly

After the driver compiles the SPIR-V to machine code, `UMin(i, 255)` is one integer instruction per access. Section 0.3 explains why one instruction can cost 15%: it sits on the address's dependency chain, and it can block the driver from folding the index into the load's addressing mode. Here we only note the result: in the v4 campaign on `83a90e0`, removing 2 clamps from `scan_shared` made it **1.1579× faster on NVIDIA and 1.1162× on Intel** (RECORDED, `runs-v4/summary-nvidia-83a90e0.json` and `summary-intel-83a90e0.json`, field `effect_geomean`; values copied by script this session).

---

## The Causation Chain (with the legality argument)

```
stranger writes s[lid + stride]
  → WGSL §6.5.7: if out of range, the implementation MUST NOT touch memory outside `s`
      → mechanism choice (WebGPU §2.1.4): driver robustness OR compiler check
          → `s` is workgroup memory: no descriptor ⇒ driver robustness does not apply
              → wgpu-hal Vulkan sets index: Restrict (adapter.rs:2815)
                  → Naga emits UMin(lid + stride, 255) before the OpAccessChain
                      → every execution pays one clamp inside the hot loop
                          → IF the compiler can PROVE lid + stride ≤ 255 on every path reaching the access,
                            the clamp's output always equals its input
                              → removing it changes no observable behaviour  ⇒ elision is legal
```

**Legality, stated precisely.** Let `c(i) = min(i, 255)`. If for every execution reaching the access `i ∈ [0, 255]`, then `c(i) = i`. The program with `c` and the program without `c` then compute identical addresses on every execution, so they are observationally equivalent. The proof obligation is **"on every execution"**. An analysis that says "in bounds" when some execution is out of bounds has turned a contained access into an uncontained one. That breaks the promise of WHY 1. Section 0.8 shows two times this happened in this project and how they were caught.

---

## ASCII diagram — where the proof is made and where it is consumed

```
 WGSL / GLSL / SPIR-V text
        │
        ▼  wgpu-core/src/device/resource.rs:2369-2414
 ┌──────────────┐   Naga front end → naga::Module (arenas of Expression/Statement)
 │   PARSE      │
 └──────┬───────┘
        ▼  resource.rs:2444  create_validator(...).validate(&module)
 ┌──────────────┐   valid/function.rs:1942  elide_bounds_checks(fun, module, &mut info)
 │  VALIDATE +  │──► FunctionInfo.safe_accesses : set of Access handles PROVEN in bounds
 │  ANALYSE     │    (valid/analyzer.rs:254)
 └──────┬───────┘
        ▼  resource.rs:2459  hal::ShaderInput::Naga { module, info }
 ┌──────────────┬──────────────┬──────────────┬──────────────┐
 │ vulkan/      │ metal/       │ dx12/        │ gles/        │  each picks BoundsCheckPolicies
 │ Restrict idx │ Restrict all │ restrict_idx │ Unchecked    │
 └──────┬───────┴──────┬───────┴──────┬───────┴──────┬───────┘
        ▼              ▼              ▼              ▼
   back/spv        back/msl       back/hlsl      back/glsl       spv/msl/hlsl ask "is this access in
   index.rs:522    writer.rs:832  writer.rs:3961  (no lookup:     safe_accesses?" → skip the check.
                                                  GLES emits      glsl never looks: nothing to skip.
                                                  no checks)
        │              │              │              │
        ▼              ▼              ▼              ▼
    SPIR-V           MSL            HLSL           GLSL  ──►  driver ──► GPU ISA
```

---

## Code / IR example — the real path, with real output

The mini-project below reproduces this. Here is the *recorded* result for the running example, on fork `19ffb4e`, from the v4 campaign (VERIFIED this session, `sha256sum` over `runs-v4/20260929T125306Z-reduce_tree/artifacts/*.spv`):

```
f97b7fe657bbd86c…ebe09  bce_dup.spv     ← elision ON  (second pipeline, same words = control)
f97b7fe657bbd86c…ebe09  bce.spv         ← elision ON
3f1d6c2c6fbe4b10…5181e  no_bce.spv      ← elision OFF, production Vulkan policies
f97b7fe657bbd86c…ebe09  unchecked.spv   ← ALL checks off (unsafe upper bound)
```

Counting clamps with `spirv-dis` (VERIFIED this session):

```
== no_bce:    4 UMin, 1 OpULessThan, 151 lines
== bce:       0 UMin, 1 OpULessThan, 146 lines
== unchecked: 0 UMin, 1 OpULessThan, 146 lines
```

- **`bce.spv` is byte-identical to `unchecked.spv`.** On `19ffb4e`, elision removed *every* check in this kernel. The safe binary is exactly the unsafe-but-fastest one, and the analysis carries the proof that the unsafe binary is in fact safe.
- The one `OpULessThan` in all three is the program's own `if (lid < stride)`, not a bounds check. Section 0.3 shows why the opcode alone cannot tell you which is which. That confusion caused a real checker bug (the "opcode-166" incident, Section 0.9).
- `input[wid.x * 256u + lid]` has **no** clamp even in `no_bce`, because `buffer: Unchecked` (RBA2 assumed). Only the 4 accesses to the workgroup array `s` are clamped.

---

## Surface Detection Self-Audit (FM1–FM6)

| # | Failure mode | Test | Result |
|---|---|---|---|
| FM1 | Vocabulary without mechanism | Can you say *which line of which file* makes `s[...]` clamped on Vulkan? (`adapter.rs:2815`, `index: Restrict`) | PASS if you can |
| FM2 | Layer skipping | Can you name all four crates/stages between `create_shader_module` and the driver, and which one runs the elision analysis? (wgpu → wgpu-core [parse, validate, **analyse**] → wgpu-hal [policy, emit] → driver) | |
| FM3 | Missing "why" | Why does WGSL list a *menu* of outcomes instead of one? (so implementations can pick a cheap mechanism, and so a provably-unneeded check can be deleted) | |
| FM4 | No causation | Why can't driver robustness remove the check on `s[lid+stride]`? (workgroup memory has no descriptor) | |
| FM5 | No counterfactual | What changes if you run on GLES? (nothing is checked, so elision saves nothing) On Metal? (buffers are checked too, so elision can save more) | |
| FM6 | No correctness link | What exactly goes wrong if an analysis wrongly marks an access safe? (the clamp is dropped; an OOB index reaches memory outside the variable; the WebGPU promise is broken) | |

---

## Correctness and Testing Dimension

- **What must never happen:** an access in `safe_accesses` that some execution drives out of range. That is the only way this project can make wgpu *less safe than upstream*.
- **What the project tests for it** (details in 0.9):
  - stress tests that assert on the proof set (`naga/tests/naga/bounds_check_stress.rs`, 89 `#[test]` functions, VERIFIED by `grep -c`);
  - exhaustive interval property sweeps;
  - on-GPU differential execution of checked vs elided binaries (`diff-exec/results/diff-exec-20260929T020659Z.json`: `class_a_identical: 8`, `class_a_total: 8`, VERIFIED). **Caveat:** the JSON does not record the wgpu revision. It was produced 76 s after commit `19ffb4e` (02:05:43Z), so "on `19ffb4e`" is REPORTED, not VERIFIED;
  - must-fail control fixtures for the expectation checker.
- **What a test here cannot show:** containment on a device with *no* checks at all. Testing an elided access with an out-of-range index is meaningless, because a correct analysis never elides one. Negative tests therefore assert that the *proof is absent* (the check stays), not that the GPU behaves.
- **The one-line invariant to remember:** *every elision is a theorem about every execution, and wgpu turns the analysis on by default (in this fork), so every theorem ships to every user.*

---

## Common Bugs and Misconceptions

1. **"WebGPU returns 0 for out-of-bounds reads."** No. Zero is one allowed outcome among several; clamping to an in-bounds element is equally legal (§6.5.7). Vulkan wgpu uses clamping.
2. **"wgpu uses ReadZeroSkipWrite natively."** Naga's doc comment says so (`proc/index.rs:28-29`); the wgpu-hal code does not (Layer F.4). The v3 benchmark measured the doc's policy, not the code's.
3. **"Passing SPIR-V to wgpu runs that SPIR-V."** Only through `create_shader_module_passthrough`. `ShaderSource::SpirV` is parsed and re-emitted (`wgpu_core.rs:1047-1056`).
4. **"Robust buffer access makes all checks unnecessary."** It covers buffers bound through descriptors. Workgroup and private arrays are never covered, and plain RBA (not RBA2) is too loose for WebGPU.
5. **"Bounds checks protect correctness."** They protect *containment*. An out-of-bounds program is still wrong, just harmlessly wrong.
6. **"Elision is an optimisation, so a bug in it is a performance bug."** A false proof is a *security* bug: it removes the containment the browser relies on.
7. **"All backends benefit equally."** GLES checks nothing (no gain). Vulkan+RBA2 checks only non-buffer arrays. Metal checks everything (largest potential gain, unmeasured).
8. **"Struct field accesses need checks too."** No: WebGPU §2.1.4, "They aren't needed for plain field access of shader structures due to the minBindingSize validation on the host side."

---

## Integrated Research

Only sources present in this repo's `references/` or recorded in the ledger:
- **WGSL spec**, W3C CRD 25 Aug 2026, §6.5.7 (archived: `references/raw/specs/WGSL.html`).
- **WebGPU spec**, W3C CRD 20 Aug 2026, §2.1.4 (archived: `references/raw/specs/WebGPU.html`).
- **Vulkan robustness**: not archived. `references/raw/specs/Vulkan.html` is a 3,803-byte *directory index*, not the spec (VERIFIED this session). Quotes above come from context7 retrieval and are labelled as such. **DEVIATION:** expected an archived Vulkan spec, observed a directory listing; evidence: `head -c 800 references/raw/specs/Vulkan.html`; consequence: the Vulkan quotes are not pinned to a revision; next: archive `chapters/shaders.adoc` at a fixed commit if the poster cites Vulkan robustness.
- **Tint `IntegerRangeAnalysis`** (Dawn/Chrome) and **wgpu#4360** (2021): RECORDED, ledger line 69, audit report §4. Treated in 0.4 and 0.13.

---

## Connection to the Real Project

| What | Where |
|---|---|
| wgpu fork under study | `wgpu/`, branch `bce/sound-branch-refinement`, HEAD `19ffb4e` |
| Upstream reference (read-only) | `wgpu-upstream/` @ `688e476` |
| Default-checked API | `wgpu/src/api/device.rs:139-144`; `wgpu-types/src/shader.rs:7-31,64-97` |
| SPIR-V re-parse (the v3 trap) | `wgpu/src/backend/wgpu_core.rs:1046-1056`; `wgpu-core/src/device/resource.rs:2388-2398` |
| Where the analysis is invoked | `naga/src/valid/function.rs:1942-1943`, flag default `naga/src/valid/mod.rs:623` |
| Per-backend policy | `wgpu-hal/src/vulkan/adapter.rs:2814-2836`; `metal/device.rs:169-208`; `dx12/device.rs:1489`; `gles/device.rs:333-348` |
| Harness that mirrors the Vulkan policy | `bounds-check-testing-framework/paired-vulkan-v4/src/main.rs:68-80,369` |
| Running-example binaries | `bounds-check-testing-framework/runs-v4/20260929T125306Z-reduce_tree/artifacts/` |

---

## Resources

- WGSL spec §6.5.7 and WebGPU spec §2.1.4, archived copies above. Read the two passages in full; they are short.
- `wgpu-hal/src/lib.rs:1-60`, the best short description of the crate layering, written by its authors.
- Vulkan Guide, "Robustness" chapter (`KhronosGroup/Vulkan-Guide`, `chapters/robustness.adoc`).

---

## Note-Taking Instructions

On one page, draw the four-layer stack (wgpu / wgpu-core / wgpu-hal / driver). Next to each layer write:
1. the one function you would set a breakpoint in (from the table above);
2. what it decides about bounds checks;
3. whether it is safe or `unsafe` Rust, and why that matters.

Under the stack, copy the production policy table *from memory*, then check it against Layer F.4. Mark every cell you got wrong. Those cells are what you do not yet understand.

---

## Mini-Project — trace one shader from `create_shader_module` to the words the driver sees

**Goal:** see with your own eyes that elision is on by default in the fork, and what it changes.

**Steps** (run from repo root; compiling the tool needs a quiet machine if a benchmark campaign is running):

1. Look at the recorded binaries (no build needed):
   ```bash
   cd bounds-check-testing-framework/runs-v4/20260929T125306Z-reduce_tree/artifacts
   sha256sum *.spv
   for v in no_bce bce unchecked; do
     echo "$v: $(spirv-dis $v.spv | grep -c UMin) UMin"
   done
   ```
   **Expected** (VERIFIED 2026-09-29): `bce.spv`, `bce_dup.spv` and `unchecked.spv` share sha256 `f97b7fe6…ebe09`; `no_bce` has 4 `UMin`, the others 0.

2. Diff them: `diff <(spirv-dis --raw-id no_bce.spv) <(spirv-dis --raw-id bce.spv)`.
   **Expected:** the only semantic change is that the four `OpExtInst %3 %1 UMin %x %42` (where `%42 = OpConstant %3 255`) disappear, and each `OpAccessChain` into `%14` (the workgroup array `s`) indexes the raw value directly. IDs are renumbered; the block structure is otherwise unchanged.

3. Find the policy in code: open `wgpu-hal/src/vulkan/adapter.rs` at line 2814. Predict what `no_bce.spv` would contain on a device *without* RBA2. (Answer: an extra `UMin` against `OpArrayLength − 1` on the `input[...]` access. The buffer policy becomes `Restrict`. Section 0.3 shows that code path, `back/spv/index.rs:341-376`.)

4. **Branches** (AGENTS.md Rule 3):
   - *success*: hashes and counts match the expected values above.
   - *partial*: counts match but hashes differ. You are on a different wgpu revision; read `meta.json` → `sources.wgpu.rev`.
   - *bad*: `bce` still has `UMin`. You are looking at an `83a90e0`-era run (before `2ffe1b7`), where `s[lid + stride]` was unproven. Check `meta.json`.
   - *disaster*: `spirv-dis` missing or files absent. Install `spirv-tools`; the runs directory is committed in `bounds-check-testing-framework`.

---

## Mastery Checklist

**WHY**
- [ ] Why does a browser need a containment promise that native Vulkan does not?
- [ ] Why does WGSL give a menu of OOB outcomes rather than one?

**Trace**
- [ ] Name, in order, the functions a WGSL shader passes through from `Device::create_shader_module` to `vkCreateShaderModule`.
- [ ] In which crate is `safe_accesses` computed, and in which is it consumed?

**Causation**
- [ ] Why is `index` `Restrict` on Vulkan even with RBA2?
- [ ] Why did passing SPIR-V to `create_shader_module` make the v3 benchmark measure the wrong thing?

**Counterfactual**
- [ ] If wgpu-hal Vulkan switched `index` to `ReadZeroSkipWrite`, what would the SPIR-V for `s[lid]` look like? (branch + conditional load + `OpPhi`/select; Section 0.3)
- [ ] If an analysis bug marked `s[lid + stride + 1]` safe, which WGSL outcome in §6.5.7 would the program then violate? (none of them. It would access memory outside `s`, which is outside the menu entirely.)

---

## Teach-Back

Explain in five sentences, to someone who has never written a shader:
1. Why a web page's GPU program needs a containment guarantee.
2. The two ways WebGPU allows it to be kept.
3. What wgpu is, and why it must choose the mechanism per backend.
4. Why the compiler, not the hardware, must handle `s[lid + stride]`.
5. Why deleting a check is legal only with a proof about *every* execution.

---

## Interview Fireproofing

- **Q: "Doesn't Vulkan's robustness make compiler bounds checks obsolete?"**
  A: Only for descriptor-backed buffers, and only RBA2 is strict enough for WebGPU's binding-range semantics. Workgroup and private arrays have no descriptor; wgpu keeps `index: Restrict` on Vulkan for exactly that reason (`adapter.rs:2815`).
- **Q: "Is elision a WGSL-spec change?"**
  A: No. WGSL defines the *allowed outcomes* of an OOB access. An access proven in bounds has no OOB execution, so removing its check changes no outcome. The spec is untouched.
- **Q: "Where in wgpu would a bug in your analysis become a security issue?"**
  A: At `back/spv/index.rs:522` (and the MSL/HLSL equivalents). If the access handle is in `safe_accesses`, the back end emits the raw index, and nothing downstream re-checks workgroup memory.
- **Q: "Does upstream wgpu do this?"**
  A: Not as of `688e476`. Upstream `access_needs_check` only skips checks for compile-time-constant in-bounds indices (VERIFIED, `wgpu-upstream/naga/src/proc/index.rs:322-343`). Chrome's Tint has a range analysis (RECORDED, ledger:69). Section 0.4 compares them.
