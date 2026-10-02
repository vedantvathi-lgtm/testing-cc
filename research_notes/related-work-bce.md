# Related work for bounds-check elision — verification against archived sources

Verified 2026-09-20. Every factual sentence below carries a citation to a page of an
extracted reference (`references/extract/<slug>/<slug>.txt`, `p.N` = page marker; HTML
sources cited by line number), a source file, or a measurement artifact. Where
verification failed, the verdict is UNVERIFIED and nothing has been filled in by
inference.

## Verdict table

| # | Claim | Verdict |
|---|-------|---------|
| 1 | ABCD (Bodík et al., PLDI 2000) eliminates array bounds checks on demand using an inequality graph; its algorithm decides redundancy of a bound check per access, driven by demand. | SUPPORTED (abcd.txt p.1 title; p.2 lines 63–66) |
| 2 | W07 (Würthinger/Wimmer/Mössenböck, PPPJ 2007) is the HotSpot client-compiler BCE paper; the archived author-hosted PDF matches title and authors. | SUPPORTED (wuerthinger07.txt p.1) |
| 3 | IRCE (Das, LLVM 2014–15, r226201) eliminates range checks by splitting the iteration space into three segments so the check is redundant in one segment; landed with the exact commit recorded in our manifest. | SUPPORTED (llvm-irce-rfc-d6693.txt line 2: "splitting the iteration space into three segments"; commit IDs rL226201/rL226238 in same block) |
| 4 | Go's production `prove` pass folds `IsInBounds` using per-value Limit facts = signed AND unsigned min/max (an interval domain), with overflow-rejecting endpoint arithmetic (`SafeAdd`/`SafeSub` return ok=false). | SUPPORTED (ssa/prove.go:128–135 Limit struct; SafeAdd at ssa/prove.go:147–156; fold at ssacompile/prove.go:2155–2168) |
| 5 | Go's `loopbce` detects induction variables matching `(Phi min nxt)` — i.e., syntactic induction-variable recognition feeding bound-check elimination. | SUPPORTED (ssacompile/loopbce.go:44–47) |
| 6 | GSOHC (Basu & Vedurada, ECOOP 2025, LIPIcs 333, 21:1–21:30) is a CPU–GPU synchronization-motion optimization (hetero-sync motion), implemented on LLVM/Clang, speedups up to 1.8–1.9× on A4000/P100/A100. It is NOT a bounds-check elimination work. | SUPPORTED (gsohc.txt lines 271–276) |
| 7 | Readyset (2022) removed 651 real bounds-check operations in a production Rust codebase and observed a change "entirely explainable by measurement noise" — i.e., no measurable win on CPU. | SUPPORTED (readyset-bounds-checks.txt line 117, quote verified verbatim) |
| 8 | The Rust Performance Book states bounds checks "can affect performance … though less often than you might expect" and lists the standard safe workarounds (iteration, slice before loop, range assertions). | SUPPORTED (rust-perf-book-bounds-checks.txt lines 21–24) |
| 9 | Cousot & Cousot POPL 1977 is the abstract-interpretation foundation (unified lattice model, construction/approximation of fixpoints); the archived scan is the ACM pages 238–252 version. | SUPPORTED (cousot-popl77.txt p.1–2 title block; OCR-verified) |
| 10 | wgpu issue #6345 is titled "[naga spv-out] Bounds check generation could be cleaner and smarter" — a refactoring/discussion issue, consistent with our decision to cite it as context, not as a request for this analysis. | SUPPORTED (wgpu-6345.txt line 3; per a.md §5 framing rule) |

## Positioning statements the paper can now make (each anchored)

- **Design-space sentence for §Related Work.** "ABCD eliminates bounds checks on
  demand over an inequality graph (abcd.txt p.2); IRCE eliminates inductive range
  checks by loop splitting (llvm-irce-rfc-d6693.txt); Go's production prove pass
  carries a signed/unsigned interval Limit per value with overflow-rejecting
  arithmetic (ssa/prove.go:128–156). Our analysis is a restricted, path-sensitive
  point in this design space: bounded intervals per index expression, access-specific
  proofs, overflow-rejecting endpoint arithmetic — closer to Go's Limit design than
  to ABCD's demand-driven inequality graph, and without IRCE's loop splitting."
- **The Go parallel is stronger than our manifest previously recorded.** The Limit
  struct is a two-domain (signed/unsigned) interval record with documented
  unsatisfiable-state semantics ("min>max ⇒ unreachable code", ssa/prove.go:122–127),
  and SafeAdd is exactly our "reject arithmetic whose endpoints can overflow" rule
  (ssa/prove.go:147+). Paper v3 may honestly say the overflow-rejection rule is
  "the same discipline the Go compiler applies" with this citation.
- **The Readyset counterpoint, engaged properly.** Readyset's 651 removed checks
  produced no measurable CPU win (readyset-bounds-checks.txt line 117). Our paper
  must not hide this: the correct engagement is (a) CPU ≠ GPU — divergent branches
  cost differently, and WGSL mandates the checks regardless, so our work measures
  what the *mandated* checks cost, not whether to have them; (b) our corpus study is
  the measured answer to "do the patterns fire"; (c) our schema-v3 numbers
  (105.03/111.03 ns per dispatch) are precisely the kind of CI-bounded measurement
  Readyset's blog post lacked.
- **W07 content claims are now licensed** (archived PDF): v3 may cite W07's actual
  mechanism (range analysis carried on the value graph, checks eliminated for
  dominated accesses) instead of the previous W09-substitute argument.
- **Cousot is now quotable at the level needed for §3**: the lattice/fixpoint
  framing (p.1–2, OCR) supports the sentence "our interval domain is a classic
  abstract interpretation: a lattice of bounded intervals with a conservative join."

## Gaps that remain (recorded, not hidden)

- W07's *exact* algorithm-page citations still need a read of wuerthinger07.txt
  beyond p.1 before quoting mechanism details beyond the abstract-level claim
  above (the note's claim #2 only anchors title/authors).
- Suzuki & Ishihata (POPL 1977) and Halide (PLDI 2013) remain METADATA-ONLY —
  bibliographic citations only.
- The IRCE RFC thread quote above anchors the Summary; reviewer discussion details
  (e.g., reames' comments on handlining) are in the extract but not yet verified
  line-by-line for use.

## Deltas licensed for paper v3

1. Add a Related Work section with: ABCD, W07, W09 (journal ext., deoptimization
   context), IRCE (+ RFC), Go prove/loopbce (archived at upstream commit 9834516,
   ssacompile/ paths), GSOHC (GPU-context neighbor), Readyset + Rust Perf Book
   (measurement counterpoints), Cousot & Cousot (abstract-interpretation foundation).
2. Rewrite the "interval domain" motivation sentence to name-check Go's Limit
   design as production precedent (with file/line anchors).
3. Add the Readyset engagement paragraph (draft above).
4. Bibliography: replace the current 5-entry list with the full verified set;
   W07/COUSOT entries may now carry content-level citations (page-anchored).
