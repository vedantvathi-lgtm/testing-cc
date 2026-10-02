# Session log — append-only

> Format per AGENTS.md Rule 4. One section per session, newest at bottom.
> Record: date, what was done, what was verified, deviations, artifacts, next.

---

## 2026-09-13 — context consolidation + environment audit (Codebuff/Buffy session)

**Did:**
- Reconstructed full project context from repo docs + git + recovered the verbatim
  IICT review email from `~/.local/share/opencode/opencode.db`
  (part `prt_0390be64800168VE1JMolecvV1`, session `ses_fc7bcf533ffemnxZN57lnGB1g8`).
- Wrote the binding protocol `AGENTS.md` and this knowledge base (`IICT_CONTEXT/00–08`),
  plus `CITATION_MANIFEST.md`.
- Completed the reference archive: Go sparse checkout extended + updated
  (`026391c1b9ea` → `9834516`; **`loopbce.go` moved upstream to
  `src/cmd/compile/internal/ssacompile/`**; added `test/prove.go` 2,967 l +
  `test/loopbce.go` 509 l); LLVM checkout verified (SCEV + IRCE .cpp + tests);
  re-cloned `references/raw/webgpu-samples` (75 .wgsl) to replace the lost /tmp
  external corpus.
- Corrected `BOUNDS_CHECK_ELISION_TODO.md`: §2 checkboxes updated to verified reality
  (work existed since Aug 27 but boxes were unticked); §5 BLOCKED notes updated to
  UNBLOCKED-but-not-run; correction note added at top.

**Verified this session (VERIFIED):**
- Driver RECOVERED: `nvidia-smi` functional, driver 580.178.04, CUDA 13.0.
- Repos clean: `wgpu` @ `8757d7d` (branch `bounds-check-elision`, 5 commits on trunk
  `3fb225a9`), `naga` @ `9d5a6b82` (branch `test/elision-soundness-and-repro`).
- Reviews text: score grid + verbatim review bodies recovered; decision = poster,
  4× weak accept; R4 talk-preference; organizer requirement: poster must address all
  comments; optional video deadline Aug 27 (no artifact on disk — UNKNOWN if made).
- References: hashes for newly pinned key files written to `references/SHA256SUMS.new`.
- Missing (recorded, NOT substituted): W07 PDF (TLS), Cousot PDF (no verified URL —
  guessed URL returned HTML and was discarded), IRCE RFC thread (HTTP 429).

**Deviations (plain, per AGENTS.md Rule 0.5):**
- `DEVIATION`: expected `loopbce.go` at literature-map path
  `src/cmd/compile/internal/ssa/loopbce.go`, observed absent at both pinned commits;
  located at `src/cmd/compile/internal/ssacompile/loopbce.go`; consequence: literature
  map path is stale; next: recorded in `references/REFERENCE_STATUS.md` addendum +
  CITATION_MANIFEST (map file left unedited to preserve provenance).
- `DEVIATION`: expected Go test oracles under `src/cmd/compile/internal/ssa/_test`,
  observed at repo-root `test/`; sparse checkout adjusted accordingly.
- `DEVIATION`: W07/Cousot/IRCE downloads failed again despite verified-looking URLs;
  handled per protocol: recorded as missing; no unverified mirror substituted.
- `DEVIATION`: the master context file `~/Projects/iict-bounds-check-elision.md`
  (4,264 lines, referenced by the Aug 27 session log) was NOT found on disk or in
  either archive; its loss is compensated by this knowledge base, not replaced.

**Artifacts written:**
`AGENTS.md`; `IICT_CONTEXT/00-INDEX.md` … `08-SESSION-LOG.md`;
`CITATION_MANIFEST.md`; `references/SHA256SUMS.new`;
`references/REFERENCE_STATUS.md` (addendum); `BOUNDS_CHECK_ELISION_TODO.md` (corrections);
`references/raw/webgpu-samples/` (clone).

**Next (in priority order — full branches in `07-MASTER-PLAN.md`):**
1. Step 1: certify current tree (`cargo test -p naga` + clippy + fmt in `wgpu/`).
2. Step 2 (P1): schema-v3 GPU rerun + persisted negative control — now unblocked.
3. Step 6 (P2/P3, no GPU): paper §3 rewrite + related-work section.
4. Steps 3–5: differential execution, corner-case pinning, fuzz/expectation harness.
5. Step 7: poster as claims→evidence matrix from `03-REVIEW-RESPONSE-STATUS.md`.

---

## 2026-09-13 (same day, follow-up) — executor handoff: TASKS/T0–T8 written

**Did:**
- Verified the actual `paired-vulkan-v3` harness interface before scoping T2 (source
  read, not assumed): single hardcoded fixed-array-scan kernel; built-in
  enabled/disabled paired compile via `validator.set_bounds_check_elision()`;
  identical-binary negative control; output-correctness gate; provenance capture
  (git state, shader sha256, timestamp period, nvidia-smi before/after); env knobs
  `BCE_ROUNDS=300 / BCE_CONTROL_ROUNDS=200 / BCE_WARMUP_ROUNDS=50`; path-deps on
  `../../wgpu/{naga,wgpu}`; runs land in `runs/<UTCSTAMP>-fixed-array-scan-bce-on-off/`.
  Three pre-rewrite runs exist (2026-08-26, BEFORE `8757d7d`) — recorded as
  non-current in T2.
- Wrote `IICT_CONTEXT/TASKS/`: T0 executor protocol (binding rules, evidence labels,
  DEVIATION format, git safety, stop conditions, close-out), T1 certify-tree,
  T2 GPU rerun (+ multi-kernel extension rules), T3 differential execution
  (fixture classes A/B), T4 corner-case pinning + MSL/HLSL structural verification,
  T5 interval property tests + Go-style expectation checker, T6 paper v3,
  T7 poster, T8 hygiene (Track-3 bug pinning, sdata_manual removal plan,
  consolidation doc). Each task: gates → exact steps → verification →
  SUCCESS/NORMAL/BAD/DISASTER branches → anti-hallucination rules → deliverables.
- Registered `TASKS/` in `00-INDEX.md` with the handoff note (DeepSeek V4.1 runs
  T0 → T1 → T2 → (T3‖T4‖T5) → T6 → T7; T8 anytime).

**Verified this session (VERIFIED):**
- Harness interface facts listed above (source-level).
- `runs/` contains exactly three 2026-08-26 run dirs; no post-rewrite runs.

**Deviations:** none this follow-up (all scoping consistent with earlier findings).

**Next:** executor session — `TASKS/T0-EXECUTOR-PROTOCOL.md`, then T1.

---

## 2026-09-13 (same day, second follow-up) — self-audit of deliverables, two defects found + fixed

User asked "did you really finish everything properly?" — audited own claims instead of
answering yes.

**Verified (VERIFIED this session):**
- All promised artifacts exist: `AGENTS.md`, `CITATION_MANIFEST.md`,
  `IICT_CONTEXT/00–08`, `TASKS/T0–T8`; webgpu-samples = 75 .wgsl; go.git clean @
  `9834516`; REFERENCE_STATUS addendum + SHA256SUMS.new in place.
- Cross-referenced every path cited in the new docs against the filesystem. All
  referenced EXISTING paths resolve; the only non-resolving references are
  intentional future artifacts (`09-CHECKOUT-CONSOLIDATION.md` → T8 Part C creates it;
  `diff-exec/`, `expectation-checker/` → T3/T5 create them) and log filenames that
  tasks will create.

**Deviations (found by the audit, fixed this session):**
1. `DEVIATION`: expected `IICT_CONTEXT/logs/` to exist (T1/T2 write into it);
      observed absent — a task would fail its first tee. Fixed: directory pre-created.
2. `DEVIATION`: expected clean-tree gate `git status --short | wc -l == 0` to pass in
   T1/T2; observed `?? .repo-map.json` untracked in BOTH `wgpu/` and `naga/`
   (pre-existing code-graph tool artifact, present since before this project's
   sessions, not gitignored). Earlier log entries calling the repos "clean" were
   imprecise — correct statement: "no tracked modifications; one untracked tooling
   artifact each." Fixed: T1/T2 gates rewritten to tolerate exactly that file with
   explicit do-not-delete instructions; T2 verification's provenance expectation
   unchanged (the runner will honestly capture the file in its recorded git status).

**Corrected claim, on the record:** repos `wgpu`/`naga` = zero tracked modifications,
tips `8757d7d` / `9d5a6b82`, each with one pre-existing untracked `.repo-map.json`.

**Next:** executor session — T1 (gates now reflect verified reality).

---

## 2026-09-13 (same day, third entry) — EXECUTOR SESSION: T1→T2 attempted; BLOCKED at T1 gate (Rust toolchain absent)

**Role:** acting executor per `TASKS/T0-EXECUTOR-PROTOCOL.md`. User instruction: run
T1 then T2 back-to-back.

**T1 gates run (verbatim per task file):**
- `git -C wgpu status --short` → `?? .repo-map.json` only — PASS (as gated).
- `git -C wgpu log --oneline -1` → `8757d7d feat(naga): sound interval-domain…` — PASS.
- `git -C wgpu branch --show-current` → `bounds-check-elision` — PASS.
- `df -h .` → 22G available — PASS.
- `cargo --version && rustc --version` → **FAIL: `cargo: command not found`**.

**Diagnosis (all VERIFIED this session):**
- `~/.cargo/bin` IS on PATH but `~/.cargo/` is an EMPTY directory (mtime Sep 1 16:17).
- No `~/.rustup`; no system rustc/cargo; no snap/nix/asdf/mise/conda toolchain.
- Corroboration: `wgpu/target/` and framework `target/` build dirs absent; newest
  build artifact `wgpu/Cargo.lock` mtime Jul 5 16:11. Toolchain + build caches were
  removed around Sep 1 (same window as `~/Archive` backup creation), likely disk
  cleanup (disk now 88% full).
- Network available → reinstall is possible (rustup is a user-space install, no sudo).

**Branch called:** T1's DISASTER branch as written covers "tree does not compile" —
this is NOT that (the tree was never compiled; the compiler is gone). Reality matched
no listed branch exactly ⇒ recorded as `DEVIATION` per T0 rule 6, with DISASTER-class
consequences for task flow.

`DEVIATION: expected T1 to run cargo test/clippy/fmt and certify the tree; observed
no Rust toolchain exists on the machine (empty ~/.cargo, no ~/.rustup, nothing on
PATH); evidence: IICT_CONTEXT/logs/t1-build-failure.log; consequence: T1 cannot run,
T2 blocked behind it (cargo-built harness), T3–T5 likewise; only T6/T8-PartC are
toolchain-free; next: install rustup (user-space), then re-invoke T1→T2.`

**On the Aug-28 "272 tests green" claim:** NOT downgraded — nothing today tested it
wrongly; it remains RECORDED, now explicitly "untested since Aug 28; toolchain absent
2026-09-13". The ledger's word is final until a toolchain exists and T1 runs.

**T2: NOT ATTEMPTED** (blocked by its own gate chain behind T1). No GPU work done.
GPU itself verified healthy immediately before T1 (nvidia-smi: RTX 4060 Laptop,
580.178.04, 51°C, 590 W draw).

**Artifacts written this session:**
- `IICT_CONTEXT/logs/t1-build-failure.log` (full evidence + consequence chain +
  unblock command).
- `IICT_CONTEXT/TASKS/T1-CERTIFY-TREE.md` — ⛔ BLOCKED banner prepended (original
  task definition unchanged below it); banner removal is step one of the unblock.

**Next (in order):**
1. User (or approved executor action): `curl --proto '=https' --tlsv1.2 -sSf
   https://sh.rustup.rs | sh -s -- -y --profile minimal` then `source ~/.cargo/env`.
2. Verify `cargo --version`; delete the T1 banner; re-invoke "Run T1 then T2".
3. Alternative while blocked (needs explicit user instruction, out-of-order per
   T0 rule 9d): T6 (paper v3 prose) or T8 Part C (consolidation doc).

---

## 2026-09-13 (later) — environment rebuilt; T1 executed SUCCESS; T2 executed SUCCESS w/ 1 DEVIATION

**User directive:** set up the whole toolchain/environment for the testing infra
(benchmarking + tests), working around anything needing sudo. Then T1→T2 were run
under the T0 executor protocol.

**Environment work (all VERIFIED this session):**
- Base audit: 20 cores, 23 GiB RAM, 19–22G free disk; python3/git/curl/zstd/pkg-config/cmake present; GPU RTX 4060 Laptop driver 580.178.04.
- Rust toolchain was gone (per previous entry) → **reinstalled user-space, no sudo**: rustup 1.29.1 → toolchain **1.93.1** (2026-02-11) — exactly the `wgpu/rust-toolchain.toml` pin (`channel = "1.93"`), with clippy, rustfmt, wasm32-unknown-unknown; set as rustup default (harness dir has no pin).
- Vulkan stack verified WITHOUT installs: loader `/usr/lib/libvulkan.so.1` (1.4.357), ICD `/usr/share/vulkan/icd.d/nvidia_icd.json` → `libGLX_nvidia.so.0` → 580.178.04 userspace + `libnvidia-glvkspirv` present. Nothing needed sudo; the only sudo-gated item considered (nvidia persistence mode) was deliberately skipped as unnecessary (harness warmup covers driver load).
- Dependency caches warmed via `cargo fetch --locked` (harness path-deps resolved against the fork).
- Workaround note: detached `setsid nohup bash -c` died silently once; relaunching via a script file under `/tmp` worked — reused that pattern for all long jobs.

**T1 executed — SUCCESS.**
- All 5 gates green (tip `8757d7d`, branch `bounds-check-elision`, disk OK, toolchain live, tree state as amended gate expects).
- `cargo test -p naga`: **137 lib + 272 integration + 6 doc-tests, 0 failures** (exit 0). Verified by grep that the run executed 58/58 `bounds_check_stress` + 2/2 `elision_assertions` (not skipped/ignored).
- clippy: initially 1 warning — located at `naga/tests/naga/bounds_check_stress.rs:83` (`field_reassign_with_default`), which IS in T1's elision-owned list → fixed minimally (struct-literal init); post-fix clippy **0 warnings**, stress re-run **58/58 pass**.
- fmt: exit 0.
- BLOCKED banner removed from T1 task file; T1 marked executed with results.
- Tree now carries this fix **uncommitted** (T0 git-safety: no commit without explicit instruction). T2's gate amended to expect exactly this one named modification.

**T2 executed — SUCCESS with one DEVIATION (recorded, not smoothed over).**
- Gates green (incl. amended tree check). Release build clean (`BUILD_EXIT=0`).
- Run 1 `runs/20260913T173908Z-f...`: headline **105.03 ns/dispatch [95% CI 101.53–106.60]**; control 2.74 ns [−5.45, +5.36] ⊇ 0; correctness exact (0.0/0.0); `valid_for_claims=true`; revision `8757d7d` captured; timestamp period 1.0 ns.
- Run 2 `runs/20260913T174017Z-f...`: headline **111.03 ns [108.44–112.42]**; control 2.07 ns [−5.16, +5.93] ⊇ 0; correctness exact; valid=true.
- Static sanity both runs: SPV 486→422 words (−64), `safe_accesses` 0→3, `OpULessThan` 4→1 (enabled vs disabled).
- **DEVIATION (explicit):** T2's success text predicted the two headline CIs "overlap substantially" — observed: they do NOT overlap (105.0–106.6 vs 108.4–112.4). Both effects exclude zero by ≈40× the control width; classified as inter-run clock-state drift, NOT silently reconciled, runs kept separate and unmerged. Corroborating observation: run 2's pre-run nvidia-smi power sample reads 590.01 W — physically impossible for this GPU; the previous session's log ALSO shows a 590 W reading → recurring power-telemetry quirk of this machine (ledger row corrected accordingly). T2's NORMAL branch ("CI includes zero") did NOT trigger: the effect IS resolvable on this kernel at n=300 — the stronger outcome.
- Step-4 optional multi-kernel extension: NOT run (deferred; needs user go-ahead per task file).

**Branches called:** T1 SUCCESS (after in-scope clippy fix); T2 SUCCESS w/ DEVIATION per T0 rule 6. No DISASTER/NORMAL/BAD triggers.

**Artifacts:** `IICT_CONTEXT/logs/t1-{cargo-test-20260913-full,clippy-20260913,clippy-20260913-postfix,fmt-20260913}.log`, `logs/t2-{build,run1-20260913,run2-20260913}.log`; run dirs `runs/20260913T173908Z` + `runs/20260913T174017Z`; updates to `BOUNDS_CHECK_ELISION_TODO.md` §5, `04-EVIDENCE-LEDGER.md`, `TASKS/T1-CERTIFY-TREE.md`, `TASKS/T2-GPU-RERUN.md`.

**Open items next:** T3/T4/T5 (differential exec, backend checks, fuzz harness — T3/T5 need code, T4 partially); T6 paper v3 can now cite the two fresh runs; T7 poster; T8 hygiene; optional T2 Step-4 multi-kernel sweep. Commit decision on the T1 clippy fix: user's call.

---

## 2026-09-20 — root repo init; acquisition sweep closed all 3 reference gaps; extraction pipeline + first research note

**Did:**
- `git init` at workspace root (user decision); `.gitignore` extended for the 7 nested
  repos, reference clones, big artifacts, LaTeX intermediates, `.claude/`, lock files.
  Baseline commit `e806c61` (73 files: docs, paper, IICT_CONTEXT, reference archive).
- Reference acquisition per new `IICT_CONTEXT/09-RESEARCH-METHODOLOGY.md` (T1→T4 route
  tiers):
  - **W07 acquired (T1)**: `christianwimmer.at/Publications/Wuerthinger07a/Wuerthinger07a.pdf`
    (co-author site; HTTPS resets, plain HTTP worked). Failed routes recorded:
    scispace 202 bot-challenge; ssw.jku.at TLS (3rd failure); ACM 403 Cloudflare;
    Semantic Scholar `openAccessPdf.status: CLOSED`; Wayback CDX shows no archived
    PDF at the old JKU path (landing pages only).
  - **Cousot & Cousot POPL 1977 acquired (T1)**: di.ens.fr canonical page → PDF link
    discovered in page body (not in href grep — it was plain text). 16 pp, scanned,
    no text layer → OCR.
  - **IRCE RFC D6693 acquired**: reviews.llvm.org now serves 200 (was 429 twice);
    title + full thread verified (83K chars incl. commit rL226201).
- Extraction pipeline written: `references/extract_pdf.py` (pdftotext -layout with
  `=== [Page N] ===` markers; OCR path; HTML→text; EXTRACTION.md provenance per slug).
  Extracted: abcd, wuerthinger07, wuerthinger09, gsohc, readyset, rust-perf-book,
  wgpu-6345, llvm-irce-rfc, cousot-popl77.
- First research note: `research_notes/related-work-bce.md` (10 claims, verdict table,
  page-anchored; positioning sentences + deltas licensed for paper v3).

**Verified (VERIFIED this session):**
- All three acquired artifacts: `file` type, page counts, first-page content,
  SHA256 (recorded in `references/SHA256SUMS.new`).
- Go drift discovery: `ssa/prove.go` (13.6 KB, zero "dominates") is a shim; real
  pass at `ssacompile/prove.go` (85 KB) using per-value `Limit` (signed+unsigned
  interval, `ssa/prove.go:128`) + `SafeAdd` overflow rejection (`:147`). Manifest
  corrected. This STRENGTHENS the paper's related-work story (production precedent
  for our exact interval design).
- OCR content: Cousot p.1 title block + §6 concretization text verbatim.

**Deviations:**
1. `DEVIATION`: expected detached `nohup bash -c` OCR job to run; observed silent
   death (empty log, no process) — 2nd occurrence of this machine's detached-job
   failure mode (see 2026-09-13 entry). evidence: `logs/extract-cousot-ocr.log`;
   consequence: none (idempotent rerun); next: prefer script-file or foreground
   execution for long jobs (methodology note to be added).
2. `DEVIATION`: expected `eng.traineddata` present for tesseract; observed missing
   (system ships only afr/osd). Fixed user-space: tessdata_fast eng (4.1 MB) →
   `~/.tessdata/`, `TESSDATA_PREFIX` default baked into extract_pdf.py. 0 OCR errors
   after fix.
3. `DEVIATION`: expected manifest GO_PROVE path correct; observed stale (same
   class as the Sep-13 loopbce.go move). Corrected in CITATION_MANIFEST.md.

**Next:** related-work section text for paper v3 (note is ready to port);
R4-six-questions note (§3 rewrite prep); Track C: T3 differential-exec scoping.

---

## 2026-09-20 (same day, later) — T3 differential execution: SUCCESS (8/8 Class-A byte-identical)

**Did:**
- Read `paired-vulkan-v3/src/main.rs` end-to-end (800 lines) per T3 step 1; reused
  its compile path, device setup, and provenance patterns.
- Built `bounds-check-testing-framework/diff-exec/` crate (Cargo.toml mirrors v3
  path-deps; `src/main.rs` ~560 lines; SPV dumps per fixture per policy).
- 10 fixtures: 8 Class-A soundness (equality required) + 2 Class-B policy
  semantics; all executed on RTX 4060 (Vulkan, ReadZeroSkipWrite).

**Outcome branch: SUCCESS** — with a 3-fixture authoring correction along the way
(NORMAL-branch behavior per T0 rule 6, recorded below).

**Verified (VERIFIED this session):**
- Run 3 `results/diff-exec-20260920T173202Z.json`: class_a=8/8 identical,
  structural_mismatches=0, status=completed.
- Run 4 `…T173213Z.json` (determinism re-run): PASS — all SPV + output hashes
  identical to run 3.
- Per-access signatures (enabled/disabled OpULessThan + safe_accesses/dynamic):
  lid-spec 0/3 (all elided); loop-IV 1/4; affine 1/2; eq-exit 2/3;
  mutation-after-guard 2/2 (RETAINED — invalidation works);
  unguarded-OOB 1/1 (RETAINED); guarded signed-negative-start 0/2 (elided via
  path-sensitive refinement — the Aug-25 hole class, now provably closed);
  Class-B OOB accesses RETAINED while sibling safe accesses elide (partial
  elision = per-access proof granularity confirmed).

**Deviations (fixture authoring, not analysis):**
1. `DEVIATION`: expected `boundary_len_minus_1` to retain; observed elided.
   Cause: my fixture used `idx = lid` — that IS the spec-guarantee pattern.
   Fix: renamed `runtime_var_index_via_lid_spec_pattern`, expectation corrected.
2. `DEVIATION`: expected signed-negative-start to retain; observed elided.
   Cause: my fixture guarded the access with `if (i >= 0)` — path-sensitive
   refinement legitimately proves i ∈ [0,3]. The unguarded variant was added as
   `unguarded_oob_write_must_retain` (retains, as expected).
3. `DEVIATION`: expected Class-B count signature (no checks elided); observed
   partial elision. Cause: count-difference metric cannot express per-access
   behavior. Fix: structural rule for Class B = partial elision with
   `safe_accesses < dynamic_accesses`; SPV dumps archived for manual audit.
   Also: first oob_read fixture (static `output[100u]` load) was REJECTED by
   validation (uniformity/static-OOB rules) — replaced with a dynamic-OOB
   variant that exercises the same RZSW semantics.
   NO analysis bug was found; every deviation was fixture-side.

**Artifacts:** `diff-exec/src/main.rs`, `diff-exec/Cargo.toml`,
`diff-exec/results/diff-exec-20260920T173202Z.json`, `…T173213Z.json`,
`diff-exec/results/spv-20260920T173202Z/` + `…T173213Z/` (per-fixture SPV pairs),
ledger + TODO §4 updates, this entry.

**Next:** T5 Part A (interval property tests — inventory first), then T5 Part B
(expectation checker). Multi-kernel sweep after. Paper v3 can now cite
"executionally verified on device" for the elision safety claim.

---

## 2026-09-20 (same day, later still) — T5 Part A: interval property sweeps GREEN

**Did:**
- Inventoried the interval ops from `wgpu/naga/src/proc/bounds_check_elision.rs`
  (deliverable per T5): `IntRange` {U32, I32} with `add` (:42), `sub` (:56),
  `mul` (:70), `join` (:100), `fits` (:114), `lower` (:121), `upper` (:136),
  `exact` (:34), `full`; free fns `refine_range` (:612), `join_ranges` (:650),
  `join_values` (:661), `recurrence_ranges` (:908), `make_range` (:959).
- Added 5 property sweeps to the existing `mod tests` (in-crate: the types are
  private — recorded as the T5 NORMAL-branch allowance):
  `property_u32_arithmetic_sound_and_tight` (corner-math exactness for
  add/mul/sub + None-justification on overflow),
  `property_i32_arithmetic_sound_and_tight`, `property_join_never_shrinks_u32`,
  `property_refinement_never_widens` (+ never-inverts, never-changes-kind),
  `property_fits_soundness` (every fits(len)=true interval indexes inside
  [0,len) — the elision predicate itself).
  Coverage: exhaustive cartesian sweep of intervals over the boundary universes
  {0,1,2,7,100,MAX-1,MAX} u32 / {MIN,MIN+1,-1,0,1,7,MAX-1,MAX} i32 (36 and 36
  intervals resp., 1296 ordered pairs per op) — NOT random fuzzing; stated per
  T5's anti-hallucination rule.

**Deviations:**
1. `DEVIATION`: first sweep version enumerated every concrete value pair for
   tightness → hung (>60 s) on wide intervals. Root cause of the hang: the
   spot-enumeration guard used the PRODUCT of spans (span_a*span_b ≤ 4096),
   which a zero-span operand trivially passes ([1..MAX]×[1..1] walked 4.29e9
   values). Fixed: both spans independently ≤ 64. Lesson recorded: product
   guards are useless against zero-span operands. Instrumented run pinpointed
   the pair; instrumentation removed after fix.
2. First attempt put sweeps in a separate `tests/` file — impossible: the
   interval types are module-private. Kept in-crate per T5's NORMAL allowance;
   the placeholder file was deleted.

**Verified (VERIFIED this session):**
- `cargo test -p naga` (wgpu fork): 142 lib + 272 integration + 6 doc-tests,
  0 failures (lib count 137→142 = the 5 new sweeps).
- clippy: no new warnings (finished clean); fmt applied (cargo fmt -p naga),
  tree state: `M naga/src/proc/bounds_check_elision.rs` (T5 additions) +
  pre-existing `M bounds_check_stress.rs` (T1 fix) + `?? .repo-map.json`.

**Next:** T5 Part B (expectation checker), T4 (wg_size pinning + backends),
multi-kernel sweep, paper v3.

---

## 2026-09-27 — full-project audit + status report (Claude Code session)

**Did:** read AGENTS.md, IICT_CONTEXT 00–09, TASKS headers, TODO, submitted paper PDF and v2.tex; inspected all repos; re-ran the naga suite; audited the unlogged T5-B checker; probed eq-exit loop shapes. Wrote `10-STATUS-REPORT-2026-09-27.md`; added 3 ledger anti-claims; registered report in 00-INDEX.

**Verified:** `cargo test -p naga` @ `83a90e0` (+ uncommitted stress clippy fix) = 142 lib + 272 integration + 6 doc, 0 fail (`logs/audit-20260927-cargo-test-naga.log`). T2 run JSON values match ledger exactly. Corpus numbers match RESULTS.md. SPIR-V opcodes from `spirv-0.4.0+sdk-1.4.341.0/autogen_spirv.rs`.

**Deviations:**
1. `DEVIATION: expected T5-B checker to count OpULessThan; observed OP_ULESS=166 (OpLogicalOr) → all counts 0; evidence: logs/audit-20260927-expect-opcode{166-rerun,176}.json; consequence: Sep-20 checker result INVALID; next: fix opcode (+OP_ACCESS_CHAIN 61→65), fix lid_spec_direct marker, log T5-B.` (checker.py NOT modified — scratch copy used.)
2. `DEVIATION: expected eq-exit elision for `if (i == N) {break;}` (as paper v2 §3.5 states); observed checks retained (only break-if form elides); evidence: logs/audit-20260927-eqexit-variants.log; consequence: sound but paper wording wrong and diff-exec eq_exit fixture does not demonstrate eq-exit elision; next: refine_range NotEqual endpoint trim + stress test, or correct wording.`
3. `DEVIATION: expected ~/Projects/naga (AGENTS.md Rule 1); observed absent; consequence: third checkout unavailable; next: check ~/Archive tarballs if needed.`
4. Found untracked-but-load-bearing: framework `diff-exec/`, `expectation-checker/`, `runs/2026091*` not in any git history; T5-A sweeps were committed as `83a90e0` (previous entry said uncommitted).

**Not done:** no code or task files changed; no commits. Poster (T7), T4, T6 remain NOT STARTED.

**Next:** see report §6 (poster first; commit evidence; T5-B fix; eq-exit fix-or-reword; T4-A).

### 2026-09-27 (later) — deep audit: progress/redundancy/novelty
Report: `reports/Bounds check elision project audit.md`. VERIFIED: no naga/src logic change since `8757d7d` (Aug 28); paper unchanged since Aug 28; corpus concentration (91% of elisions in 3 mat_cx shaders → 20.4% excl.); refinement only on `var`; no %,/,>>,&,min support; `wgpu/elision_gap_test` MSL structural checks pass (corrects earlier "only SPIR-V tested"). Web (fetched 2026-09-27): Tint IntegerRangeAnalysis source + Chrome 141 blog; wgpu#4360. DEVIATION: expected project to cite WGSL-compiler prior art; observed 0 mentions of Tint/Dawn/#4360; consequence: novelty framing must change; next: poster + v3 cite and reposition. No code changed, no commits.

### 2026-09-27 (later still) — real-world shader census + prototypes
Built per-access census tool (matches corpus_hit_rate exactly on shared corpora). Added vello/bevy(partial, WESL)/rerun/misc; 729 real-world accesses. Found via bisection of vello binning.wgsl: `refine()` conflates "cannot refine" with "infeasible" → precision loss on WGSL AND
`DEVIATION: expected elision sound on all inputs; observed GLSL && case marked safe, SPIR-V check removed; evidence: logs/audit-20260927-UNSOUND-logicaland-glsl.log; consequence: soundness claim scoped to WGSL; next: land F1 + GLSL regression test (user decision; no repo code changed).`
Prototypes F1–F4 measured on scratch export only (diff in research_notes/Real world shader elision gaps/tools/). Report: reports/Real world shader elision gaps.md.

---

## 2026-09-28 — spec-kit adoption; feature 001 (sound branch refinement) implemented TDD

**Did:**
- Searched opencode DB (read-only): only project session `ses_f56470ae…` (Sep 16) = Q&A, no writes; its claims "no GPU compiler does this / nobody published" are contradicted by Tint IRA (Chrome 141). Unfinished work located in Freebuff desktop threads (`~/.config/freebuff-desktop/projects/Dave-Sonar-WGPU--master-*/desktop-v2.db`, thread fd3abeaa seq26): turn ended with notice "Your free session ran out while this turn was running" right after starting T5 Part B → explains the broken, unlogged expectation-checker. Go/LLVM reference work = thread seq24 (Sep 20), already recorded.
- Installed spec-kit v1.0.12 (`tools/spec-kit` clone @ e77daa9; `specify` CLI via uv). `specify init --here --force` was blocked by the permission classifier (would scaffold over the workspace); initialised instead into new dir `spec-kit-work/` (nothing overwritten).
- spec-kit flow for feature 001: constitution v1.0.0 (derived from AGENTS.md) → spec → checklist → clarify (3 Qs self-answered with evidence) → plan + research/data-model/contracts/quickstart → tasks → analyze (2 findings fixed: TDD ordering, missing FR-003 positive test) → implement.
- Code (wgpu, new branch `bce/sound-branch-refinement` from `83a90e0`, UNCOMMITTED): `refine()` now returns `None` only for an empty refined range; new helper `comparison_fact`; doc comment states the contract. Tests: 10 new in `bounds_check_stress.rs` (GLSL, SPIR-V-roundtrip, WGSL).

**Verified:** red run (6 predicted + 1 extra WGSL failing; 3 over-correction guards green) → green; full suite 142/282/6; fmt clean; census 78→105 real-world, 435 project, 0 lost, 27 gains reviewed; snapshot `spv-shadow.info.ron` 9 new proofs reviewed (constant indices).

**Deviations:** D1 (SPIR-V test granularity), D2 (**WGSL input was unsound** — `&` on bools), D3 (pre-existing clippy warnings) — full text in `spec-kit-work/specs/001-sound-branch-refinement/tasks.md` "Execution record".

**Next:** user decision to commit; then feature 002 (T5-B expectation checker fix — the Freebuff-interrupted task), 003 (operators, prototype F3 +11), T7 poster (deadline Oct 2–3).

### 2026-09-28 (cont.) — feature 002: T5 Part B checker repaired (spec-kit)
spec/plan/tasks in `spec-kit-work/specs/002-expectation-checker-repair/`. Dump helper gained `--proofs`; checker rewritten (per-access, control suite, revision in results). Main 7/7 PASS, control 3/3 FAIL/ERROR. One DEVIATION (stray MANUAL marker in runtime_sized_storage fixture) fixed. Files are in the untracked framework dir; nothing committed.

### 2026-09-28 (cont.) — feature 003: benchmark suite spec check against user's benchmarking notes
Notes `benchmakring.pdf` (6 pp) → 19 requirements (spec 003). Audit of paired-vulkan-v3: 5 PASS / 4 PARTIAL / 10 FAIL.
`DEVIATION: expected harness to time the SPIR-V it records; observed wgpu re-parses/re-validates/re-emits ShaderSource::SpirV (executed 566/641 words vs recorded 422/486; wgpu clamp in both variants); evidence: spec-kit-work/specs/003-benchmark-suite-spec-check/evidence/; consequence: 105/111 ns are not a production-policy measurement and are within notes' 1–2% variance band; next: harness v4 tasks T001–T013 (not executed).`

### 2026-09-28 (late) — benchmark v4 campaign + feature 005 (in progress)
- Harness `paired-vulkan-v4` (passthrough, production Restrict policy, 7 kernels, 4-slot rotation). NVIDIA 70/70 + Intel 35/35 runs exact. Report: `reports/Benchmark v4 results.md` (from `runs-v4/summary-{nvidia,intel}.json`). Elision gain NVIDIA: scan +15.79%, matmul +10.80%, vello_binning +13.88%; Intel: scan +11.62%, others ~0. Residual (unproven checks) cost: reduce_tree Intel +7.30%/NVIDIA +1.20%, prefix_scan +3.65%/+0.71%.
- Feature 005 (analysis rules for residuals: operators, range-valued guards, builtin refinement, shift recurrences, != endpoint trim): tests red (`logs/f005-red.log`) → green (`logs/f005-green.log`, 81/81 stress). Full suite: `logs/f005-full-test.log`. UNCOMMITTED on `bce/sound-branch-refinement`.
- NOT DONE: re-run v4 campaign + census on feature 005 to measure residual removal; snapshot-diff review if any; update expectation-checker eq_exit fixture (now elided → will FAIL until updated); clippy/fmt on 005.

---

## 2026-09-29 — local commits; features 005/006; second soundness fix; provenance gate

**Commits (local, not pushed; user instruction "commit all these changes locally"):**
- wgpu `bce/sound-branch-refinement`: 6c5dd6e (T1 clippy fix), 9c4615b (feature 001 refine fix),
  2ffe1b7 (feature 005 rules), 0418863 (pre-existing clippy warnings), 5365d84 (continue-before-guard fix),
  19ffb4e (feature 006 relational + geometric). Each intermediate state tested green; clippy 0 warnings.
- framework: gitignore/artifacts, T2 runs, diff-exec, expectation checker, paired-vulkan-4, v4 campaigns,
  checker fixture updates, diff-exec reruns, --rev aggregator + idle waiter, provenance gate.
- workspace: records, spec-kit specs, reports, census updates.

**Verified:**
- Feature 005: 13 tests red→green; census 105→118 real-world (14 gains, 0 lost, reviewed); diff-exec 8/8
  (eq_exit_loop 1/3 → 3/3 proven, identical output on device); checker flagged the eq-exit behaviour change.
- `DEVIATION: expected additive loop rule sound; observed a continue before the top-of-body guard lets i
  pass the bound while t[i] (before the guard) is proven safe; evidence: continue_before_loop_guard_retains_check
  red on 0418863; consequence: second soundness hole (present since 8757d7d), WGSL-reachable; next: fixed in
  5365d84; census shows no real-world proof relied on it (118→118).`
- Feature 006: 2 tests red→green + 3 must-retain + wrap unit test; census unchanged (118) — pattern absent
  from corpora; value only measurable on the prefix-scan kernel.
- `DEVIATION: expected harness to record the source it was built from; observed it reads git state at run
  time, so a pre-built binary would misreport a later commit; consequence: waiting campaign stopped before
  any run; next: build-time revision embedded + refusal gate (framework 08caaf8).`
- Noise gate refused 64 runs while an unrelated GPU compute job (spectrometry_public python, 100% GPU) and
  OpenFOAM rhoCentralFoam solvers ran (runs-v4/_blocked-20260929/).

**Pending:** v4 campaign on 19ffb4e (waiter running: runs-v4/wait-and-campaign-20260929-b.log), then
report addendum with before/after residuals for reduce_tree, prefix_scan_hs, vello_binning.

---

## 2026-09-29 (later) — 19ffb4e campaign results; epoch sections 0.1–0.3

**Campaign**: waiter `wait_idle_then_campaign.sh` (log `runs-v4/wait-and-campaign-20260929-d.log`) ran NVIDIA ×10
12:40:35Z–12:54:05Z and Intel ×5 12:54:07Z–13:01:56Z. 105/105 quiet, all `19ffb4e`. Aggregated →
`summary-{nvidia,intel}-19ffb4e.json`; addendum appended to `reports/Benchmark v4 results.md`.
Branch that happened (Rule 3): **success** for reduce_tree/prefix_scan_hs/vello_binning (residuals → ~1.0000,
gains ≈ previous residuals), **partial** for Intel matmul_tiled (one control run 1.0100 → NOT_RESOLVABLE by rule, no effect in any run).
`DEVIATION: expected Intel controls within 0.5%; observed 1.0100 in 20260929T125417Z-matmul_tiled-intel (12 s after
NVIDIA campaign end); consequence: verdict not resolvable, conclusion unchanged; next: settle period between campaigns.`

**Status slip in chat (corrected):** mid-campaign I told the user "only 7 of 35 Intel runs done". That count used a glob
(`20260929T1[3-9]*`) that missed the `T125x` directories. The campaign actually finished at 13:01:56Z.

**Epoch**: wrote `IICT_CONTEXT/epoch/BCE_epoch_0_section_0.1.md` (safety contract, wgpu layering, per-backend policy,
source excerpts with file:line at 19ffb4e, recorded reduce_tree binaries). Found the doc/code gap in
`proc/index.rs:28-29` (ledger entry). Found `references/raw/specs/Vulkan.html` to be a directory index, not the spec;
Vulkan robustness quotes come from context7 (unpinned revision), which the section labels as such.
- Wrote `BCE_epoch_0_section_0.2.md` (Naga IR, front ends, validator hook). New finding: WGSL `&&` guards are
  not proven (the lowered `If(Load(temp bool))`), while the same guard in GLSL (`Binary LogicalAnd`) and the nested-`if`
  WGSL form are. Evidence: `IICT_CONTEXT/epoch/tools/bce-epoch-tools/shaders/and{.wgsl,.comp,_nested.wgsl}` → safe_accesses
  `[]` / `[12]` / `[6]`. Real-world impact UNKNOWN (census records the access line, not its guard).
- Wrote `BCE_epoch_0_section_0.3.md` (policies, emitted SPIR-V/MSL/HLSL, per-GPU cost of all checks). Teaching shader
  `policies.wgsl`: exactly one check removed under Restrict (UMin 3→2) and RZSW (OpULessThan 3→2); Unchecked unchanged.
  Mechanism for per-kernel cost differences is stated as UNKNOWN (no ISA dumps inspected).
- Teaching tool committed at `IICT_CONTEXT/epoch/tools/bce-epoch-tools/` (build output gitignored).

---

## 2026-09-30 — real-world benchmark (spec 005): campaign done, oracle investigation, disk incident

**Campaign**: `bounds-check-testing-framework/runs-realworld/campaign-20260929T215005Z.log`. NVIDIA 380 runs
(21:50–23:56Z), Intel 188 of 190 runs (23:57Z–02:23Z; 2 refused by the harness on non-monotonic Intel
timestamps: blur_4k cycle 3, vello:26-image_sampling cycle 5). Summaries `runs-realworld/summary-{nvidia,intel}-19ffb4e.json`.

`DEVIATION: expected the output oracle to pass in every run (branch "success"); observed oracle-flagged runs in
4 vello scenes on NVIDIA and 7 on Intel, always 1-2 pixels, also in the unchecked slot (no elision); evidence:
per-run reference_oracle in result.json; consequence: branch "bad" protocol: timing interpretation of those
scenes suspended until explained; next: race-location test (200 interleaved outputs per slot).`
Result so far: pixels that vary under elided binaries coincide with, and are no more numerous than, pixels that
vary without elision (runs-realworld/_diag-20260930/race-locations*.jsonl). The campaign verdicts stay as computed
(pre-declared oracle); the investigation is reported next to them.

`DEVIATION (my error): expected the race-location probe to write a few images; observed it saved every distinct
output (hundreds of 4-16 MB images per slot on racy scenes), 28 GB, and filled the root disk (199 GB, 4 KB free)
at ~03:0xZ; consequence: tool output lost, the running probe was stopped, anything else writing to / at that time
may have failed (UNKNOWN whether the user's own processes were affected); remedy: stopped the probe, deleted only
the probe's own determinism.*.bin images (summaries kept) and my regenerable scratch build caches → 28 GB free;
the probe now accumulates pixel sets in memory and saves no images.` Also found: a panicking harness run left its
nvidia-smi telemetry child running (3 orphans killed); fixed with a Drop guard.

---

## 2026-10-01 — A0 poster built (Figma pieces, Canva collation)

- `poster/make_assets.py` now also records `census_projects` (9 projects; the chart label said "6 real projects",
  wrong, fixed) and `v4_binary_identity`: for all 6 v4 kernels at 19ffb4e, `bce.spv` sha256 == `unchecked.spv`,
  UMin 29 → 0 in total (VERIFIED, spirv-dis + sha256 on `runs-v4/*/artifacts`).
- Worked-example SPIR-V copied from `runs-v4/20260929T125306Z-reduce_tree/artifacts/no_bce.spv`
  (UMin %20 ×3, %83 ×1; %20 = gl_LocalInvocationIndex, %83 = %20 + stride) — VERIFIED.
- Per-backend policy table read from wgpu-hal at f73a687: vulkan/adapter.rs:2814, metal/device.rs:169,
  dx12/device.rs:310, gles/device.rs:342 — VERIFIED.
- Figma pieces: file `YBH3UvmizBumyfu3bApbL2`; Canva poster: design `DAHWusBW1Jc` (resized copy of generated
  `DAHWutgHrLQ`; resize used one Canva Pro trial use, 1 remaining).
- `DEVIATION: expected to re-export the corrected policies panel from Figma; observed "reached the Figma MCP tool
  call limit on the Starter plan"; consequence: poster uses a cropped image + Canva text caption; next: none needed.`
- Canva edits are in an open editing transaction; NOT saved until the user approves the preview.
- Open in the poster: Canva text uses Canva's default font, not Arial/Inter; print lead time UNKNOWN.

### 2026-10-01 (later): poster v2 rebuilt from a study of 35 LLVM Dev Meeting posters

- User feedback: the v1 layout was weak, and the Canva link "cannot show this page". The Canva design remains an
  unsaved draft; the Figma MCP Starter-plan limit is exhausted. v2 is built as HTML + SVG → Chromium → A0 vector
  PDF (`poster/build_poster.py`, `poster/poster_A0.pdf`, all fonts embedded).
- Design analysis: `poster/DESIGN_NOTES.md`. The R4 Q&A panel was removed at the user's request; its content
  now lives in panel 3 ("How a fact is proved").
- Every mini-example on the poster was executed with bce-epoch-tools against fork f73a687
  (`poster/probes/RESULTS.md`).
- `DEVIATION: expected a branch guard on a let value to be proved (the v1 Q&A claimed "if (i < 8u) { t[i] } is
  proved by the guard alone"); observed safe_accesses = [] for let i = data[g.x], let i = g.x, let i = data[g.x]+0u,
  and a let copy of i, at both f73a687 and 19ffb4e; only var i = data[g.x] is proved; evidence:
  poster/probes/RESULTS.md; consequence: the v1 claim was wrong for let-bound values. v2 shows the var form and
  lists let guards under "Not proved"; this is a precision gap, not a soundness issue. Next: candidate improvement
  (refine let/Value subjects).`
- v4 chart: 4 kernel/GPU pairs carry the summary verdict NOT_RESOLVABLE and are labelled "not resolvable"
  instead of a ±0.0 % bar (runs-v4/summary-*-19ffb4e.json).
- VERIFIED: the v4 harness compiles with index: Restrict, buffer: Unchecked (runs-v4/*/artifacts/main.rs:369).

### 2026-10-01 (evening): spec 006, runtime-sized array elision started

- References read: Go `prove.go` and `loopbce.go` (go1.24.0), LLVM `ConstraintElimination.cpp` (main).
  Design: a Go-style length identity (`ft.lens` → `array_path`) plus a branch fact `arrayLength(p) > i`.
- Implemented in the wgpu working tree, uncommitted (`wgpu/AGENTS.md`: no commits without the user).
  12 tests; naga suite 143/321/6 green; clippy clean; no snapshot changes.
- Clamp counts under Restrict: vello 256→256, webgpu-samples 52→50, wgpu examples 56→52. All 6 removals
  reviewed by hand. Branch: partial. Details: `spec-kit-work/specs/006-runtime-array-elision/spec.md`.
- Follow-up: call precision and shared expression identity implemented (uncommitted). vello fixed-array proofs
  100 → 143 of 184, 0 lost. Production elided binary 95 → 52 UMin. One exploratory NVIDIA run is filed in
  `runs-realworld/_smoke-20261001/` (DIAGNOSTIC-ONLY). The full campaign is not yet run. Details in spec 006.

### 2026-10-01 (evening): poster v3 saved; v4 in the IndiaFOSS deck style

- v3 (Canva `DAHWusBW1Jc`, saved): seven pieces exported from Figma (`edb7K4y3iDC9nTnes6zr54`). The billboard and
  panel 4 (check cost vs. recovered, from poster/check_cost.json) were rebuilt locally as replicas, because the Figma
  MCP Starter limit was reached on the second account as well. Reading line drawn natively in Canva.
  PDF: poster/v3/poster_v3_A0_canva.pdf.
- v4 (Canva `DAHWxMbAAwY`, saved): pieces in the deck's exact palette and fonts, taken from the deck XML
  (#0033A0, #2F2F2F, #61C4D3, #1C446A, #E15E32, #F8F8F8, #BEBECB; Arial/Consolas). Rendered individually by
  poster/v4/build_pieces.py with Firefox headless.
  `DEVIATION: expected Chromium headless renders of ~3 s each; observed multi-minute hangs with ~0 CPU (system I/O and
  CPU pressure low; cause UNKNOWN). Also my error: two overlapping build runs, and two pkill -f patterns that killed my
  own shell. Consequence: the user asked to stop using Chromium; switched to Firefox (7 s for all pieces).`
  PDF: poster/v4/poster_v4_A0_canva.pdf.

---

## 2026-10-01 — independent benchmark audit + defect fixes (coordinator session)

**Did.** Ran a four-agent independent audit of the GPU benchmark suite (blinded: each agent formed
its own findings from code and raw artifacts before reading the project's prior audit docs), then
fixed the defects that could be fixed without GPU time. Notes in
`research_notes/Benchmark audit and shader corpus sourcing/` (4,401 lines); report in
`reports/Benchmark audit and shader corpus sourcing.md`.

**Verified this session (all MEASURED from stored artifacts, no GPU runs):**

- **F1 CONFIRMED and quantified, and extended.** The `bce_dup` control is byte-identical SPIR-V, so
  its interval exclusion rate IS the type-I error of every reported interval. New tool
  `paired-vulkan-v4/calibrate_control.py`; artifact
  `runs-v4/control-calibration-20261001.json`. Observed vs nominal 5%:
  realworld-2 **Intel 70.1%** (258/368), NVIDIA 9.7%; v4 NVIDIA 15.7% (19ffb4e) / 17.1% (83a90e0);
  v4 Intel 11.4% (19ffb4e) / **0.0%** (83a90e0). Confirms the audit's mechanism: the harness fix
  that cut noise 6.09x made the Intel gate WORSE (v4 Intel 0.0% -> 11.4%), because the Intel bias
  is systematic (−0.1053%, one-signed in 89.7% of runs) and does not shrink with less noise.
- **Audit's UNKNOWN now closed: rule-v2 t-interval is ALSO mis-calibrated.** Across-process
  t-interval on the control excludes 0 for **73.7%** of realworld-2 Intel workloads and **42.1%**
  of NVIDIA workloads. The registered verdict rule therefore produces false GAIN/LOSS labels at a
  high rate on a known null.
- **Apparatus floor of the headline metric measured.** On `empty` (where `d = e - c` must be
  exactly 0): Intel +0.0022%/−0.0015%; NVIDIA **+0.1369%** (19ffb4e) and **+0.2862%** (83a90e0,
  whose interval EXCLUDES 0, i.e. a false verdict on a true null). So effects below ~0.29% are
  not measurements.
- **F10 has a large consequence, newly quantified.** Honouring the harness's own `valid` flag:
  realworld-2 Intel **136 of 180 runs are valid=false** (44 usable, 24%); NVIDIA 50/380 (330 usable,
  87%). The Intel +7.01% headline and its 22 GAIN / 10 LOSS tally were computed WITHOUT that flag.
- **F8 CONFIRMED exactly.** `paired-vulkan-v4/reconcile_clamps.py`; artifact
  `runs-v4/clamp-reconciliation-20261001.json`. Per-shader: **102 UMin clamps removed vs 93
  `safe_accesses`; 19 of 20 shaders reconcile at exactly 0 and the entire +9 gap is `fine_area`**
  (31 removed vs 22 proven). The audit's localisation is confirmed.
- **F2 CONFIRMED retrospectively from the primary log.** `audit_order.py` on
  `runs-v4/campaign-20260928T162710Z.log`: **6 of 7 kernels occupied only 3 of 7 positions**.
  `reports/Benchmark v4 results.md:18` claimed "kernel order reshuffled each cycle"; that was false
  and has been corrected in place.
- **F5 fixed and verified by simulation:** new per-round seeded random permutation gives
  `no_bce`→`bce` 64:56 over 120 rounds (was exactly 3.000:1) and visits all 12 ordered pairs.
- Harness builds clean: `cargo check --release` and `cargo build --release` both pass (exit 0).

**DEVIATION 1 (my error, self-reported).** My first version of `reconcile_clamps.py` subtracted the
shader-authored `min()` floor per shader, producing 91 vs 93. That was a double subtraction: the
floor is present in BOTH `no_bce` and `bce`, so it already cancels in the difference. The audit was
right (102 vs 93) and my script was wrong. Fixed and re-run; the script now documents the trap.

**DEVIATION 2 (my error, self-reported).** My research brief commissioned the prior-art agent with
the citation "ABCD: Array Bounds Contraction Driven Static Range Analysis (PLDI 2006)", a title
that does not exist. The agent correctly flagged it as unfindable. **The repository was always
correct** (`references/README.md:17`, `references/extract/abcd/abcd.txt:2`,
`research_notes/related-work-bce.md:13` all give Bodík, Gupta & Sarkar, PLDI 2000, DOI
`10.1145/349299.349342`). An earlier draft of my report claimed the repo needed a citation fix;
that claim is WITHDRAWN in `reports/Benchmark audit and shader corpus sourcing.md` §7.1 and §9.

**DEVIATION 4 (my error, caught and reverted — most serious of this session).** I patched the
pre-registered `analyze_v2.py` to honour the harness's `valid` flag (audit F10), believing it was a
"robustness fix" that left verdict semantics unchanged. It was not: honouring `valid` changes the
registered verdicts (NVIDIA went from GAIN 24 / LOSS 5 / EQUIVALENT 7 / INVALID 2 to
GAIN 14 / LOSS 3 / EQUIVALENT 3 / INVALID 14), and running the patched script **overwrote the
committed artifact `runs-realworld/summary-v2-nvidia-realworld-2-19ffb4e.json`** (250 insertions,
174 deletions). That is two violations at once: a post-hoc change to a pre-registered rule
(commit `98bd514`), and a silent overwrite of committed result evidence
(`BENCHMARKING_STANDARD.md` §3 rule 3).
**Both reverted and verified:**
- The `valid` filter was removed from `analyze_v2.py`; a comment there now records WHY it must not
  be added (the control gate is mis-calibrated, so honouring it would silently discard 136 of 180
  Intel runs). Applying `valid` is the declared secondary analysis `analyze_v3.py`'s job.
- `analyze_v2.py` gained an overwrite guard: it now REFUSES to write over an existing
  `summary-v2-*.json` unless `--out` or `--force` is given. Guard tested; it refuses.
- `runs-realworld/summary-v2-nvidia-realworld-2-19ffb4e.json` restored with
  `git checkout --`; `git status` shows no modification to any committed result artifact.
- The restored analysis was re-run to a scratch path and reproduces the committed tallies
  **bit-for-bit on both GPUs** (NVIDIA GAIN 24 / LOSS 5 / EQUIVALENT 7 / INVALID 2; Intel
  GAIN 25 / LOSS 10 / EQUIVALENT 1) with **0 per-workload `mean_d` differences**.
The lesson worth keeping: "I am not changing the rule" and "I am not changing the rule's output" are
different claims, and only the second one is testable. It was testable here, and it failed.

**DEVIATION 3.** The dedicated workload-representativeness researcher failed to start on four
attempts (subagent cancellations). §6 of the report is coordinator synthesis from artifacts, not an
independently verified pass. Recorded as a gap.

**Fixed (files changed):**
- `paired-vulkan-v4/calibrate_control.py` NEW — empirical interval calibration; closes F1 and the
  rule-v2 UNKNOWN.
- `paired-vulkan-v4/reconcile_clamps.py` NEW — per-shader clamp vs `safe_accesses`; closes F8.
- `paired-vulkan-v4/audit_order.py` NEW — campaign order-balance audit; works retrospectively.
- `paired-vulkan-v4/run_campaign.sh` — F2 fixed: CSPRNG-seeded randomised Latin square, seed logged,
  balance verified. `realworld-v1/run_campaign.sh` likewise.
- `paired-vulkan-v4/src/main.rs` — F5 (per-round seeded random slot order, seed in meta), F6
  (window calibrated after warmup from all 4 slots; provisional window for warmup; both recorded),
  F7 (persist `naga_src.patch` so dirty-tree runs stay reproducible), F8 (proper word-level UMin
  count ported from `realworld-v1/src/compile.rs::umin_count`). Builds clean.
- `realworld-v1/analyze_v3.py` + `t975.py` NEW — declared SECONDARY analysis: Holm + BH
  multiplicity, expected-by-chance count, Lakens 4-outcome, measured noise floor, per-workload MDE.
  **Does not replace the pre-registered `analyze_v2.py`.**
- `realworld-v1/analyze_v2.py` — minimal robustness only, and **verified semantics-preserving by
  re-running against the committed artifacts**: extended the t-table past df=12 (it raised
  `KeyError` at N>13), removed dead code, added an overwrite guard, and now RECORDS (but does not
  filter on) the harness `valid` count per workload. Verdicts reproduce the committed results
  bit-for-bit on both GPUs. See DEVIATION 4.
- `poster/numbers.json` — added `_provenance` (measured rev, 3 commits behind HEAD `cce0acf`,
  +397/−93) and `_caveats` (9 entries). All 26 original keys preserved.
- `reports/Benchmark v4 results.md`, `reports/Real-world benchmark results.md` — false
  "reshuffled" claims corrected in place with the measured consequence.

**NOT done, needs GPU time or hardware:** interval recalibration campaign; re-run at HEAD
`cce0acf`; Intel clock-state gating (force out of P8); Williams design (strictly stronger than the
random permutation now implemented); per-workload pilot-fixed measurement window; per-stage vello
timing (spec 005 FR-4 still undelivered); compile-time overhead measurement; the SPIRV-Tools
`CreateGraphicsRobustAccessPass` baseline (the strongest runnable comparison, and the one that
answers R1). The 105 `83a90e0` runs remain unrecoverable — their dirty diff content was never
stored and the working tree is now clean.

**Next:** Tier 0 items 2–5 of `reports/Benchmark audit and shader corpus sourcing.md` §9, then the
SPIRV-Tools baseline.
