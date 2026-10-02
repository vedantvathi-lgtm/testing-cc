# Citation manifest — every reference the paper may use, with verification status

> Companion to `references/` (archive + hashes) and `references/REFERENCE_STATUS.md`.
> Rule: an entry may appear in the paper only with status **VERIFIED-ARCHIVED** or
> **VERIFIED-METADATA** (bibliographic facts confirmed, artifact not locally archived —
> usable for a citation, not for quoting contents). Never cite from memory.

## Core (normative)

| ID | Entry | Status | Archive |
|---|---|---|---|
| WGSL | WebGPU Shading Language, W3C TR | VERIFIED-ARCHIVED (HTTP 200, Aug 27 2026) | `references/raw/specs/WGSL.html` |
| WEBGPU | WebGPU, W3C TR | VERIFIED-ARCHIVED | `references/raw/specs/WebGPU.html` |
| SPIRV | SPIR-V unified spec, Khronos | VERIFIED-ARCHIVED | `references/raw/specs/SPIR-V.html` |
| VULKAN | Vulkan 1.3 spec, Khronos | VERIFIED-ARCHIVED | `references/raw/specs/Vulkan.html` |

## BCE literature

| ID | Entry | Status | Archive / note |
|---|---|---|---|
| ABCD | Bodík, Gupta, Sarkar. *ABCD: Eliminating Array Bounds Checks on Demand.* PLDI 2000, pp. 321–333. | VERIFIED-ARCHIVED | `references/raw/papers/ABCD.pdf` |
| W07 | Würthinger, Wimmer, Mössenböck. *Array Bounds Check Elimination for the Java HotSpot™ Client Compiler.* PPPJ 2007. | **VERIFIED-ARCHIVED** (2026-09-20: author-hosted PDF via christianwimmer.at, HTTP 200, 9 pp, first-page text verified; full route history in REFERENCE_STATUS addendum) | `references/raw/papers/Wuerthinger07a.pdf` |
| W09 | Würthinger, Wimmer, Mössenböck. *…in the Context of Deoptimization* (journal ext.). 2009. | VERIFIED-ARCHIVED | `references/raw/papers/Wuerthinger2009.pdf` |
| IRCE | Das. *Inductive Range Check Elimination.* LLVM, 2014–15 (Phabricator D6693; r226201). | VERIFIED-ARCHIVED (source+tests; **RFC thread now also archived** 2026-09-20, HTTP 200) | `references/raw/llvm/llvm-project.git/llvm/lib/Transforms/Scalar/InductiveRangeCheckElimination.cpp` + `llvm/test/Transforms/IRCE/`; RFC: `references/raw/issues/llvm-irce-rfc-D6693.html` |
| SCEV | LLVM ScalarEvolution (AddRec `{start,+,step}`, backedge-taken counts, ranges). | VERIFIED-ARCHIVED | `references/raw/llvm/.../Analysis/ScalarEvolution.{h,cpp}` |
| COUSOT | Cousot & Cousot. *Abstract Interpretation: A Unified Lattice Model…* POPL 1977, pp. 238–252. | **VERIFIED-ARCHIVED** (2026-09-20: author-hosted PDF via di.ens.fr canonical page, HTTP 200, 16 pp; scanned — OCR extraction) | `references/raw/papers/CousotCousot-POPL-77-ACM-p238--252-1977.pdf` |
| SUZUKI | Suzuki & Ishihata. *Implementation of an array bound checker.* POPL 1977. | METADATA-ONLY (via literature map; not yet archived) | none |
| GSOHC | Basu & Vedurada. GSOHC. ECOOP 2025 (LIPIcs vol.). | VERIFIED-ARCHIVED (landing page) | `references/raw/papers/GSOHC.html` |

## Production implementations

| ID | Entry | Status | Archive |
|---|---|---|---|
| GO_PROVE | Go compiler, `prove.go` (facts/limits pass). **Path moved upstream (found 2026-09-20)**: full implementation now at `src/cmd/compile/internal/ssacompile/prove.go` (85 KB); `ssa/prove.go` retains the `Limit` struct def (line 128) + `SafeAdd` (line 147). Design note: per-value signed+unsigned Limit (interval domain) with unsatisfiable-state semantics — see research_notes/related-work-bce.md claim 4. | VERIFIED-ARCHIVED @ commit `9834516` | `references/raw/go/go.git/src/cmd/compile/internal/{ssa,ssacompile}/prove.go` |
| GO_LOOPBCE | Go compiler, `loopbce.go` (`parseIndVar`). **Path moved upstream**: now `src/cmd/compile/internal/ssacompile/loopbce.go`. Induction detection: `(Phi min nxt)` shape (loopbce.go:44–47, verified 2026-09-20). | VERIFIED-ARCHIVED @ `9834516` | same checkout |
| GO_TEST | Go BCE test oracles `test/prove.go` (2,967 l), `test/loopbce.go` (509 l) — inline-annotation regression style. | VERIFIED-ARCHIVED | `references/raw/go/go.git/test/` |
| HALIDE | Ragan-Kelley et al. PLDI 2013 (static bounds inference). | METADATA-ONLY | none |

## Context / counterpoint

| ID | Entry | Status | Archive |
|---|---|---|---|
| WGPU6345 | gfx-rs/wgpu issue #6345 — **refactoring/discussion issue**; cite as context, NOT as a request for this optimization (per `a.md` §5). | VERIFIED-ARCHIVED | `references/raw/issues/wgpu-6345.html` |
| READYSET | Readyset (2022). *How much does Rust's bounds checking actually cost?* — 651 removed checks, ≈ no measurable win (CPU). Engage explicitly. | VERIFIED-ARCHIVED | `references/raw/blogs/Readyset-bounds-checks.html` |
| RUSTPERF | Nethercote. *Rust Performance Book* — Bounds Checks chapter. | VERIFIED-ARCHIVED | `references/raw/blogs/Rust-Performance-Book-bounds-checks.html` |
| NAGA_REPO | gfx-rs/wgpu (naga) repository. | VERIFIED-ARCHIVED (shallow) | `references/raw/wgpu.git` @ `18da915` |

## Honesty notes carried forward

- ~~W07, COUSOT~~ **resolved 2026-09-20 (both VERIFIED-ARCHIVED)**. Still metadata-only:
  SUZUKI, HALIDE — fine for `\cite{}`, not for content claims.
- The dead IRCE-slides URL from paper v1 stays dead — do not resurrect it.
- Dead/missing items are listed here and in `REFERENCE_STATUS.md`; if one is later
  obtained, update both, with access date + hash (done for W07/COUSOT/IRCE-RFC 2026-09-20).
