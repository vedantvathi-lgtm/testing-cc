# Repositories & references map

> Provenance: git-inspected 2026-09-13. All commit hashes below verified via
> `git log` this session unless marked otherwise.

## Code repositories (all under this project root unless noted)

| Path | Branch | Tip commit | Role | Status |
|---|---|---|---|---|
| `wgpu/` | `bounds-check-elision` | `8757d7d` "feat(naga): sound interval-domain bounds-check elision rewrite" | **Primary implementation.** Commits on trunk `3fb225a9`: `86eb81c` loop-induction elision → `87c6744` `enable subgroups` → `f105ff1` stress suite → `500dd82` verify crates → `8757d7d` rewrite | Zero tracked modifications; one pre-existing untracked `.repo-map.json` (tooling artifact — leave alone) |
| `naga/` (standalone) | `test/elision-soundness-and-repro` | `9d5a6b82` (barrier repro example); `d45bcb65` (assertions); `f08a9bf1` (Track 2 + Track 3-lite, Jun 27) | Older lineage; **Track 3 back-edge bug lives here** (documented, unfixed, out of paper scope) | Zero tracked modifications; one pre-existing untracked `.repo-map.json` (leave alone) |
| `wgpu-upstream/` | trunk | `688e4761c` | Clean upstream reference — **read-only, never modify** | pristine |
| `~/Projects/naga` | `feat/elision-hardening-and-soundness-tests` | `5a24b9d` refactor + `353dad6` tests | Third checkout of naga-side work | RECORDED (not re-inspected 2026-09-13) |
| `bounds-check-testing-framework/` | own repo | `dd4f5bb` schema-v3 runner; `e35287b` standard | Benchmark harness + runs | Clean per earlier session; runs pending |
| `corpus_hit_rate/` | own repo | `6ff062b` v5 final | Corpus hit-rate harness + RESULTS.md | Clean |

### Key implementation files (wgpu repo)

- `wgpu/naga/src/proc/bounds_check_elision.rs` — the interval-domain pass (rewrite).
- `wgpu/naga/src/valid/analyzer.rs` — lid seeding, proof computation, `safe_accesses`.
- `wgpu/naga/src/valid/function.rs` — wiring (elision call site).
- `wgpu/naga/src/proc/index.rs` — `access_needs_check` disjunct consuming proofs.
- `wgpu/naga/src/back/spv/{index,writer,block}.rs` — SPIR-V consumers (elision +
  policy interplay; the `sdata_manual` name-special-case hack still to remove before
  any upstream PR — RECORDED from session analysis).
- `wgpu/naga/tests/naga/bounds_check_stress.rs` — 58 adversarial stress tests.
- `wgpu/naga/tests/naga/elision_assertions.rs` — structural SPIR-V assertions.

## Reference archive (`references/`)

Manifest + status: `references/README.md`, `references/REFERENCE_STATUS.md`
(+ 2026-09-13 addendum), hashes in `references/SHA256SUMS` / `SHA256SUMS.new`.

| ID | Artifact | Verified state |
|---|---|---|
| WGSL/WebGPU/SPIR-V/Vulkan | specs (HTML) | downloaded, HTTP 200 (Aug 27) |
| ABCD (PLDI 2000) | PDF | present |
| W09 (Würthinger 2009) | PDF | present |
| W07 (PPPJ 2007) | — | **MISSING**: TLS fail (Aug 27 + Sep 13). Use W09 only; do not cite W07 contents from memory |
| Cousot & Cousot 1977 | — | **MISSING**: no verified URL; guessed URL discarded. Bibliographic metadata only |
| GSOHC (ECOOP 2025) | HTML | present |
| wgpu#6345 | HTML | present (it is a refactoring-discussion issue — do not over-claim it as a target, per `a.md` §5) |
| Readyset / Rust Perf Book | HTML | present (Readyset = the "≈ no win on CPU" counterpoint — must be engaged in related work) |
| Go compiler | sparse git @ `9834516` | `prove.go` (ssa, 498 l), **`ssacompile/loopbce.go` (496 l; path moved)**, `ssacompile/prove.go` (2,850 l), oracles `test/prove.go` (2,967 l) + `test/loopbce.go` (509 l) |
| LLVM | sparse git @ `39e3e92e` | `ScalarEvolution.{h,cpp}`, `InductiveRangeCheckElimination.cpp`, `test/Transforms/IRCE/*.ll` |
| webgpu-samples | shallow git clone | 75 `.wgsl` files (re-clone of the lost /tmp corpus) |

## Document map (root)

| Question | Open |
|---|---|
| What happened when | `IICT_CONTEXT/01-CHRONOLOGY.md` |
| What did reviewers say | `IICT_CONTEXT/02-REVIEWS.md` (+ verbatim record) |
| What's done about it | `IICT_CONTEXT/03-REVIEW-RESPONSE-STATUS.md` |
| What may be claimed | `IICT_CONTEXT/04-EVIDENCE-LEDGER.md` |
| How experiments work | `IICT_CONTEXT/06-EXPERIMENTS-AND-METHODS.md` |
| What next | `IICT_CONTEXT/07-MASTER-PLAN.md` |
| Original paper drafts | `iict2026_bounds_check_elision_v2.tex/.pdf` (current), `iict2026_bounds_check_elision.tex` (v1), `IICT2026_Naga_BoundsCheck_PracticePaper.tex` (superseded) |
| Submission copy | `~/Downloads/IICT-2026_paper_55.pdf`, `~/MITACS-CANADA/IICT-2026_paper_55.pdf` |
| Venue doc | `IICT 2026-3-1.pdf` |
| Deep audit | `a.md` |
| Honesty infrastructure | `AUDIT_INVENTORY.md`, `TEST_BENCHMARK_AUDIT.md`, `BOUNDS_CHECK_ELISION_TODO.md` |
