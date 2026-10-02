# Literature Map: Bounds-Check Elision & Loop Induction Variables
### Curated for: IICT 2026 paper #55 follow-through ("Static Analysis for Bounds Check Elision in the Naga WGSL Compiler")
Scope rule: **paper title and framing stay as-is**. This map serves two goals only:
(1) answer Reviewer 4's induction-variable/recurrence questions properly,
(2) upgrade the existing static analyses toward what 40 years of practice says they should be.

---

## 0. Reading order (if you read nothing else)

| # | What | Why first |
|---|------|-----------|
| 1 | Go `loopbce.go` + `prove.go` (§4.2 below) | Closest production cousin of our pass; shows exactly how recurrences are identified |
| 2 | Würthinger et al. 2007 + 2009 journal version | The cleanest written end-to-end BCE story: algorithm → deopt semantics → overflow handling → eval |
| 3 | ABCD (PLDI 2000) §1–4 | The vocabulary reviewers expect: inequality graph, demand-driven, PRE |
| 4 | IRCE commit + RFC thread | How a range check on an *affine function* of an IV is eliminated by loop splitting |
| 5 | GSOHC (ECOOP'25) | Nearest prior GPU/WebGPU-era work; positions our novelty honestly |

---

## 1. Foundations (books & canonical chapters)

- **S. S. Muchnick, *Advanced Compiler Design and Implementation*, Morgan Kaufmann 1997 — Ch. 14 "Induction Variables"** *(cited in our paper as [5])*
  Definitions we must be able to recite: basic IV (`i = i + c` each iteration), family of IVs (linear functions of a basic IV), strength reduction, IV elimination. **R4's "recurrences" = recognizing these phi-recurrences. We currently do none of this.**
- **Aho, Lam, Sethi, Ullman, *Compilers* (Dragon Book 2e)** — Ch. 9.1: induction variables & strength reduction, worked example.
- **Allen, Cocke & Kennedy, "Reduction of Operator Strength" (1970s, reprinted in *A History of Programming Languages*/CACM collection)** — the original strength-reduction/IV formalism everything above descends from.
- **Wolfe, *Optimizing Supercompilers for Parallel Computers*** — dependence + IV context for loops.

## 2. The BCE canon (papers, chronological)

| Year | Paper | Live link | What to steal |
|------|-------|-----------|---------------|
| 1977 | Suzuki & Ishihata, array index range checking (POPL-era theorem-proving approach) | via ACM DL | First formal statement of "prove indices safe"; our soundness-theorem page should cite this lineage |
| 1992 | Asuru, "Optimization of Array Subscript Range Checks" (ACM LPLS 1(2)) | ACM DL | Cheap range-analysis variant; baseline family |
| 1998 | Xi & Pfenning, Dependent ML (eliminating array bounds via types, PLDI'98) | ACM DL | "Make safety a type/spec fact" — spiritual ancestor of our spec-guarantee trick |
| **2000** | **Bodík, Gupta, Sarkar — "ABCD: Eliminating Array Bounds Checks on Demand", PLDI 2000, pp. 321–333** | PDF: https://bernsteinbear.com/assets/img/abcd.pdf · IBM page: https://research.ibm.com/publications/abcd-eliminating-array-bounds-checks-on-demand · Slides transcript: https://slideum.com/doc/5151329/abcd--eliminating-array-bounds-checks-on-demand | **The paper reviewers mean when they say "ABCD".** eSSA + inequality graph + demand-driven shortest path; removes ~45% dynamic checks with <10 steps per check. Our `safe_indices` is a degenerate special case of its constraint propagation |
| 2007 | **Würthinger, Wimmer, Mössenböck — "Array Bounds Check Elimination for the Java HotSpot™ Client Compiler", PPPJ'07** | https://ssw.jku.at/Research/Papers/Wuerthinger07/ · DOI: https://dl.acm.org/doi/10.1145/1294325.1294343 | Conditions-on-index-expressions in SSA; fully-redundant removal + hoisting partially-redundant checks out of loops; SciMark ≈ theoretical-max speedup. Model for our eval section rewrite |
| 2009 | Same authors, journal extension *"…in the Context of Deoptimization"* | Free PDF: http://www.christianwimmer.at/Publications/Wuerthinger09a/Wuerthinger09a.pdf | **Integer-overflow handling for bound conditions** (directly answers a hole in our analysis) + deopt-based exception semantics + honest DaCapo result: *bounds checks often don't matter* |
| 2013 | Ragan-Kelley et al., "Decoupling algorithms from schedules…" (**Halide**), PLDI'13 | https://dl.acm.org/doi/10.1145/2185520.2185528 | GPU-relevant DSL that *infers buffer bounds statically* and drops checks entirely — closest architectural cousin to "know lengths up front" |
| 2015 | Sanjoy Das — **IRCE** lands in LLVM (r226201) | RFC: https://discourse.llvm.org/t/rfc-inductive-range-check-elimination-via-iteration-space-splitting/34549 · Review: https://reviews.llvm.org/D6693 · Commit+algorithm text: https://lists.llvm.org/pipermail/llvm-commits/Week-of-Mon-20150112/253005.html · Landing note: https://blog.llvm.org/2015/01/llvm-weekly-55-jan-19th-2015.html | Eliminates `0 ≤ A·I + B < Length` by **splitting the iteration space into pre/main/post loops** so the middle loop needs no check. This is the general form of our if-break pattern: ours matches `A=1,B=0` literal-only |
| 2025 | **GSOHC** (ECOOP 2025, GPU shader optimization correctness/optimizations) | https://drops.dagstuhl.de/entities/document/10.4230/LIPIcs.ECOOP.2025.21 · Artifact: https://zenodo.org/records/15302892 | Nearest recent GPU-shader static-analysis work; cite to position novelty + reuse their artifact methodology |

## 3. Production implementations — actual code to read

| System | File(s) | Why it matters to us |
|--------|---------|----------------------|
| **LLVM SCEV** | `llvm/include/llvm/Analysis/ScalarEvolution.h`, `lib/Analysis/ScalarEvolution.cpp` — https://llvm.org/docs/doxygen/classllvm_1_1ScalarEvolution.html | **The reference recurrence engine**: AddRec `{start,+,step}`, backedge-taken counts (`getBackedgeTakenCount`, exact/constant-max/symbolic-max), `getSignedRange/getUnsignedRange`. Any upgrade of our loop analysis should be specced against these APIs even if we implement a tiny subset |
| **LLVM IRCE** | `lib/Transforms/Scalar/InductiveRangeCheckElimination.cpp` (+ `test/Transforms/IRCE/*.ll`) | Loop-splitting transform; `computeSafeIterationSpace`; intersection of multiple checks. Note: it *clones* loops rather than proving globally — a design alternative worth discussing |
| **Go compiler (closest cousin)** | `src/cmd/compile/internal/ssa/prove.go` — https://github.com/golang/go/blob/master/src/cmd/compile/internal/ssa/prove.go · **`loopbce.go`** — `parseIndVar`: `ind = Phi(min, nxt)`, `nxt = ind + inc` (**this IS recurrence identification**) · test oracle: https://github.com/golang/go/blob/master/test/prove.go | Dominator-tree fact propagation + poset; induction limits `[min,max)` attached to blocks; the `test/prove.go` file is a *gold-standard regression style*: every expected elimination annotated inline (`// ERROR "Proved IsInBounds$"`). Adopt this pattern for naga tests |
| **Go eSSA proposal (2024–25, active)** | https://github.com/golang/go/issues/80146 | Modern redesign citing **ABCD directly** (Property nodes on conditions, widening+narrowing ranges). Shows where the field is heading; also admits current pass is "finnicky about constructions" — validates our fragility concerns |
| **HotSpot C2** | loop predication / RangeCheckNode, counted-loop split (pre/main/post) — described in Würthinger 2009 §server-compiler comparison | Alternative strategy: predicate checks, version loops |
| **Rust/rustc** | Bounds check emission: `compiler/rustc_mir_build/src/build/expr/as_place.rs` (lower_index_expression) — then *delegates everything to LLVM* | Confirms rustc does no loop-aware BCE itself; all via LLVM passes above. Internals thread (2026): https://internals.rust-lang.org/t/bounds-checking-optimizations/24021 — IRCE-style traps, panic-vs-trap tradeoffs |
| **Halide** | bounds inference over box algebra (see PLDI'13 + repo `src/Bounds.cpp`) | Static length reasoning à la our constant-length gate, generalized |

## 4. Talks, slides, video

- **Keith Randall, "Generating Better Machine Code with SSA"** (GopherAcademy, 2017) — https://www.youtube.com/watch?v=uTMvKVma5ms — nil/BCE sections show Go-side reasoning end-to-end.
- **LLVM Developers' Meeting 2015** — IRCE talk (Finkel/Das). ⚠️ The old `llvm.org/devmtg/2015-slides/Hal_Finkel_IRCE.pdf` URL cited by our earlier drafts is **dead**; search the LLVM YouTube channel + llvm.org devmtg archive for the live copy before citing. (Fix flagged by audit.)
- **ABCD author slides** (transcript mirror): https://slideum.com/doc/5151329/abcd--eliminating-array-bounds-checks-on-demand
- SCEV API walkthroughs: doxygen class docs (link in §3) double as reference reading.

## 5. Practitioner blogs, issues, measurement culture

- **Go**: Ardan Labs, "Bounds Check Elimination In Go" (2018) — https://www.ardanlabs.com/blog/2018/04/bounds-check-elimination-in-go.html · go101 book chapter: https://go101.org/article/bounds-check-elimination.html — user-facing patterns (`_ = b[7]` hints) + `-gcflags=-d=ssa/prove/debug` inspection flags. **Steal the debug-flag idea for naga.**
- **Rust cost reality-check**: Readyset, "How much does Rust's bounds checking actually cost?" (2022) — https://readyset.io/blog/bounds-checks — removed 651 real checks in a production hot path: **≈ no measurable win** (branch prediction + LLVM local opts). Direct counterweight to our motivation section — engage with it or reviewers who know it will.
- **Rust Performance Book**, N. Nethercote, "Bounds Checks": https://nnethercote.github.io/perf-book/bounds-checks.html — safe idioms that unlock elimination.
- Missed-elimination bug reports worth mining for test cases: rust-lang/rust#110971 (`j < len/2`), golang/go#80146 discussion.
- **WebGPU/Naga home turf**: gfx-rs/wgpu#6345 (bounds-check elision tracking — cite upstream intent), #5878 (related).

## 6. Mapping sources → required upgrades (paper stays; analysis grows)

R4 asked three things. Each maps to a concrete, sourced upgrade:

| R4's question | Upgrade | Source blueprint |
|---|---|---|
| "Only loop induction variables, no other variables?" | Accept any variable whose range is provable at the access point, starting with: lid-derived (done), IV-bound (done), **dominating-condition facts** (`if j < len { … }`) | Go `prove.go` addBranchRestrictions/addLocalFacts (dominator walk); ABCD demand-driven assertion check |
| "What exactly do you mean by loop induction variable analysis?" | Rename internally to **loop-exit bound extraction** in docs/comments; implement true IV recognition: find `Phi(init, nxt)`, `nxt = i + c`, attach `[min,max)` per block | Go `loopbce.go parseIndVar`; Muchnick ch.14 definitions; SCEV `{start,+,step}` |
| "Identifying recurrences?" | Yes-class features, smallest-first ladder:<br>① step tracking (`+= c`, any sign) → enables decrement loops<br>② derived IVs `arr[i+j]` = affine `{j,+,1}`<br>③ trip-count/back-edge bound from exit condition incl. **eq-exits** (`i == N`) via monotonicity proof<br>④ overflow discipline: unsigned wrap rules, i32 neg-start guard (fixes our live soundness hole) | IRCE `SplitRangeCheckCondition` (affine forms); Go indVar limits + downward-counting rewrites; Würthinger 2009 §overflow handling |

**Must-achieve checklist (definition of done for the analysis upgrade):**
- [ ] Signedness/init guard + negative-start regression test (soundness hole closed)
- [ ] True recurrence recognition (phi + step), replacing comparison-shape matching
- [ ] Eq-exit support via monotone-step argument
- [ ] Affine index forms `arr[i±c]`
- [ ] Dominating-condition facts (any variable, not just IVs) — smallest ABCD subset
- [ ] Inline-annotated expectation tests à la Go `test/prove.go`
- [ ] Corpus hit-rate number using the new coverage (feeds poster + next submission)
- [ ] Related-work paragraph built from §2 rows above (ABCD, Würthinger, IRCE, GSOHC minimum)

---
*Curated Aug 25 2026. Dead-link caveat: verify every URL at citation time; the previous draft cycle lost points to a 404'd slide deck.*
