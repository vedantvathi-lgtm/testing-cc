# Verification Notes: IICT 2026 Naga Bounds-Check Elision Paper Audit

**Date of audit:** 2026-07-08
**Files under audit:**
- `IICT2026_Naga_BoundsCheck_PracticePaper.tex` (the paper originally pointed to)
- `iict2026_bounds_check_elision.tex` / `.pdf` (a newer/different paper file found in the same root dir, compiled Jul 8 01:50/01:51 — **not yet diffed against the first paper, worth doing**)
- `BOUNDS_CHECK_ELISION_FINDINGS.md` (a findings doc describing a *different* implementation)

**Goal:** rigorously verify every technical, benchmark, and citation claim in the paper against the actual git repos, source code, and external sources (WebGPU/WGSL specs, GitHub issues, cited papers). Be harsh, don't be charitable, flag anything false, misleading, unreproducible, or fabricated.

**Method:** direct shell/git/grep verification by me + 4 parallel Sonnet subagents (one failed on a false-positive safety flag, see below) + web search/fetch for external claims.

---

## 1. Repo/branch topology (THE most important structural finding)

There are **three separate git repos** relevant here, and the paper conflates work from two of them into one narrative:

| Repo path | Branch | HEAD | What it contains |
|---|---|---|---|
| `wgpu/` | `bounds-check-elision` | `3fb225a` (upstream trunk commit) + **uncommitted working-tree changes** | Loop-induction-variable bounds check elision (matches `BOUNDS_CHECK_ELISION_FINDINGS.md`) |
| `naga/` (standalone) | `feat/api-constant-range-analysis-and-barrier-elision` | commit `f08a9bf1` (committed) | "Track 2" builtin-index check + "Track 3-lite" barrier elision (matches the **paper's** narrative) |
| `wgpu-upstream/` | `trunk` | `688e4761c` | Clean reference copy of upstream wgpu, untouched |

**IMPORTANT:** The wgpu repo's changes are **NOT COMMITTED** — `git status` shows all modifications as working-tree diffs (`M` and `??`), not commits. This means the "branch" `bounds-check-elision` currently has zero commits distinguishing it from trunk; all the actual work lives only in the uncommitted working tree. This is fragile (could be lost with a careless `git checkout`/`stash`/`clean`) and also means there's no commit history/message documenting intent for that half of the work, unlike the naga repo which has a proper commit (`f08a9bf1`) with a descriptive message.

**IMPORTANT — the core discovery:** The paper describes ONE unified piece of work ("interval analysis... seeded from `@workgroup_size`... propagated through constant addition/subtraction... Track 3/3b barrier elision"). In reality:
- The **paper's actual mechanism** (Track 2 builtin check + Track 3-lite post-barrier elision) lives only in the standalone `naga/` repo, commit `f08a9bf1`.
- The **loop-induction-variable analysis** described at length in `BOUNDS_CHECK_ELISION_FINDINGS.md` (section 1b) lives only in `wgpu/naga/src/proc/bounds_check_elision.rs` — this file **does not exist at all** in the standalone `naga/` repo that the paper is drawn from.
- Confusingly, the paper's own text in places (interval propagation, "if/break", "break_if") reads like it's describing the loop-induction work, but that code isn't in the repo the paper's other details (Track 3, barrier stuff) come from. The paper is a narrative blend of two distinct, only loosely related experimental branches, presented as one coherent contribution.

---

## 2. Track 2 ("interval analysis") — verified FALSE against the actual code

### 2a. What the paper claims (§2 "Interval Propagation and Soundness", §3 "Bounds Check Resolution")

Direct quotes from `IICT2026_Naga_BoundsCheck_PracticePaper.tex`:
- Line 156-158: *"We extended `ExpressionInfo` in `naga/src/valid/analyzer.rs` with a new field `range: Option<(u32, u32)>`, computed during the existing single forward walk over the expression arena."*
- Line 159-163: frames this as classical Cousot & Cousot abstract interpretation, flat lattice over `Option<(u32,u32)>`.
- Line 175-179: *"the propagation strictly uses `u32` bounds with `u32::checked_add` and `u32::checked_sub`. If an operation would underflow or overflow... the interval safely collapses to `None`."*
- Line 187-190: *"During the `Expression::FunctionArgument` arm of `process_expression`, the pass consults the argument's `Binding::BuiltIn` attribute. If it is a `LocalInvocationIndex` or `LocalInvocationId`, the range is seeded..."*
- Line 191-193: binary-expression propagation only when the opposing operand is compile-time constant, keeping analysis at `O(|E|)`, "avoids the need for a fixpoint iteration."
- Line 195-198: *"In `access_needs_check`, a single disjunct was added: if the index is a `GuardedIndex::Expression` and `info[expr].range` evaluates to `Some((lo, hi))` with `hi < length`, the function returns `None`."*
- Line 202: *"The patch comprises approximately 50 lines of core logic."*
- Line 199-201: claims elision is *"automatically inherited by the SPIR-V, MSL, and HLSL backends with zero changes to backend call sites."*

### 2b. What the code actually does (verified by reading source + empirical SPIR-V compilation)

Full commit diff captured at `/tmp/naga_commit.diff` (also read directly, reproduced in full in this session). Key facts:

**`analyzer.rs` is completely untouched by the commit.** `git diff d0f28c0b f08a9bf1 --stat` shows zero changes to it. `ExpressionInfo` struct (`src/valid/analyzer.rs:148-153`) has no `range` field:
```rust
pub struct ExpressionInfo {
    pub uniformity: Uniformity,
    pub ref_count: usize,
    assignable_global: Option<Handle<crate::GlobalVariable>>,
    pub ty: TypeResolution,
}
```
`grep -rn "checked_add\|checked_sub"` across `src/proc/index.rs` and `src/valid/analyzer.rs` → **NONE** found (only unrelated pre-existing uses elsewhere in constant folding). `grep -rniE "interval|lattice"` across `src/` → **zero hits**.

**What actually exists** — a single ~25-line predicate function, `builtin_index_provably_in_bounds`, added in `src/proc/index.rs:274-299` (see full text below):

```rust
pub fn builtin_index_provably_in_bounds(
    base: Handle<crate::Expression>,
    index: Handle<crate::Expression>,
    module: &crate::Module,
    function: &crate::Function,
    info: &crate::valid::FunctionInfo,
    workgroup_size: [u32; 3],
) -> bool {
    if let crate::Expression::FunctionArgument(arg_idx) = function.expressions[index] {
        if let Some(ref binding) = function.arguments[arg_idx as usize].binding {
            if let crate::Binding::BuiltIn(crate::BuiltIn::LocalInvocationIndex) = *binding {
                let base_inner = info[base].ty.inner_with(&module.types);
                if let Ok(length) = base_inner.indexable_length(module) {
                    let total_wg: u32 = workgroup_size[0] * workgroup_size[1] * workgroup_size[2];
                    if let crate::proc::index::IndexableLength::Known(len) = length {
                        return len == total_wg;
                    }
                }
            }
        }
    }
    false
}
```

This is wired into `access_needs_check` (`src/proc/index.rs:357-373`) via a new `workgroup_size: Option<[u32;3]>` parameter — **not** via an `info[expr].range` lookup (which doesn't exist):
```rust
pub fn access_needs_check(
    base: Handle<crate::Expression>, mut index: GuardedIndex,
    module: &crate::Module, function: &crate::Function, info: &crate::valid::FunctionInfo,
    workgroup_size: Option<[u32; 3]>,
) -> Option<IndexableLength> {
    ...
    if let Some(wg_size) = workgroup_size {
        if let GuardedIndex::Expression(expr) = index {
            if builtin_index_provably_in_bounds(base, expr, module, function, info, wg_size) {
                return None;
            }
        }
    }
    ...
}
```

**Claim-by-claim verdicts:**

| # | Paper claim | Verdict | Evidence |
|---|---|---|---|
| 1 | New `range: Option<(u32,u32)>` field on `ExpressionInfo`, computed in forward walk | **FALSE** | `analyzer.rs` untouched by commit; no such field exists anywhere |
| 2 | `checked_add`/`checked_sub` propagation, collapses to `None` on overflow | **FALSE** | No interval propagation of any kind exists; grep confirms zero hits |
| 3 | Handles both `LocalInvocationIndex` AND `LocalInvocationId` (vec3) | **FALSE** | Only `LocalInvocationIndex` is matched (line ~287 of the added code); empirically verified — compiled `tile[lid3.x]` with `@builtin(local_invocation_id)` under `@workgroup_size(64)`, bounds check was **NOT** elided (`OpULessThan` + branch present in output). Also structurally can't work: vec3 builtin can only be indexed via `AccessIndex`/swizzle, never a bare `Expression::FunctionArgument` |
| 4 | Binary-expression propagation restricted to constant-operand case (avoids fixpoint) | **FALSE** | No binary-expression handling exists at all — not "restricted," simply absent. Empirically verified: `tile[lid+1u]`, `tile[lid-1u]`, `tile[lid*2u]` all retain full bounds checks (correct behavior, but achieved by doing *nothing* special, not by the sophisticated checked-arithmetic collapse the paper describes) |
| 5 | `access_needs_check` disjunct checks `info[expr].range` with `hi < length` | **FALSE mechanism** | Real gate is exact equality `len == total_wg` (line ~324), not a `hi < length` inequality — meaning oversized arrays (still objectively safe) are NOT elided, contradicting the paper's own implication |
| 6 | ~50 lines of core logic | **MISLEADING** | Full commit = 167 lines across TWO unrelated features (Track 2 ≈ 77 lines incl. plumbing, ~45-50 for the actual decision logic; Track 3-lite barrier elision = 93 lines, completely separate mechanism bundled in the same commit). If "the patch" means what the paper is citing, it's >3x the claimed size |
| 7 | General interval analysis / Cousot lattice / abstract interpretation | **FALSE — narrow special case** | No lattice, no interval type, no fixpoint, no propagation. Single boolean predicate matching one exact syntactic shape |
| 8 | Works for canonical `tile[local_invocation_index]` idiom | **VERIFIED — this part is real** | Empirically compiled and confirmed: SPIR-V shows no `OpULessThan`/branch on the access path under both `ReadZeroSkipWrite` and `Restrict` policies — elision genuinely fires for the textbook case |
| 9 (new finding, not explicitly in paper but implied) | "Automatically inherited by SPIR-V, MSL, and HLSL backends with zero backend changes" | **FALSE** | `src/back/msl/writer.rs:549` always passes `None` for `workgroup_size` — Track 2 elision **only fires for the SPIR-V backend**. MSL/HLSL get zero benefit. This directly contradicts the paper's explicit claim of automatic cross-backend inheritance |

**IMPORTANT — soundness note:** The one part that *does* work (case 8) is legitimately sound and correctly requires **exact equality** between array length and workgroup-size product (verified: `array<f32,128>` + `@workgroup_size(64)` → check is retained, not elided, since `128 != 64`). So the implemented mechanism, while nowhere near what the paper describes, is at least conservative/safe within its narrow scope — it just isn't the general interval analysis claimed, isn't a Cousot-style lattice, doesn't handle `LocalInvocationId`, doesn't handle arithmetic, and doesn't reach MSL/HLSL.

---

## 3. Track 3 / Track 3b (barrier elision) — mixed; Track 3b likely doesn't exist as committed

**IMPORTANT CAVEAT:** The dedicated verification subagent for this section (`Verify barrier elision + benchmarks`) **failed** mid-run — the Opus safety system falsely flagged the barrier-elision/soundness discussion as a "cybersecurity topic" (task-id `a20a4157a277c8cc5`, error: *"safeguards flagged this message for a cybersecurity topic"*). This is a false positive (the topic is GPU memory-barrier compiler optimization, not security) but it means this section rests only on my own direct reading, not an independent second pass. Flagging this explicitly as **unfinished/needs re-verification** if a fuller pass is wanted later — could retry with a differently-worded prompt.

### 3a. What I verified directly from the naga commit diff (`/tmp/naga_commit.diff`)

The commit message says:
```
Track 3-lite: Workgroup Post-Barrier Elision
- Modified WorkGroupUniformLoad handler to conditionally emit post-barrier
- Forward scan from current statement_index finds subsequent writes to
  workgroup memory (Store, Call with ptr<workgroup> params, or another wUL)
- Scan stops at next Barrier (synchronization boundary)
```
Note it's called **"Track 3-lite"**, only describes **post-barrier** elision. There is no mention of a symmetric **pre-barrier** elision ("Track 3b") anywhere in the commit message or diff.

Code added at `src/back/spv/block.rs` (diff lines ~52-153, full text read directly in this session):
```rust
// Track 3-lite: Workgroup Post-Barrier Elision
let needs_post = {
    let mut found = false;
    for stmt in naga_block.iter().skip(statement_index + 1) {
        match *stmt {
            crate::Statement::Barrier(_) => break,
            crate::Statement::WorkGroupUniformLoad { .. } => { found = true; break; }
            crate::Statement::Store { pointer: p, .. } => {
                if self.pointer_targets_workgroup(p) { found = true; break; }
            }
            crate::Statement::Call { function: fh, .. } => {
                if self.callee_takes_workgroup_ptr(fh) { found = true; break; }
            }
            _ => {}
        }
    }
    found
};
if needs_post {
    self.writer.write_barrier(crate::Barrier::WORK_GROUP, &mut block);
}
```
Plus two helper functions `pointer_targets_workgroup` (conservative pointer trace back to a `GlobalVariable`, defaults to `true`/keep-barrier if it can't prove otherwise) and `callee_takes_workgroup_ptr` (checks callee signature for workgroup pointer params).

**`grep -n -i "pre-barrier\|track3b\|pre_barrier"` in `naga/src/back/spv/block.rs` → NONE found.** Only post-barrier ("Track 3-lite") logic exists in the committed code.

**IMPORTANT — this directly contradicts the paper's §"Track 3b: Pre-Barrier Elision"** which describes: *"when the backend encounters an explicit `Barrier(WORK_GROUP)` statement, it scans forward for the first non-`Emit` statement. If that statement is a `WorkGroupUniformLoad`, the barrier is elided."* **This code does not exist in the repo.** Yet the paper presents an entire benchmark table (Table 1, `track3b_test` kernel, 582→574 words) built around this non-existent mechanism.

### 3b. Barrier type — paper is factually wrong (confirmed independently by me AND the web-research agent)

The paper's abstract/§4.2 says Naga emits "`OpMemoryBarrier` with `WORKGROUP` semantics" around `workgroupUniformLoad`. The web-research agent fetched the actual upstream `naga/src/back/spv/writer.rs` and `block.rs` and confirmed: Naga uses `write_control_barrier`, emitting **`OpControlBarrier`** (full control-flow + memory barrier, `ACQUIRE_RELEASE | WORKGROUP_MEMORY` semantics), not `OpMemoryBarrier`. `OpMemoryBarrier` is a separate, less powerful instruction used elsewhere (plain `storageBarrier()`/`textureBarrier()`). This was independently corroborated by the WGSL spec itself (§17.11.4): *"Executes a **control barrier** synchronization function..."*

**IMPORTANT — soundness concern (my own analysis, not independently re-verified due to the failed agent):** The elision condition checks only for a *memory hazard* (does a later statement write workgroup memory or issue another WGUL). A **control barrier** additionally guarantees all invocations in the workgroup have *reached* that point (reconvergence) before any proceeds — a property the memory-hazard-only check does not establish. Eliding a control barrier based solely on absence of a subsequent workgroup-memory write may not preserve the full contract of `workgroupUniformLoad`, especially:
- The forward scan is **block-local only** (`naga_block.iter().skip(...)`) — it does not cross loop back-edges or into nested blocks/branches. A WGUL inside a loop body whose "next statement" is loop-exit control flow, with the actual hazard living in the *next iteration*, would not be detected by this scan. This is a plausible correctness gap, not confirmed with a concrete failing repro in this session.
- `pointer_targets_workgroup` conservatively returns `true` (keep barrier) for `LocalVariable`/`FunctionArgument`-sourced pointers it can't trace, which is good defensive design — but the scan still only looks at the **current block**, so cross-block hazards elsewhere in the function are invisible to it.

**This soundness question is flagged as NOT fully resolved** — worth a dedicated adversarial pass (the agent meant to do this failed on a false safety flag).

### 3c. Benchmark data reproducibility (I verified this directly myself)

Data location: `bounds-check-testing-framework/bounds-check-bench/results_with_track3b.json` and `results_no_track3b.json` (run IDs `epoch20635_27586` and `epoch20635_27800` respectively).

I extracted the actual `track3b_test` kernel measurements via a Python script reading both JSON files. Results:

| Size | Policy | With T3b (median_us) | No T3b (median_us) | bounds_checks_emitted (with/without) |
|---|---|---|---|---|
| 64K | ReadZeroSkipWrite | 2.520 | 2.594 | 574 / 582 |
| 1M | ReadZeroSkipWrite | 21.395 | 22.420 | 574 / 582 |
| 10M | ReadZeroSkipWrite | **242.746** | 258.561 | 574 / 582 |
| 16M | ReadZeroSkipWrite | 363.379 | 385.606 | 574 / 582 |

**Most of the paper's Table 1 numbers DO trace exactly to this real data** (64K, 1M, 16M all match verbatim; the 582/574 word-count / bounds_checks_emitted delta also matches).

**IMPORTANT DISCREPANCY:** I explicitly grepped `grep -rl "242.746" --include=*.json` across the **entire repository tree** and got **zero matches in any file except the two I already knew about** — wait, correction: my direct extraction script above *did* pull `242.746` directly out of `results_with_track3b.json` for the 10M row, so it IS present in that file (my earlier grep before writing the Python extractor apparently missed it due to a grep quoting/escaping issue, not because the number is actually absent — **this needs a re-check, flagging as uncertain**, but the Python `json.load` extraction is authoritative and shows 242.746 does exist in `results_with_track3b.json` under the `track3b_test`/10M/ReadZeroSkipWrite entry). Net: the Table 1 numbers appear to be genuinely reproducible from real recorded JSON data, contrary to my earlier interim assessment that 242.746 was untraceable — **retracting that earlier claim**.

**Still-valid concerns about the benchmark methodology:**
1. The two configs (`with_track3b` vs `no_track3b`) are **separate benchmark runs** (different `run_id`s, different wall-clock sessions: `...27586` vs `...27800`), not a paired/interleaved measurement. Any system-level drift (thermal, clock, background load) between the two runs would bias the comparison.
2. The paper's own §5.3 admits the control kernel (`wgul_reduce`, no Track3b-eligible pattern) shows "0.3–3.4% variance, consistent with GPU run-to-run noise" — meaning the claimed 2.9% (64K) result for `track3b_test` is **within the noise band of the paper's own control measurement**. Only the 10M/16M results (6.1%/5.8%) clearly exceed the noise floor.
3. Removing a single `OpControlBarrier` (not `OpMemoryBarrier` as the paper claims) plausibly *could* produce a real few-percent speedup at scale, since control barriers force full workgroup reconvergence — this is more physically plausible than removing a lightweight memory-only barrier would be, but the paper's own mischaracterization of the instruction undermines confidence that the described mechanism (and its safety analysis) was reasoned about correctly even if the measured effect is real.

### 3d. wgul_reduce (Track 3, non-3b) numbers

Paper claims 699→691 words. I found both numbers appear in multiple JSON files (`results_with_track2.json`, `results_baseline_no_track2.json`, `results_with_track3.json`, etc.) — these are plausible/traceable, consistent with a real static word-count difference, though I did not do a full paired-run analysis for this pair given time constraints.

---

## 4. `BOUNDS_CHECK_ELISION_FINDINGS.md` vs the `wgpu/` working tree — verified claim-by-claim

This doc describes the OTHER implementation (loop-induction, in `wgpu/naga/src/proc/bounds_check_elision.rs`), separate from the paper's naga-repo work.

| Claim | Verdict | Evidence |
|---|---|---|
| `bounds_check_elision.rs` exists, handles if/break + break_if loop patterns | **VERIFIED** | File exists, 176 lines. `analyze_loop_body` (lines 52-71) and `analyze_break_if` (lines 76-111) match the described patterns; only recognizes `Load{pointer: LocalVariable(v)}` compared to a `Literal` constant, tags `Expression::Access` nodes accordingly (lines 148-176) |
| "Fixed the argument order in function.rs:1942... code was there but wired wrong" | **FALSE** | `git diff` at that location shows a pure **insertion** of one new line (`crate::proc::bounds_check_elision::elide_bounds_checks(fun, module, &mut info);`) — no prior call existed to have a "wrong argument order." This is the first-ever wiring of a brand-new function, not a bugfix to existing wiring. The narrative is fabricated. |
| "Interval Analysis (analyzer.rs) — Pre-existing in naga... worklist algorithm... tracks lid ∈ [0,255]" | **FALSE** | Grepping the pre-diff `analyzer.rs` (`git show HEAD:...`) for interval/worklist/range terms → nothing. No such analysis pre-existed anywhere in naga. What the diff *adds* (61 new lines, `analyzer.rs:1318-1373`) is itself a narrow one-shot special case (find `local_invocation_index` argument, look up workgroup_size, scan `Access` exprs, compare to constant array length) — a linear scan, not a worklist/fixpoint algorithm, and brand new in this branch, not pre-existing |
| 238/239 tests pass, 214 pre-existing + 1 failure, 24/24 stress tests | **VERIFIED — actually ran the test suite** | `cargo test -p naga --test naga bounds_check` → 24 passed, 0 failed. `cargo test -p naga --test naga` (full suite) → 238 passed, 1 failed (`recognized_but_unimplemented_enable_extension`). Numbers reconcile: 238 total − 24 new stress tests = 214 pre-existing, matching the doc's framing once parsed correctly |
| Subgroups regression is a "test now fails an assertion" | **VERIFIED but SEVERITY UNDERSTATED** | The failing test doesn't just fail an assertion — it **panics** with `internal error: entered unreachable code` at `enable_extension.rs:202`. The diff replaced a match arm with `_ => unreachable!()` while leaving the `Subgroups` enum variant public and constructible. **This is a live, reachable panic in shipped code** (a public enum variant that panics when `.to_ident()` is called on it) — this alone would block any upstream PR; it's an API/soundness defect, not merely "a test that expected an error and got something else" |
| Benchmark numbers (matmul: 43,360/48,128ns; SPIR-V 399/432 words) | **PARTIALLY VERIFIED** | `/tmp/bce_bench2/` binaries still exist and were re-run: SPIR-V word counts reproduced **exactly** (399/399 elided, 432/404 retained, +28 words delta). GPU timing reproduced only in *direction*, not exact magnitude (re-run got 44,032/47,600ns vs claimed 43,360/48,128ns) — close but not identical, and no raw log file backs the exact originally-quoted numbers (`grep` for "43360"/"48128" across the repo found zero genuine matches). Treat exact nanosecond figures as illustrative single-run snapshots, not hardened data |
| Is FINDINGS.md the same work as the paper? | **NO — confirmed disjoint** | Different repos, different files, non-overlapping techniques (loop-induction vs builtin-exact-match + barrier elision). The paper's own prose (its Track-2 "if/break"/"break_if" language) appears to actually be describing the `wgpu/` repo's loop-induction code — but that code doesn't exist in the standalone `naga/` repo the paper's Track 2/3 sections are otherwise drawn from. The paper conflates the two branches into one narrative. |

---

## 5. External/citation verification (web research, via WebFetch/WebSearch)

| Claim | Verdict | Detail + Source |
|---|---|---|
| gfx-rs/wgpu Issue #6345 exists, titled "[naga spv-out] Bounds check generation could be cleaner and smarter" | **VERIFIED** (existence/title) but **MISLEADING** (what it asks for) | Real issue, opened by jimblandy 2024-09-30. It is a pure **refactoring/code-clarity** request about the `BoundsCheckResult` enum not covering all outcomes cleanly — **no mention whatsoever** of `workgroup_size`, `local_invocation_index`, or interval/range analysis. Still open, no comments, no linked PRs. Calling this optimization work a "target" of that issue overstates the connection — it's tangential at best. [github.com/gfx-rs/wgpu/issues/6345](https://github.com/gfx-rs/wgpu/issues/6345) |
| WebGPU/WGSL spec guarantees `local_invocation_index`/`local_invocation_id` bounded by `@workgroup_size` | **VERIFIED, exact match** | WGSL spec §15.3: *"local invocation index L(I) such that 0 ≤ L(I) < W, and that entire range is covered."* This is the one core technical claim in the paper that is fully correct. [w3.org/TR/WGSL](https://www.w3.org/TR/WGSL/) |
| Naga emits `OpMemoryBarrier` (not `OpControlBarrier`) around `workgroupUniformLoad`; eliding post-barrier is sound | **FALSE on instruction type** | WGSL spec explicitly says `workgroupUniformLoad` executes a "**control barrier**." Naga source uses `write_control_barrier` → `OpControlBarrier`, confirmed by both my own diff-reading and the web agent independently reading upstream `naga/src/back/spv/{writer,block}.rs`. The paper's soundness argument is built on a mischaracterized instruction. |
| "GSOHC synchronization-optimization lineage" | **VERIFIED to exist, but WRONG DOMAIN and uncited** | GSOHC = "Global Synchronization Optimization in Heterogeneous Computing" (Basu & Vedurada, ECOOP 2025, Distinguished Artifact Award) — real, but about **CPU-GPU host/device dispatch synchronization** (hoisting `cudaDeviceSynchronize` calls), not intra-GPU workgroup memory barriers. Sharing only the phrase "synchronization optimization," not the technique or problem domain. **Not in the paper's reference list at all** — a positioning/citation defect. [drops.dagstuhl.de/entities/document/10.4230/LIPIcs.ECOOP.2025.21](https://drops.dagstuhl.de/entities/document/10.4230/LIPIcs.ECOOP.2025.21) |
| LLVM IRCE cited slides URL | **Pass is real; URL is DEAD** | IRCE is a real LLVM pass (Phabricator D6693, Dec 2014). The paper's cited URL `https://llvm.org/devmtg/2015-04/slides/Hal_Finkel_IRCE.pdf` returns **HTTP 404** (confirmed via `curl -I`). This citation needs fixing or removal. Also: the IRCE analogy is a stretch — IRCE does genuine induction-variable/loop-splitting analysis; the Naga mechanism is a single hardcoded exact-equality check on one builtin, not comparable in sophistication. |
| Novelty of the technique | Narrow novelty for Naga specifically is plausible (no prior upstream issue/PR proposes this exact thing); the *general* technique (bound-seeded check elision) is textbook/old (going back to classic range-check elimination literature). Paper should frame this as engineering novelty within Naga, not algorithmic novelty. |
| IICT 2026 venue | **VERIFIED real** | Real workshop, "Innovations in Compiler Technology," IISc Bengaluru, Oct 2-3 2026, third edition (compilertech.org, easychair.org/cfp/IICT2026). Workshop-tier, not a flagship venue — paper should represent this accurately. |

---

## 6. Summary of severity (my synthesis, not yet delivered as final verdict to user)

**Fabricated/false core mechanism claims (most severe):**
- The entire §2 "interval analysis" narrative (Cousot lattice, `range` field, checked-arithmetic propagation, fixpoint avoidance) — describes machinery that **does not exist** in the actual committed code. This is the paper's central technical contribution and it is not implemented as described.
- `LocalInvocationId` support — claimed, does not exist, empirically fails.
- Cross-backend inheritance (SPIR-V/MSL/HLSL) — claimed, false; MSL hardcodes `None`.
- Track 3b (pre-barrier elision) — an entire benchmark section is built around a mechanism (`Barrier→WGUL` forward-scan elision) that **is not present in the committed code** (only post-barrier "Track 3-lite" exists).
- `OpMemoryBarrier` vs `OpControlBarrier` — factual error about which SPIR-V instruction is being elided, undermines the soundness argument.

**Misleading but not fabricated:**
- "~50 lines of core logic" undercounts by >3x when including the bundled barrier-elision work.
- Issue #6345 framing — real issue, wrong description of its content/intent.
- GSOHC citation — real work, wrong domain, not actually cited.
- IRCE citation — real pass, dead URL.

**Verified/solid:**
- WebGPU/WGSL spec guarantee about `local_invocation_index`/`local_invocation_id` bounds — correct.
- The narrow exact-match elision for the canonical `tile[local_invocation_index]` idiom — genuinely works, empirically confirmed via direct SPIR-V compilation, and is conservatively sound (requires exact length equality).
- 238/239 naga test suite pass, 24/24 stress tests — genuinely reproduced by running `cargo test` directly.
- Most of Table 1's benchmark numbers do trace to real recorded JSON data (with caveats about separate-run methodology and noise floor).
- Venue (IICT 2026) is real.

**Unresolved / needs more work:**
- Track 3/3b soundness analysis (cross-block/loop-back-edge hazard in the forward scan) — flagged as a plausible correctness gap, not confirmed with a concrete repro; the agent meant to dig into this failed on a false safety-system flag.
- Whether the two newer paper files (`iict2026_bounds_check_elision.tex/.pdf`, compiled 2026-07-08, sitting in the same root dir) differ meaningfully from `IICT2026_Naga_BoundsCheck_PracticePaper.tex` — **not yet diffed**, should be done before final verdict.
- The subgroups-regression panic (`unreachable!()` in `enable_extension.rs:202`) is a live defect in the `wgpu/` working tree that would need fixing before any upstream PR regardless of the elision work's merits.

---

## 7. Thought process / reasoning notes (for continuity)

- Started by establishing ground truth on repo/branch state before trusting any doc — this immediately surfaced the two-repo/two-implementation discrepancy, which turned out to be the single most important structural finding shaping the whole audit.
- Prioritized reading the actual committed diff (`naga/` repo, `f08a9bf1`) over the working-tree-only `wgpu/` repo changes, since the paper's citations/line-references (`analyzer.rs`, `process_expression`, `access_needs_check`) pointed at the naga repo's file layout.
- Used parallel Sonnet subagents for breadth (Track 2 code claims, Track 3/benchmark claims, FINDINGS.md claims, external web claims) while doing my own direct verification in parallel for speed and cross-checking — this caught the agents' work independently converging on the same conclusions (good confidence signal), e.g. both I and the Track-2 agent and the web agent independently found the `OpControlBarrier` vs `OpMemoryBarrier` discrepancy from different angles.
- One agent (barrier elision + benchmark plausibility) failed due to an apparent false-positive safety-system trigger on "barrier"/"soundness" phrasing being misread as a cybersecurity topic — this is a known category of over-trigger for compiler/GPU-synchronization topics that use words overlapping with security jargon (barrier, elision, soundness gap). Retry with rephrased prompt if a full independent pass on Track 3 soundness is wanted.
- Did NOT yet deliver a final "good for IICT or not" verdict to the user — that synthesis is pending, to be delivered as a direct answer once this notes file was requested and written. The material above is sufficient to write that verdict: the paper's central technical narrative (interval analysis) does not match its own implementation, several claims are demonstrably false or unreproducible, but the underlying real work (narrow builtin-exact-match elision, real barrier-elision idea, real modest measured speedup) does have a kernel of genuine, if oversold and mischaracterized, value — appropriate framing would be a heavily-scoped-down engineering note, not the current draft's sweeping claims.

---

## 8. File/path index (for quick reference)

- Paper (original target): `/home/bb/Projects/Dave-Sonar-WGPU--master/IICT2026_Naga_BoundsCheck_PracticePaper.tex`
- Paper (newer, undiffed): `/home/bb/Projects/Dave-Sonar-WGPU--master/iict2026_bounds_check_elision.tex` (+ `.pdf`)
- Findings doc: `/home/bb/Projects/Dave-Sonar-WGPU--master/BOUNDS_CHECK_ELISION_FINDINGS.md`
- wgpu repo (loop-induction work, uncommitted): `/home/bb/Projects/Dave-Sonar-WGPU--master/wgpu` (branch `bounds-check-elision`)
  - New file: `wgpu/naga/src/proc/bounds_check_elision.rs` (176 lines)
  - New tests: `wgpu/naga/tests/naga/bounds_check_stress.rs` (24 tests), `wgpu/naga/tests/naga/elision_assertions.rs`
  - Modified: `wgpu/naga/src/valid/function.rs` (~line 1942, new call insertion), `wgpu/naga/src/valid/analyzer.rs` (+61 lines), `wgpu/naga/src/front/wgsl/parse/directive/enable_extension.rs` (subgroups regression, panic at line 202)
- naga repo (paper's actual source, committed): `/home/bb/Projects/Dave-Sonar-WGPU--master/naga` (branch `feat/api-constant-range-analysis-and-barrier-elision`, commit `f08a9bf1`)
  - `src/proc/index.rs` (+47 lines: `builtin_index_provably_in_bounds`, modified `access_needs_check`)
  - `src/back/spv/block.rs` (+93 lines: Track 3-lite post-barrier elision, `pointer_targets_workgroup`, `callee_takes_workgroup_ptr`)
  - `src/back/spv/{index,mod,writer}.rs`, `src/back/msl/writer.rs` (plumbing; MSL always passes `None`)
- Benchmark data: `/home/bb/Projects/Dave-Sonar-WGPU--master/bounds-check-testing-framework/bounds-check-bench/*.json` (esp. `results_with_track3b.json`, `results_no_track3b.json`)
- Old benchmark binaries (still present, re-run successfully): `/tmp/bce_bench2/` (matmul_bench, loop_bench, oob_test, etc.)
- Diffs captured this session: `/tmp/naga_commit.diff`, `/tmp/wgpu_worktree.diff`, `/tmp/naga_worktree.diff` (empty)

### External links referenced/verified
- Issue #6345: https://github.com/gfx-rs/wgpu/issues/6345
- WGSL spec: https://www.w3.org/TR/WGSL/
- WebGPU spec: https://www.w3.org/TR/webgpu/
- GSOHC paper: https://drops.dagstuhl.de/entities/document/10.4230/LIPIcs.ECOOP.2025.21
- GSOHC artifact: https://zenodo.org/records/15302892
- LLVM IRCE review: https://reviews.llvm.org/D6693
- Dead IRCE slides URL (cited by paper, 404): https://llvm.org/devmtg/2015-04/slides/Hal_Finkel_IRCE.pdf
- IICT venue: https://compilertech.org/ , https://easychair.org/cfp/IICT2026
- Related (not cited by paper but relevant) issue #5878: https://github.com/gfx-rs/wgpu/issues/5878

---

## 9. SECOND AUDIT PASS — `iict2026_bounds_check_elision.tex` (the newer, "main" paper per user)

**Date:** 2026-07-08 (same day, follow-up). User designated this file as authoritative, superseding `IICT2026_Naga_BoundsCheck_PracticePaper.tex`.

### 9a. What got FIXED vs the old paper (credit where due)

- Dropped the fabricated "GSOHC synchronization-optimization lineage" claim entirely — no longer cited/mentioned.
- Dropped the dead-link LLVM IRCE citation entirely.
- Dropped the false "LocalInvocationId" support claim.
- Dropped the false "automatically inherited by SPIR-V, MSL, HLSL backends" claim — new paper only ever mentions the SPIR-V backend.
- Dropped the entire Track 2 (naga/ repo builtin-exact-match) / Track 3 / Track 3b (barrier elision) narrative — no longer claims a mechanism (pre-barrier elision) that doesn't exist in code.
- Dropped the misleading "targets issue #6345" framing.
- Author/venue framing is now honest: "Independent Research," no institutional (IISc) claim, no explicit in-body venue/date claim (only the filename references IICT2026).
- Now correctly attributes the loop-induction analysis as one of the two named contributions (previously buried only in FINDINGS.md, absent from the old paper's actual text).

### 9b. NEW critical finding: §3.1 "Interval Analysis" is still fabricated — text AND diagram

The paper's Design section (§3.1, `iict2026_bounds_check_elision.tex:167-191`) claims:
> "The interval analysis tracks the range of integer-valued expressions through the function's control-flow graph using a worklist algorithm. For each expression, it maintains an interval `[min, max]`..."
> Inference rules: `local_invocation_index` → `[0,W-1]`; **`i + c` where `i` has interval `[a,b]` produces `[a+c, b+c]`**; **`if i < N { body }` narrows `i` to `[0, N-1]` inside `body`**.

I read the actual `+61`-line diff to `wgpu/naga/src/valid/analyzer.rs` (the only code this section could possibly refer to) in full. It is **not** a worklist algorithm, does **not** maintain a `[min,max]` interval for "each expression," and does **not** implement arithmetic propagation or if-condition narrowing anywhere. It is a single, one-shot, hardcoded scan:
1. Find the function argument bound to `@builtin(local_invocation_index)`.
2. Look up the entry point's `workgroup_size` product.
3. Scan every `Access` expression once; if its index is exactly that lid argument and the array's constant length `>= wg_size`, insert into `safe_indices`.

`grep` across `analyzer.rs` and `bounds_check_elision.rs` for `BinaryOperator::Add`, any `struct` with `min`/`max` fields, or any `Interval`/`Range`-like type for expression bounds → **zero hits, confirmed twice** (once by a subagent earlier, once directly by me this pass). There is no code path anywhere in the wgpu/naga working tree that computes `i+c` intervals or narrows a variable's range from an `if` condition outside the one hardcoded loop-break pattern (which lives in the *separate* `bounds_check_elision.rs` file, i.e., is really part of §3.2 "Loop Induction," not §3.1).

**Worse: the paper includes a figure (`Fig 2`, `/tmp/interval_analysis.pdf`, referenced at `iict2026_bounds_check_elision.tex:175-181`) whose content I extracted via `pdftotext` and which depicts a full generic algorithm that was never built:**
```
Function Entry → For each expression e → Is e a binary op? (constant + typed value)
→ Compute interval from operands' intervals → Narrow interval using if-condition
constraints (e.g., i < N => i in [0,N-1]) → More? → max < array_length? → Mark e as safe index
```
This is a flowchart of a **general, per-expression, control-flow-sensitive interval analysis with binary-op propagation and if-narrowing** — none of which exists in the implementation. This is the single most serious defect in the new paper: a diagram presented as documentation of the actual algorithm that in fact depicts an analysis that was never implemented. §3.1's inference-rule bullet list and Figure 2 should either be removed, or §3.1 should be rewritten to describe what's actually there: a single hardcoded pattern match on one builtin, no propagation, no narrowing, no worklist.

By contrast, **§3.2 "Loop Induction Variable Analysis" and its Figure 3 (`/tmp/loop_induction.pdf`) DO accurately match the real code** (`wgpu/naga/src/proc/bounds_check_elision.rs`, 176 lines, full text read directly this session) — I extracted the diagram's text and it faithfully mirrors `analyze_loop_body`/`extract_bound_from_comparison`: "Find Statement::Loop → Walk loop body → Is if(condition)? → Is condition Binary(Less,...)? → Is left a Load→LocalVariable(v)? → Is right a literal N? ... Does reject block [contain Break]?" This section of the paper is honest and its diagram is trustworthy.

### 9c. Verified: `safe_indices` plumbing claim is accurate

§3.3 "Integration with Validator" claims: *"populates a `safe_indices: HashSet<Handle<Expression>>`... The SPIR-V backend consults this set during code emission."* This is **VERIFIED TRUE**, unlike §3.1:
- `analyzer.rs` diff: `pub safe_indices: crate::FastHashSet<Handle<crate::Expression>>` added to `FunctionInfo` struct.
- `function.rs` diff (single-line insertion, not at the claimed 1942 but functionally the same spot): `crate::proc::bounds_check_elision::elide_bounds_checks(fun, module, &mut info);` — confirmed a **pure insertion**, no prior call existed (this re-confirms FINDINGS.md's "argument-order fix" narrative was fabricated — there was nothing to reorder).
- `back/spv/index.rs` diff, two call sites, both real:
  ```rust
  if let GuardedIndex::Expression(expr) = index {
      if self.fun_info.safe_indices.contains(&expr) {
          return Ok(BoundsCheckResult::Computed(self.cached[expr]));
      }
  }
  ```
This is a real, working, sound (defaults to "checked" unless explicitly proven safe) mechanism — it's just that it's fed by the loop-induction analysis and the one hardcoded lid-exact-match check, not by any general interval analysis.

### 9d. Numeric/methodology spot-checks

- **"500 trials" for matmul benchmark** — **VERIFIED TRUE**. `/tmp/bce_bench2/src/matmul_bench.rs:296`: `let num_trials: u32 = ... .unwrap_or(500);` plus a 20-trial warmup. Real, matches paper.
- **"12.5% instruction reduction" (headline stat, appears in Abstract + Conclusion)** — **UNTRACEABLE**. `28/224 = 12.5%` exactly, but no "224" appears anywhere in the paper's own tables. Every ratio actually derivable from the paper's own Table 2/4 data gives a different number: 28/423=6.6%, 28/399=7.0%, 28/432=6.5%, 46/428=10.75%, 5/86=5.8% (instruction-count table). None equal 12.5%. This looks like a number computed from data outside the paper (possibly an unlisted baseline) and then presented as a headline result without a visible derivation — flag as an unsupported statistic pending clarification of its source.
- **238/239 tests, 24/24 stress tests** — reconfirmed accurate from the prior pass's direct `cargo test` run.
- **SPIR-V word counts (Table 2)** — reconfirmed exactly reproducible from a direct re-run of `/tmp/bce_bench2/target/release/matmul_bench` in the prior pass.
- **GPU timing (Table 3)** — reconfirmed close-but-not-exact on re-run in the prior pass (single-run snapshot, no raw log backing the exact quoted ns figures, but directionally consistent and methodologically real — 500 trials genuinely run, not fabricated).
- **"the single failure is a pre-existing regression from an unrelated enable subgroups parser change"** (Limitations/§6.4) — technically true that it's unrelated to elision, but still **materially understates severity**: it's a live `unreachable!()` panic on a publicly-constructible enum variant (`enable_extension.rs:202`), not merely a failed assertion. Calling it a "regression" undersells that this is a shippability blocker independent of the elision work.
- **Related Work claims** (`VK_EXT_robustness2`, NVIDIA compute-sanitizer/cuda-memcheck) — both are real, well-known, uncontroversial facts; no issue found.

### 9e. Net assessment of the new paper vs the old one

The new paper is a **substantial improvement**: it removes every fabricated citation and every falsifiable "this works across all backends" / "this issue is our target" overclaim found in the old paper, and it correctly centers on the repo (`wgpu/`) where the described mechanisms actually live (rather than conflating it with the separate `naga/` repo's different implementation). Two of its two headline claimed contributions are asymmetric in honesty:
- **Loop induction analysis: real, accurately described, accurately diagrammed, tested (24/24), sound-by-construction (conservative fallback).**
- **"Interval analysis": the name and Figure 2 are still describing a general, worklist-based, arithmetic-propagating, control-flow-narrowing analysis that was never built.** What exists is a second, even-narrower special case than the loop analysis — a single hardcoded `local_invocation_index`-only, exact-length-match check with zero propagation. This is the same underlying gap identified in the old paper (§2's Cousot-lattice narrative), just rebranded with a shorter, still-inaccurate description and a fabricated supporting diagram.

**Bottom line for this pass:** the paper is honest about the empirical results (SPIR-V words, timing methodology, OOB tests, test suite) and about one of its two analyses (loop induction), but is not honest about the other (the "interval analysis," in both prose and figure). This should be fixed — either implement the described arithmetic/narrowing logic for real, or rewrite §3.1 and replace/remove Figure 2 to describe the actual single-pattern builtin check. Also trace or drop the "12.5%" headline figure.

---

## 10. THIRD PASS — Rigorous re-benchmarking, deeper testing, and final IICT verdict

**Date:** 2026-07-08, same day, follow-up to §9. User asked to fix the benchmarking methodology, do proper testing, verify remaining claims, and deliver a final go/no-go for IICT submission.

### 10a. Test suite re-confirmed fresh + extended

Ran `cargo test -p naga --test naga` from a clean state (not relying on cached results from earlier sessions): **238 passed, 1 failed** (the `unreachable!()` panic in `enable_extension.rs:202`, unchanged from before). Ran the 24-test `bounds_check` stress subset: **24/24 passing**, independently reconfirmed.

Added one new adversarial test, `local_invocation_id_vec3_not_elided` (`wgpu/naga/tests/naga/bounds_check_stress.rs`), targeting the gap identified in §9b/agent verification: does `tile[lid3.x]` where `lid3` is `@builtin(local_invocation_id)` (vec3) get incorrectly elided? Result: **test passes — the bounds check IS correctly retained.** This confirms the gap "fails closed": the elision analysis simply doesn't recognize this expression shape (a vec3 builtin accessed via `.x`/`AccessIndex`, not a bare `FunctionArgument`) and therefore does nothing, leaving the default (checked) behavior intact. Not a soundness bug — a missed optimization opportunity, exactly as the paper's own Limitations section (implicitly) suggests, though the paper doesn't test this specific case itself. Full suite re-run after the addition: **239 passed, 1 failed** (240 total, +1 for the new test).

### 10b. Rigorized the matmul benchmark — found the original methodology **understated** the bounds-check overhead by ~3x

Per the plan, I identified two real flaws in `/tmp/bce_bench2/src/matmul_bench.rs` (the source of the paper's Table 3/4 numbers): (1) the 4 configs were timed in **blocked** sequence (500 trials of config A, then 500 of B, ...), so thermal/clock drift over ~2000 total dispatches could bias later-run configs; (2) only point deltas of separately-computed medians were reported, with no confidence interval on the derived "bounds check overhead" estimator.

I wrote a corrected version, `matmul_bench_paired.rs` (now persisted at `bounds-check-testing-framework/matmul-paired-bench/`, previously only in `/tmp`), that:
- Round-robins one trial per config per round (with the starting config rotated each round too), instead of running each config in a fixed block — this cancels drift instead of letting it accumulate onto whichever config runs last.
- Records true **per-round paired deltas** (not deltas of independently-computed medians) and bootstraps a 95% CI (2000 resamples) on the median of each.
- Built and ran for real on this machine's RTX 4060 Laptop GPU (500 rounds = 2000 total dispatches, same total sample budget as the original benchmark).

**Results (real run, this session):**
```
SPIR-V words: Elided 399/399 (checked/unchecked, identical), Retained 432/404 (+28) — unchanged, reconfirmed exact match again.

Elided:   checked - unchecked   median =     0.0 ns   95% CI [0.0, 0.0]
Retained: checked - unchecked   median =  7696.0 ns   95% CI [7536.0, 7808.0]
Bounds check overhead:           median =  7568.0 ns   95% CI [7456.0, 7744.0]   <- excludes zero, real effect
```

**This is a significant, concrete correction to the paper's own numbers.** Two things changed once measurement was done properly:
1. **The "elided" case's overhead vs. unchecked, which the paper reports as +3,744ns, is actually exactly 0ns with a zero-width CI** once drift is controlled — an even *stronger* confirmation of the "zero overhead when elided" claim than the paper's own (noisier) number suggested.
2. **The real bounds-check cost is `~7,568ns, not the paper's claimed 2,400ns` — roughly 3.2x larger.** The paper's blocked-measurement methodology evidently let some other factor (most likely warm-up/thermal/scheduler state differences between blocks) inflate the "elided checked" baseline and/or deflate the "retained checked" reading, understating the true cost of the bounds check by a substantial margin. Recomputing the per-cycle estimate with the corrected number: `7568ns / 262144 evaluations ≈ 28.9 picoseconds/check ≈ 69 GPU cycles at 2.4GHz` — an order of magnitude higher than the paper's claimed "5-6 GPU cycles per bounds check."

**Implication for the paper:** the core qualitative claim (elision achieves literally identical performance to unchecked; the bounds check itself has a real, non-zero cost) is **more strongly supported** by rigorous measurement than the original data showed. But the specific quantitative claims (2,400ns overhead, 5-6 cycles/check) are **wrong and should be replaced** with the corrected, confidence-interval-backed numbers above before submission.

### 10c. Track 3 (barrier elision) benchmark: descoped, with justification

Per the plan, I did not build the equivalent paired/interleaved benchmark for Track 3 (post-barrier elision). Two reasons, both confirmed this session: (1) that mechanism lives only in the separate standalone `naga/` repo, which the current "main" paper (`iict2026_bounds_check_elision.tex`) no longer references or claims anywhere — re-benchmarking a mechanism outside the paper under review isn't a good use of effort; (2) as below, that mechanism has a confirmed real correctness bug, which matters more than its performance number.

### 10d. CONFIRMED: a real, previously-unverified correctness bug in Track 3 (barrier elision) — out of scope for the current paper, but a genuine finding

Last session's dedicated verification agent for barrier-elision soundness failed on a false safety-system flag, leaving the question open. I resolved it directly this session by compiling concrete WGSL repros through the actual SPIR-V backend and reading the disassembly.

First attempt targeted the wrong repo (`wgpu/naga` has **no** barrier-elision code at all — confirmed by grep; that mechanism only exists in the standalone `naga/` repo, commit `f08a9bf1`). Retargeted to `naga/` and built a minimal reproducer (`naga/examples/barrier_repro.rs`, still present in that working tree):

```wgsl
var<workgroup> data: array<f32, 64>;
var<workgroup> flag: u32;

@compute @workgroup_size(64)
fn main(@builtin(local_invocation_index) lid: u32) {
    var i = 0u;
    loop {
        if i < 4u { } else { break; }
        data[lid] = f32(i);
        let v = workgroupUniformLoad(&flag);
        continuing {
            i = i + 1u;
        }
    }
}
```
Here the `workgroupUniformLoad` is the **last statement in its static block** (the increment lives in a separate `continuing` block), and the hazardous write `data[lid] = f32(i)` sits at the **top** of the same loop body — meaning, in real execution, that write happens right after this WGUL's *own* barrier-protected load completes, on the very next iteration, reachable only via the loop's back-edge. The forward hazard-scan (`wgpu/naga/src/back/spv/block.rs`'s `naga_block.iter().skip(statement_index + 1)` — confirmed identical logic in the `naga/` repo) only walks forward through the *same static block*; it cannot see across the back-edge. Compiled SPIR-V confirms the bug directly:
```
OpStore %52 %51                  ; data[lid] = f32(i)   (the hazard write)
OpControlBarrier %38 %38 %39     ; pre-barrier (present, correct)
%53 = OpLoad %6 %11              ; the actual uniform load of `flag`
OpBranch %44                     ; <-- POST-BARRIER MISSING. Falls straight through to `continuing`.
```
Only 2 `OpControlBarrier` instructions appear in the whole function (one for the mandatory workgroup zero-init, one for the WGUL's pre-barrier) — the post-barrier that should immediately follow the load is **absent**. A control barrier (per the WGSL spec and Naga's own codegen, confirmed in §5 of this document last session) enforces workgroup-wide reconvergence, not just a memory fence; dropping it means some invocations can race ahead into the next iteration's write to `data` before other invocations have finished/synchronized on the uniform load — a genuine potential data race, not merely a missed optimization.

I confirmed this is a real gap in the scan's design, not user error: a control ablation (moving the hazard write to literally the next statement in the *same* block, no loop) correctly retained all 3 barriers (see the Case A / same-block control run in this session's transcript) — so the scan **does** work for the case it was designed for; it just doesn't account for loop back-edges, which is exactly the kind of gap a per-block-only, non-fixpoint scan would have.

**This bug is scoped to the `naga/` repo's Track 3, which is not part of the paper now under review.** It doesn't affect the current paper's claims (interval analysis / loop induction, both in `wgpu/naga`) at all. It's reported here because the user asked to verify all open threads, and this resolves the one dedicated agent that failed last session — it should NOT block the current paper's submission, but IS a real defect that would need fixing before any future work revives the Track 3 barrier-elision idea (e.g., a resubmission of the old paper, or a future paper).

### 10e. Final synthesis: is `iict2026_bounds_check_elision.tex` ready for IICT?

**Not yet, but it's close, and the remaining fixes are well-defined and small:**

**Must-fix before submission:**
1. **§3.1 "Interval Analysis" and Figure 2 must be rewritten or removed.** They describe a general worklist/lattice algorithm with arithmetic propagation and if-condition narrowing that was never implemented (confirmed twice, by direct code reading and by the diagram's own extracted content). What's actually there — a single hardcoded `local_invocation_index`-exact-match check — should be described plainly, likely folded into or presented alongside §3.2 rather than as a separate "interval analysis."
2. **Table 3's GPU timing numbers should be replaced with the rigorously re-measured ones from this session**: bounds-check overhead is `~7,568 ns [95% CI 7,456-7,744]`, not 2,400ns; the "5-6 GPU cycles per check" estimate should become ~69 cycles. The qualitative "identical to unchecked when elided" claim gets *stronger* support (exact 0ns delta), which is good news worth keeping/highlighting.
3. **The 12.5% instruction-reduction headline stat (Abstract + Conclusion) has no traceable derivation** from the paper's own tables and should be recomputed transparently or dropped.
4. **The subgroups-regression description should be corrected** from "a pre-existing regression" to what it actually is: a live, reachable `unreachable!()` panic on a publicly-constructible enum variant (`enable_extension.rs:202`) — a real defect independent of the elision work, but one a reviewer or CI run would immediately hit.

**Should-fix / nice-to-have:**
5. Add the newly-written `local_invocation_id` adversarial test (now in the stress suite) to the paper's own enumerated stress-test list (§5.6), since it directly demonstrates the gap "fails closed."
6. Consider a brief, honest footnote that a related barrier-elision idea was explored on a separate branch and found to have a synchronization bug in its hazard-detection scan — this is optional (out of scope for this paper's actual claims) but would preempt a sharp reviewer who finds the other branch/history.

**What's genuinely solid and should be emphasized, not hedged:**
- The loop-induction-variable analysis (§3.2) is real, accurately described, accurately diagrammed, and passes 25/25 targeted stress tests (24 original + 1 new) plus 239/240 (post-addition) of the full naga suite, with the one failure demonstrably unrelated to elision.
- SPIR-V word-count claims are exactly reproducible, twice now, independently.
- The OOB verification on real hardware is real and confirms the safety mechanism genuinely works.
- With the corrected timing data, the central engineering result — bounds-check elision achieves byte-for-byte identical SPIR-V and statistically indistinguishable-from-zero timing overhead relative to `Unchecked`, while a real, now more accurately quantified cost is avoided — is **more** defensible than the original draft's own (noisier, underestimated) numbers made it look.

**Bottom line:** this is solid, publishable workshop-tier engineering work with one real documentation-honesty problem (§3.1) and a few numbers that don't hold up to scrutiny (12.5%, the 2,400ns figure). None of these require new research to fix — they require rewriting one section, swapping in the corrected (now-available) benchmark numbers, and correcting two sentences elsewhere. With those four "must-fix" items addressed, this is a reasonable, honest, submittable IICT paper. As currently written, it is not, because §3.1 actively misrepresents what was built.

## 11. FOURTH PASS — honest audit of my own testing rigor, and a negative control

The user directly asked whether the benchmarking/testing in §10 was actually rigorous, or had loopholes. Answering that honestly rather than defensively:

### 11a. Gaps that were real, named plainly

1. **No negative control had been run.** The paired matmul benchmark (§10b) was never checked against itself — i.e., I never verified that measuring two *identical* configs through the exact same interleaved+bootstrap pipeline yields a CI that includes zero. Without that, a systematic bias in the measurement pipeline itself could masquerade as a "real" bounds-check-overhead effect. This was the most load-bearing gap and is closed below (§11b).
2. **Single machine, single GPU vendor, single run.** All GPU timing (both the original paper's and my corrected re-measurement) comes from one RTX 4060 Laptop GPU, one session, no reboot/thermal-state variation, no AMD/Intel/Apple cross-check. N=1 machine is not a replicated result.
3. **Single synthetic workload.** The corrected 7,568ns figure comes from one kernel shape (workgroup_size 256, 1024-iteration inner loop, matmul-style access pattern). Generalizing "the bounds-check overhead" from this one shape has the same overreach risk as the original paper's own number — a more rigorously *measured* number is not automatically a more *generalizable* one.
4. **No GPU clock/power-state locking** (e.g. `nvidia-smi -lgc`) during any run. Interleaving cancels linear drift but not correlated multi-round boost-state transitions, which laptop GPUs are prone to.
5. **Warm-up (20 rounds) was not empirically validated** — picked as a round number, not by watching the series actually converge to steady state.
6. **No differential/execution-based correctness testing anywhere in this project.** Every adversarial test added (including this session's `local_invocation_id_vec3_not_elided`) asserts against *disassembly text* (e.g. `contains("UMin")`), never against actual numeric output from running both the elided and non-elided SPIR-V on real buffers and diffing results. For a feature whose failure mode is silent memory-safety violation, execution-based/fuzz differential testing is the actual gold standard, and none was done this session.
7. **The confirmed barrier back-edge soundness bug (§10d) has only an ad hoc repro** (`naga/examples/barrier_repro.rs`), not a committed regression test — a real finding that isn't locked in and could silently regress back to "looks fine."
8. **No independent profiler cross-check** (e.g. Nsight Compute) to corroborate the Vulkan-timestamp-query-derived numbers against a second measurement tool.

### 11b. Negative control: closing the most important gap

Wrote `negative_control.rs` (same `VkCtx`/interleave/bootstrap machinery as `matmul_bench_paired.rs`, persisted at `bounds-check-testing-framework/matmul-paired-bench/src/negative_control.rs`), which compiles `WGSL_ELIDED` under `BoundsCheckPolicy::ReadZeroSkipWrite` **twice**, independently, asserts the two SPIR-V blobs are byte-identical (399 words, confirmed equal), then times them as if they were two different configs (config "A" = compile #1, config "B" = compile #2), through the identical interleaved-round + 2000-resample-bootstrap pipeline used for the real result, 500 rounds, rotating start order.

Result (RTX 4060 Laptop GPU):
```
timestampPeriod = 1 ns/tick, timestampValidBits = 64
A (compile#1) median=44032.0ns  stddev=534.2ns
B (compile#2) median=44032.0ns  stddev=801.5ns
A - B paired delta: median=0.0ns   95% CI [0.0, 0.0]
=> PASS: no spurious effect detected between identical configs at 95% confidence.
```

This closes gap #1: the measurement pipeline does not fabricate an effect between things that are actually identical, which is direct evidence (not just an assumption) that the 7,568ns bounds-check-overhead result in §10b reflects a real effect rather than a pipeline artifact. It also incidentally closes gap #8-adjacent concern about timer quantization: `timestampPeriod = 1ns/tick` with 64 valid bits confirms the earlier "exactly 0.0ns" elided-overhead result isn't a coarse-quantization illusion — the timer genuinely resolves to 1ns.

### 11c. Updated honest answer to "was this tested to standard?"

**No, not to the full standard a peer-reviewed systems paper would need** — gaps #2, #3, #4, #5, #6, #7, #8 above remain open. What changed this session is gap #1, the one most likely to invalidate everything else, is now empirically closed rather than assumed.

For an IICT workshop submission specifically (lower bar than a top-tier systems venue), the current state — interleaved+bootstrapped measurement, negative-control-verified pipeline, one real workload, reproducible SPIR-V word counts, a passing regression suite — clears a reasonable bar for "the numbers in this table are real and not noise," which is what §10's must-fix item #2 asked for. It does **not** clear the bar of "these results generalize" or "this feature was fuzz-tested for correctness," and the paper should not claim either of those things. If asked directly by a reviewer "was this benchmarked rigorously," the honest answer is: the specific numbers reported are statistically real (verified via negative control), but the benchmark covers one workload on one GPU, and the correctness testing is structural/textual, not execution-based — both are fair, specific limitations to disclose in a Limitations subsection rather than risk a reviewer finding them first.
