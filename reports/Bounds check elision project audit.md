# Bounds check elision project audit

**Scope:** IICT 2026 paper #55 (bounds-check elision in Naga). Audit date 2026-09-27.
Poster due Oct 2–3.
**Questions:** (1) What is claimed vs. what exists? (2) Is the work redundant? (3) Is real
progress happening? (4) Is it novel against the state of the art?
**Method:** I checked every claim against running code, git history and raw artifacts,
following AGENTS.md. Labels: **VERIFIED** = checked this session; **RECORDED** = from a
project doc (path given). This session's evidence is in `IICT_CONTEXT/logs/audit-20260927-*`.
Two limits on method: `spec-kit` is not installed here, and research was done in-session
without subagents at your request. Companion report:
`IICT_CONTEXT/10-STATUS-REPORT-2026-09-27.md`.

---

## Verdict in five lines

1. **The engineering is real.** A 1,370-line interval-domain analysis exists, builds, and passes
   142 + 272 + 6 tests with 0 failures (VERIFIED). It is sound on everything probed. No probe
   produced a wrong elision.
2. **Progress stopped on Aug 28.** In the 30 days since, no analysis logic and no paper text
   has changed. The output has been about 220 KB of process and records markdown, about 1,150
   lines of test and harness code (one harness broken), and reference acquisition.
3. **The corpus headline is fragile.** 91% of the elisions behind "74.1%" come from **three
   near-duplicate matrix test shaders**. Without them the rate is **20.4%** overall and
   **15.7%** in compute (VERIFIED from the per-file JSONL).
4. **The novelty claim is exposed.** Google's Tint (Chrome's WGSL compiler) shipped a very
   similar integer-range analysis for exactly this purpose (Chrome 141, Sept 2025). An upstream
   wgpu issue has requested this feature since 2021. The project **cites neither**.
5. **What is genuinely new is narrower than claimed:** path-sensitive guard refinement,
   mutation invalidation, non-unit steps and equality exits, in *Naga*. Each of these has
   coverage gaps that I found in this audit.

---

## 1. Is progress happening? (measured, not asserted)

### 1.1 Code and commit deltas (VERIFIED, `git log --shortstat`)

| Period | Analysis code (`naga/src`, non-test) | Tests | Harness / tools | Paper | Docs / records |
|---|---|---|---|---|---|
| Jun 25–27 | standalone `naga` `f08a9bf1` (+167) | — | CUDA bench | — | Jul 5–8: ~370 KB of design and audit markdown (`COMPREHENSIVE_DECISION_LOG`, `FIXED_BENCHMARKING_SUITE`, `NAGA_PROBLEMS…`, `a.md`) |
| **Aug 25–28** | `86eb81c` (+261), **`8757d7d` (+2,014 / −719, 22 files)** | `f105ff1` (+1,135), naga `d45bcb65` (+406) | corpus harness (3 commits), `paired-vulkan-v3` (`dd4f5bb`), verify crates (`500dd82`, +778) | **v2 written (Aug 28)** | audit and inventory docs |
| Aug 29 – Sep 12 | **0** | 0 | 0 | 0 | teaching guide (Sep 4) |
| Sep 13–27 | **0** (`git log --since=2026-08-29 -- naga/src` → only `83a90e0`, which is `#[cfg(test)]` sweeps) | `83a90e0` (+276) | `diff-exec` (590 l, **untracked**), `expectation-checker` (283 l, **untracked, broken**) | **0** (v2.tex mtime Aug 28; no v3) | ~220 KB markdown across 26 files: `AGENTS.md`, `IICT_CONTEXT/` (1,849 l + tasks), `CITATION_MANIFEST.md`, research note, session logs; reference archive (+11,531 l) |

### 1.2 Reading this honestly

- **Real progress** since Aug 28:
  - The toolchain was restored and T1 certified.
  - The schema-v3 GPU rerun with a negative control was executed. This is the first
    trustworthy timing: 105.03 / 111.03 ns.
  - Differential execution was run on device.
  - Exhaustive interval-arithmetic property sweeps were added.
  - Missing references (W07, Cousot, IRCE) were acquired.

  These are verification and evidence gains, and they matter. The review said "Experimental
  Evaluation = 0" from all four reviewers.
- **No progress** on the analysis and the deliverables:
  - No coverage gap has been closed since `8757d7d`.
  - Paper v3 does not exist.
  - The poster does not exist. It is due in 5–6 days and is the organizers' only binding
    requirement.
- **Process overhead:**
  - Per session, the protocol and record system (AGENTS.md, evidence ledger, T0 executor
    rulebook, 8 task files with 4-branch outcome trees, a chronology, a session log) now
    exceeds the output it governs.
  - It did catch real errors: the eq-exit wording, stale paths, the "3 vs 2 assertions" drift.
  - It also let a harness with the wrong opcode ship unnoticed. That harness was never run
    against a known-positive control.
  - Net: it is an accountability system, not a progress engine.

**Verdict: progress is real but has shifted almost entirely to verification and paperwork.**
The research and deliverable frontier has been static for a month.

## 2. Redundancy

| Item | What is duplicated | Evidence (VERIFIED unless noted) | Assessment |
|---|---|---|---|
| Three-plus Naga implementations | standalone `naga/` (Track 2 builtin + Track 3 barrier elision, `f08a9bf1`); `wgpu/naga` fork (the real one); `~/Projects/naga` (`feat/elision-hardening…`) | The `~/Projects/naga` path **does not exist**. Standalone `naga/` is superseded by `8757d7d` for everything except the barrier work, which has a confirmed bug and is out of scope | Two of three are dead weight. Consolidation (T8-C) has not been done |
| GPU harnesses (5) | `bounds-check-bench` (1,945 l), `matmul-paired-bench` (768 l), `paired-vulkan-v3` (800 l), `diff-exec` (590 l), `wgpu/gpu_dispatch_verify` (367 l) | All legacy JSON has been declared SUPERSEDED/INVALID (`TEST_BENCHMARK_AUDIT.md`) | About 3,100 lines of legacy harness remain in-tree. Only `paired-vulkan-v3` and `diff-exec` produce citable evidence |
| Structural checkers (4) | `elision_assertions.rs`; `elision_gap_test` (MSL + SPIR-V, 4 cases, passes today); `diff-exec` structural gate; `expectation-checker` | All four count checks by string or opcode matching | Overlapping. The newest (`expectation-checker`) is the only broken one. It duplicates the stress suite's `access_proofs()` API with a weaker text-based method |
| Planning docs | `COMPREHENSIVE_DECISION_LOG` (83 KB), `FIXED_BENCHMARKING_SUITE` (104 KB), `NAGA_PROBLEMS…` (45 KB), `IICT_TEACHING_GUIDE` (49 KB), `a.md` (63 KB), then `IICT_CONTEXT/*` restating them | `IICT_CONTEXT/01` itself lists stale numbers inside the teaching guide | Heavy restatement. The project has written more about the work than the work itself (624,472 bytes of markdown across root, IICT_CONTEXT, research_notes and the framework README set — measured with `du -cb` — vs a 1,370-line analysis) |
| Re-done tasks | "tests green" re-certified (Aug 28, Sep 13, Sep 20, Sep 27); GPU rerun redone under a new harness 3× in history | logs | Mostly justified, since each run found drift. The protocol is not the waste; restating it is |

**Verdict: moderate redundancy in harnesses and docs, but no duplicated *research*.** The single
analysis in `wgpu/naga` is the only one that matters, and it is not duplicated. The redundancy
is in scaffolding around it.

## 3. Claims vs. reality (new findings; earlier ones are in the companion report §4)

| # | Claim (where) | Reality | Label / evidence |
|---|---|---|---|
| C1 | "74.1% overall / 81.9% compute" on the project corpus (paper v2, ledger, poster plan) | The numbers are arithmetically right, but **396 of 435 elisions (91.0%) come from `mat_cx2.wgsl`, `mat_cx2_f16.wgsl`, `mat_cx3.wgsl`**. These are matrix-layout regression tests, and two of them differ only in `f32`/`f16`. Excluding them gives **20.4% overall (39/191), 15.7% compute (17/108)**, and `OpULessThan` 145 → 122 (**−16%**, not −73%) | VERIFIED `logs/audit-20260927-corpus-concentration.log` |
| C2 | "Path-sensitive condition refinement" (8757d7d, v2 §3.5) | Refinement fires only when the index is a **`var`**. `if (lid < 8u) { t[lid] }` and `let j = lid; if (j < 8u) { t[j] }` **keep their checks**; `var i = lid; if (i < 8u) { t[i] }` elides. `let` is the idiomatic WGSL form | VERIFIED `logs/audit-20260927-operator-probes.log` |
| C3 | "Affine indices" / general interval arithmetic | Only `+`, `−`, `×` propagate (`bounds_check_elision.rs:431-433, 582-584`). `lid % 8u`, `lid >> 1u`, `lid / 2u`, `min(lid, 7u)` and `lid & 7u` all **keep** their checks. These are common GPU tiling idioms | VERIFIED probes + grep |
| C4 | "Equality exits" | Only `continuing { break if i == N }`. See companion report §4.2 | VERIFIED |
| C5 | MSL/HLSL consume proofs (v2, ledger) | MSL is **structurally tested** for 4 builtin cases by `wgpu/elision_gap_test`, which passes today. This **corrects my earlier "only SPIR-V tested" statement.** HLSL is untested. Neither has execution tests (no device) | VERIFIED run, exit 0 |
| C6 | Runtime OOB protection works (ledger, a.md) | Outputs are correct: read → 0.0, write dropped (hash-verified). But the storage-buffer fixtures run with Vulkan `robustBufferAccess` enabled and have **no `Unchecked` arm**, so Naga's check is not isolated. Also, real wgpu on this adapter compiles buffers as `Unchecked` when `robustBufferAccess2` is present (`wgpu-hal/src/vulkan/adapter.rs:2816`). The paper's RZSW setting is not wgpu's production setting | VERIFIED code; device attribution of `robustBufferAccess2` not separated per GPU |
| C7 | Soundness | Every probe, test and diff-exec fixture was conservative. **No unsound elision was found.** All gaps are missed opportunities, not bugs | VERIFIED |

## 4. Novelty and redundancy against the state of the art

### 4.1 Production WGSL compilers: the finding the paper must address

- **Tint (Dawn / Chrome), `IntegerRangeAnalysis`**:
  - File: `src/tint/lang/core/ir/analysis/integer_range_analysis.{h,cc}`, header "Copyright 2024". Fetched from `raw.githubusercontent.com/google/dawn/main` on 2026-09-27.
  - Used by the Robustness transform under `RobustnessConfig.use_integer_range_analysis` (Chromium issue 348701956 search snippet).
  - Announced in "What's New in WebGPU (Chrome 141)", page last updated 2025-09-24: "estimates the minimum and maximum values an integer variable can take … aims to improve efficiency by reducing the need for costly bounds checking and will soon be enabled by default across all platforms."
  - What the source shows it covers:
    - ranges for `local_invocation_index` / `local_invocation_id` from a constant workgroup size
    - signed and unsigned inclusive ranges
    - overflow-rejecting add, sub, mul, div, `<<`, `>>`, `%`, `min`, `max`
    - loop control variables with a constant initializer, ±1 update, and a body-top comparison that exits the loop
- **wgpu #4360 "Range analysis for bounds checks"** (kvark, opened 2021-09-15; fetched 2026-09-27) asks for exactly `workgroup_memory[invocation_id]` elision, citing the gpuweb#1202 TF.js perf regression.
- **Citations:** none of `iict2026_bounds_check_elision_v2.tex`,
  `LITERATURE_BOUNDS_CHECK_ELISION_AND_IV.md`, `research_notes/related-work-bce.md` or
  `CITATION_MANIFEST.md` mentions Tint, Dawn, 4360 or 1202 (VERIFIED grep, 0 hits).
- **Upstream Naga** (`wgpu-upstream` @ `688e4761c`, 2026-07-03): only constant-index checks are skipped (`proc/index.rs:335`). There is no range analysis upstream yet, so the Naga contribution itself is not redundant.

### 4.2 Feature comparison

| Capability | This project | Tint IRA | Classical (ABCD / W07-09 / IRCE / Go) |
|---|---|---|---|
| Builtin-index range from workgroup size | ✔ | ✔ | n/a (GPU-specific) |
| Signed + unsigned intervals, overflow rejection | ✔ | ✔ | ✔ (Go `prove` `Limit` + `SafeAdd`, per `related-work-bce.md` row 4) |
| add / sub / mul | ✔ | ✔ | ✔ |
| div / mod / shifts / min / max | ✘ (C3) | ✔ | ✔ (Go) |
| Loop IV, ±1 step | ✔ | ✔ | ✔ (Go `loopbce`, IRCE) |
| Non-unit constant step, decreasing loops | ✔ (`recurrence_ranges`, step2 probe elides) | ✘ per the header ("add one or minus one") | ✔ |
| Equality exits with reachability / no-wrap proof | ✔ `break if` only (C4) | not in the documented pattern | ✔ (Go) |
| Path-sensitive refinement from arbitrary `if` guards | ✔ on `var` only (C2) | not in the documented pattern | ✔ (ABCD inequality graph; Go dominator facts) |
| Invalidation on mutation | ✔ | n/a (IR form) | ✔ |
| Per-access proof granularity | ✔ | ✔ (per `Access`) | ✔ |
| Shipped in a browser | ✘ | ✔ (Chrome 141+) | — |

**Novelty verdict:**
- The *general idea* is not new. Interval-domain BCE is decades old (Cousot '77 → ABCD 2000 → Go/LLVM), and in the WGSL setting Tint shipped it a year earlier.
- R1 and R3 scored novelty 0, and this audit **confirms their concern** more strongly than the reviewers knew.
- What is defensibly new:
  - (a) the first range-based elision **in Naga/wgpu**, closing a 5-year-old upstream issue;
  - (b) three capabilities beyond Tint's documented pattern: guard refinement, non-unit steps, and equality exits with reachability proofs;
  - (c) the evaluation methodology: paired schema-v3 timing with a negative control, differential execution, and corpus hit-rate by segment.
- The paper must be repositioned as **"bringing and extending Tint-class range analysis to Naga, with a measured evaluation"**. It must cite Tint IRA and wgpu#4360 prominently.
- If a reviewer or poster visitor from the WebGPU community raises Tint and it isn't cited, that is the most damaging question this project could face.

## 5. What to do in the 5 days before the poster (in order)

1. **Poster (T7).** Use the claims→evidence layout. Change the headline to:
   - elision rates stated per segment: project corpus **with and without** `mat_cx*`; external 1–2.5%;
   - "extends the Tint-style approach to Naga": cite Tint IRA (Chrome 141) and wgpu#4360;
   - drop "only constant?" confusion (R4); state that soundness held under every probe.
2. **Fix the three cheap coverage gaps** (each is a small, sound change plus a stress test):
   - `NotEqual` endpoint trim (eq-exit top guard, C4);
   - refinement through `let` bindings and builtin arguments (C2);
   - `%` by a positive constant and `>>`/`/` by a constant (C3).

   Then re-run the corpus. This is the only work that can move the external 1–2.5% number and
   the non-`mat_cx` 20%. **Branches:**
   - success: the non-`mat_cx` rate rises measurably;
   - partial: probes pass but the corpus is flat, so report that honestly;
   - bad: a diff-exec divergence, so stop, revert, and keep the claim unchanged;
   - disaster: n/a.
3. **Commit the untracked evidence** (`diff-exec/`, `runs/`, the stress clippy fix). This is
   your call under Rule 1.
4. **Fix or delete `expectation-checker`.** It duplicates `access_proofs()`. The cheapest honest
   option is to replace it with stress tests that assert on `safe_accesses`.
5. **Stop writing new process documents** until paper v3 exists. The existing protocol is
   sufficient.

## 6. Limits of this audit

- Tint's coverage is taken from its header and doc comments plus a partial source extract. I did not run Tint on the same probes, so the "✘ per header" cells are UNKNOWN beyond what those comments state.
- The Chromium issue page could not be fully rendered; only the search snippet was read.
- I didn't search for academic GPU-BCE papers from 2024–26 beyond the project's archive, so other prior art may exist (UNKNOWN).
- I couldn't compute the pre-rewrite corpus rate with `mat_cx*` excluded: the older run's schema has no per-file records. RESULTS.md records 8 total elisions before the rewrite, so the excluded pre-rewrite count is ≤ 8.
