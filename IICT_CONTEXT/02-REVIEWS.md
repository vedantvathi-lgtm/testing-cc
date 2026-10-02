# IICT 2026 — Paper #55 Reviews (verbatim record + score grid)

> **Provenance.** Decision email from `iict2026@easychair.org`, Wed 19 Aug 2026, 20:13,
> to the author, signed "Govind (and Ramana)". The email was pasted verbatim into an
> opencode session on 2026-09-04; the full text was recovered from
> `~/.local/share/opencode/opencode.db` (part `prt_0390be64800168VE1JMolecvV1`,
> session `ses_fc7bcf533ffemnxZN57lnGB1g8`) on 2026-09-13 and is preserved here
> word-for-word (whitespace-normalized). This is the only surviving copy of the
> review text on this machine as of 2026-09-13; the original lives in the author's
> Gmail (4 of 32, "IICT-2026 notification for paper 55").

## Decision

**Accepted for poster presentation.** All four reviewers: overall score **1 (weak accept)**.
R4 alone preferred talk (score 3 = Talk). Every reviewer: Relevance 3 (Very Relevant),
Writing clarity 2 (Okay, readable), Research Track 1.

Organizer feedback (verbatim, condensed): authors "are requested to address all the
review comments to the best of their ability, in the final poster." Optional 90-second
teaser video was due **Aug 27, 11:55 p.m.** (no evidence on disk it was made — UNKNOWN).

## Score grid

| Axis | R1 | R2 | R3 | R4 |
|---|---|---|---|---|
| Overall | 1 weak accept | 1 weak accept | 1 weak accept | 1 weak accept |
| Confidence | 2 Medium | 2 Medium | **3 High** | **3 High** |
| Talk/Poster | 2 Poster | 2 Poster | 2 Poster | **3 Talk** |
| Track | Research | Research | **Industry Practice Talk** | Research |
| Novelty | **0 No** | 1 Yes | **0 No** | 1 Yes |
| Implementation/Engineering | 1 Yes | 1 Yes | 1 Yes | **0 No** |
| Experimental Evaluation | **0 No** | **0 No** | **0 No** | **0 No** |

Note: Experimental Evaluation = 0 from **all four** reviewers is the single loudest
signal in the record. Engineering = Yes from three of four; R4's 0 was explained in the
follow-up analysis as "could not assess what was built from the text" (sketchiness),
not absence of engineering.

## Review texts (condensed to substantive content; full text in email record)

### REVIEW 1 (Medium confidence, Poster)
- Accepts: production-compiler implementation using WGSL spec guarantees is valuable.
- Improves-if: larger, more diverse benchmark suite; **comparison with existing
  bounds-check-elimination techniques or related prior work**.
- "Loop induction variable analysis appears to be a work in progress. A complete
  implementation and evaluation … will further strengthen the paper."
- Novelty 0: "novelty of the bound check elimination optimization is limited."

### REVIEW 2 (Medium confidence, Poster) — most detailed
- Strengths: real deployed compiler (Naga → wgpu/Firefox), actual pipeline integration
  (`naga/src/valid/analyzer.rs`, `function.rs`, SPIR-V writer) — "a genuine artifact";
  spec-guarantee approach "clean, sound"; conservative gating "the right default";
  SPIR-V word counts and side-by-side disassembly make zero-overhead claim checkable.
- Weaknesses: thin evaluation (one minimal shader + one matmul; no real corpus; no
  hit-rate reporting); **timing methodology under-specified (sample count, variance);
  exact "0.0 ns" delta reads as within-noise rather than measured zero**; **no BCE
  related work cited** (ABCD, Würthinger — only specs + one textbook, 5 refs);
  loop analysis believed unimplemented; soundness corner cases undiscussed
  (**overridable workgroup size, syntactic-match fragility, Restrict interaction**);
  writing needs a pass.

### REVIEW 3 (High confidence, Poster / Industry-Practice-Talk)
- "Template patterns" framing; algorithms "could have been presented better";
  work-in-progress; evaluation "quite limited".
- Core critique: "As with any template driven technique, without extensive experiments,
  **it is unclear if the selected patterns cover enough optimisation opportunities**."
- Novelty 0.

### REVIEW 4 (High confidence, Talk) — the algorithm-details reviewer
- "The description of static analysis is rather **sketchy** and many important details
  have been omitted."
- Exact questions (condensed from email, wording preserved):
  1. "What kind of index expressions do you consider?"
  2. "Are you considering only constant indices? If yes, then my interest in the paper
     reduces significantly."
  3. "If you are not considering the constant indices, how are you getting the values
     of the variables?"
  4. "Are you assuming that the index expressions consist only of loop induction
     variables and no other variables can be used?"
  5. "What exactly do you mean by loop induction variable analysis?"
  6. "Are you referring to identifying recurrences for induction variables?"
- Engineering 0 (sketchiness-driven, per follow-up analysis).

## The answers, as established by code reading (recorded in the same session, 2026-08-19→09-04 era; re-checked against code commits this session)

| R4 question | Established answer | Evidence |
|---|---|---|
| Only constant indices? | **No — the opposite.** Constant-index elision is 6 lines of pre-existing upstream constant folding (`try_resolve_to_constant`); the contribution elides **runtime-varying** indices. | `proc/index.rs` (`try_resolve_to_constant` pre-existing; `safe_indices` disjunct added in `86eb81c`) |
| How are values obtained? | Trick 1: WGSL **spec guarantees** lid ∈ [0, workgroup_size product] (proof by specification, no dataflow). Trick 2 (then-state): bound N extracted **literally from the if-break exit condition**; any body execution just passed `i < N`. | `analyzer.rs` lid scan; `bounds_check_elision.rs` `extract_bound_from_comparison` |
| Only loop IVs, no other variables? | (Then) yes — exactly `Load(LocalVariable v)` matching the identified IV; lid-derived also handled. (Now, post-`8757d7d`) any variable with a provable interval at the access point: path-sensitive condition refinement, mutation invalidation, recurrences. | `8757d7d` commit + `BOUNDS_CHECK_ELISION_TODO.md` §3 |
| What does "loop induction variable analysis" mean? | (Then) honestly: **loop-exit-bound extraction**, not recurrence identification — the name oversold it. (Now) true recurrence recognition is implemented (monotone recurrences, eq-exits). | Same session analysis; `8757d7d` |
| Identifying recurrences? | (Then) No. (Now) Yes — smallest-first ladder implemented in the rewrite. | `LITERATURE_…IV.md` §6 ladder; `8757d7d` |

## Status snapshot of review items (2026-09-13) — see 03-REVIEW-RESPONSE-STATUS.md for the full matrix

Already answered by artifacts: hit-rate study (74.1%/81.9%), loop analysis actually
implemented + stress-tested, soundness rewrite, corrected timing numbers with negative
control (exists but must be re-run under schema v3). Still open: related-work section in
the paper text, §3 algorithm presentation matching code, overridable-wg_size pinning
test, differential execution + fuzzing, GPU rerun (now unblocked), poster assembly.
