# Paper and poster presentation of IICT 2026 paper #55: gap analysis and corrected outline

Research date: 2026-10-01. Written read-only under `AGENTS.md`. The only file written is this one. No builds, no test runs and no GPU runs were done.

Evidence labels follow AGENTS.md Rule 0.2:
- **VERIFIED**: I read the file, ran the read-only command, or fetched the URL in this session.
- **RECORDED**: a project document states it. I did not re-check it here.
- **UNKNOWN**: nobody has established it.
- **CANDIDATE**: an inference I drew from reading code. It has not been executed, so it is not a finding until a test confirms it.

Code under study (VERIFIED with `git -C wgpu branch --show-current` and `git log`):
- repo: `/home/bb/Projects/Dave-Sonar-WGPU--master/wgpu`
- branch: `bce/sound-branch-refinement`
- HEAD: `19ffb4e`
- source file: `naga/src/proc/bounds_check_elision.rs`, 1979 lines

All line numbers below refer to that HEAD.

---

## Q1. Which statements in the paper texts are now wrong, outdated, unsupported, or superseded?

### Takeaway

Almost every technical and numerical statement in the submitted paper, and in the v2 draft, is now outdated or disqualified:
- the algorithm descriptions (§3.1 to §3.3);
- the timing table;
- the word-count framing;
- the "full safety" wording;
- the corpus table, which has inconsistent denominators;
- the "Vulkan only" scope;
- the 5-entry bibliography.

One file-identity fact matters here. `IICT 2026-3-1.pdf` is **not** paper #55: it is a different author's SourceKit-LSP paper. The submitted text is `static-analysis-naga-iict.pdf`, which is identical to `~/Downloads/IICT-2026_paper_55.pdf`.

### Cited Findings

**Which file is which (VERIFIED)**

- `IICT 2026-3-1.pdf`:
  - 4 pages, rendered by Google Docs.
  - Title "Enabling Documentation-Aware Language Features in SourceKit-LSP Using Symbol Graphs".
  - References a SourceKit-LSP PR by "Padmashree S S".
  - It is not paper #55 and it is not a venue-requirements document.
  - `IICT_CONTEXT/05-REPOS-AND-REFERENCES.md:60` labels it "Venue doc". That is record drift.
  - Evidence: `pdftotext` of [IICT 2026-3-1.pdf](/home/bb/Projects/Dave-Sonar-WGPU--master/IICT%202026-3-1.pdf).
- The submitted paper text:
  - `pdftotext -layout` of `~/Downloads/IICT-2026_paper_55.pdf` (7 pp, Aug 25) and of [static-analysis-naga-iict.pdf](/home/bb/Projects/Dave-Sonar-WGPU--master/static-analysis-naga-iict.pdf) (7 pp, created Jul 8 2026) are **identical**: the diff is empty.
  - v1 [iict2026_bounds_check_elision.tex](/home/bb/Projects/Dave-Sonar-WGPU--master/iict2026_bounds_check_elision.tex) is its source.
  - v2 differs from v1 only by: the added abstract paragraph, §3.4 "Interval-Domain Soundness Rewrite", §4.1 corpus study, a rewritten §6, and `\fbox` placeholders in place of the three `/tmp/*.pdf` figures. Evidence: `diff` of the two .tex files.

**Statements that are wrong, outdated or unsupported.** v2 line numbers; the same text is in the submitted PDF unless noted. All rows are VERIFIED unless the row says otherwise.

| # | Where (v2.tex) | Statement | Status now | Evidence |
|---|---|---|---|---|
| 1 | Table 2 (l.378-393); §4 text (l.371-376, 395-399); abstract (l.112-113) | "Elided … 0.0 ns"; "7,568 to 8,192 ns per dispatch"; "29 to 31 picoseconds per bounds check"; "~8 µs per dispatch cost". Table rows are also inconsistent with each other: "7,696 to 8,192" vs "7,568–8,192" | **SUPERSEDED/INVALID.** The legacy numbers are frozen. The v3 replacement numbers (105.03 / 111.03 ns) were themselves later found to time wgpu-recompiled binaries, which is **not valid for a performance headline** | [04-EVIDENCE-LEDGER.md](/home/bb/Projects/Dave-Sonar-WGPU--master/IICT_CONTEXT/04-EVIDENCE-LEDGER.md) Measurements rows + 2026-09-28 "BENCHMARK VALIDITY" entry (RECORDED) |
| 2 | Table 1 (l.354-369); §4 text | SPIR-V word counts 176/176, 213/185, 399/399, 432/404, measured under `ReadZeroSkipWrite` | RECORDED (`a.md` §10b). Produced at an old commit under RZSW, which is not wgpu's production policy on any backend | ledger; [Real world shader elision gaps.md](/home/bb/Projects/Dave-Sonar-WGPU--master/reports/Real%20world%20shader%20elision%20gaps.md) §2.3 |
| 3 | §3.1 (l.212-259); Fig. 2 | Builtin analysis is "a single forward pass over `ModuleInfo::process_function` (`analyzer.rs`)" and inserts into `safe_indices: HashSet` | **Outdated.** The analysis lives in `proc/bounds_check_elision.rs`. It is called from `valid/function.rs:1943` and fills the per-access set `FunctionInfo.safe_accesses` (`valid/analyzer.rs:254`) | grep of `wgpu/naga/src` |
| 4 | §3.1 l.251-252 | "Two properties distinguish it from a general-purpose range analysis:" is followed by no list (a dangling sentence) | Writing defect (R2 "writing needs a pass") | v2.tex |
| 5 | §3.2 (l.261-282); Fig. 3; abstract l.109-111 | "loop induction variable analysis" = matching `if i < N {} else {break;}`, `break if i >= N` / `> N` | **Replaced.** Now: path-sensitive interval refinement at every `If`, plus closed-form header ranges for additive, geometric and shrinking recurrences (`loop_range`, l.418-495). The name "induction variable analysis" oversold the old matcher (R4 Q5) | code; [02-REVIEWS.md](/home/bb/Projects/Dave-Sonar-WGPU--master/IICT_CONTEXT/02-REVIEWS.md) answer table |
| 6 | §3.3 (l.284-292) | "SPIR-V backend reads `safe_indices`" | Outdated. `proc/index.rs:344` (`access_needs_check`) and `back/spv/index.rs:522` read `safe_accesses`. MSL and HLSL consume proofs too, covered by structural tests only | grep; ledger 2026-09-28 CORRECTION (RECORDED) |
| 7 | §1 l.152 | "Currently targeting vulkan" | Outdated in the code (MSL/HLSL structurally tested). Still true for **execution** (Vulkan only) | ledger |
| 8 | §3.4 "Mutation invalidation" (l.312-314) | "Any Store … invalidates its interval" | Imprecise. A `Store` of a value with a known range is a *strong update* (l.357-364). Stores through pointers into a local reset it (l.365-366). Atomics reset it (l.368-372). Calls reset **all** locals (l.373) | code |
| 9 | §3.4 "Monotone recurrences" (l.315-318) | "equality exits (`if i == N { break; }`) accepted" | False at `8757d7d` (ledger anti-claim, 2026-09-27). Became true in `2ffe1b7` via the `!=` endpoint trim, now `refine_range` l.911-916. Divisibility is required for `Equal` exits (l.1345, 1360) | ledger; commit `2ffe1b7` message; [08-SESSION-LOG.md](/home/bb/Projects/Dave-Sonar-WGPU--master/IICT_CONTEXT/08-SESSION-LOG.md) 2026-09-29 ("eq_exit_loop 1/3 → 3/3 proven", RECORDED) |
| 10 | §3.4 "Affine indices" (l.319-322) | "`A*x + B` over `local_invocation_index` or an induction variable" | Understated. `expression_range` (l.512-600) interprets any expression tree of: `+ - *`, `/ % >> << &`, `min/max/clamp`, `select`, and int casts, over any operand with a known range | code |
| 11 | Abstract l.112-113; Discussion l.535-541 | "…preserving full safety for all other accesses"; "spec-compliant safety at zero runtime cost" | **Unsupported as unqualified soundness.** Two unsound cases were found after submission and both fixed: (1) `refine()` treated "cannot model" as "infeasible", reachable via WGSL `&`, GLSL `&&` and SPIR-V; (2) a `continue` before the loop guard, WGSL-reachable, present since `8757d7d`. Ledger wording: "two unsound cases found by audit, both fixed with regression tests" | ledger entries 2026-09-28, 2026-09-29 (RECORDED); commits `9c4615b`, `5365d84` |
| 12 | Abstract l.120-125; Table 3 (l.412-428); §4.1 text | "1.2% → 74.1% overall (81.9% compute)"; "295 validated WGSL programs" | Numbers match `RESULTS.md`, but **the headline is forbidden without its qualifier**: 396/435 elisions (91.0%) come from 3 near-duplicate matrix tests. Excluding them gives 20.4% overall and 15.7% compute. See also rows 13 and 14 | [RESULTS.md](/home/bb/Projects/Dave-Sonar-WGPU--master/corpus_hit_rate/results/RESULTS.md) l.24-28, 34-38; [audit-20260927-corpus-concentration.log](/home/bb/Projects/Dave-Sonar-WGPU--master/IICT_CONTEXT/logs/audit-20260927-corpus-concentration.log) |
| 13 | Table 3 | "Checks" column plus "before → after" rates | **Internally inconsistent.** The before and after rates use different denominators. Before: naga 644 checks, 8 elided, 1.24%; wgpu 871, 2, 0.23%; samples 93, 2, 2.15%. After: 587 / 79 / 92. The table prints the after denominators next to before counts, e.g. "8/504 = 14.6%" is arithmetically false (the 14.55% came from a different denominator). "wgpu 79 checks, 2 → 2, 0.2% → 2.5%" also mixes denominators | RESULTS.md l.30-38 vs v2.tex l.422-425 |
| 14 | Table 3 row webgpu-samples | elided "2 → 1" | A regression that neither the paper nor RESULTS.md explains | RESULTS.md |
| 15 | §4.1 "external code elides 1–2.5%" | Current real-world coverage is the 729-access census: 118/729 on `19ffb4e`, ≤320 (44%) provable in principle | ledger 2026-09-29; gaps report §2.2 and Summary (RECORDED) |
| 16 | §6 l.547-563 | "full naga test suite (272 tests), and 58 stress tests"; remaining work = "re-run under schema-v3", "differential execution" | Outdated. RECORDED counts: 142 lib + 282 integration + 6 doc at feature 001. `bounds_check_stress.rs` now has 89 `#[test]` functions (VERIFIED by grep count, not executed). v3 rerun done then invalidated; v4 plus a real-world campaign exist; diff-exec exists at small scale | ledger; `grep -c '#\[test\]'` |
| 17 | Figure 4 (l.447-528) | Hand-annotated SPIR-V at an old commit, RZSW policy | Should be regenerated at `19ffb4e` under the production policy. A recorded replacement exists: `reduce_tree` no_bce vs bce, with `bce.spv == unchecked.spv` (same sha256) | [BCE_epoch_0_section_0.3.md](/home/bb/Projects/Dave-Sonar-WGPU--master/IICT_CONTEXT/epoch/BCE_epoch_0_section_0.3.md) l.292-322 (RECORDED, from `runs-v4/20260929T125306Z-reduce_tree/artifacts/`) |
| 18 | Bibliography (l.569-591) | 5 entries: 2 specs, naga repo, docs.rs, Muchnick | No BCE prior art. In particular no **Tint IntegerRangeAnalysis** (Dawn, shipped Chrome 141) and no **wgpu#4360**: 0 hits in all project docs | [Bounds check elision project audit.md](/home/bb/Projects/Dave-Sonar-WGPU--master/reports/Bounds%20check%20elision%20project%20audit.md) §4.1 (RECORDED) |
| 19 | Intro l.133-146; §2.2 l.188 | "This paper and ongoing research envisions to solve this thorough optimizations that do not give correctness penalty. Correct work includes:"; "Background About Findings"; "A WGSL shader once compiled generated SPIR-V code that runs on Vulkan for a linux system"; "The need for availability of resources irrespective of results from a shader result mandates…" | Writing defects that R2 flagged ("writing needs a pass") | v2.tex |
| 20 | §2.1 / abstract | "Naga compiler, used in wgpu and Mozilla Firefox" | REPORTED: no in-repo citation. The real-world report says Firefox's exact settings are UNKNOWN. Needs a citation or softer wording | Real-world benchmark results.md §"Who actually runs these checks" (RECORDED) |
| 21 | §2.1 / §4 | Evaluation policy is RZSW | wgpu's production policies are: Vulkan index `Restrict`, buffer `Unchecked` with robustBufferAccess2; Metal `Restrict`/`Restrict`; GLES `Unchecked`. The paper must say which policy each number used | gaps report §2.3 (RECORDED at `83a90e0`); ledger doc/code gap `proc/index.rs:28-29` |

**Other doc/code gaps found in this session**

- **Epoch 0.2 attributes the running example's proof to the wrong feature (probable; code reading, not executed).**
  - `BCE_epoch_0_section_0.2.md` B.2, item 3 says proving `s[lid + stride]` needs a *relational* fact (feature 006, `19ffb4e`).
  - Reading the code, feature 005 (`2ffe1b7`) alone appears sufficient:
    - `lid < stride` refines `lid` against a range-valued bound: `comparison_fact`, l.713-718, gives `lid ≤ 127` when `stride ∈ [1,128]`.
    - `Add` is then plain interval addition, l.529-531: `[0,127] + [1,128] = [1,255]`, which fits a length of 256.
    - The relation list is consulted only for `Subtract`, via `related_difference`, l.532-543 and 764-778.
  - Commit `2ffe1b7`'s message itself cites "`if lid < stride`" as a target. `19ffb4e`'s message names the Hillis–Steele `s[lid - off]` as its target.
  - Resolve by dumping `reduce_tree.wgsl` proofs at `2ffe1b7` versus `19ffb4e`. The v4 table (3/6 → 4/6) spans both commits, so it cannot separate them.
- **CANDIDATE soundness gap (code reading only; untested).** Facts from `continue` edges are not joined into the `continuing` block's entry state.
  - `analyze_loop` (l.383-416) analyzes `continuing` from `body_flow` (the fall-through state) only.
  - The `If` handler (l.293-325) keeps only the branches that fall through, so a branch ending in `continue` contributes nothing.
  - Probe that should *retain* the check on `t[i]` (array of 4):

    ```wgsl
    var<workgroup> t: array<u32, 4>;
    @compute @workgroup_size(1) fn main() {
      var i = 0u;
      loop {
        if (i >= 4u) { continue; }
        continuing { t[i] = 1u; i = i + 1u; break if i >= 10u; }
      }
    }
    ```

  - By my reading, the analysis gives `i ∈ [0,9]` at the loop header, `[0,3]` on the fall-through edge, and uses `[0,3]` for `t[i]` in `continuing`. It would therefore prove the access although `i ∈ [4,9]` reaches it via `continue`.
  - WGSL allows an assignment as a `for` update, so a `for` loop can reach this too.
  - No stress test has an array access inside `continuing` (VERIFIED: grep of `bounds_check_stress.rs`; the `continue` tests at l.1721-1745 cover only continue-before-guard).
  - This must be run as a red test before any soundness wording is printed.
- **R2 "overridable workgroup size": the ledger's wording is not accurate.**
  - The ledger's earlier wording was "override can't produce bogus product".
  - For an overridden dimension, the WGSL lowerer stores a **placeholder of 1** (`front/wgsl/lower/mod.rs:1645`, `let mut workgroup_size_out = [1; 3];`).
  - `elide_bounds_checks` reads `entry_point.workgroup_size` without consulting `workgroup_size_overrides` (l.230-242). Before override processing, the analysis therefore sees a product that is too small, and it would prove `lid`-indexed accesses on arrays shorter than the true workgroup.
  - Production safety depends on `process_overrides` re-validating: `back/pipeline_constants.rs:119-121` and `286-293` (`revalidate` → `validate_resolved_overrides`), which re-runs elision on concrete sizes.
  - Whether every backend refuses a module whose `workgroup_size_overrides` are still set is UNKNOWN (not checked).
  - There is still no pinned test: 0 hits for "override" in `bounds_check_stress.rs` (VERIFIED).
- **T7-POSTER.md is out of date.** It prescribes "74.1% / 81.9%" as the poster headline ([T7-POSTER.md](/home/bb/Projects/Dave-Sonar-WGPU--master/IICT_CONTEXT/TASKS/T7-POSTER.md) "Layout contract" item 1). The ledger's 2026-09-27 anti-claim forbids that without the 20.4% qualifier.

### Inferences

- The submitted paper's two-analysis story is gone: builtin pass plus syntactic loop matcher. The correct framing now is one analysis: an interval abstract interpretation over Naga's structured IR, with the builtin fact as one input (`argument_range`, l.798-810). The paper's section structure has to change, not just its wording.
- No number in v2 §4 can be carried over unchanged. Table 1 could be kept only as "recorded, old commit, RZSW". Better to replace it with the v4 sha256-identity fact: elided binary = unchecked binary.

### Gaps

- I did not check whether the Aug-25 PDF is byte-identical to `static-analysis-naga-iict.pdf`. Only the extracted text matches; the file sizes differ (119474 vs 116211 bytes).
- The CANDIDATE continue/continuing gap and the reduce_tree feature attribution both need execution to confirm or refute. That was out of scope here: no builds.

---

## Q2. What the ledger forbids, and which claims are allowed

### Takeaway

The ledger's anti-claims and scope qualifiers amount to a short list of forbidden sentences:
- any speedup without scope;
- "0.0 ns", 7,568 ns, 29 ps or 105/111 ns as performance headlines;
- 74.1%/81.9% without the 20.4% qualifier;
- "novel range-based BCE for WGSL";
- unqualified soundness;
- "vello gets faster";
- "fuzzed";
- "eq-exit" or "verified on MSL/HLSL" beyond what was tested.

The allowed set is narrower but still substantive: synthetic-kernel gains per GPU, a real-world vello result on Intel with scope, a coverage funnel, and a correctness evidence stack.

### Cited Findings

**Forbidden, or needing a qualifier.** All rows are RECORDED from [04-EVIDENCE-LEDGER.md](/home/bb/Projects/Dave-Sonar-WGPU--master/IICT_CONTEXT/04-EVIDENCE-LEDGER.md) "Explicitly NOT established" unless noted.

| Forbidden claim | Required replacement or qualifier |
|---|---|
| Speedups on arbitrary real-world workloads | Scope every gain: synthetic kernels (v4) or vello *as web content*, per GPU |
| "0.0 ns", 7,568 ns [7,456–7,744], "29–31 ps/check", "12.5% instruction reduction", "2,400 ns / 5–6 cycles" | Do not print. They are SUPERSEDED/INVALID |
| 105.03 / 111.03 ns as a performance headline | Only with the audit caveat: the timed binaries were re-parsed and re-emitted by wgpu (566/641 executed words vs 422/486 recorded); both variants carry a `UMin` clamp; effect 1.15% / 1.21%, inside the 1–2% variance band. Better: drop it and use v4 |
| "74.1% / 81.9%" alone | Always add "396/435 of those elisions are in 3 near-duplicate matrix tests; excluding them 20.4% overall / 15.7% compute". Better: lead with the real-world census |
| Novelty of range-based BCE for WGSL | Cite Tint IntegerRangeAnalysis (Dawn, ©2024; "What's New in WebGPU (Chrome 141)", 2025-09-24) and wgpu#4360 (2021-09-15). Reposition as the first range-based elision **in Naga/wgpu**, with capabilities beyond Tint's documented pattern: path-sensitive guard refinement, non-unit steps, equality exits with a divisibility (reachability) check. Audit §4.2 |
| "No unsound elision" / "sound" | "Two unsound cases found by audit, both fixed with regression tests" (2026-09-29 wording). Plus the CANDIDATE continue/continuing case from Q1: run it before printing |
| "vello gets faster" | Native vello uses `ShaderRuntimeChecks::unchecked()` (vello `a9f11bb` `wgpu_engine.rs:848-853`). Gains apply to vello *as web content* in a wgpu-based browser |
| Results beyond RTX 4060 Laptop + Intel RPL-S iGPU, Vulkan | None measured on Metal, D3D12, AMD or desktop |
| "Verified across SPIR-V/MSL/HLSL" | MSL/HLSL: proof consumption is **structurally** tested (`msl_uses_access_specific_proof`, `hlsl_uses_access_specific_proof`). Execution: Vulkan only |
| "Fuzzed" | The T5-A sweeps are exhaustive boundary enumeration, not fuzzing ([10-STATUS-REPORT](/home/bb/Projects/Dave-Sonar-WGPU--master/IICT_CONTEXT/10-STATUS-REPORT-2026-09-27.md) §5) |
| "8/8 byte-identical on device" | "Identical outputs on 6 fixtures whose binaries differ, plus 2 must-retain fixtures checked structurally" (status report §4.3). eq_exit became 3/3 proven after `2ffe1b7` (session log 2026-09-29) |
| T5-B Sep-20 result (2 pass / 4 fail) | INVALID: it counted opcode 166 (`OpLogicalOr`). The repaired checker gives 7/7 main PASS and 3/3 controls FAIL/ERROR |
| "Guards on `&&` refine" (WGSL) | False for WGSL runtime `&&`, which is lowered to `If` plus a temporary bool. Nested `if`s work (ledger 2026-09-29 PRECISION GAP) |
| Track 3 barrier elision sound | Out of scope; it has a back-edge bug |

**Allowed, with artifacts (RECORDED)**

- *Non-constant indices are the contribution.* Upstream Naga skips checks only for constant indices (`proc/index.rs:335` upstream @ `688e4761c`; audit §4.1).
- *Coverage funnel (real-world census).*
  - 729 runtime accesses across 9 projects.
  - Committed analysis at `83a90e0`: 78 (10.7%).
  - After the F1 soundness fix: 105.
  - After feature 005: 118.
  - After feature 006: 118 (no change).
  - Ceiling: ≤320 (≈44%) provable in principle; 46.2% are runtime-sized storage bounded only by host-side values.
  - Sources: ledger 2026-09-27/28/29; gaps report Summary and §2.2.
  - 118/729 = 16.2% is my arithmetic, not an artifact field. The artifact is `census/F006_*.jsonl` per the v4 addendum.
- *Synthetic kernels, v4 at `19ffb4e`, per GPU, production Vulkan policy, passthrough SPIR-V* ([Benchmark v4 results.md](/home/bb/Projects/Dave-Sonar-WGPU--master/reports/Benchmark%20v4%20results.md) addendum):
  - Every clamp in the 6 elidable kernels is removed.
  - Leftover-check cost is 0.9998–1.0004.
  - NVIDIA: scan_shared 1.1579, matmul_tiled 1.1080, vello_binning 1.1522, reduce_tree 1.0131, prefix_scan_hs 1.0086.
  - Intel: scan_shared 1.1161, reduce_tree 1.0733, prefix_scan_hs 1.0365; matmul, vello_binning and private_array are about 1.0 (the driver hides the clamp cost).
  - Scope sentence: "synthetic kernels chosen to expose the gaps; not a real-world performance claim".
  - One Intel control deviated to 1.0100 (recorded DEVIATION).
- *Real-world, spec 005 first campaign* ([Real-world benchmark results.md](/home/bb/Projects/Dave-Sonar-WGPU--master/reports/Real-world%20benchmark%20results.md)):
  - Elision removes 102 of vello's 186 bounds clamps.
  - Intel: vello suite geomean +8.48% over 27 unflagged scenes (+6.71% over all 34); bce/unchecked 1.0006; 21 resolved gains, max +34.27%; 5 resolved losses, which are also slower when fully unchecked.
  - NVIDIA: +0.27% over 30 unflagged scenes; 4 resolved gains, max +4.45%; 26 of 34 not resolvable.
  - imageBlur: no resolvable effect.
  - **New caveat (VERIFIED, commit `98bd514`, spec 005 "Pre-registration 2", written 2026-09-30):**
    - Diagnosis of the first campaign found "the harness submits one slot's sample, waits for it, then submits the next. The GPU idles in every gap and changes clock, so the 4 slots of a round do not see the same clock state. That breaks the premise of per-round ratios."
    - A confirmatory campaign with a batched harness (`realworld-2`) and verdict rule v2 was pre-registered and, per the caller, is running now.
    - **Poster consequence:** print first-campaign numbers only as "first campaign; confirmatory campaign with corrected harness in progress". Or replace them with the confirmatory results if they finish, under the pre-registered branch.
    - The diagnosis text names NVIDIA data. Intel controls were already tight (mostly ≤0.3%), but whether the Intel numbers are affected is UNKNOWN until the rerun.
- *Correctness evidence stack:*
  - 89 stress-test functions, count only; the last recorded full-suite pass is "each intermediate state tested green" (session log 2026-09-29).
  - Must-retain cases, structural assertions.
  - Differential execution on 6 meaningful fixtures.
  - Repaired per-access expectation checker.
  - Exhaustive interval property sweeps (`bounds_check_elision.rs` l.1742-1941).
  - vello race-location test: 8 elided-only vs 11 no-elision-only varying pixels, which points to vello's own nondeterminism.
- *Found bugs in the wild:* webgpu-samples imageBlur (filterDim 16, blockDim 113) reads `tile[r][128]` out of bounds. The analysis correctly left it unproven (ledger 2026-09-30). Not reported upstream: the user's call.

### Inferences

- The poster's headline should shift from "74%" to three statements:
  - the synthetic-kernel mechanism result (removing a hot clamp is worth +10.8% to +15.8% on NVIDIA);
  - the real-world result with scope (vello as web content: about 8% on Intel, about 0% on NVIDIA);
  - the coverage funnel (16% of real-world runtime accesses proven, ≤44% provable).

  That answers the "Experimental Evaluation = 0 ×4" signal with the strongest evidence that survives audit.

### Gaps

- Outcome of the confirmatory real-world campaign: UNKNOWN at writing (it is running).
- The full test suite was not re-run at `19ffb4e` in this session.

---

## Q3. How the analysis should be presented, with the answers to R4 (and R3/R2)

### Takeaway

Present one analysis. It is a forward, path-sensitive abstract interpretation over Naga's structured IR:
- **Domain:** 32-bit signed/unsigned intervals per local variable (per program point) and per computed value, plus a set of `≥` / `>` relations.
- **Transfer functions:** overflow-rejecting.
- **Branches:** refine both edges.
- **Joins:** hulls, with relations intersected.
- **Loops:** no fixpoint iteration. Loop-mutated locals are widened to ⊤ in one step, *unless* a recurrence rule gives a closed-form header interval.
- **Proofs:** attached to the individual `Access` expression.

The honest limits, stated beside the rules:
- loop recurrences are pattern-based (single update, constant step, constant exit bound);
- interprocedural facts are absent;
- values loaded from memory are unknown, but operations like `%`, `&`, `min` and `>>` can still bound them.

### Cited Findings: the precise description from code (VERIFIED, `bounds_check_elision.rs` @ `19ffb4e`)

**Abstract domain**
- `IntRange = U32{min,max} | I32{min,max}`, l.57-60. Only 32-bit scalars get a range: `full`, l.63-75; `local_full_range`, l.829-834. Other types get no fact.
- **State.**
  - `Ranges: local → IntRange` (l.10) is flow-sensitive: the value a `var` holds *here*.
  - `Values` holds `ranges: expr → IntRange` plus `relations: Vec<{big, small, strict}>` (l.15-55). A `Relation` names a loaded local *by the local*, and is dropped whenever that local may change (`forget_local`, l.40-44). Otherwise it names the value handle.
  - Epoch 0.2 explains that the two-map design is forced by Naga IR: each `Load` of a local is a new expression handle. RECORDED, `BCE_epoch_0_section_0.2.md` l.114.
- **Initial state.**
  - Every 32-bit local starts at the full range of its type.
  - A local with a constant initializer starts at that constant's range (`default_ranges`, l.245-260).
  - Expression-level sources: literals (l.853-865); module constants via `global_range`, which does add/sub/mul over constant expressions (l.867-883).
  - `local_invocation_index` gets `[0, x·y·z − 1]` from the entry point's `@workgroup_size`, with checked multiplication and `total > 0` (`argument_range`, l.798-810).
  - `local_invocation_id.{x,y,z}` gets `[0, size_c − 1]` (l.812-827).
  - Every other function argument, and every `Load` from non-local memory (uniform, storage, workgroup), has **no** range (l.524, 528).
- **Transfer functions** (`expression_range`, l.512-600). Endpoint arithmetic uses Rust `checked_*`: any overflow or underflow returns `None`, meaning "no fact", and the check is kept.
  - `+`: l.85-97.
  - `−`: l.99-111. If a relation `left ≥ right` is known, `related_difference` gives `[0 or 1, max(left) − min(right)]` (l.764-778).
  - `×`: four-corner, l.113-141.
  - `/`, `%`, `>>`, `&`: bound their result even when the left operand is unknown, by using its full type range via `range_or_full`, l.602-609. So `hash & 63u` or `x % 6u` are provable with `x` unknown.
  - `<<`, `min`, `max`, `clamp`, `select` (join of both arms), and `As` casts when representable.
  - Helper functions: l.1376-1556.
- **Proof rule** (`prove_access`, l.611-627):
  - applies to `Expression::Access{base, index}`, meaning non-constant indices; constant `AccessIndex` is handled upstream;
  - the base must have a statically known length (`indexable_length_resolved`); runtime-sized arrays never qualify;
  - `fits`: unsigned `max < len`; signed `min ≥ 0 ∧ max < len` (l.171-176);
  - if the rule holds, the access is inserted into `info.safe_accesses`.
  - It is evaluated at the `Emit` point of the access, with the state at that point (l.266-277).
- **Branch refinement** (`refine`, l.635-688; `comparison_fact`, l.690-728; `refine_range`, l.905-920):
  - For `If c`, both edges are analyzed under `c = true` / `c = false`.
  - `!c` flips. `&&`-true and `||`-false refine both operands. `&&`-false and `||`-true refine nothing.
  - A comparison refines either a loaded local (`Ranges`) or any already-computed integer value with a known range (`Values`, e.g. a builtin or a `let`).
  - The bound may itself be a range: `x < e` with `e ≤ hi` gives `x ≤ hi − 1`.
  - `!=` trims an endpoint.
  - Every ordering comparison also records a relation (l.731-756).
  - Contract, after the F1 fix (doc comment l.629-634): `None` ("edge infeasible") only when a refined range is **empty**. A condition the analysis cannot model returns the facts unchanged.
- **Joins:** interval hull per local and per value, and a local missing on either side is dropped (`join_ranges`, l.945-954). Relations are kept only if present on both sides (`join_values`, l.956-976). Branches that do not fall through (`break`, `continue`, `return`, `kill`) are excluded from the join (l.307-324).
- **Side effects:**
  - `Store` of a value with a known range: strong update, and relations naming that local are dropped.
  - `Store` of an unknown value, or a store through a pointer into the local: reset to ⊤ (l.357-367).
  - `Atomic`: reset (l.368-372).
  - `Call`: reset **all** locals, since pointers may alias them (l.373, 846-851).
  - `Switch`: locals mutated in any case are reset before and after (l.326-340).
- **Loops** (`analyze_loop`, l.383-416; `loop_range`, l.418-495):
  - Locals mutated anywhere in the body or continuing block are reset to ⊤ at the header.
  - The body is analyzed once, then `continuing` is analyzed from the body's fall-through state. There is no fixpoint iteration and no widening operator: ⊤ is the one-step widening.
  - A mutated local can get a closed-form **header interval** from one of three recurrence rules:
    - **Additive** `i = i ± c` (`c > 0` constant, the *only* mutation; `single_update`, l.1034-1068). The exit test is either `break_if` or the first `if` in the body whose branch ends in `break` and mentions `i` (`loop_guard`, l.1258-1284). It is rejected if a `continue` can precede that guard (`may_continue`, l.1243-1256, fix `5365d84`). The exit bound must be an **exact constant** (`comparison` + `exact_constant`, l.1300-1329). `recurrence_ranges` (l.1331-1374) computes the iteration count, the active range, and the exit value. `Equal` exits require `distance % step == 0`, a reachability condition. If the guard is at the top, the exit value is joined in.
    - **Geometric** `i = i*c` (`c ≥ 2`) or `i = i << k`: `[start, max(start, last)·c]`, rejected if that product overflows (`geometric_range`, l.1149-1166; feature 006).
    - **Shrinking** `i = i >> c` or `i = i / c`: `[0, start]` for `start ≥ 0` (l.440-442; feature 005).
  - The start value must be exact: the incoming range is a singleton, or there is a constant `Store` before the guard (l.428-437).
  - After the loop, all mutated locals are reset to ⊤ (l.346-349).
- **Pipeline integration:**
  - `valid/function.rs:1943` calls `elide_bounds_checks(fun, module, &mut info)` during validation.
  - `proc/index.rs:344` (`access_needs_check`) and `back/spv/index.rs:522` skip the check for accesses in `safe_accesses`, under every policy. This is what answers R2's "Restrict interaction": the proof is consulted before any policy is applied.
  - MSL and HLSL writers call the same helper. RECORDED line numbers `msl/writer.rs:832`, `hlsl/writer.rs:3961` at `83a90e0`; not re-checked here.

**Draft pseudocode for the paper and poster** (condensed from the lines above; about 30 lines)

```
state  R : Local → Interval        (flow-sensitive; ⊤ = full 32-bit range)
       V : Expr  → Interval,  Rel : set of (a ≥ b | a > b)
init   R[v] = range(init(v)) if constant init else ⊤ ;  V[lid] = [0, wx·wy·wz−1]

analyze(block, R, V):
  for s in block:
    Emit(e):      V[e] = eval(e, R, V);  if e = Access(base,idx) ∧ len(base) known
                                           ∧ fits(eval(idx), len(base)): mark e safe
    If(c,A,B):    FA = analyze(A, refine(c,true,R,V));  FB = analyze(B, refine(c,false,R,V))
                  (R,V) = join of the edges that fall through     // break/continue/return excluded
    Store(v,x):   R[v] = eval(x) if known else ⊤;  drop Rel mentioning v
    Call/Atomic/Switch: reset every local they may write (Call: all)
    Loop(body,cont,brk):
                  H = R with every mutated local := ⊤
                  for mutated v: if recurrence(v) = (start, step|factor|shrink, exit)
                                   then H[v] = closed-form header interval
                  F = analyze(body, H);  analyze(cont, F.fallthrough ? F : H)
                  R = R with every mutated local := ⊤
eval: interval arithmetic with checked endpoints (overflow ⇒ unknown ⇒ check kept);
      a − b uses Rel (a ≥ b) ⇒ [0, max a − min b];  x % c, x & m, x >> k, min(x,c) bound unknown x
refine(a op b, t): intersect the subject's interval with {x | x op' bound} (bound may be a range);
      record Rel; return ⊥ (edge infeasible) only if an interval becomes empty
```

**Worked example: the tree reduction** (`reduce_tree`, workgroup 256, `s: array<u32,256>`). The IR is RECORDED in `BCE_epoch_0_section_0.2.md` l.180-210. The derivation below is my reading of the code, not an execution.

```wgsl
for (var stride = 128u; stride > 0u; stride = stride >> 1u) {
  if (lid < stride) { s[lid] = s[lid] + s[lid + stride]; }
  workgroupBarrier();
}
```

| Program point | Abstract state | Rule (lines) |
|---|---|---|
| entry | `lid ∈ [0,255]`; `stride = [128,128]` | `argument_range` 798-810; `default_ranges` 245-260 |
| loop header | `stride ∈ [0,128]`: its only update is `>> 1` | shrinking rule 440-442 |
| after `if stride > 0u {} else {break}` | `stride ∈ [1,128]` | `refine` on the accept edge |
| inside `if (lid < stride)` | `lid ≤ max(stride) − 1 = 127` gives `lid ∈ [0,127]`; `Rel: stride > lid` | `comparison_fact` range-valued bound 713-718; `relation` 731-756 |
| `s[lid]` | `[0,127]` fits 256 | `prove_access` |
| `s[lid + stride]` | `[0,127] + [1,128] = [1,255]` fits 256 | `add` 85-97 |

Recorded binaries (the SPIR-V figure for the poster), from `BCE_epoch_0_section_0.3.md` l.292-322, taken from `runs-v4/20260929T125306Z-reduce_tree/artifacts/`:
- `no_bce` has 4 `UMin` clamps, including `UMin (lid+stride) 255`.
- `bce` has 0, and `bce.spv == unchecked.spv` (same sha256 `f97b7fe6…`).
- Both keep 1 `OpULessThan`, which is the *user's own* `lid < stride`.

The v4 gain from this one kernel is 1.0131 on NVIDIA and 1.0733 on Intel.

The relational companion example, for the "relations" panel, is the Hillis–Steele scan (commit `19ffb4e` message):
- `if (lid >= off) { … s[lid - off] … }`.
- Plain interval subtraction underflows. The relation `lid ≥ off` gives `lid − off ∈ [0, 255 − min(off)]`.
- The v4 gain is 1.0086 on NVIDIA and 1.0365 on Intel.

**Soundness argument, to state as an argument rather than a theorem**
- Invariant: at every program point, `R[v]` contains every value `v` can hold on any execution reaching that point, `V[e]` contains the value of `e`, and every relation in `Rel` holds.
- Each rule preserves it:
  - checked arithmetic never produces an interval narrower than the true result set; overflow gives no fact;
  - refinement intersects with a condition that holds on that edge;
  - joins take hulls over every edge that falls through;
  - every write site resets or strongly updates;
  - loop headers use ⊤ or a closed form proved by induction on iterations from `start`, `step` and the exit test, with the `continue`-before-guard exclusion.
- `prove_access` then implies `0 ≤ idx < len`.
- Supporting evidence: unit/property sweeps comparing interval operations against enumeration (l.1562-1941), must-retain stress tests, and diff-exec.
- Honest record: two violations of this invariant were found (F1: unknown treated as infeasible; `continue` before guard) and fixed with regression tests. The `continue`-into-`continuing` join (Q1) is where this argument has a visible hole in the code as read.

**R4's six questions: exact answers** (current code; each points to lines above)

1. *"What kind of index expressions do you consider?"*
   - Any non-constant index expression over 32-bit integers built from: literals and constants; `var` locals; `let` values; `local_invocation_index` and `local_invocation_id` components; `+ − × / % << >> &`; `min`, `max`, `clamp`, `select`; int casts.
   - The array must have a compile-time length: fixed arrays in workgroup, function, private or uniform space, plus vectors and matrices.
   - Runtime-sized `array<T>` is never proven.
2. *"Only constant indices?"*
   - **No.** Constant indices were already skipped by upstream Naga (`proc/index.rs`, `try_resolve_to_constant`). Every proof this work adds is for a runtime-varying index.
3. *"How are you getting the values of the variables?"*
   - By abstract interpretation. Each variable carries an interval per program point, derived from its initializer and stores, and narrowed by the branch conditions that dominate the access.
   - Builtins get their interval from the WGSL spec guarantee and the `@workgroup_size`.
   - Loop variables get a closed-form interval from their recurrence.
   - Values read from buffers are unknown, but operations like `& 63u`, `% n` and `min(x, c)` still bound the result.
4. *"Only loop induction variables, no other variables?"*
   - **No.** Any variable or value with a derivable interval is used.
   - Loops need not be involved: `if (i < 8u) { t[i] }` with arbitrary `i` is proven by the guard alone.
   - Two variables can be related (`lid < stride`, `lid ≥ off`).
   - Honest limits: guards compare against constants or range-bounded values, not symbolic buffer lengths. Facts do not cross function calls. WGSL `&&` guards currently do not refine (nested `if`s do).
5. *"What exactly do you mean by loop induction variable analysis?"*
   - In the submitted paper it meant syntactic loop-exit-bound extraction, and that name oversold it.
   - Now it means: for a loop-mutated local whose *only* update is `i ± c`, `i · c` / `i << k`, or `i / c` / `i >> c` (with `c` a constant), compute a closed-form interval valid at the loop header from the start value and the exit test. Every other loop-carried variable is widened to ⊤ in one step; there is no fixpoint iteration.
6. *"Identifying recurrences for induction variables?"*
   - Yes, for basic recurrences with constant step and constant exit bound, in either direction, including `==` exits (with a divisibility check) and `break if`.
   - Not included: SCEV-style chains of recurrences, derived induction variables (handled only because `2*i+1` is evaluated at the use), multiple-update variables, and symbolic trip counts.
   - Note that for a top-of-loop guard, the path-sensitive refinement supplies one side of the interval and the recurrence supplies the other (e.g. `i ≥ start` for signed `i`).

**R3 ("template driven … unclear if the selected patterns cover enough")**
- Index expressions are no longer templates: they go through compositional transfer functions.
- The loop recurrence rules still are templates, and the paper should say so.
- Coverage is answered by measurement: the census funnel from Q2, and the v4 residual-cost column showing that `bce/unchecked` ≈ 1.000 after `19ffb4e` on all synthetic kernels.

**R2 corner cases**
- Restrict interaction: the proof is consulted before any policy (`proc/index.rs:344`).
- Syntactic fragility: replaced by the interval domain.
- Overridable workgroup size: the placeholder-1 / re-validation facts in Q1. No pinned test.

### Inferences

- The single most important presentation fix for R4 is a two-column "rule table" (IR construct → abstract effect → code lines) plus the worked-example table. Together they make the analysis checkable in about 30 seconds of poster reading.
- The paper should rename §3.2 from "Loop induction variable analysis" to "Loop recurrences (closed-form header intervals)". It should say explicitly: "no fixpoint iteration; ⊤ widening for other loop-carried locals".

### Gaps

- MSL/HLSL line numbers are RECORDED at `83a90e0` and were not re-checked at `19ffb4e`.
- The worked-example derivation is code-reading. Only the final proof set (`safe_accesses = [3, 19, 21, 26]`) and the binaries are recorded artifacts.

---

## Q4. Which figures and tables best answer the reviews on an A0 poster

### Takeaway

Eight panels, each tied to a reviewer:
1. updated pipeline figure;
2. R4 Q&A box;
3. rule table and pseudocode;
4. worked example with before/after SPIR-V;
5. coverage funnel;
6. per-GPU performance (synthetic, then real-world);
7. related-work comparison;
8. soundness, limits and threats.

Two of the three lost figures describe the old algorithm and should be replaced. Only the pipeline figure is worth recovering, and it too needs edits.

### Cited Findings

- **Poster format (VERIFIED, [IICT 2026 Speakers Manual](https://compilertech.org/assets/docs/IICT-2026-Speakers-Manual.pdf), fetched 2026-10-01).**
  - "POSTERS **A0** 841 × 1189 MM · PORTRAIT".
  - "Upload it by 28 September and we will print it, or bring your own printed copy. The boards at the venue are set up for this size."
- **Lost figures.**
  - v1 referenced `/tmp/naga_pipeline.pdf`, `/tmp/builtin_index_safety.pdf` and `/tmp/loop_induction.pdf` (v1 .tex diff; VERIFIED).
  - The rendered figures survive as vector drawings inside [static-analysis-naga-iict.pdf](/home/bb/Projects/Dave-Sonar-WGPU--master/static-analysis-naga-iict.pdf), pages 2-4. `pdfimages -list` reports no raster images, and `pdftotext` shows the figure labels (VERIFIED).
  - Figure 1 (pipeline) is therefore recoverable by cropping page 2. Its content is outdated: it shows "safe_indices HashSet" and only the SPIR-V backend.
  - Figures 2 and 3 depict the superseded algorithms.
  - `BOUNDS_CHECK_ELISION_TODO.md` §6 still lists "Regenerate the three lost figure PDFs" and "Replace the inaccurate interval-analysis figure" as open.
- **Proposed panels, with artifacts** (numbers to be copied from the named artifacts at print time):

| Panel | Answers | Content | Source artifact |
|---|---|---|---|
| A. Problem and policies | R2 ("Restrict interaction"), context | WGSL mandates checks; per-backend policy table (Vulkan index Restrict, buffer Unchecked with RBA2; Metal Restrict/Restrict; GLES Unchecked) | gaps report §2.3; epoch §0.1 |
| B. Pipeline (redrawn Fig. 1) | R2 "genuine artifact", R4 | WGSL/GLSL/SPIR-V front ends → Naga IR → Validator: `elide_bounds_checks` (`valid/function.rs:1943`) → `FunctionInfo.safe_accesses` → `access_needs_check` (`proc/index.rs:344`) → SPIR-V/MSL/HLSL writers | code (VERIFIED) |
| C. R4 Q&A box | R4 ×6 | The six answers from Q3, one line each, with "NOT constant indices" in bold | Q3 above |
| D. Rule table plus pseudocode | R3 "presented better", R4 "sketchy" | IR construct → abstract effect → code lines; the 30-line pseudocode | Q3 above |
| E. Worked example | R4, R2 | reduce_tree source; abstract-state table; SPIR-V `UMin` ×4 → 0, `bce.sha256 == unchecked.sha256` | epoch 0.2 B.2; epoch 0.3 l.292-322 |
| F. Coverage funnel | R3 coverage, R1/R2 evaluation | 729 real-world accesses → ≤320 provable in principle → 78 → 105 (F1) → 118 (005/006). Residual bar: 46.2% host-bounded storage, 10.7% helper params, 6.7% vertex/instance, … Add the naga-test corpus 74.1% **with** "20.4% without 3 matrix tests" | ledger; gaps report §2.2 and §4; census `F006_*` |
| G. Performance | all four ("Experimental Evaluation 0") | G1: v4 synthetic, per GPU, gain plus leftover-cost bars (NVIDIA/Intel). G2: vello scene-level ratios, Intel vs NVIDIA (dot plot of 34 scenes), with suite geomeans and the web-content scope line and the campaign status line | Benchmark v4 addendum; real-world report; `98bd514` pre-registration |
| H. Related work | R1, R2 (no BCE prior art), novelty 0 ×2 | Comparison table: this work vs Tint IRA vs ABCD / W07 / IRCE / Go prove+loopbce (the audit §4.2 table, updated because div/mod/shift/min/max are now ✔ after `2ffe1b7`) | audit §4.2; related-work-bce.md |
| I. Soundness and limits | R2 corner cases, R1 "WIP" | Evidence stack; "two unsound cases found by audit, both fixed with regression tests"; threats (Q5) | ledger |

- **Audit §4.2 table update.** The audit's "div / mod / shifts / min / max ✘ (C3)" row is **outdated**. `2ffe1b7` added those operators (VERIFIED in code at l.545-586). The "Path-sensitive refinement … on `var` only" row is also outdated: `Value` targets exist since `2ffe1b7` (l.780-796).

### Inferences

- For a poster, the funnel (F) and the per-GPU split (G) carry most of the argument. They show honestly that the technique is precise where it can be (all synthetic clamps removed) and that the ceiling is structural.
- The before/after SPIR-V excerpt should use the production `Restrict` (`UMin`) form, not the RZSW diamond of the submitted Fig. 4. That matches what wgpu actually emits.

### Gaps

- I found no IICT guidance on poster content or template beyond size and orientation.
- Whether the confirmatory campaign produces printable G2 numbers before Oct 2 is UNKNOWN.

---

## Q5. Status of related work, threats to validity, lost figures, teaser video, poster logistics

### Takeaway

State of each item:
- **Related work:** researched and verified, but still absent from the paper text.
- **Threats to validity:** absent from the paper text; enough recorded material exists to write it.
- **Lost figures:** recoverable from the submitted PDF; mostly obsolete.
- **Teaser video:** its deadline passed (the website says Aug 26; the project record says Aug 27). No artifact on disk.
- **Poster:** the organizer print-upload deadline (Sep 28) has passed, so the author must bring a printed A0 portrait poster. No `poster/` directory exists.

### Cited Findings

- **Related work.**
  - [related-work-bce.md](/home/bb/Projects/Dave-Sonar-WGPU--master/research_notes/related-work-bce.md), verified 2026-09-20, supports statements on: ABCD (inequality graph, on demand); W07 (title and authors only); IRCE (three-segment iteration-space split); Go `prove` (signed+unsigned `Limit` with `SafeAdd`/`SafeSub`); Go `loopbce`; GSOHC (not BCE); Readyset (651 checks removed, "entirely explainable by measurement noise"); the Rust Perf Book; Cousot '77; wgpu#6345. RECORDED.
  - Tint IntegerRangeAnalysis and wgpu#4360 were fetched 2026-09-27 (audit §4.1). They are **not in any related-work draft**.
  - `CITATION_MANIFEST.md` and v2.tex: 0 hits (audit, RECORDED).
  - Still missing or metadata-only: W07 mechanism pages, Suzuki & Ishihata, Halide (related-work-bce.md "Gaps").
- **Threats to validity: material for the section** (all RECORDED; paths in Q2):
  - *Internal (soundness):*
    - informal argument, not a mechanized proof;
    - two unsound cases found by audit and fixed;
    - the CANDIDATE continue/continuing case is untested;
    - front-end-dependent precision (WGSL `&&`);
    - override correctness relies on re-validation, with no pinned test;
    - diff-exec covers 6 execution-meaningful fixtures.
  - *Construct:*
    - v3 timing measured wgpu-recompiled binaries (invalid);
    - the paper's word counts and Fig. 4 use RZSW, not production policy;
    - `OpULessThan` counting also counts user comparisons (epoch 0.3; opcode-166 incident).
  - *Conclusion:*
    - NVIDIA real-world mostly "not resolvable";
    - first-campaign clock-state flaw (pre-registration 2);
    - an order effect of up to about 0.8% in v4;
    - v4's ±0.5% margin was declared after the first run, but before kernel results were inspected;
    - one Intel control deviated (1.0100).
  - *External:*
    - two GPUs in one laptop, Vulkan only;
    - native vello runs unchecked;
    - the synthetic kernels are the ones used to find the gaps (selection bias, stated in the v4 addendum);
    - census limits: bevy partial (10/39 compute shaders), vello `#ifdef`s unevaluated, runtime-generated ML shaders (TF.js/ORT Web) absent;
    - the naga-test corpus is inflated by 3 matrix tests.
- **Lost figures:** see Q4. Only Fig. 1 is worth recovering.
- **Teaser video.**
  - The [IICT 2026 home page](https://compilertech.org/) (VERIFIED 2026-10-01) lists "26 August 2026, 11:55 PM IST — 90-second video upload (accepted and Round 2 papers)".
  - [02-REVIEWS.md](/home/bb/Projects/Dave-Sonar-WGPU--master/IICT_CONTEXT/02-REVIEWS.md) records the decision email's date as "Aug 27, 11:55 p.m.".
  - That is a discrepancy between the two sources. Both dates have passed. No video artifact is on disk (RECORDED, UNKNOWN whether made elsewhere).
- **Other IICT dates** (VERIFIED, same page):
  - "28 September 2026 Poster upload";
  - "30 September 2026 Presentation slides upload";
  - workshop "2 – 3 October 2026", IISc Bengaluru.
- **Tracks and proceedings.**
  - The submissions page says Research Papers are "Extended abstract 2 – 4 pages / Full paper 4 – 8 pages" and "Some submissions may be accepted as posters" ([compilertech.org/submissions](https://compilertech.org/submissions), VERIFIED).
  - I found no statement about camera-ready versions or proceedings for regular accepted papers. The camera-ready mention applies to the premier-venue special session only.
- **Poster on disk:** `/home/bb/Projects/Dave-Sonar-WGPU--master/poster` does not exist (VERIFIED, `ls` error). T7 is not started per the status report of 2026-09-27; nothing later records otherwise.
- **Paper v3:** does not exist (status report §3). `BOUNDS_CHECK_ELISION_TODO.md` §6 has 7 unchecked paper items (VERIFIED).

### Inferences

- Given one day to the workshop, the poster is the binding deliverable (organizer: "address all the review comments … in the final poster", 02-REVIEWS.md). A v3 paper appears optional unless IICT publishes proceedings, which is UNKNOWN.
- Recommended order:
  1. Run the continue/continuing probe. If it is red, the soundness panel must say "three unsound cases found by audit" with the fix status stated plainly.
  2. Assemble the poster from panels A–I.
  3. Then write v3 with the same structure.
- Recommended v3 section plan:
  1. Introduction and contribution: non-constant indices, Naga/wgpu, measured.
  2. Background: WGSL mandate, policies, where checks are emitted.
  3. The analysis: domain, transfer functions, refinement, loops, proofs, integration; pseudocode plus the worked example.
  4. Soundness: argument, found-and-fixed cases, tests.
  5. Evaluation: coverage census, then synthetic v4, then real-world, then correctness oracles.
  6. Related work, including Tint IRA and wgpu#4360.
  7. Threats to validity.
  8. Limitations and next steps: `arrayLength` facts, check dedup for Metal, interprocedural.

### Gaps

- It is UNKNOWN whether IICT publishes proceedings or accepts a revised paper.
- It is UNKNOWN whether the organizers accept late teaser videos.
- It is UNKNOWN whether a poster was drafted outside this repo.
- Which of the two video dates is authoritative (Aug 26 on the website, Aug 27 in the email as recorded) is UNKNOWN. It is moot now.
