# T6 — Paper v3: §3 accuracy + related work + verified numbers only

**Objective:** close the two presentation complaints from the high-confidence
reviewers (R3 "algorithms could be presented better", R4 "sketchy"), add the missing
related-work section (R1/R2), and make every number artifact-traceable.

## Ground rules

- Ground truth for §3 is the CODE: `wgpu/naga/src/proc/bounds_check_elision.rs`,
  `valid/analyzer.rs` (read both fully before writing; the file names, function
  names, and semantics you describe must exist in them).
- Work on a copy: create `iict2026_bounds_check_elision_v3.tex` (copy of v2) — never
  edit v2 in place (it's the compiled record of the submitted lineage).
- Every number in the text must carry a comment `% SRC: <artifact path>` (LaTeX
  comments don't print; they make the provenance audit trivial).
- Banned numbers (do not reintroduce): 12.5% instruction reduction (untraceable),
  2,400 ns / 5–6 cycles (methodology-flawed; see `a.md` §10b).

## Steps

1. **§3 rewrite (algorithm accuracy):**
   - Present the analysis under its real name (interval-domain analysis with the six
     implemented features: signed/unsigned bounded intervals; overflow-rejecting
     endpoint arithmetic; path-sensitive condition refinement + conservative joins;
     local-variable mutation invalidation; monotone loop recurrences; eq-exits;
     affine index expressions — verify each against the source before listing).
   - Write pseudocode that mirrors the actual function structure (name real
     functions; if the code has `analyze_block`/refinement helpers, the pseudocode
     shows those shapes).
   - Answer R4's six questions explicitly in prose (the answer table lives in
     `IICT_CONTEXT/02-REVIEWS.md` — port it): what index expressions; not-only-
     constants (and what stock naga's constant folding already does — pre-existing,
     6 lines, cite honestly); how values are obtained; scope beyond loop IVs;
     precise meaning of "loop induction variable analysis"; recurrence status.
   - Replace the placeholder figures: re-export the mermaid diagrams from
     `IICT_TEACHING_GUIDE.md` §7.2–7.5 (mmdc or hand-draw to PDF into `figures/`);
     if export tooling is unavailable, keep the placeholder box but write real
     content inside it (the diagram's actual logic) and mark `TODO-figure` —
     do NOT describe a figure that isn't there.
2. **Related-work section (new):** from `LITERATURE_BOUNDS_CHECK_ELISION_AND_IV.md`
   + `CITATION_MANIFEST.md`:
   - ABCD (demand-driven inequality graph; our analysis as a restricted,
     path-sensitive point in that design space),
   - W09 (overflow handling; the honest "checks often don't matter" caveat),
   - IRCE (affine checks + loop splitting vs our bounded intervals),
   - Go prove/loopbce (dominator facts, parseIndVar; cite the archived commit
     `9834516` path — the moved `ssacompile/loopbce.go`),
   - GSOHC (nearest GPU-context work; positioning),
   - Readyset (the CPU counterpoint — engage explicitly: why GPU divergent-branch
     cost differs; our corpus study as the measured answer),
   - Suzuki–Ishihata lineage (one sentence, METADATA-ONLY citation).
   - Citation-status rule: entries marked METADATA-ONLY in CITATION_MANIFEST.md may
     be cited but their CONTENTS must not be characterized beyond the verified
     bibliographic claim.
3. **Numbers swap:** any timing claims must come from T2's run JSONs (if T2 SUCCESS/
   NORMAL). If T2 did not run/succeed: keep v2's frozen-number framing
   (RECORDED, CI'd, with "awaiting provenance-complete rerun" wording) — do not
   invent, and do not reuse legacy JSON. Corpus numbers: copy mechanically from
   `corpus_hit_rate/results/RESULTS.md` (74.1/81.9/1–2.5/505→134 — verify each
   against the file as you paste).
4. **Threats to Validity (new short section):** port the anti-claims from
   `IICT_CONTEXT/04-EVIDENCE-LEDGER.md` (single machine/GPU; structural-vs-
   executional testing status as of this writing; single-kernel-family timing if
   T2 extension wasn't run; runtime-sized storage residual).
5. Compile: `tectonic iict2026_bounds_check_elision_v3.tex` (or pdflatex — match
   whatever v2 used; the build log `iict2026_bounds_check_elision_v2.log` exists).

## Verification

- Zero LaTeX errors/warnings-of-substance in the build log (overfull boxes OK).
- Every `% SRC:` comment resolves to an existing file.
- A fresh reader could answer R4's six questions from §3 alone (self-check by
  listing them and pointing at the paragraph that answers each).

## Outcome branches

- **SUCCESS** — v3 compiles; provenance comments complete; reviewer-presentation
  items closed. Record PDF/log paths; T7 unlocked.
- **NORMAL** — some figure(s) remain placeholders (tooling missing) or T2 numbers
  absent (frozen framing kept): v3 is honest but partially complete; record exactly
  which items remain and why; those become T7 poster caveats.
- **BAD** — during writing you discover code contradicts something §3 wants to
  claim (the a.md pattern recurring): fix the CLAIM to match code, never the
  reverse; log the discrepancy in `01-CHRONOLOGY.md` discrepancies register; if the
  claim is load-bearing for the paper's contribution, stop and hand back to the
  reasoning agent.
- **DISASTER** — n/a (writing task).

## Deliverables

- `iict2026_bounds_check_elision_v3.tex` + compiled PDF/log, `figures/` exports,
  session-log entry with the R4-question→paragraph map.
