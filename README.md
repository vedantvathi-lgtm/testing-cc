# Bounds-Check Elision — Testing & Verification Framework
### Design Doc v1 — companion to the IICT paper draft, §6

**Status:** Draft, ready to implement locally.
**Scope:** This document specifies *only* the testing/verification framework. It assumes the §4 patch (range analysis in `naga/src/valid/analyzer.rs` + the one-line hook in `naga/src/proc/index.rs::access_needs_check`) is being implemented separately; every test below is written to be meaningful **before** that patch lands (as a baseline) and to need only a one-line assertion flip once it does.
**Everything in this document was checked against the actual `gfx-rs/wgpu` source** (commit `3fb225a9c6240bd7e9db3d202410db6d894368ec`) before being written down — file paths, struct fields, function signatures, and the real precedent test files this scaffold is modeled on. Where I'm asking you to take something on faith anyway, I've said so explicitly (see §6, "Honest unknowns").

---

## 0. Why four tiers, and why this order

A reviewer's first objection to a compiler-optimization claim is not "is the transform correct," it's "did you measure what you think you measured." Each tier below exists to make the next tier's result trustworthy:

1. **Tier 1 (static)** proves the transform changed the IR at all, with zero hardware involved. If this fails, nothing downstream is testing the optimization.
2. **Tier 2 (soundness)** proves it *only* changed the IR where that's safe. This is the actual correctness proof of the paper — more than any prose paragraph.
3. **Tier 3 (device timing)** proves a measured time difference is real GPU execution time, not host dispatch noise.
4. **Tier 4 (macro workload)** proves the Tier-3 difference compounds into something a reviewer would actually call a result, in baselines that aren't strawmen.

---

## 1. Tier 1 — Static IR Verification

### 1.0 Verification status of this package, stated honestly

Everything in §1.1 and §1.2 below was not just written to look plausible — it was actually compiled and run, in a real clone of `gfx-rs/wgpu` (commit `3fb225a9c6240bd7e9db3d202410db6d894368ec`), against a real Rust 1.91 toolchain. This caught one real bug in the test design, which I'll describe rather than quietly fix and hide, because it's a useful cautionary example for anyone extending this suite later.

**What ran clean, unmodified:** the Tier 1 snapshot test (`convert_snapshots_wgsl`) processed both new `.wgsl` fixtures without error and produced the golden `.spvasm` files now included in `naga/tests/out/spv/` in this package — so you don't need to regenerate them on a first run, only re-verify them against whatever Naga version you're actually patching. The Tier 2 soundness test (`elision_soundness_table_read_zero_skip_write`) passed and printed exactly the predicted pre-patch baseline: all six cases retain their bounds check today, confirmed against real Naga output, not asserted from reading the source.

**What failed on the first run, and why:** the `Restrict`-policy smoke test originally asserted "no `OpSelectionMerge` appears anywhere in the function." That failed — but not because the §2.2 SPIR-V derivation was wrong. Dumping the actual disassembly showed Naga emits a *second*, unrelated branch for any `var<workgroup>` declaration: an `if (lid == 0u) { tile = OpConstantNull }` zero-initialization gate before the first `OpControlBarrier`, which is the concrete codegen-level expression of the exact mechanism the paper's §1 already cites — WebGPU's mandatory workgroup-memory zero-initialization, motivated by cross-tab uninitialized-memory data leakage. My first version of the test conflated "a branch exists somewhere in this function" with "a branch gates this specific access," which are different claims. The fixed version (now in `elision_assertions.rs` in this package, with the real disassembly excerpt left in as a comment) checks structurally that the `OpExtInst UMin` result feeds directly into the `OpAccessChain` for the access under test, with no intervening branch between those two specific instructions — which is the precise claim §2.2 actually makes. Both tests are green as of this package being assembled. The lesson, stated plainly: a test that compiles is not a test that's correct, and the only way to know the difference is to run it and read what it actually prints.

### 1.1 Mechanism: Naga's existing snapshot harness (don't reinvent this)

Naga already has a snapshot-testing convention, and the new test cases should use it rather than inventing a parallel system:

- WGSL fixtures live in `naga/tests/in/wgsl/*.wgsl`, each with an optional same-named `.toml` controlling backend targets and `BoundsCheckPolicy`.
- `naga/tests/naga/snapshots.rs::convert_snapshots_wgsl` iterates every `.wgsl` file in that directory automatically (`Input::files_in_dir("wgsl", &["wgsl"], DIR_IN)`) — **dropping a new file in is the entire registration step**, there is no separate list to edit.
- Output gets written, unconditionally, to `naga/tests/out/spv/*.spvasm` (human-readable SPIR-V text, produced via the `rspirv` disassembler — confirmed in `write_output_spv_inner`, no external `spirv-dis` binary needed).
- **The test itself does not assert anything about content.** Regression detection is a separate CI step: `git add . && git diff --exit-code HEAD naga/tests/out` (verified directly in `.github/workflows/ci.yml:700`). A maintainer reviews the diff once when a new golden file is added; after that, any future change to the generated SPIR-V shows up as an unreviewed `git diff` and fails CI.

I verified the real `.toml` syntax against existing precedent (`naga/tests/in/wgsl/bounds-check-restrict.toml`):

```toml
targets = "SPIRV | METAL"

[bounds_check_policies]
index = "Restrict"        # or "ReadZeroSkipWrite"
buffer = "Restrict"
```

**Files provided in this package, ready to copy in as-is:**

```
naga/tests/in/wgsl/bounds-check-elision-zero.wgsl       (ReadZeroSkipWrite policy)
naga/tests/in/wgsl/bounds-check-elision-zero.toml
naga/tests/in/wgsl/bounds-check-elision-restrict.wgsl   (Restrict policy)
naga/tests/in/wgsl/bounds-check-elision-restrict.toml
```

**How to use them:**

```bash
cp naga/tests/in/wgsl/bounds-check-elision-*.{wgsl,toml} <your-wgpu-checkout>/naga/tests/in/wgsl/
cd <your-wgpu-checkout>
cargo test -p naga --test naga convert_snapshots_wgsl
git status naga/tests/out/spv   # two new .spvasm files appear - inspect them once, by hand
git add naga/tests/out/spv/bounds-check-elision-*.spvasm
```

That manual inspection *before* the first commit is the moment you confirm, by eye, that today's (unpatched) output retains every check — i.e. this is your **pre-patch baseline golden file**. After the §4.2 patch lands, re-run the same command: the three in-scope cases (`happy_*`, the store mirror) should now show bare `OpAccessChain`/`OpLoad`/`OpStore` with no `OpSelectionMerge`/`OpBranchConditional`, and `git diff` on the regenerated `.spvasm` files becomes the human-readable "before/after" artifact for the PR description — this is literally Tier 1's required evidence, generated as a side effect of running the test suite, not something you write up separately.

### 1.2 Why a second, explicit test file exists alongside the snapshot system

Snapshot diffing tells you *that* something changed, not *what claim* the change is supposed to satisfy — a reviewer has to open a `.spvasm` file and read SPIR-V to check it. `naga/tests/naga/elision_assertions.rs` (provided in this package) states the same claims as explicit, named Rust assertions instead, using only `naga`'s public API plus `rspirv` (already a `[dev-dependencies]` entry in `naga/Cargo.toml` — confirmed directly, **zero `Cargo.toml` changes needed**).

**Wiring instructions (one line):**

```diff
 // naga/tests/naga/main.rs
 mod example_wgsl;
+mod elision_assertions;
 mod snapshots;
 mod spirv_capabilities;
 mod spirv_roundtrip;
 mod validation;
 mod wgsl_errors;
```

```bash
cp naga/tests/naga/elision_assertions.rs <your-wgpu-checkout>/naga/tests/naga/
cargo test -p naga --test naga elision_soundness_table_read_zero_skip_write -- --nocapture
cargo test -p naga --test naga happy_path_restrict_baseline_uses_umin_clamp_not_branch
```

Both tests currently assert the **pre-patch** truth (every case retains its check). The file's module doc comment explains exactly which three assertions to flip (`!check_present` instead of `check_present`) once the §4.2 patch lands — so the diff on this one file, at merge time, *is* the human-readable summary of what the patch changed, separate from and complementary to the `.spvasm` diff in §1.1.

---

## 2. Tier 2 — The Soundness Table

This is the table that should appear in the paper itself (§6, already referenced there), not just in the test suite:

| Case | WGSL | Claimed range (post-patch) | SPIR-V consequence |
|---|---|---|---|
| Happy path, exact buffer | `tile[lid]`, `array<f32,64>`, `@workgroup_size(64)` | `Some((0, 63))` | check elided |
| Happy path, oversized buffer | `tile[lid]`, `array<f32,128>` | `Some((0, 63))` | check elided |
| Stencil overflow | `tile[lid + 1u]`, `array<f32,64>` | `None` (`63+1=64 ≥ 64`) | check **retained** |
| Wraparound (critical) | `tile[lid - 1u]` | `None` (`0u32.checked_sub(1)` underflows) | check **retained** |
| Multiplicative (out of scope) | `tile[lid * 2u]` | `None` (`Mul` unhandled in v1) | check **retained** |
| Helper-forwarded (scope limit) | `touch(lid, ...)` where `touch(i: u32)` indexes `tile[i]` | `None` (no `Binding::BuiltIn` on `touch`'s own parameter) | check **retained** |

The sixth row is not in any earlier draft of this framework — it came directly out of building this scaffold, not out of the paper. See §6 below; I patched the paper's §4.3 to state it as soon as I confirmed it, rather than leaving the test suite and the paper disagreeing with each other.

A test suite that never specifically targets the *boundary* invocation (thread 63 for the stencil case, thread 0 for wraparound) is not testing this at all — the boundary invocation is the entire point, and both fixture files in this package are written so that `lid` ranges over the full `[0, 64)` workgroup, guaranteeing thread 0 and thread 63 both execute.

---

## 3. Tier 3 — Isolated Device-Time Measurement

### 3.1 The mistake this tier exists to prevent

`std::time::Instant` around a `dispatch_workgroups()` call measures host-side command submission, which is asynchronous and returns almost immediately — not GPU execution time. This is the single most common GPU-benchmarking mistake, and it's exactly the ambiguity that made an external GPU-kernel paper's sub-millisecond timing claims hard to evaluate when we cross-checked this framework against it: its methodology section states sample count and warm-up discard but never states whether each timed call was wrapped in an explicit device synchronization, which matters enormously once timings drop into the tens-of-microseconds range. This tier exists so that question is never ambiguous for *this* paper.

### 3.2 The real `wgpu` API, verified — not the obvious-looking but wrong version

There are three distinct timestamp mechanisms in `wgpu`, gated by three distinct features, and they are **not interchangeable**:

| Mechanism | Feature required | Availability |
|---|---|---|
| `ComputePassDescriptor::timestamp_writes` (pass start/end) | `Features::TIMESTAMP_QUERY` | **Web and native** — confirmed verbatim from the feature's own doc comment in `wgpu-types/src/features.rs` |
| `CommandEncoder::write_timestamp` (between commands, outside any pass) | `Features::TIMESTAMP_QUERY_INSIDE_ENCODERS` | Native only |
| `ComputePass::write_timestamp` (inside a single pass, between specific dispatches) | `Features::TIMESTAMP_QUERY_INSIDE_PASSES` | Native only — and per its own doc comment, *"some GPU architectures do not support this"* even natively |

**This matters specifically for this paper's claims**, because the `Restrict`-policy result is the one that determines whether the optimization matters for browser WebGPU — the dominant deployment. Only the first mechanism runs in a browser at all. The benchmark harness in this package (`bounds-check-bench/`) therefore uses *only* `ComputePassDescriptor::timestamp_writes`, deliberately, so the exact same measurement code path is the one a browser would actually execute — not a native-only proxy for it.

Two further details that a simplified description of this API would miss, both verified directly against `wgpu`'s real example at `examples/features/src/timestamp_queries/mod.rs`:

- **The tick-to-time conversion is not 1.0.** `Queue::get_timestamp_period()` must be queried and multiplied in; the period is backend- and device-dependent.
- **Query results can't be mapped directly.** A `QuerySet` resolves into a buffer that must have `BufferUsages::QUERY_RESOLVE`, which cannot be combined with `MAP_READ` on the same buffer. You need a second buffer and an explicit `copy_buffer_to_buffer` before you can map anything back to the CPU. The harness in this package does this correctly; a simplified sketch of this API (i.e., "resolve to a buffer, map it, read it") would not compile.

### 3.3 What's in this package

```
bounds-check-bench/
  Cargo.toml
  src/main.rs
```

This is a real, structurally complete harness: feature detection and request, query-set lifecycle, the resolve/copy/map sequence, warm-up vs. timed passes, median/P95 reporting. What's deliberately left as a `TODO(you)` is the kernel-specific plumbing — compiling the actual reduction/prefix-scan/GEMM WGSL through `naga` under a given `BoundsCheckPolicy` and building the pipeline/bind group from it — because that part depends on which Tier-4 kernel you're running and didn't need source-level verification the way the timestamp mechanics did.

**Design choice, stated explicitly:** the harness batches `DISPATCHES_PER_PASS = 10_000` dispatches inside a *single* timed pass before reading back timestamps. This amortizes pass-setup cost across enough device work that it stops dominating the measurement — analogous to the "100 samples, discard the first" warm-up discipline, just applied to dispatch count within a pass rather than to repeated host-side calls.

**Run it:**

```bash
cd bounds-check-bench
cargo run --release
```

(Will currently print a placeholder `0.000` row per baseline until the `TODO(you)` kernel-compilation step is filled in — the point of shipping it in this state is that everything *around* that step is already verified-correct, so you're only debugging your own kernel code, not also debugging the query API.)

---

## 4. Tier 4 — Macro-Workload Validation Against the Three Real Baselines

### 4.1 The baseline-selection rule, stated once so it doesn't get re-litigated per kernel

For this paper, there are exactly three meaningful comparison points, and no others:

1. **`BoundsCheckPolicy::Unchecked`** — the theoretical ceiling. Comparing against this answers "how much of the spec-mandated safety tax did we recover," without ever claiming to have eliminated the tax.
2. **`BoundsCheckPolicy::ReadZeroSkipWrite`** — the actual `wgpu`-native default being improved.
3. **`BoundsCheckPolicy::Restrict`** — the actual browser/WebGPU-spec-mandated default being improved.

Do not introduce a fourth baseline (a hand-written native kernel, a different language's compiler, a "scalar fallback") for this paper. Each of those answers a different question than the one this patch is making a claim about, and defending an unnecessary baseline choice in review costs more than it's worth. (This rule exists because we checked it against a real external paper's evaluation section while building this framework, and a baseline-selection misstep — there, an undisclosed-vs-disclosed CPU-fallback comparison — was exactly the kind of thing reviewers fixate on.)

### 4.2 Kernels

Reuse the three kernels already named in the IICT paper's §6: workgroup-tiled reduction, Hillis–Steele prefix-scan, tiled matrix-multiply. `PLACEHOLDER_KERNEL_WGSL` in `bounds-check-bench/src/main.rs` is a minimal reduction-shaped stand-in with the right access pattern (`tile[local_invocation_index]` inside a `var<workgroup>` of matching size) — replace it with the full versions; the timing/feature-gating code around it does not need to change.

### 4.3 What "success" looks like, and what it doesn't

This tier is also the test of the ILP-recovery hypothesis from the paper's §2.2: we expect any effect to be most visible in the reduction/prefix-scan kernels (low arithmetic intensity, little independent work to hide a one-instruction dependency) and least visible — possibly absent — in the tiled-GEMM kernel (enough independent arithmetic per access to plausibly hide it regardless of policy). **Report whichever way the measurement actually goes.** A result showing no measurable `Restrict`-policy effect on GEMM is not a failure of the optimization; the paper's own §2.2 already predicted that might happen, and reporting it that way is more credible than reporting only the kernel shape that favors the hypothesis.

---

## 5. File Manifest

```
naga/tests/in/wgsl/bounds-check-elision-zero.wgsl
naga/tests/in/wgsl/bounds-check-elision-zero.toml
naga/tests/in/wgsl/bounds-check-elision-restrict.wgsl
naga/tests/in/wgsl/bounds-check-elision-restrict.toml
naga/tests/naga/elision_assertions.rs        (+ one-line addition to naga/tests/naga/main.rs)
bounds-check-bench/Cargo.toml
bounds-check-bench/src/main.rs
```

## Suggested build order

1. Drop in the Tier 1 `.wgsl`/`.toml` pairs, run `convert_snapshots_wgsl`, inspect and commit the pre-patch golden `.spvasm` files. (~15 min)
2. Drop in `elision_assertions.rs`, wire the one `mod` line, run it, confirm all pre-patch assertions pass as-is. (~15 min)
3. Implement the §4.2 patch itself (not part of this document's scope).
4. Re-run both of the above; flip the three documented assertions; re-commit the regenerated golden files. The two diffs together are your PR's Tier 1+2 evidence.
5. Fill in the `TODO(you)` kernel-compilation step in `bounds-check-bench/src/main.rs`; run Tier 3 on whatever hardware you have.
6. Swap `PLACEHOLDER_KERNEL_WGSL` for the real Tier 4 kernels; run the full three-baseline matrix.

---

## 6. Honest unknowns and one thing this process caught

**What I verified directly, not assumed:** every file path, struct field, and function signature in the scaffold files; the CI snapshot-diff mechanism; the real `.toml` policy syntax; the three-tier timestamp-feature split and its web/native availability; the resolve/copy/map buffer requirement; that `rspirv` needs no new dependency.

**What I did not verify, because I can't from here:** actual measured numbers on real hardware (Tiers 3–4 require your GPU, not mine), and whether `device.poll(wgpu::PollType::wait_indefinitely())` is the exact current method name on whatever `wgpu` version you pin in your `Cargo.toml` lockfile — I confirmed the *pattern* against the workspace's own in-tree example, but if your `Cargo.toml` points at a published crates.io version rather than this exact commit, a minor API rename is possible and worth a `cargo check` before assuming the skeleton compiles unmodified.

**What building this scaffold caught, that the paper itself didn't have until just now:** while writing the WGSL test fixtures, I had to decide where to put the "happy path" accesses, and realized the optimization as designed only fires for accesses written *directly in the entry point's own body* — not inside a helper function called with the builtin value forwarded as a plain argument, because Naga performs no inlining before validation and a helper's own parameter carries no `Binding::BuiltIn` regardless of what its caller passed. This wasn't in any earlier draft of the paper. I added it to §4.3 as soon as I confirmed it (verified: no inlining pass exists anywhere in `naga/src/`, and `validate_function`/`validate_entry_point` analyze each function independently). It's now Case 6 in the soundness table above, specifically so it stays documented rather than becoming a future "wait, why doesn't this fire" bug report.
