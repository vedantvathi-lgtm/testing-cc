# IICT Project Chronology — verified event history

> Provenance rule: every event below cites its source. Items marked [VERIFIED-2026-09-13]
> were re-checked this session against git/logs/files; items marked [RECORDED] come from
> project docs (`IICT_TEACHING_GUIDE.md` §8, `a.md`, `corpus_hit_rate/results/RESULTS.md`,
> `BOUNDS_CHECK_ELISION_TODO.md`) and were not independently re-verified.

## Phase 0 — Origin (March–June 2026)

| When | What | Source |
|---|---|---|
| 2026-03-16 | DAVE sonar WGPU port skeleton (`dave_ws/`, README). The parent project that motivated GPU compute in wgpu. | [RECORDED] teaching guide §8 flowchart; `dave_ws/` exists (empty-ish) [VERIFIED] |
| 2026-06-25→27 | Sonar pipeline + CUDA bench (`cuda_bench_harness.cu`, `benchmark_comparison.*`). CUDA baseline: 2–3 µs scatter kernel, zero bounds checks. | [RECORDED] teaching guide §8.2; artifacts exist at repo root [VERIFIED] |
| 2026-06-27 | Standalone `naga/` commit **`f08a9bf1`** "API-constant range analysis and workgroup barrier elision": Track 2 (builtin exact-match elision, backend-level `builtin_index_provably_in_bounds`) + Track 3-lite (WGUL post-barrier forward-scan elision). 167 insertions. | [VERIFIED-2026-09-13: `git -C naga log` shows `f08a9bf1`; branch now `test/elision-soundness-and-repro`] |

## Phase 1 — Paper v1 + benchmark suite (July 1–8)

| When | What | Source |
|---|---|---|
| 2026-07-01→05 | Loop-induction analysis built in `wgpu/` working tree (uncommitted at the time); five benchmark binaries in /tmp (`bce_bench2`, `loop_bench`, `matmul_bench`, `oob_test`, `spv_saver`); `COMPREHENSIVE_DECISION_LOG.md` v1.0 (Jul 5); `NAGA_PROBLEMS_AND_IMPROVEMENTS.md` v1.0 (Jul 5); `FIXED_BENCHMARKING_SUITE.md` (Jul 5, 103 KB redesign). | [RECORDED] docs themselves (dated headers); `/tmp` binaries since lost [RECORDED in `a.md` §8] |
| 2026-07-08 | `BOUNDS_CHECK_ELISION_FINDINGS.md`; paper v1 `iict2026_bounds_check_elision.tex` compiled; four-pass adversarial audit `a.md` (63 KB) — verdict: v1's central "interval analysis" narrative **described machinery that did not exist** (no `range` field, no checked_add/sub, no lattice; MSL passes `None` so no cross-backend inheritance; Track 3b described but not committed; `OpMemoryBarrier` vs `OpControlBarrier` confusion; GSOHC misattributed; IRCE slides URL dead). | [VERIFIED-2026-09-13: `a.md` read in full this session] |
| 2026-07-08 (later) | Rigorous re-benchmark (a.md §10b): paired interleaved matmul → bounds-check overhead **7,568 ns [CI 7,456–7,744]** not 2,400 ns; elided = exactly 0 ns [CI 0,0]; negative control passed. Scope-down of paper: "Independent Research", dropped false claims. | [RECORDED] `a.md` §10b/§11 |

## Phase 2 — Submission & reviews (Aug 19–25)

| When | What | Source |
|---|---|---|
| ≤ 2026-08-19 | Paper #55 submitted to EasyChair: **"Static Analysis for Bounds Check Elision in the Naga WGSL Compiler"**, author Bhuvan B, scoped-down version (two analyses; no barrier tracks; WIP section). Copy: `~/Downloads/IICT-2026_paper_55.pdf` (Aug 25 timestamp), `~/MITACS-CANADA/IICT-2026_paper_55.pdf`. | [VERIFIED-2026-09-13: files exist; PDF text extracted this session] |
| 2026-08-19 20:13 | Decision email from `iict2026@easychair.org` (Govind and Ramana): **accepted, poster presentation**; 4× weak accept; reviews attached; poster must address all comments; optional 90-sec video due Aug 27. | [VERIFIED-2026-09-13: full email text recovered verbatim from opencode session DB; full transcript preserved in `02-REVIEWS.md`] |

## Phase 3 — Review-driven expansion (Aug 25–27)

| When | What | Source |
|---|---|---|
| 2026-08-25 | `LITERATURE_BOUNDS_CHECK_ELISION_AND_IV.md` curated explicitly to answer R4's three questions; same-day wgpu commits `86eb81c` (loop-induction elision), `87c6744` (`enable subgroups`; causes the one known test flip), `f105ff1` (24-test stress suite + assertions + snapshots), `500dd82` (elision_gap_test + gpu_dispatch_verify crates); corpus_hit_rate v1 `96ac42a`. Signed-IV negative-start soundness hole discovered in loop analysis (documented in opencode session; fixed by later rewrite). | [VERIFIED-2026-09-13: commits exist in wgpu log; literature map read in full] |
| 2026-08-26→27 | (Codex session, ~2000 lines uncommitted) interval-domain rewrite; corpus_hit_rate v5; `paired-vulkan-v3` schema-v3 runner; `BENCHMARKING_STANDARD.md`; `AUDIT_INVENTORY.md` + `TEST_BENCHMARK_AUDIT.md` (Aug 27) invalidate legacy benchmark JSON for headline use. Repos committed across wgpu + two naga checkouts (per opencode session summary). `8757d7d` committed Aug 28 00:25 per file mtime of docs referencing it. | [VERIFIED-2026-09-13: audit docs read; git logs; session-continuation doc `~/MITACS-CANADA/03-naga-paper-session-continuation.md`] |

## Phase 4 — Rewrite, corpus result, paper v2 (Aug 28)

| When | What | Source |
|---|---|---|
| 2026-08-28 | **`8757d7d`** "feat(naga): sound interval-domain bounds-check-elision rewrite" — signed/unsigned interval domain, overflow-rejecting arithmetic, path-sensitive refinement + conservative joins, mutation invalidation, monotone recurrences, eq-exits, affine indices; access-specific proofs (`FunctionInfo.safe_accesses`); SPIR-V/MSL/HLSL consumers. 272 naga + 58 stress + 3 assertion tests green; clippy/rustfmt clean. Corpus v5 final `6ff062b`: **74.1% overall / 81.9% compute** on project corpus (was 1.24%/14.55%); external 1–2.5%; `OpULessThan` 505→134 (−73%). Paper v2 (`iict2026_bounds_check_elision_v2.tex`, 8 pp) compiled 00:30. GPU schema-v3 rerun **BLOCKED: NVIDIA driver down** (`nvidia-smi` cannot communicate). | [VERIFIED-2026-09-13: commit in git log; RESULTS.md read in full; v2 abstract read; TODO §5 states blocker] |

## Phase 5 — Archive + present (Aug 30 – Sep 13)

| When | What | Source |
|---|---|---|
| 2026-08-30→09-01 | Repos archived to `~/Archive/unzip-after-sih.tar.zst` (Aug 30) and `~/Archive/projects-source-backup.tar.zst` (Sep 1). Master context file `~/Projects/iict-bounds-check-elision.md` (4,264 lines) referenced by session log — **now missing from disk** [VERIFIED-2026-09-13: find returned nothing; not in archive listings either]. | [VERIFIED-2026-09-13: archive listings; teaching guide §8 references] |
| 2026-09-04→13 | opencode session (Sep 4) reads literature map; no new IICT engineering found. 2026-09-13 (this session): environment audit — **driver RECOVERED** (580.178.04, `nvidia-smi` functional) [VERIFIED]; references/ completed (Go checkout extended+updated, webgpu-samples re-cloned, W07/Cousot/IRCE still missing as recorded); IICT_CONTEXT/ created; TODO §2 checkboxes corrected. | [VERIFIED-2026-09-13] |

## Known doc-vs-reality discrepancies to keep in mind (none currently actionable)

1. `IICT_TEACHING_GUIDE.md` §7.6 says GPU timing "≈ ~9 ps per bounds check (~5–6 GPU cycles)" — that is the OLD 2,400 ns-derived figure; `a.md` §10b corrected to ~28.9 ps (~69 cycles). Trust `a.md` §10b; treat guide §7.6 as stale on this number.
2. `IICT_TEACHING_GUIDE.md` §8.5 says TODO "GPU rerun explicitly blocked on the NVIDIA driver" — true on Aug 28; **obsolete as of 2026-09-13** (driver works). The plan (07) treats the rerun as UNBLOCKED-but-not-yet-run.
3. `AUDIT_INVENTORY.md` (Aug 27) describes wgpu implementation as "uncommitted" — superseded same/next day by commits `f105ff1`/`8757d7d`; `BOUNDS_CHECK_ELISION_TODO.md` §3/§4 note "committed as 8757d7d". Current truth: clean tree, all committed [VERIFIED via git status 2026-09-13].
4. The literature map's Go path for loopbce.go was stale; corrected 2026-09-13 (see `references/REFERENCE_STATUS.md` addendum).
