# AGENTS.md — MANDATORY OPERATING PROTOCOL (READ BEFORE ANYTHING)

This repository contains compiler research (IICT 2026 paper #55: bounds-check elision
in the Naga WGSL compiler) with a documented history of **claims that outran code**.
An earlier paper draft described analyses that did not exist; a findings doc described
a pre-existing analysis that never existed. Every agent working here — human or AI,
any harness, any prompt chain — operates under the rules below. They are not style
preferences; they are the failure-prevention mechanism this project was built around.

---

## Rule 0 — No hallucinations. No assumptions. Stated plainly, unsugared.

1. **Every factual claim must have provenance**: a file path (+line/section where
   practical), a git commit hash, a measured artifact (JSON/log/PDF), or a URL with
   access date. If you cannot name where a fact lives, you must not assert it.
2. **Vocabulary is not optional.** Use exactly these evidence labels:
   - `VERIFIED` — you personally checked it this session (ran the command, read the file, saw the output).
   - `RECORDED` — it exists in a project document or artifact written earlier; cite the path. Not re-checked this session.
   - `REPORTED` — stated by a document or prior session but never independently confirmed. Treat as unproven.
   - `UNKNOWN` — nobody knows. Say so.
   - `SUPERSEDED/INVALID` — evidence exists but is disqualified (e.g., legacy benchmark runs: see `IICT_CONTEXT/06-EXPERIMENTS-AND-METHODS.md`).
3. **Never upgrade a label silently.** `REPORTED` does not become `VERIFIED` because
   it "seems right" or appears in many documents. Repetition is not verification.
4. **Missing things are recorded as missing.** If a download fails, a test errors, a
   figure is lost, or a driver is down — write that down with the error text. Do not
   substitute an unverified mirror, an invented number, or a plausible-looking URL.
   (Precedent: `references/REFERENCE_STATUS.md` records the W07 TLS failure and the
   discarded Cousot URL instead of fabricating replacements.)
5. **If something breaks mid-run, or a result differs from expectation, say so
   explicitly and plainly, in this shape:**
   > `DEVIATION: expected <X>, observed <Y>; evidence: <artifact>; consequence: <Z>; next: <action>`
   Never frame a partial, failed, or assumed result as a success. Never write "this
   confirms" unless the confirmation was actually measured. An expected-but-unobserved
   result is *not* a success and must not be written as one.
6. **Numbers are copied, never recalled.** Copy numeric values from artifacts
   mechanically (JSON field, log line, PDF text). If you find yourself writing a
   number from memory, stop and find the artifact. If the artifact doesn't exist,
   the number doesn't exist.
7. **Distinguish the three layers of this project explicitly** (history of conflation):
   - what the **code** does (read the source),
   - what the **paper/docs claim** it does (read the tex/md),
   - what the **measurements** support (read the artifacts).
   Any gap between layers is a finding, not something to paper over.

## Rule 1 — Repo safety

- Three real code repos live here: `wgpu/` (branch `bounds-check-elision`), `naga/`
  (branch `test/elision-soundness-and-repro`), `wgpu-upstream/` (clean reference,
  **read-only**). Also `~/Projects/naga` (branch `feat/elision-hardening-and-soundness-tests`).
  Map and state: `IICT_CONTEXT/05-REPOS-AND-REFERENCES.md`.
- Never discard, stash, reset, or broad-stage anything you did not create. Several
  histories contain deliberate checkpoints; `wgpu-upstream/` must stay pristine.
- Do not push. Do not commit without an explicit request.

## Rule 2 — Evidence gates (claims are gated, not free)

No performance or correctness claim may be stated as a headline result unless it has
passed the gates in `AUDIT_INVENTORY.md` (§ "Required evidence gates"): full CPU/static
suite, differential enabled/disabled codegen, provenance-complete GPU run (schema v3),
identical-binary negative control with CI containing zero, and tables regenerated only
from those artifacts. Until then, legacy numbers are `SUPERSEDED/INVALID` for headline
use (see `IICT_CONTEXT/06-EXPERIMENTS-AND-METHODS.md` for exactly why).

## Rule 3 — Plan in branches, not straight lines

Every experiment and task must be written with explicit expected outcomes for at
least: **success**, **normal/partial**, **bad**, and **disaster**, each with its next
action — before running it. The master plan (`IICT_CONTEXT/07-MASTER-PLAN.md`) is
built this way; follow that shape for any new work. After running, record which
branch actually happened, with evidence. A result that lands on an unlisted branch is
a `DEVIATION` (Rule 0.5) and must be written up, not absorbed.

## Rule 4 — Session continuity

- Working notes, chronology, review records, and plan state live in `IICT_CONTEXT/`.
  Start from `IICT_CONTEXT/00-INDEX.md`. When you finish a session, append what you
  did, what you verified, what deviated, and what is next — dated, with artifacts.
- Update `BOUNDS_CHECK_ELISION_TODO.md` checkboxes only to reflect verified reality
  (this file itself was corrected on 2026-09-13 after unchecked-but-done items were
  found; see the note at its top).

## Rule 5 — When documents disagree

Documents in this repo disagree with each other and with the code in places (this is
documented). Precedence: **(1) running code > (2) measured artifacts > (3) committed
docs > (4) older docs > (5) AI-session transcripts**. On conflict, do not average or
hedge — investigate, then record which source won and why, in the relevant context file.
