# Master Plan — gated steps with explicit outcome branches

> How to use this file: execute steps in dependency order. Each step lists Gates
> (must be true before starting), Procedure, and four outcome branches — **SUCCESS /
> NORMAL (partial) / BAD / DISASTER** — with the exact next action for each. After
> running, append the outcome to the step's log line with artifact paths. If reality
> lands on NO listed branch, that is a `DEVIATION`: stop, record it (AGENTS.md Rule
> 0.5), and re-plan. Do not silently proceed on an unlisted outcome.
>
> Restart-safety invariant for every step: it must be safe to abort at any point —
> work goes to new files/branches, nothing destructive runs without a checkpoint,
> and each step ends by writing its artifact so the next run can resume from disk.

---

## STEP 1 — Certify the current tree (cheap, do first, gates everything)

**Gates:** repos clean (verified 2026-09-13). **Procedure:**
`cd wgpu && cargo test -p naga` (full suite + stress + assertions), `cargo clippy`,
`cargo fmt --check`. Same for standalone `naga/` if touched later.
- **SUCCESS** (272+58+3 green, clean lint/fmt): record output to
  `IICT_CONTEXT/logs/step1-<date>.log`; mark P4-partial satisfied; proceed to Step 2.
- **NORMAL** (flaky test(s) but deterministic on retry): log which, retry once,
  record both runs; proceed with a note; file the flake in this step's log.
- **BAD** (genuine failures in elision code): `DEVIATION` if not explainable →
  fix-forward on a new branch `fix/step1-<desc>`; do NOT touch `8757d7d` history;
  re-run until green; only then proceed.
- **DISASTER** (build broken at `8757d7d`, i.e., a committed state doesn't build):
  record full error; this invalidates "272 green" as a claim (update
  `04-EVIDENCE-LEDGER.md`); bisect within the 5 wgpu commits; do not attempt GPU
  work until fixed.

## STEP 2 — E1: schema-v3 GPU rerun + negative control (P1 — the big unblock)

**Gates:** Step 1 SUCCESS; `nvidia-smi` functional (verified 2026-09-13, driver
580.178.04); `bounds-check-testing-framework/paired-vulkan-v3` builds.
**Procedure:** `cargo run --release` in `paired-vulkan-v3` → writes
`runs/<stamp>/` with `meta.json` (revisions, driver, GPU, warmup, CIs). Run matmul
+ ≥2 more kernel shapes (reduction, prefix_scan — sizes per
`FIXED_BENCHMARKING_SUITE.md` §7). Then the identical-binary negative control in the
same harness revision, persisted beside the run (audit finding: it previously lived
in a separate binary — keep them together now).
- **SUCCESS** (control CI ⊇ 0; enabled/disabled delta measured with CIs; provenance
  complete): write numbers into `04-EVIDENCE-LEDGER.md` as VERIFIED (superseding the
  frozen `a.md` numbers); regenerate paper Table 3 from these artifacts only;
  update TODO §5. Headline timing claim becomes citable.
- **NORMAL** (control passes but some kernel shapes show CI containing 0 for the
  check cost — i.e., effect unresolvable on that shape): that is a *publishable
  honest result* ("bounds-check overhead not statistically resolvable on kernel X"),
  report it as such; do not pool shapes to manufacture significance; proceed with
  the shapes that resolved + the negative findings stated plainly.
- **BAD** (control FAILS — CI excludes 0 between identical binaries): measurement
  pipeline is broken; every number from this session is INVALID; do not proceed to
  paper updates. Diagnose (clock state, resolve protocol, timestamp period),
  fix harness on a branch, re-run. Record the failure in this file.
- **DISASTER** (Vulkan adapter init fails again / driver dies mid-run — this exact
  failure mode happened Aug 27→Sep 13): record `nvidia-smi` state + error text,
  mark Step 2 BLOCKED with date, fall back to CPU-side steps (3–5) which need no GPU;
  do not fabricate or reuse legacy numbers to fill the gap.

## STEP 3 — E2: differential execution testing (the biggest testing gap)

**Gates:** Step 1 SUCCESS; GPU available (Step 2 SUCCESS or NORMAL).
**Procedure:** new crate (e.g., `wgpu/naga/examples/diff_exec.rs` or a
`bounds-check-testing-framework/diff-exec/`): compile each WGSL fixture under the
same policy with elision enabled vs disabled; run both on device buffers with
known inputs; assert byte-identical outputs. Fixtures: the 58 stress shaders +
boundary-value specials (index = len-1, len, len+1, u32 wrap candidates,
negative-start i32 where analysis must retain the check).
- **SUCCESS** (all fixtures byte-identical): record per-fixture table artifact;
  TODO §4 "differential execution" → done; strongest correctness sentence in the
  paper becomes executable ("verified by execution on N fixtures on hardware X").
- **NORMAL** (all identical except fixtures whose *disabled* arm is genuinely OOB —
  e.g., intentional-OOB negative tests where policies legitimately differ):
  expected if fixtures include those; separate them into "policy-semantics
  fixtures" (differences REQUIRED) vs "elision-soundness fixtures" (equality
  REQUIRED); record both tables; document the distinction.
- **BAD** (a soundness fixture diverges): **this is a real bug in the analysis** —
  stop everything; minimize the WGSL repro; write regression test; fix on branch;
  the paper claim "preserving full safety" is on hold until fixed; update ledger.
- **DISASTER** (systematic divergence — i.e., elision wrong on a whole fixture
  class, implying the rewrite's soundness argument is broken): treat as the
  project's top-severity event; full write-up in this file; re-derive the
  soundness argument; all elision claims downgraded in ledger until root-caused.

## STEP 4 — E3 + E4: corner-case pinning + cross-backend behavioral checks (P4)

**Gates:** Step 1 SUCCESS.
**Procedure:** (a) regression test: entry point with explicit `@workgroup_size`
override path (overridable size / nonstandard dims incl. 1×1×64 and 0-product edge)
must never elide against a bogus product. (b) MSL + HLSL: compile the stress
fixtures to both backends, assert access-specific proof consumption (structural:
no UMin-clamp on proven handles; behavioral where a Metal/D3D device is available,
else structural-only and say so).
- **SUCCESS**: TODO §4 "Verify SPIR-V, MSL, HLSL separately" → done; R2's corner
  cases all have named tests; record artifact paths.
- **NORMAL** (MSL/HLSL structural verification passes; no Metal/D3D device on this
  Linux box for behavioral): record as structural-only with the limitation stated
  — this matches audit's repair standard only partially; keep the item half-open.
- **BAD** (a backend elides where SPIR-V does not, or vice versa): backend-specific
  bug; fix on branch; ledger note that cross-backend inheritance is now *tested*,
  not just claimed.
- **DISASTER**: not applicable (pure test additions; worst case is BAD).

## STEP 5 — E5 + E7: fuzz + Go-style expectation tests (hardening)

**Gates:** Step 1 SUCCESS (can run parallel to 2–4).
**Procedure:** (a) property-test crate over the interval domain: arbitrary
intervals/ops — assert no overflow is ever accepted, joins are conservative
(≥ meet of runtime sets), refinement never widens. (b) port the Go
`test/prove.go` annotation pattern: inline `// ERROR "elided"` / `// OK retained`
comments in WGSL fixtures with a small checker (Python) that runs the compiler and
greps the disassembly — makes every future analysis change self-verifying.
- **SUCCESS**: fuzz findings (if any) triaged into fixes or documented
  conservative-behavior notes; expectation suite green; TODO §4 "Fuzz interval
  operations" → done.
- **NORMAL**: fuzz clean but harness only exercises subset of ops — record coverage
  honestly; expand later.
- **BAD** (fuzz finds a real unsoundness): same handling as Step 3 BAD.
- **DISASTER**: not applicable.

## STEP 6 — Paper v2 → v3: §3 pseudocode + related work (P2 + P3; no GPU needed)

**Gates:** Step 1 SUCCESS. For §3: read `bounds_check_elision.rs` as ground truth.
**Procedure:** (a) rewrite §3 pseudocode + figure to match the implementation
exactly (name it what the code names it: interval-domain analysis; show the six
features with the actual function shapes; kill the lost-figure placeholders by
re-exporting the teaching-guide mermaid diagrams). (b) write the related-work
section from `LITERATURE_BOUNDS_CHECK_ELISION_AND_IV.md` + archived papers: ABCD
(demand-driven, inequality graph), W09 (overflow handling + "checks often don't
matter" honesty), IRCE (affine + loop splitting), Go prove/loopbce (dominator
facts, parseIndVar), GSOHC (positioning), Readyset (the CPU counterweight — engage,
don't hide), Suzuki–Ishihata lineage. (c) replace all numbers with Step-2 outputs
(nothing from legacy JSON). (d) add Threats to Validity from
`04-EVIDENCE-LEDGER.md` anti-claims verbatim in spirit.
- **SUCCESS**: v3 compiles clean; every number traces to an artifact path; every
  R3/R4 presentation complaint has a visible fix; compile artifact recorded.
- **NORMAL**: writing done but one reference still unverified (e.g., W07) — cite
  the verified 2009 version and note it; or a number must stay frozen because
  Step 2 is BLOCKED — keep v2 wording and mark placeholder; do not invent.
- **BAD**: discover during writing that code contradicts an already-written claim
  (the a.md situation recurring): fix the claim, not the code description; log in
  `01-CHRONOLOGY.md` discrepancies.
- **DISASTER**: not applicable (writing step; worst case is NORMAL/BAD).

## STEP 7 — Poster + optional video (P5; deadline Oct 2–3 workshop)

**Gates:** Steps 1–6 as complete as possible; GPU results whatever they honestly are.
**Procedure:** assemble poster as claims → evidence matrix over
`03-REVIEW-RESPONSE-STATUS.md` (one block per reviewer, verbatim complaint → our
artifact/number). Loudly state the R4 answer: **non-constant** index elision is the
contribution. Include hit-rate pie + `OpULessThan` 505→134. If Step 2 landed
NORMAL, present the unresolvable-shape result honestly.
- **SUCCESS**: poster PDF/draft artifact in repo (`poster/`), each reviewer point
  mapped; video optional — only if time permits (deadline passed Aug 27; if the
  organizer still accepts late teasers, UNKNOWN — do not assume).
- **NORMAL / BAD**: incomplete coverage of some reviewer point — state which remain
  and why; do not imply full compliance in the poster.
- **DISASTER**: not applicable.

## STEP 8 — Hygiene (follow-up paper material; no deadline)

- Fix standalone `naga/` Track 3 back-edge bug (turn `barrier_repro.rs` into a
  regression test; decide the correct fix: block-local scan must consider back-edges
  or post-barriers become unconditional in loops).
- Remove the `sdata_manual` name special-case in `spv/index.rs` (upstream blocker).
- Consolidate the three naga-checkout implementations or explicitly deprecate two.
- Consider upstream PR packaging (branch hygiene, commit messages, CHANGELOG).

---

## Standing decision rules

1. If any step's BAD/DISASTER branch fires, priorities re-order to: root-cause →
   regression test → fix → re-run gates. Paper/poster work pauses on affected claims.
2. Numbers rule (permanent): no number enters any document without an artifact path
   next to it. Numbers from legacy JSON stay banned regardless of convenience.
3. If the environment changes (driver, wgpu version, corpus contents), re-run the
   cheapest gate (Step 1) before trusting anything above it.
4. Anything discovered mid-step that contradicts `04-EVIDENCE-LEDGER.md` updates the
   ledger first, then proceeds. The ledger is the single source of claim-truth.
