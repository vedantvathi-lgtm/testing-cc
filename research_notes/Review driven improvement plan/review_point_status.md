# Review point status matrix: IICT 2026 paper #55 (every reviewer and organizer point, mapped to code / artifacts / paper / poster)

Compiled 2026-10-01 in a read-only session. No builds, no cargo, no GPU programs, no state-changing git. A GPU campaign was running.
Evidence labels follow AGENTS.md Rule 0.2:
- **VERIFIED**: I read the file, ran a read-only `grep`/`git log`/`ls`, or read the log output this session.
- **RECORDED**: taken from a project document or artifact (path cited) and not re-checked here.
- **REPORTED**: stated somewhere but never independently confirmed.
- **UNKNOWN**: nobody has established it.

Sources are local paths, relative to `/home/bb/Projects/Dave-Sonar-WGPU--master/` unless absolute.

Layers used throughout (AGENTS.md Rule 0.7):
- **CODE**: `wgpu/` on branch `bce/sound-branch-refinement`, HEAD `19ffb4e` (2026-09-29 07:35 +0530). VERIFIED with `git -C wgpu log`.
- **ARTIFACTS**: reports, run summaries, census files and logs.
- **PAPER**: `iict2026_bounds_check_elision_v2.tex`. Its only git commit is the baseline `e806c61`; file mtime is 2026-08-28 00:30. VERIFIED.
- **POSTER**: the organizer's binding deliverable.

Status vocabulary (as requested):
- **DONE**: code, artifacts and paper/poster all reflect it.
- **DONE-BUT-NOT-IN-PAPER**: code and/or artifacts close the point, but no paper or poster text says so.
- **PARTIAL**: some layer closes part of the point.
- **MISSING**: nothing closes it.
- For praise, **HOLDS / HOLDS-WITH-CAVEAT / NO-LONGER-HOLDS**: does the praised property still hold?

---

## Q1. Complete point-by-point status matrix (all 4 reviews + organizer)

### Takeaway
All 48 extracted points (praise, criticism, questions, score rationales) are in the matrix below. Most technical criticisms are now closed in CODE and ARTIFACTS: loop analysis implemented, hit-rate and real-world corpora, a rigorous v4 and real-world benchmark, named soundness tests. **None of them is closed in the PAPER, and no POSTER exists on disk.** The conference is 2026-10-02/03 (RECORDED `IICT_CONTEXT/10-STATUS-REPORT-2026-09-27.md` l.4), so every "DONE-BUT-NOT-IN-PAPER" row is one poster block away from DONE. Several rows, including novelty and related work, comparison with other approaches, and the overridable workgroup size, stay PARTIAL or MISSING even in code and artifacts.

### Cited Findings

Source of the review wording: `IICT_CONTEXT/02-REVIEWS.md` (verbatim record of the 2026-08-19 decision email; RECORDED) plus the brief supplied by the caller.

#### Review 1 (weak accept, confidence Medium; Novelty 0, Engineering 1, Evaluation 0)

| ID | Point (type) | CODE now | ARTIFACTS now | PAPER v2 / POSTER | Status | What would close it |
|---|---|---|---|---|---|---|
| R1.1 | Summary: "index safety analysis exploiting WGSL spec + loop induction variable analysis" (neutral description) | Both still exist. The lid range comes from the workgroup size (`wgpu/naga/src/proc/bounds_check_elision.rs:797-807` `argument_range`; `:811-826` per-component `local_invocation_id`). The loop analysis is now a general interval domain (see R1.4). VERIFIED | — | Abstract still describes the old two-analysis framing plus the "hardened" rewrite (`v2.tex:104-125`). VERIFIED | HOLDS (description accurate; framing stale) | Poster headline: "interval-domain range analysis; spec-guaranteed builtin ranges are one input" |
| R1.2 | Praise: implemented in a production compiler | Implemented in a Naga fork. Not upstreamed. No PR evidence on disk (UNKNOWN). An upstream blocker is still present: the `sdata_manual` hack at `wgpu/naga/src/back/spv/writer.rs:2824` skips zero-init for any workgroup var of that name. VERIFIED | — | — | HOLDS-WITH-CAVEAT (production compiler, but a fork; not shipped) | Remove the `sdata_manual` hack (T8). State "fork of wgpu/Naga, not yet upstream" on the poster |
| R1.3 | "More comprehensive evaluation would further strengthen" | n/a | v4 synthetic suite: 7 kernels × 2 GPUs, 105/105 runs exact (`reports/Benchmark v4 results.md` l.9-50). Real-world: vello, all 34 scenes, full pipeline, plus imageBlur (`reports/Real-world benchmark results.md`). Hit-rate corpus of 295 programs (`corpus_hit_rate/results/RESULTS.md` l.20-28). Real-world census of 729 accesses (`reports/Real world shader elision gaps.md` l.23-26). VERIFIED (read) | Paper has only the corpus table (`v2.tex:401-441`) and legacy timing. No poster | DONE-BUT-NOT-IN-PAPER | Poster evaluation block from v4 + real-world + census, with both GPUs and the scope caveats |
| R1.4 | "A complete implementation of loop analysis would further strengthen" | Implemented and extended. `86eb81c` (loop-exit bound), `8757d7d` (interval rewrite), `2ffe1b7` (operators, range guards, builtin refinement, `>>`/`÷` loop vars, `!=` endpoint trim), `5365d84` (continue-before-guard fix), `19ffb4e` (relational facts, geometric `*c`/`<<k` loop vars). VERIFIED `git -C wgpu log 3fb225a..HEAD` + commit bodies | Stress file has **89** `#[test]` (VERIFIED grep). Full suite 143 lib + 303 integration + 6 doc, 0 failed (VERIFIED `IICT_CONTEXT/logs/f006-full-test.log`; attribution to `19ffb4e` RECORDED in commit `9a53103` message) | Paper §3.3 still describes the syntactic `if i<N {} else {break;}` matcher (`v2.tex:261-282`) with a figure placeholder (`:273`). §3.5 lists interval features (`:294-330`) but not 2ffe1b7/19ffb4e features. It reports "58 stress tests / 272 tests" (`:549-551`), which is stale | DONE-BUT-NOT-IN-PAPER | Poster "Algorithm" block describing the current rule set. Paper v3 §3 rewritten from `19ffb4e` |
| R1.5 | "Evaluation should be done on larger and more diverse benchmark suite" | n/a | Same as R1.3. Diversity is limited to 2 GPUs in one laptop (NVIDIA RTX 4060 Laptop, Intel RPL-S iGPU). No AMD, Apple, desktop, Metal or D3D12 (`reports/Benchmark v4 results.md` "Honest limits") | Not in paper; no poster | DONE-BUT-NOT-IN-PAPER (bounded diversity) | Poster: list workloads and GPUs explicitly and state the hardware limit |
| R1.6 | "Comparison with existing bounds-check elimination techniques or related prior work" | n/a | Related-work note with ABCD, W07, IRCE, GSOHC and Cousot rows (`research_notes/related-work-bce.md` l.13-21). Papers archived (`references/raw/papers/`: ABCD.pdf, Wuerthinger07a.pdf, Wuerthinger2009.pdf, CousotCousot…pdf, GSOHC.html; VERIFIED `ls`). A feature-comparison table vs **Tint IntegerRangeAnalysis** exists only in the audit report (`reports/Bounds check elision project audit.md` §4.2). **No experimental comparison** (e.g. Tint's robustness pass on the same shaders) exists anywhere (no artifact found) | Paper: 5 bibitems (`v2.tex:571-586`), no related-work section. VERIFIED | PARTIAL (literature collected; no written section; no head-to-head) | Poster related-work strip: ABCD / W07 / IRCE / Go prove / **Tint IRA (Chrome 141) / wgpu#4360**. Optional: run Tint on the census shaders for an empirical comparison (not started; UNKNOWN feasibility) |
| R1.7 | "Loop induction variable analysis appears to be a work in progress" | Same as R1.4 | Same as R1.4 | Same as R1.4 | DONE-BUT-NOT-IN-PAPER | Same as R1.4 |
| R1.8 | Novelty limited (score 0) | n/a | The audit **strengthened** this concern. Tint `IntegerRangeAnalysis` (Dawn, ©2024; Chrome 141 blog 2025-09-24) and wgpu#4360 (2021) predate the work (`reports/Bounds check elision project audit.md` §4.1; ledger `IICT_CONTEXT/04-EVIDENCE-LEDGER.md` anti-claim 2026-09-27). RECORDED. **Neither source is archived** in `references/raw/` (VERIFIED `ls`: no Tint/Dawn/4360 file). Neither is in `research_notes/related-work-bce.md` or `CITATION_MANIFEST.md` (VERIFIED grep) | Paper makes an implicit novelty claim; no Tint citation (VERIFIED grep, 0 hits in `.tex`) | MISSING (positioning); PARTIAL (narrower defensible claims identified in audit §4.2) | Reposition as "range-based BCE brought to Naga/wgpu, extended with guard refinement / non-unit & geometric steps / equality exits / relational facts, with measured evaluation". Cite and archive Tint IRA + wgpu#4360 |
| R1.9 | "Evaluation is limited and lacks comparison with other existing approaches" (Evaluation 0 rationale) | n/a | Breadth: DONE (R1.3). Comparison: MISSING (R1.6). Baselines present in the harness: `no_bce` (Naga as-is), `unchecked` (upper bound), `bce_dup` (identical-binary control) (`reports/Benchmark v4 results.md` l.17-18) | Not in paper | PARTIAL | Same as R1.3 + R1.6 |
| R1.10 | Engineering 1 (implementation valued) | Code grew; the analysis file is 1,979 lines (VERIFIED `wc -l`) | Tests green (R1.4) | — | HOLDS | — |

#### Review 2 (weak accept, confidence Medium; Novelty 1, Engineering 1, Evaluation 0). Strengths (praise)

| ID | Praised property | Does it still hold? (CODE / ARTIFACT) | PAPER / POSTER | Status |
|---|---|---|---|---|
| R2.S1 | Real deployed compiler (Naga → wgpu/Firefox) | Naga/wgpu/Firefox are real. The elision itself is in an un-upstreamed fork (see R1.2). Firefox's exact bounds-check settings are UNKNOWN from this repo (`reports/Real-world benchmark results.md` "Who actually runs these checks"). Wrong-default doc comment found: `naga/src/proc/index.rs:28-29` claims native wgpu defaults to RZSW; no wgpu-hal backend does (ledger 2026-09-29 DOC/CODE GAP; RECORDED) | Paper says "used in wgpu and Mozilla Firefox" (`v2.tex:100`). True of Naga, not of this pass | HOLDS-WITH-CAVEAT |
| R2.S2 | Integration into `analyzer.rs` / `function.rs` / SPIR-V writer ("a genuine artifact") | Still true and wider. Called from `wgpu/naga/src/valid/function.rs:1943`. Stored in `FunctionInfo.safe_accesses` (`valid/analyzer.rs:254`). Consumed by SPIR-V (`back/spv/index.rs:519-525`) and by the shared `access_needs_check` (`proc/index.rs:342-347`) used by MSL/HLSL. VERIFIED. MSL/HLSL structural tests `msl_uses_access_specific_proof`, `hlsl_uses_access_specific_proof` pass (VERIFIED `f006-full-test.log`). No Metal/D3D execution (no device; RECORDED ledger 2026-09-28) | Paper still says function-wide `safe_indices` and a SPIR-V-only story (`v2.tex:284-292`) | HOLDS (strengthened; paper stale) |
| R2.S3 | Spec-guarantee approach "clean, sound" (lid range invariant) | The builtin part holds. `argument_range` uses checked multiplication and requires product > 0 (`bounds_check_elision.rs:801-806`, VERIFIED). **Whole-analysis soundness did NOT always hold**, as three holes show, all fixed with regression tests: (a) signed-IV negative-start in the 86eb81c-era matcher, found Aug 25 (RECORDED ledger); (b) `refine()` treating unmodelled conditions as infeasible, unsound on WGSL `&`, GLSL `&&` and SPIR-V input, fixed `9c4615b` (VERIFIED commit body); (c) `continue` before the top-of-body guard, present since `8757d7d`, fixed `5365d84` (VERIFIED commit body). See also the override observation in R2.W9 | Paper claims soundness without qualification; the fixes post-date it | HOLDS for the builtin rule; HOLDS-WITH-CAVEAT for the analysis ("two unsound cases found by audit, both fixed with regression tests": wording RECORDED ledger 2026-09-29) |
| R2.S4 | Conservative gating: elide only when both wg_size and array length are statically known ("the right default") | Holds. `argument_range` returns `None` without `workgroup_size` (`:801`). Elision requires `IndexableLength::Known` (`:619-624`, VERIFIED grep). Runtime-sized arrays always keep their checks. Caveat: for override-sized workgroups the "known" size read pre-override is a placeholder; see R2.W9 | Paper states it (`v2.tex:323-324`) | HOLDS (caveat R2.W9) |
| R2.S5 | SPIR-V word counts + side-by-side disassembly make the zero-overhead claim checkable | Still true, and stronger under the production policy. In v4, where elision removes every clamp, `bce` and `unchecked` binaries are **byte-identical (same sha256)** (`reports/Benchmark v4 results.md` l.18). vello UMin 197→95, of which 11 remaining are source `min()` calls (`reports/Real-world benchmark results.md` Step 0). Paper word counts 176/176, 213/185 etc. are RECORDED (`a.md` §10b) under **ReadZeroSkipWrite**. wgpu's production Vulkan policy is Restrict for arrays and Unchecked for buffers (RECORDED audit C6; v4 table) | Paper disassembly uses RZSW `OpULessThan` (`v2.tex:186-205, 453-492`), not the production policy | HOLDS (paper shows a non-production policy) |

#### Review 2. Weaknesses

| ID | Point | CODE now | ARTIFACTS now | PAPER v2 / POSTER | Status | What would close it |
|---|---|---|---|---|---|---|
| R2.W1 | Thin evaluation: one minimal shader + one matmul microbenchmark | n/a | v4: 7 kernels. NVIDIA gains: scan_shared 1.1579, matmul_tiled 1.1080, vello_binning 1.1388. Intel: scan_shared 1.1162, the others 0.9999-1.0001 (`reports/Benchmark v4 results.md` tables). On `19ffb4e`: reduce_tree NVIDIA 1.0131 / Intel 1.0733, prefix_scan_hs 1.0086 / 1.0365, vello_binning NVIDIA 1.1522 (RECORDED ledger 2026-09-29, `runs-v4/summary-{nvidia,intel}-19ffb4e.json`). Real-world vello: Intel +8.48% geomean over 27 unflagged scenes (+6.71% all 34); NVIDIA +0.27% over 30 unflagged; imageBlur no resolvable effect (`reports/Real-world benchmark results.md` "Reading the results"). VERIFIED (read) | Paper: legacy minimal + matmul only (`v2.tex:336-400`). No poster | DONE-BUT-NOT-IN-PAPER | Poster timing block. Scope qualifiers are mandatory: synthetic kernels; native vello runs `unchecked()`, so gains apply to vello as web content (ledger 2026-09-30 SCOPE) |
| R2.W2 | No real-world WGSL corpus | n/a | Hit-rate corpora: naga tests 153, wgpu 76, webgpu-samples 66 (`RESULTS.md` l.24-26). Census of 729 runtime accesses over 9 projects incl. vello 440 (`reports/Real world shader elision gaps.md` §1). Real-world benchmark on vello + imageBlur | Paper has the 3-corpus table (`v2.tex:401-441`). No census, no real-world benchmark | PARTIAL in paper; DONE-BUT-NOT-IN-PAPER for census/benchmark | Poster: census + real-world benchmark |
| R2.W3 | No report on how often patterns fire (hit rate) | n/a | Project corpus 74.1% / 81.9% compute; **excluding 3 near-duplicate `mat_cx*` tests: 20.4% / 15.7%** (VERIFIED `IICT_CONTEXT/logs/audit-20260927-corpus-concentration.log`). External 2.5% (wgpu) / 1.1% (webgpu-samples) at `8757d7d` (`RESULTS.md` l.24-26). Real-world census: 78 (83a90e0) → 105 (F001) → 118 (F005 = 0418863) → 118 at `19ffb4e`, of 729. Ceiling ≈44% (320/729) provable in principle (`census/F005_summary.txt` l.1-9; commit `9a53103`; gaps report Summary 5) | Paper reports 74.1/81.9 **without** the mat_cx caveat (`v2.tex:120-125, 423`). Census absent | PARTIAL (paper headline fragile; newer census not in paper) | Poster: state both rates (with/without mat_cx) and census 118/729 with the ≈44% ceiling. The corpus_hit_rate study has not been re-run on `19ffb4e` (no artifact found; UNKNOWN) |
| R2.W4 | Timing methodology under-specified (sample count, variance) | n/a | v4 method is fully specified: 10 runs/kernel NVIDIA, 5 Intel; 4-slot rotating rounds; per-round ratios; in-round identical-binary control; noise and provenance gates; ±0.5% margin declared before results; order effect up to ~0.8% disclosed (`reports/Benchmark v4 results.md` l.9-20, 63-75). A first-campaign diagnosis found NVIDIA per-round control SD ~15% from idle gaps. **Pre-registration 2** (batched submission, verdict rule v2 with t-intervals) was written 2026-09-30 before a confirmatory campaign (`spec-kit-work/specs/005-real-world-benchmark/spec.md` tail; commit `98bd514`). Confirmatory results: **UNKNOWN** (campaign in progress; not read) | Paper timing section has none of this (`v2.tex:370-400`) | DONE-BUT-NOT-IN-PAPER (confirmatory campaign pending) | Poster methods box: n, rounds, control, margin, GPUs. Update after the confirmatory campaign under its pre-declared branches |
| R2.W5 | Exact "0.0 ns" delta reads as within-noise | n/a | Legacy "0.0 ns" / 7,568 ns are SUPERSEDED/INVALID (ledger Measurements). Schema-v3 105.03 / 111.03 ns are **also not valid for a headline**: wgpu re-compiled the SPIR-V, so the recorded binaries weren't the timed ones (ledger 2026-09-28 BENCHMARK VALIDITY). v4 replaces absolute ns with ratios plus a byte-identical control (`reports/Benchmark v4 results.md` l.18) | **Paper still prints "0.0 ns" and 7,568–8,192 ns** (`v2.tex:375-376, 387-390`, VERIFIED grep) | DONE-BUT-NOT-IN-PAPER (paper actively contradicts artifacts) | Delete the legacy numbers from any poster/v3. Use v4 ratios + control |
| R2.W6 | Modest novelty / missing related work (ABCD Bodík et al., Würthinger et al.; only 5 refs) | n/a | ABCD, W07, W09 archived and summarized (see R1.6). The Tint/wgpu#4360 gap is bigger than R2 knew (R1.8) | 5 bibitems; no related-work section (`v2.tex:571-586`) | PARTIAL (collected, not written; Tint absent everywhere) | Written related-work section / poster strip incl. Tint IRA |
| R2.W7 | The novel kernel is narrow | Broadened well beyond `data[lid]`: affine forms, `% / >> << & min max clamp select as` (`bounds_check_elision.rs:544-592`, VERIFIED grep), builtin/`let` refinement (2ffe1b7), relational facts (19ffb4e). Known precision gaps: WGSL `&&` guards are not refined (ledger 2026-09-29 PRECISION GAP; RECORDED). `for (...; i != N; ...)` status at HEAD is UNKNOWN (only probed at 83a90e0, where it was not elided: `10-STATUS-REPORT` §4.2) | Real-world yield is still modest: 118/729, ceiling ≈44% (R2.W3) | Paper describes the narrower 8757d7d set | PARTIAL | Poster: name the supported shapes and the ceiling honestly |
| R2.W8 | Half the contribution (loop induction analysis) not implemented | Implemented (R1.4) | Tests + diff-exec: 8/8 Class-A identical, `eq_exit_loop` now 3/3 proven (`bounds-check-testing-framework/diff-exec/results/diff-exec-20260929T020659Z.json`, VERIFIED fields). Two must-retain fixtures have identical binaries, so they are structural only (RECORDED `10-STATUS-REPORT` §4.3) | Paper §3.3 still presents the old matcher | DONE-BUT-NOT-IN-PAPER | Same as R1.4 |
| R2.W9 | Soundness corner case: overridable workgroup size | **New observation (VERIFIED by reading; not executed):** the WGSL front end fills override-sized dimensions with a **placeholder 1** (`wgpu/naga/src/front/wgsl/lower/mod.rs:1644`, `[1; 3]`, with the override captured at `:1655-1660`). The elision reads `entry_point.workgroup_size` without consulting `workgroup_size_overrides` (`bounds_check_elision.rs:231-233, 801-803`). So the **pre-override** ModuleInfo can carry lid-based proofs computed from the placeholder. In wgpu the backends use re-validated info: every wgpu-hal backend calls `process_overrides` (`wgpu-hal/src/vulkan/device.rs:770`, `metal/device.rs:157`, `dx12/device.rs:349`, `gles/device.rs:316`). That call revalidates whenever overrides exist (`naga/src/back/pipeline_constants.rs:93-103, 118-120, 284-292`), and `process_workgroup_size_override` writes the real sizes (`:297-327`). Whether a direct Naga-API user can emit code from the pre-override info is **UNKNOWN**: backends reject some `Override` expressions (`back/spv/writer.rs:2724`, `back/spv/block.rs:725`), but I did not establish that this path is always rejected. **No pinned test**: `grep -i override` finds nothing in `bounds_check_stress.rs` / `elision_assertions.rs` (VERIFIED) | `03-REVIEW-RESPONSE-STATUS.md` row 7 says "override can't produce bogus product". That wording is imprecise: a bogus product *is* computed pre-override, and safety rests on revalidation | Not discussed in paper | PARTIAL (production path safe by revalidation; direct-API path UNKNOWN; untested) | (1) Guard in code: skip builtin ranges when `workgroup_size_overrides.is_some()`. (2) A pinning test: override-sized workgroup, `arr[lid]`, assert no proof pre-override and correct proof post-`process_overrides`. (3) Poster sentence stating the mechanism |
| R2.W10 | Soundness corner case: syntactic-match fragility | Syntactic matcher replaced by the interval domain (`8757d7d`), then hardened (`9c4615b`, `5365d84`). Exhaustive boundary property sweeps (`83a90e0`; not random fuzzing, RECORDED session log 2026-09-20) | 89 stress tests incl. must-retain neighbours (VERIFIED count); expectation checker repaired: 7/7 main, 3/3 control FAIL/ERROR (RECORDED ledger 2026-09-28) | §3.5 mentions the rewrite; no soundness argument or test description beyond one paragraph (`v2.tex:294-330`) | DONE-BUT-NOT-IN-PAPER | Poster soundness box: test counts, 2 unsound cases found and fixed, diff-exec |
| R2.W11 | Soundness corner case: Restrict interaction | The proof check precedes `choose_policy` (`back/spv/index.rs:519-531`, VERIFIED), so proven accesses skip clamping under every policy. Test `elision_assertions::happy_direct_restrict_elides_umin` passes (VERIFIED f006 log) | v4 and real-world benchmarks measure the **production Restrict policy** (clamps `UMin`) (`reports/Benchmark v4 results.md` l.16) | Paper measures RZSW only | DONE-BUT-NOT-IN-PAPER | Poster: "measured under wgpu's production Vulkan policy (Restrict / Unchecked buffers)" |
| R2.W12 | Writing needs a pass (Writing clarity 2/3 from all reviewers) | n/a | n/a | v2 has 3 `\fbox` figure placeholders (`v2.tex:181, 256, 273`, VERIFIED grep). Stale test counts; old abstract; v3 does not exist (RECORDED `10-STATUS-REPORT` §3 T6; VERIFIED no newer `.tex`) | MISSING | Paper v3 (post-conference) and a clean poster |
| R2.Sc | Scores: Novelty 1, Engineering 1, Evaluation 0 | — | — | — | Evaluation addressed in artifacts (R2.W1-W5) | — |

#### Review 3 (weak accept, confidence High, Industry-Practice track; Novelty 0, Evaluation 0)

| ID | Point | CODE / ARTIFACTS now | PAPER / POSTER | Status | What would close it |
|---|---|---|---|---|---|
| R3.1 | "Template patterns" framing | The analysis is no longer template matching. It is an abstract-interpretation-style interval domain with path-sensitive refinement (`bounds_check_elision.rs` Values/Relation types l.10-40; `refine_range` l.905-920). VERIFIED | Paper abstract still leads with the pattern `if i < N {} else {break;}` (`v2.tex:109-111`), which invites the "template" reading | DONE-BUT-NOT-IN-PAPER | Poster must lead with "interval analysis", not patterns |
| R3.2 | "Algorithms could have been presented better" | Code is the spec | No pseudocode artifact matching `19ffb4e` exists (TODO §6 "Rewrite algorithms to match implemented code exactly" unchecked: `BOUNDS_CHECK_ELISION_TODO.md` l.82). Teaching write-ups exist (`IICT_CONTEXT/epoch/BCE_epoch_0_section_0.{1,2,3}.md`, RECORDED session log 2026-09-29) but cover policies, IR and validator, not the analysis algorithm | Figures are placeholders; §3 describes the old matcher | MISSING | Poster algorithm figure: domain, transfer rules, refinement, loop rule, join/invalidation. Paper v3 pseudocode |
| R3.3 | Work in progress | Loop analysis complete (R1.4) plus two soundness fixes and two feature rounds | Progress not reflected in paper (`v2.tex:544-563` still lists the schema-v3 rerun and differential execution as future work; both now exist) | DONE-BUT-NOT-IN-PAPER | Poster "status" line |
| R3.4 | Evaluation quite limited | Same as R1.3 / R2.W1 | Not in paper | DONE-BUT-NOT-IN-PAPER | Same as R1.3 |
| R3.5 | "Without extensive experiments, unclear if the selected patterns cover enough optimisation opportunities" | Census answers it directly. 729 real-world runtime accesses: 118 proven at HEAD. At most ≈44% (320/729) provable in principle; 46% are runtime-sized storage buffers, 7% vertex/instance indices, 3% binding arrays (`reports/Real world shader elision gaps.md` Summary 5). v4 measured the cost of the checks elision could *not* remove, and features 005/006 were driven by it (`reports/Benchmark v4 results.md` "What the numbers say" 3). VERIFIED (read) | Not in paper | DONE-BUT-NOT-IN-PAPER (answer is "coverage is limited, with a quantified ceiling") | Poster coverage chart: proven / provable-not-proven / unprovable-by-construction |
| R3.6 | Novelty 0; Industry track preference | See R1.8 | — | MISSING (positioning) | See R1.8 |

#### Review 4 (weak accept, confidence High, prefers Talk; Novelty 1, Engineering 0, Evaluation 0)

| ID | Point / question | Answer from CODE at `19ffb4e` (VERIFIED unless noted) | PAPER v2 | Status | What would close it |
|---|---|---|---|---|---|
| R4.1 | "Description of static analysis sketchy; important details omitted" (also the rationale for Engineering 0, RECORDED `02-REVIEWS.md` score-grid note) | Details exist in code and commit bodies (`9c4615b`, `2ffe1b7`, `5365d84`, `19ffb4e`) | §3.5 is a bullet list; no pseudocode; figure placeholders | MISSING in paper | Same as R3.2 |
| R4.Q1 | "What kind of index expressions do you consider?" | Any integer expression whose interval is computable. Literals and constants; builtins `local_invocation_index` / `local_invocation_id` (`:797-826`); loads of tracked locals; `+ − ×` with overflow rejection; `% / >> << & min max clamp select` and u32↔i32 conversion (`:544-592`; 2ffe1b7). Values refined by dominating comparisons; relations `a >= b` for `a − b` (19ffb4e) | §3.5 mentions "affine indices" only (`v2.tex:319-322`) | DONE-BUT-NOT-IN-PAPER | Poster table: "Index shape → handled?" |
| R4.Q2 | "Only constant indices? (if yes, interest drops)" | **No.** Constant indices were already skipped upstream (`proc/index.rs:335-340`, pre-existing). The contribution proves **runtime-varying** indices (RECORDED `02-REVIEWS.md` answer table; VERIFIED `proc/index.rs:335-347`) | Not stated explicitly | DONE-BUT-NOT-IN-PAPER | Poster must say it in the first block (the `T7-POSTER.md` layout contract lists it) |
| R4.Q3 | "If not constant, how do you get the values of variables?" | Three sources. (1) WGSL spec ranges for builtins from the static workgroup size. (2) Path-sensitive refinement at branches (`refine_range` `:905-920`), joined at merges, forgotten on mutation (`forget_local`, `Values` `:10-40`). (3) Loop-variable recurrence ranges from start, step and exit. Unmodelled conditions leave facts unchanged (`9c4615b`) | Partially (`v2.tex:306-322` bullets) | DONE-BUT-NOT-IN-PAPER | Poster algorithm figure |
| R4.Q4 | "Only loop induction variables, no other variables?" | Any tracked local or computed value with a provable interval at the access, not only IVs. Builtins and `let` values are refined since 2ffe1b7 (commit body: "refinement of any already-computed integer value such as a builtin"). Remaining gap: WGSL `&&` guards are not refined (RECORDED ledger 2026-09-29) | Not answered | DONE-BUT-NOT-IN-PAPER | Poster answer line |
| R4.Q5 | "What exactly do you mean by loop induction variable analysis?" | At submission: honestly, loop-exit-bound extraction (RECORDED `02-REVIEWS.md` answer table). Now: loop variables with an explicit start, a constant additive step in either direction, `>>`/`÷` shrinking steps (2ffe1b7), `*c`/`<<k` geometric steps (19ffb4e), and exits `<, <=, >, >=, ==/!=`. A top-of-body guard counts only if no `continue` precedes it (5365d84) | "Monotone recurrences" bullet (`v2.tex:315-318`); geometric/shift steps absent | DONE-BUT-NOT-IN-PAPER | Poster: define the term precisely |
| R4.Q6 | "Identifying recurrences for induction variables?" | Yes, for the forms in R4.Q5 (`recurrence_ranges`; unit test `recurrence_requires_reachable_exit_and_no_wrap` passes, VERIFIED f006 log). Equality exit `if i == N { break; }` at the loop top: elided since 2ffe1b7 (`refine_range` NotEqual endpoint trim `:912-916`; test `top_guard_equality_exit_elides` passes, VERIFIED f006 log). This was **false at 8757d7d**, the version the paper describes (RECORDED `10-STATUS-REPORT` §4.2) | v2 §3.5 claims this shape (`v2.tex:316-318`); the claim became true only after the paper was written | DONE-BUT-NOT-IN-PAPER | Poster: cite the commit where eq-exit landed |
| R4.Sc | Novelty 1, Engineering 0, prefers Talk | Engineering 0 was attributed to sketchiness (RECORDED `02-REVIEWS.md`) | — | Addressed via R4.1 | — |

#### Organizer and cross-cutting

| ID | Point | Evidence | Status | What would close it |
|---|---|---|---|---|
| O1 | Accepted as poster | `02-REVIEWS.md` Decision | HOLDS | — |
| O2 | "Address all the review comments to the best of their ability, in the final poster" (binding) | **No poster file found.** `find /home/bb -maxdepth 4 -iname '*poster*'` returned nothing, and the repo has only `IICT_CONTEXT/TASKS/T7-POSTER.md` (a plan). VERIFIED. Status report of 09-27 recorded "T7 NOT STARTED" (`10-STATUS-REPORT-2026-09-27.md` §3). A poster made elsewhere (Canva, another machine) is UNKNOWN | **MISSING** (most urgent: conference 2026-10-02/03) | Build the poster per `T7-POSTER.md` layout from this matrix, using only artifact-backed claims ("safe" list in `10-STATUS-REPORT` §5, updated by this file) |
| O3 | Optional 90-s teaser video, due Aug 27 | No artifact on disk (`find` for `*teaser*` empty; `02-REVIEWS.md` records UNKNOWN) | UNKNOWN (deadline passed) | Nothing; confirm with the author |
| X1 | Evaluation = 0 from all four reviewers ("loudest signal", `02-REVIEWS.md`) | R1.3/R2.W1-W5/R3.4-5 artifacts | DONE-BUT-NOT-IN-PAPER | Poster evaluation half |
| X2 | Writing clarity 2 (Okay) from all four | R2.W12 | MISSING | Poster + v3 |
| X3 | Relevance 3 (Very relevant) from all four | — | HOLDS | — |

### Inferences
- Across 48 rows the pattern is uniform: **CODE and ARTIFACTS have moved; PAPER and POSTER have not.** The paper has been frozen since 2026-08-28 (its only git commit is the baseline `e806c61`). Every row marked DONE-BUT-NOT-IN-PAPER becomes DONE the moment a poster block states it with its artifact.
- The paper is not only stale; it now **contradicts** the artifacts on three items: (a) "0.0 ns / 7,568 ns" (R2.W5); (b) the soundness claim, since holes were found and fixed afterwards (R2.S3); (c) 74.1% presented without the mat_cx concentration caveat (R2.W3). Any reuse of v2 text on the poster would reintroduce refuted claims.
- The claim in `03-REVIEW-RESPONSE-STATUS.md` row 7 ("override can't produce bogus product") is weaker than written (R2.W9). Revalidation on the wgpu path appears to protect production. The placeholder-1 behaviour is a real reason to add the pinning test and a one-line guard.
- `03-REVIEW-RESPONSE-STATUS.md` (dated 2026-09-13) is out of date on rows 1, 4, 8, 11, 12, 13, 14 and 17. This file supersedes it for status, though it does not edit it.

### Gaps
- Confirmatory real-world campaign under pre-registration 2: results UNKNOWN (running; not inspected, per the no-load constraint).
- Whether a poster or teaser exists outside `/home/bb` (depth 4) is UNKNOWN.
- Whether a direct Naga-API user can emit code from the pre-override ModuleInfo (R2.W9) is UNKNOWN. Establishing it needs a test run, which was not allowed this session.
- The corpus hit-rate study (74.1%) has not been re-run at `19ffb4e`; the census tool's naga-tests figure is 436/587 (`census/F005_summary.txt`). The two tools use the same denominator per the gaps report, but the RESULTS.md headline itself is from `8757d7d`.
- `for (...; i != N; ...)` elision at HEAD: not re-probed since 83a90e0.

---

## Q2. What does the CODE do now, relative to reviewer concerns?

### Takeaway
At `19ffb4e` the code is a per-access, path-sensitive interval analysis. It is far beyond the pattern matcher the reviewers read, and it answers every R4 question affirmatively for runtime indices. Two unsound cases were found and fixed since the review, and a third (signed start) was fixed before. There is one soundness-relevant code smell (override placeholder) with no test.

### Cited Findings
- Commits on the fork since trunk `3fb225a`: `86eb81c`, `87c6744`, `f105ff1`, `500dd82`, `8757d7d`, `83a90e0`, `6c5dd6e`, `9c4615b`, `2ffe1b7`, `0418863`, `5365d84`, `19ffb4e`. The working tree is clean except `?? .repo-map.json` — VERIFIED `git -C wgpu log --oneline 3fb225a..HEAD`, `git status --short`.
- Analysis size: `wgpu/naga/src/proc/bounds_check_elision.rs` is 1,979 lines — VERIFIED `wc -l`. The 09-27 audit recorded 1,370 lines at `8757d7d` (`reports/Bounds check elision project audit.md` Verdict 1).
- Operator coverage at HEAD: `ShiftLeft`, `Divide`, `Modulo`, `ShiftRight`, `And`, `Min/Max/Clamp`, `Select`, `As` — VERIFIED grep `bounds_check_elision.rs:544-592`. This closes audit gap C3, which was open at `8757d7d`.
- `!=` refinement trims endpoints — VERIFIED `bounds_check_elision.rs:912-916`. This closes audit gap C4 (top-guard eq-exit).
- The soundness fixes' commit bodies (VERIFIED `git log` of `9c4615b`, `5365d84`) describe exact repros and name their regression tests.
- Tests: 143 lib + 303 integration + 6 doc, 0 failed — VERIFIED `IICT_CONTEXT/logs/f006-full-test.log`. Named tests passing there: `continue_before_loop_guard_retains_check`, `top_guard_equality_exit_elides`, `geometric_loop_variable_is_bounded_by_its_exit`, `guarded_difference_is_non_negative`, `msl_uses_access_specific_proof`, `hlsl_uses_access_specific_proof`, `happy_direct_restrict_elides_umin`.
- Override placeholder: `front/wgsl/lower/mod.rs:1644` (`[1; 3]`), `bounds_check_elision.rs:231-233` (reads `workgroup_size` only). wgpu-hal revalidation: `pipeline_constants.rs:93-103, 118-120, 284-292`; `vulkan/device.rs:770` — VERIFIED by reading.
- Leftover hack: `back/spv/writer.rs:2824` `sdata_manual` — VERIFIED grep.

### Inferences
- R4's question set can now be answered with "yes, runtime indices from builtins, guards, and recurrences". The weak spots are WGSL `&&` guards, runtime-sized storage buffers (unprovable by construction), and vertex/instance indices.

### Gaps
- The WGSL `&&` precision gap is RECORDED (ledger 2026-09-29) but not re-probed; its real-world frequency is UNKNOWN (ledger).

---

## Q3. What do the ARTIFACTS support?

### Takeaway
Artifacts support four claims:
- Elision removes clamps and matches `unchecked` binaries where it proves everything.
- On synthetic kernels, the gain is large and reproducible on NVIDIA and hardware-dependent on Intel.
- On real vello content, the gain is about 8% on the Intel iGPU and about 0.3% on NVIDIA, with a strict web-content scope.
- Real-world proof coverage is 118/729, against a ≈44% ceiling.

They do **not** support "0.0 ns", "105/111 ns", a real-world speedup for native apps, or any claim beyond two laptop GPUs on Vulkan.

### Cited Findings
- v4 (NVIDIA 10 runs/kernel, Intel 5): gains of +15.79% / +10.80% / +13.88% on NVIDIA scan / matmul / vello_binning, and +11.62% on Intel scan — [reports/Benchmark v4 results.md](reports/Benchmark%20v4%20results.md) l.26-50.
- v4 on `19ffb4e`: every clamp in the 6 elidable kernels removed. bce/unchecked 0.9998–1.0004. One Intel control deviated 1.0100 (DEVIATION recorded) — RECORDED `IICT_CONTEXT/04-EVIDENCE-LEDGER.md` 2026-09-29 BENCHMARK entry.
- Real-world: 102 of 186 vello clamps removed; Intel vello +8.48% geomean (27 unflagged) / +6.71% (all 34); NVIDIA +0.27% (30 unflagged); imageBlur not resolvable — [reports/Real-world benchmark results.md](reports/Real-world%20benchmark%20results.md) Step 0 and "Reading the results".
- Oracle-flagged scenes: race-location test shows 8 elided-only vs 11 no-elision-only varying pixels, which points to vello nondeterminism, not a miscompile — same report, Correctness section.
- Scope: native vello uses `ShaderRuntimeChecks::unchecked()` (vello `wgpu_engine.rs:848-853`) — RECORDED ledger 2026-09-30 SCOPE.
- Differential execution 2026-09-29: `class_a_identical = 8 / 8`, `structural_mismatches = 0`, `eq_exit_loop` safe_accesses 3 of 3 dynamic — VERIFIED from `bounds-check-testing-framework/diff-exec/results/diff-exec-20260929T020659Z.json`.
- Census: REAL-WORLD (729) BASE=78, F001=105, F005=118; 14 gains, 0 lost — VERIFIED `research_notes/Real world shader elision gaps/census/F005_summary.txt` l.1-10. Unchanged at 118 on `19ffb4e` — RECORDED commit `9a53103` message.
- Corpus: 74.1% / 81.9% and OpULessThan 505→134 (`corpus_hit_rate/results/RESULTS.md` l.24, 28). Excluding mat_cx*: 20.4% / 15.7%, OpULessThan 145→122 (`IICT_CONTEXT/logs/audit-20260927-corpus-concentration.log`) — VERIFIED.
- Invalid for headline use: legacy 7,568 ns / 0 ns (ledger Measurements) and schema-v3 105.03 / 111.03 ns (ledger 2026-09-28 BENCHMARK VALIDITY) — RECORDED.

### Inferences
- The strongest, most defensible poster numbers are v4 ratios with the byte-identical control, and the vello Intel result with its scope sentence. Both are artifact-generated (`make_tables.py`, `analyze_v4.py` per the report headers), which satisfies AGENTS.md Rule 2's "tables regenerated only from artifacts".
- Real-world benchmark verdicts under rule v1 used a ±0.5% margin that the report itself judges too tight for NVIDIA vello. Pre-registration 2 exists to fix this, so NVIDIA real-world numbers should be presented as provisional until the confirmatory campaign is read.

### Gaps
- Confirmatory campaign outcome (success / partial / bad / disaster branch): UNKNOWN.
- No Metal/D3D12 execution evidence (RECORDED as absent).

---

## Q4. What does the PAPER / POSTER text say? Layer gaps

### Takeaway
The only paper text is v2 (Aug 28). It predates every improvement after `8757d7d`, still carries invalidated timing, and has no related work or figures. There is no poster. Every improvement since Aug 28 exists only in code and docs.

### Cited Findings
- v2.tex has no commits after baseline `e806c61`; mtime 2026-08-28 00:30; no v3 or newer `.tex` in the repo root — VERIFIED `git log -- iict2026_bounds_check_elision_v2.tex`, `ls -la`.
- v2 still contains:
  - "0.0 ns" (`:387`) and 7,568–8,192 ns (`:375-376, 388-390`);
  - `\fbox` figure placeholders (`:181, 256, 273`);
  - 5 bibitems (`:571-586`);
  - "58 stress tests / 272 tests" (`:549-551`);
  - future work listing the schema-v3 rerun and differential execution (`:554-563`), both since done, and v3 itself since invalidated.

  VERIFIED grep and read.
- v2's abstract headline of 74.1% / 81.9% has no mat_cx caveat (`:120-125`) — VERIFIED.
- v2's eq-exit claim `if i == N { break; }` (`:316-318`) was false at `8757d7d` (RECORDED `10-STATUS-REPORT` §4.2) and is true at HEAD (VERIFIED test pass + code).
- No Tint / IntegerRangeAnalysis / wgpu#4360 mention in any `.tex`. Mentions exist only in `IICT_CONTEXT/epoch/*`, `04-EVIDENCE-LEDGER.md`, `08-SESSION-LOG.md`, and the two reports — VERIFIED `grep -rli`.
- Poster: absent (O2 row). The T7 layout contract exists (`IICT_CONTEXT/TASKS/T7-POSTER.md` l.1-25). Its planned headline still uses 74.1% / 81.9% and "timing per T2's actual outcome". T2 (schema-v3) has since been invalidated, so the T7 contract itself is stale.

### Inferences
- Layer gaps that count as findings under Rule 0.7:
  - (1) the paper claims soundness, while the code history shows two post-paper unsound cases;
  - (2) the paper's timing is refuted by artifacts;
  - (3) the paper presents the old algorithm, while the code implements a different one;
  - (4) the T7 poster plan references an invalidated timing source.

### Gaps
- Whether the author intends a paper v3 before the proceedings deadline (if any) is UNKNOWN. No camera-ready requirement is recorded in `02-REVIEWS.md`.

---

## Q5. Which points are still open, and what exactly closes each?

### Takeaway
Seven classes of work remain:
1. **The poster (O2)**: binding, due Oct 2–3, not on disk. Almost every other row closes through it.
2. **Novelty/related-work positioning with Tint IRA + wgpu#4360** (R1.6, R1.8, R2.W6, R3.6): not written or archived anywhere citable.
3. **Algorithm presentation** (R3.2, R4.1): no pseudocode or figure matching `19ffb4e`.
4. **The override pinning test and guard** (R2.W9).
5. **Head-to-head comparison with another BCE implementation** (R1.6, R1.9): none exists.
6. **Confirmatory real-world campaign readout** (R2.W4): pending.
7. **Paper v3 hygiene** (R2.W12): remove refuted numbers, fill figures, add threats to validity.

### Cited Findings
- Poster absent — VERIFIED (`find`). The T7 plan is in `IICT_CONTEXT/TASKS/T7-POSTER.md`.
- Related-work material ready: `research_notes/related-work-bce.md` l.26-33 (design-space sentence) and l.68-76 (section plan). The Tint facts are only in `reports/Bounds check elision project audit.md` §4.1-4.2 (fetched 2026-09-27, not archived) — VERIFIED `ls references/raw/*`.
- Unchecked TODO §6 items (VERIFIED `BOUNDS_CHECK_ELISION_TODO.md` l.81-87):
  - figures;
  - §3 algorithms;
  - interval figure;
  - timing replacement;
  - related work;
  - threats to validity;
  - final PDF audit.
- No `override` test — VERIFIED grep.
- Upstream blocker `sdata_manual` — VERIFIED `writer.rs:2824`.

### Inferences
Closure recipe per open item, with Rule 3 branches where useful:
- **O2 poster** (closes R1.3-R1.7, R2.W1-W5, R2.W7-W8, R2.W10-W11, R3.1, R3.3-R3.5, R4.Q1-Q6, X1).
  - Content: one block per reviewer, each claim followed by an artifact path.
  - Numbers copied only from: v4 summaries; the real-world report; census F005/19ffb4e; corpus with the mat_cx caveat; diff-exec 2026-09-29.
  - Remove: "0.0 ns", 7,568 ns, 105/111 ns, and unqualified "sound".
  - Branches:
    - success: every row has a block;
    - partial: some rows are marked "open" on the poster (acceptable and honest);
    - bad: any number not traceable to an artifact, so pull it before printing.
- **Novelty/related work**: one poster strip plus archival of the Tint IRA source/header and the Chrome 141 blog, with access dates, into `references/raw/`. Minimum closure: cite Tint IRA and wgpu#4360 by name, and state the three capabilities the audit found beyond Tint's documented pattern (guard refinement, non-unit/geometric steps, equality exits). The audit labels "✘ per header" cells as UNKNOWN beyond the comments, so phrase them as "beyond Tint's documented pattern".
- **Algorithm presentation**: one figure with abstract domain `IntRange{U32,I32}`, transfer rules (`:544-592`), `refine_range` (`:905-920`), the loop rule with the continue-before-guard condition, join and `forget_local`.
- **R2.W9**: add a guard (ignore builtin ranges while `workgroup_size_overrides.is_some()`) plus a pinning test for pre- and post-`process_overrides`. Until then, the poster wording should be "production wgpu path re-validates after overrides are resolved; no dedicated test yet".
- **Comparison with other approaches**: cheapest honest option is a feature table (audit §4.2) labelled "documented capabilities, not measured". An empirical Tint run on the census shaders would fully close R1.6/R1.9 (not started; effort UNKNOWN).
- **R2.W4**: after the confirmatory campaign finishes, record which pre-registered branch happened. Only then state NVIDIA real-world verdicts.
- **Paper v3**: only if a post-conference version is required (UNKNOWN).

### Gaps
- No record of a camera-ready or proceedings requirement beyond the poster (`02-REVIEWS.md` lists only the poster and the optional video).
- The effort and feasibility of running Tint on the same shaders are UNKNOWN; no artifact exists.
