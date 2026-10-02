# T7 — Poster: claims → evidence matrix (IICT Oct 2–3)

**Objective:** produce the final poster content satisfying the organizers' binding
requirement ("address all the review comments … in the final poster"), structured so
every claim sits next to its evidence.

## Layout contract (fixed)

A claims→evidence poster; one block per reviewer + a headline result block:

1. **Headline:** the honest summary — spec-guarantee + interval-domain elision in
   Naga; corpus hit-rate 74.1% overall / 81.9% compute on project shaders, 1–2.5%
   external (runtime-sized storage buffers dominate the residual); SPIR-V identity
   for elided cases; timing per T2's actual outcome (SUCCESS → numbers+CI;
   NORMAL → "not statistically resolvable on kernel X at n" stated as a finding).
2. **R1 block** ("limited novelty / more evaluation / loop analysis WIP"):
   loop analysis now fully implemented (commits `86eb81c`→`8757d7d`), stress+corpus
   evidence; evaluation breadth shown via corpus study; positioning vs ABCD/IRCE
   (one line each from T6's related work).
3. **R2 block** (thin evaluation / timing rigor / related work / corner cases):
   corpus study; T2 provenance+CI+negative control; related-work section exists;
   corner cases pinned by named tests (T4 Part A/B status as actually achieved).
4. **R3 block** (template coverage / presentation): shape-classification table
   (which patterns fire, which don't, ranked next-target = runtime-length storage);
   §3 rewrite answering "presented better".
5. **R4 block** (sketchy / the six questions): the question→answer table verbatim
   from `IICT_CONTEXT/02-REVIEWS.md`, each answer pointing at code/test/artifact.
   State loudly: **elision targets NON-CONSTANT indices** (stock naga already folds
   constants) — this is the contribution statement R4 was fishing for.

## Sources (copy, don't compose)

- All text fragments derive from `IICT_CONTEXT/03-REVIEW-RESPONSE-STATUS.md`,
  `04-EVIDENCE-LEDGER.md`, `02-REVIEWS.md`, `corpus_hit_rate/results/RESULTS.md`,
  T2 run JSONs, T3/T4/T5 reports. No new claims invented for the poster.

## Steps

1. Draft as `poster/poster-v1.md` (content complete, layout-agnostic).
2. Convert to slides/poster format (LaTeX beamerposter or plain PDF via
   whatever is installed — check `which pdflatex tectonic`; if no poster class,
   deliver `poster-v1.md` + a 16:9 PDF outline; record tooling limits honestly).
3. Self-audit: every reviewer complaint from `03-REVIEW-RESPONSE-STATUS.md` rows 1–14
   must map to a poster element; list any row NOT covered (with reason) in the
   poster's own limitations note.

## Branches

- **SUCCESS** — full matrix covered; artifacts in `poster/`.
- **NORMAL** — some rows only partially covered (e.g., T2 NORMAL outcome, T4
  structural-only): covered with explicit caveats, never silently.
- **BAD** — an evidence artifact a poster block depends on is missing/failed
  (e.g., T2 DISASTER): restructure that block to present the honest current state
  (frozen RECORDED numbers labeled as awaiting rerun); do not pad.
- **DISASTER** — n/a.

## Deliverables

- `poster/poster-v1.md` + rendered artifact, coverage audit note, session-log entry.
