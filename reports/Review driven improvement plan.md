# Confirmed unsoundness reorders paper #55's fixes

> **Status update, 2026-10-01 (after this report was written; VERIFIED, executed):**
> - **S-1 to S-5 and most of S-6 are DONE** in wgpu `bce/sound-branch-refinement`: `0ca293a` fixes S1 (versioned loads plus guard freshness), S2 (`continue` states joined into `continuing`), S3 (in-body re-initialisation rule removed) and override-sized workgroups (builtin ranges unknown until overrides resolve). `f73a687` removes the `sdata_manual` exception and the dead index-keyed `safe_indices`.
> - **Regression tests:** six new tests are red at `19ffb4e` and green after the fix (`IICT_CONTEXT/logs/fix-s1s2s3-red.log`). The full naga suite passes: 143 lib, 309 integration, 6 doc; clippy is clean.
> - **Still open in S-5 and S-6:** the elision toggle is lost in `process_overrides`, and the wrong-default doc comment (`proc/index.rs:28-29`).
> - **S-8 reached its success branch.**
>   - Real-world census: 118 → **118** of 729 proven, with 0 changed accesses.
>   - Production screen: 147 entry points, identical clamp counts.
>   - Benchmark shaders (20 vello, blur, 7 v4 kernels): identical clamp counts.
>   - The fix only removes proofs, so identical counts mean identical proof sets. **No measured result depended on S1–S3**, and the `19ffb4e` benchmark numbers stand for the fixed compiler. Re-running v4/realworld-2 is therefore not needed to keep them; binary-hash identity is not separately checked.
> - **Soundness wording for the poster:** "Unsound cases found by audit and fixed with regression tests; none affected any measured binary". The analysis is **not** "proven sound": only S-7 (`discard` on Metal/D3D) remains open among the soundness items, and there is no mechanised proof.


The bounds-check elision at wgpu fork `19ffb4e` is **unsound**, so it must not be called sound anywhere: on the poster, in a paper, or in the ledger. Six probe shaders, executed on 2026-10-01, each contain an access that goes out of bounds at runtime. Every one of those accesses was proved safe, and under wgpu's production `Restrict` policy the SPIR-V clamp was deleted (`UMin` 1 → 0) ([confirmed results][CR]). The six probes reduce to three open root causes: **S1**, stale-Load identity, which idiomatic GLSL `i++` also reaches; **S2**, `continue` edges dropped before `continuing`; **S3**, the in-body initializer rule. This reverses the 2026-09-27 audit's "no unsound elision found" ([audit][AU] §3 C7). It also makes the ledger's "two unsound cases found by audit, both fixed" inaccurate. So the soundness fixes are **priority 0**, and every performance number, including the confirmatory campaign that finished today, measures binaries that contain these bugs. The confirmatory campaign still replaces the first-campaign headline: the vello suite gains **+0.59% on NVIDIA** (22 GAIN / 4 LOSS / 6 EQUIVALENT / 2 INVALID) and **+7.01% on Intel** (22 GAIN / 10 LOSS). Two Intel scenes crashed, and a slot-order imbalance biases the control correction ([confirmed results][CR]). On novelty, the technique is not new. Chrome's Tint `IntegerRangeAnalysis` has been developed since 2024-12 and is default-enabled on Chromium main. It is narrower than this work, and this work's "relational facts" amount to one `a ≥ b` fact, used only to bound `a − b` ([related-work note][RW]). The binding deliverable is the poster the organizers asked for, and it does not exist on disk. The conference is recorded as 2026-10-02/03 and today is 2026-10-01. The print-upload deadline (Sep 28) and the video deadline have passed, so the author has about 24 hours to produce and print an A0 portrait poster that answers all 48 review points without claiming soundness ([presentation note][PP] Q4–Q5).

**Scope and method.** This report synthesizes the six notes in `research_notes/Review driven improvement plan/` together with the earlier audit ([audit][AU]). The confirmed-results note overrides any "HYPOTHESIS" or "pending" statement in the other notes about the same items. Evidence labels follow `AGENTS.md` Rule 0:

- **VERIFIED**: checked on 2026-10-01 in this research round, by executing a probe, reading code or reading an artifact. The note holding the check is cited. "VERIFIED (writer)" marks the items I re-read myself while writing: the two `realworld-2` summary JSONs, the v4 report and `02-REVIEWS.md`.
- **RECORDED**: stated in a project document and not re-checked.
- **HYPOTHESIS**: hand-traced in code, not executed.
- **UNKNOWN**: nobody has established it.
- Effort figures are copied from the notes. Where a note gives none, the report says "not estimated".

## Key findings at a glance

| # | Finding | Label | Source |
|---|---|---|---|
| F1 | S1, S2 and S3 are unsound on `19ffb4e`. Across the six probes, a genuinely out-of-bounds access is proved and its `Restrict` clamp is removed (`UMin` 1 → 0 on every emitted WGSL probe). The GLSL `i++` probe is proved but not emitted | VERIFIED (executed) | [confirmed results][CR] §1; [probes][PR] |
| F2 | Override workgroup size: the pre-override `ModuleInfo` proves `a[l.x]` on a length-1 array, because the WGSL lowerer stores a placeholder size of 1. The production wgpu path re-validates, so this is a latent hazard, not a shipped bug. Re-validation also silently re-enables elision when the caller disabled it | VERIFIED (executed; toggle by code reading) | [confirmed results][CR]; [soundness note][SG] Q1 |
| F3 | Whether any proof used in the measured benchmarks (v4 kernels, vello's 102 removed clamps) depends on S1–S3 is **UNKNOWN** until the fixes land and the census and production screen are re-run | UNKNOWN | [confirmed results][CR] §1 |
| F4 | Confirmatory campaign under the pre-registered rule v2: NVIDIA vello suite 1.0059 (+0.59%); Intel 1.0701 (+7.01%). This replaces the first-campaign headline (Intel +6.71% over all 34 scenes, +8.48% over 27 unflagged) | VERIFIED (writer re-read both JSONs) | [NVIDIA summary][NV2]; [Intel summary][IN2]; [confirmed results][CR] §2 |
| F5 | 14 of NVIDIA's 22 vello GAIN verdicts are below +0.5%, and 3 of its 4 LOSS verdicts lie within −0.15% (0.9985–0.9987). Rule v2 labels statistically significant but practically negligible effects as GAIN/LOSS | VERIFIED (writer, computed from [NVIDIA summary][NV2]) | [evaluation note][EV] Q3 |
| F6 | Tint IRA: first commit 2024-12-19. It is `FEATURE_ENABLED_BY_DEFAULT` on Chromium main, but the exact Chrome version where it became default is UNKNOWN. It is narrower than this work: no general branch refinement, no relational facts, ±1 loops only | VERIFIED (source read) | [related-work note][RW] Q3, Q5 |
| F7 | The "relational facts" are minimal: a single `Relation{big, small, strict}`, used only by `related_difference` to bound `a − b`, with no transitive closure | VERIFIED (code read) | [related-work note][RW] Q1 |
| F8 | The code and artifacts have moved on since the paper; the paper and poster have not. v2.tex has not changed since 2026-08-28 and no poster file exists. v2 still prints "0.0 ns", 74.1% without its qualifier, and unqualified safety claims | VERIFIED | [status matrix note][RS] Q4; [presentation note][PP] Q1 |
| F9 | The flow layer is where the analysis is weak. The arithmetic layer is swept exhaustively, but the flow layer has only 89 hand-written shapes, with no let-snapshot, postfix or `continuing`-access tests | VERIFIED | [soundness note][SG] Q6 |
| F10 | Coverage: 118 of 729 real-world runtime accesses are proved, against a ceiling of about 44% (320/729). On the first campaign, elision recovered about 99% of the measured check cost on Intel vello and about 48% on NVIDIA (exploratory) | RECORDED / computed in note | [evaluation note][EV] Q4 |

## Six probes prove 19ffb4e unsound, so soundness becomes priority 0

The three bugs are all in the flow layer, not in the interval arithmetic.

**S1: stale-Load identity.** The analysis maps any `Load(x)` in a comparison to the *current* value of the local `x`, even if a store to `x` came between the load and the comparison ([soundness note][SG] Q2, `bounds_check_elision.rs:786-791`, `:673-676`). Three probes hit this:

- `s1_let_snapshot.wgsl`: `var i = xs[0]; let small = i < 4u; i = xs[1]; if (small) { t[i] = 1u; }` proves `t[i]`.
- `s1_loop_snapshot.wgsl`: a loop guard tests a snapshot taken before the update.
- `s1_postfix.comp`: GLSL lowers `i++` as load, store, then return the *old* value, so `if (i++ < 7u) { t[i] }` is proved while `i` reaches 7 on a length-7 array.

**S2: `continue` edges dropped.** The `continuing` block is analysed only from the body's fall-through state, so a state that reaches `continuing` via `continue` is never joined in (`:402-415`, `:363-371`, `:296-316`). Two probes (`s2_continue.wgsl` and the probe from the paper notes, `s2_continue_paper.wgsl`) prove accesses that `continue` edges reach out of bounds.

**S3: in-body initializer rule.** A constant store inside the loop body counts as the loop's start value, without being joined with the incoming value (`:1059-1065`, `:428-438`). `s3_initializer.wgsl` resets `i = 100u` inside the loop, and `t[i]` on a 9-element array is still proved. The rule was introduced in `8757d7d`, and no front-end shape that needs it was found ([soundness note][SG] Q4).

All six probes are **VERIFIED by execution** with `bce-epoch-tools` built against `wgpu/naga @ 19ffb4e` ([confirmed results][CR] table). Some traced shapes were never turned into probes and remain **HYPOTHESIS**: the S1 relation-key trace (`t16[a - i]`), the S1 `continuing { … break if d; }` variant, and the second S2 trace ([soundness note][SG] Q2, Q4).

**Why this is serious.** Proofs are consulted before any bounds-check policy is chosen: the SPIR-V consumer returns before `choose_policy` (`back/spv/index.rs:518-530`), and MSL/HLSL go through `access_needs_check` (`proc/index.rs:321-351`). So under shipping `Restrict` a wrong proof turns a clamped access into undefined behaviour. S1–S3 are therefore **memory-safety regressions**, not precision misses ([soundness note][SG] Q3). They also overturn two earlier records:

- R2's "syntactic-match fragility" was marked closed in `03-REVIEW-RESPONSE-STATUS.md` row 8. It is open again: loop recognition and refined-operand identity are still shape-matched, and S1 is a shape-match hole.
- The full soundness history is **three earlier holes fixed with regression tests** and **three confirmed open**. The fixed ones are the signed-IV negative start (RECORDED, Aug 25), `9c4615b` ("unmodelled condition treated as infeasible") and `5365d84` (continue before the guard) ([status matrix note][RS] R2.S3).

**Latent hazards found by code reading, not executed.** Two more items share the priority tier:

- **`safe_indices`.** Both consumers consult it, but it is never populated. Because it is keyed by index rather than by access, it would transfer proofs across arrays if anything ever filled it (`valid/analyzer.rs:252,1269,1407`).
- **Kill/discard.** The analysis treats `Kill` as a terminator. MSL emits `discard_fragment();` and HLSL emits `discard;`, neither followed by a return, and whether execution continues after them on Metal/D3D is UNKNOWN ([soundness note][SG] Q3–Q4).

The override case is safer than S1–S3. Every production path calls `process_overrides`, which re-validates with the real sizes: wgpu-hal `vulkan/device.rs:770`, `metal/device.rs:157`, `dx12/device.rs:349`, `gles/device.rs:316`, plus naga-cli and the snapshot tests. The fix is about one line: treat `workgroup_size_overrides.is_some()` as an unknown size, which is Tint's contract ([soundness note][SG] Q1).

**Branch for the red tests.** The notes wrote the Rule 3 branch in advance, and the branch that happened is **"success" for the red tests**: they are red at `19ffb4e` ([confirmed results][CR] §1). The pre-written **disaster** branch still applies: if a red test touches a measured benchmark kernel, stop and downgrade the soundness wording. That cannot be ruled in or out until the fixes land (F3).

## Confirmatory campaign replaces the headline: +0.59% NVIDIA, +7.01% Intel

The campaign ran under pre-registration 2 (commit `98bd514`): the batched `realworld-2` harness with verdict rule v2. It ran NVIDIA ×10 from 18:23 to 20:16Z and Intel ×5 from 20:17 to 22:22Z on 2026-09-30 ([campaign log][LOG2]).

| GPU | vello suite geomean (control-corrected) | GAIN | LOSS | EQUIVALENT | INVALID | imageBlur (contract params) |
|---|---|---|---|---|---|---|
| NVIDIA RTX 4060 Laptop | **1.0059 (+0.59%)**, 32 scenes | 22 (1.0007–1.0545) | 4 (emoji 0.9987, robust_paths 0.9985, base_color_test 0.9987, image_luminance_mask 0.9941) | 6 | 2 (cardioid_and_friends: 2 runs excluded; longpathdash butt: 4 excluded) | GAIN 1.0012 (CI 1.0006–1.0019) |
| Intel RPL-S iGPU | **1.0701 (+7.01%)**, 32 scenes | 22 (1.0148–1.3452) | 10 (0.9501–0.9940) | 0 | 0 | GAIN 1.0034 (1.0033–1.0035) |

Source: [confirmed results][CR] §2; values re-read from [NVIDIA summary][NV2] and [Intel summary][IN2] (VERIFIED, writer).

**The harness fix worked.** The pre-registered success criterion was an NVIDIA per-round control spread reduced at least 5×. The pilot met it: the spread fell from 15% to 0.6–2.7%. NVIDIA scenes are now mostly resolvable: 26 of 34 got a GAIN, LOSS or EQUIVALENT verdict, against about 4 RESOLVED under the old rule ([confirmed results][CR]).

**What the NVIDIA result means.** End-to-end elision on vello is essentially neutral on NVIDIA. Counting the verdicts in the NVIDIA summary gives:

- **14 of the 22 GAIN verdicts are below +0.5%**, for example `17-labyrinth` at 1.0007.
- **3 of the 4 LOSS verdicts are within −0.15%.**

With 38 workloads × 2 GPUs = 76 intervals, roughly 4 false GAIN/LOSS labels are expected by chance under a global null at 95%, and rule v2 applies no multiplicity control ([evaluation note][EV] Q3).

**What the Intel result means.** Intel shows real gains, but also 10 losses of up to −5% (`robust_paths` 0.9501). In the first campaign, the Intel losses coincided with `no_bce/unchecked` < 1, which suggests driver effects rather than elision itself (RECORDED, [related-work note][RW] Q4). That has not been re-checked for the confirmatory data.

**Three deviations must travel with these numbers.** They are recorded as `DEVIATION`s in the source notes ([confirmed results][CR] §2):

1. **Intel crashes.** `vello:00-splash_with_tiger` and `vello:32-luminance_mask` panicked in all 5 Intel runs with `assertion failed: ticks > 0`. These are the two smallest scenes, and the batched harness produces a zero-length timestamp interval on them. There is no Intel confirmatory data for either.
2. **Slot-order imbalance.** The cyclic rotation `order = (r + j) % 4` puts `no_bce` before `bce` in 3 of 4 rounds but `bce_dup` before `bce` in only 1 of 4. The control subtraction therefore *adds* any position effect instead of cancelling it. On the realworld-1 Intel data this effect had a consistent sign (IQR entirely above 0) ([evaluation note][EV] Q3). The sensitivity analysis is still pending.
3. **Unsound binaries.** Every binary measured was built from `19ffb4e`, which contains S1–S3. After the fixes, elision may remove fewer clamps, and the campaign must be re-run.

**The synthetic v4 suite is the mechanism evidence.** It remains valid as such, with the same soundness caveat. On `19ffb4e` every clamp in the 6 elidable kernels is removed, and `bce/unchecked` sits at 0.9998–1.0004.

| GPU | scan_shared | matmul_tiled | vello_binning | reduce_tree | prefix_scan_hs |
|---|---|---|---|---|---|
| NVIDIA | 1.1579 | 1.1080 | 1.1522 | 1.0131 | 1.0086 |
| Intel | 1.1161 | ≈1.0000 | ≈1.0000 | 1.0733 | 1.0365 |

Sources: [evaluation note][EV] Q1; the Intel scan_shared value is 1.1161 at `19ffb4e` (1.1162 at `83a90e0`), VERIFIED (writer) in [v4 report][V4] l.102.

Two scope limits apply to v4:

- The suite has a selection bias: the kernels are the ones used to *find* the gaps ([evaluation note][EV] Q1).
- One Intel control deviated to 1.0100.

**Coverage.**

- 118 of the 729 real-world runtime accesses are proved: 78 at base, 105 after the F1 fix, 118 after feature 005, and still 118 after 006.
- The structural ceiling is about 44% (320/729). 46% of accesses index runtime-sized storage buffers, 7% use vertex/instance indices and 3% hit binding arrays ([evaluation note][EV] Q1, Q4).
- The older 74.1% / 81.9% corpus rate falls to **20.4% / 15.7%** once three near-duplicate `mat_cx*` tests are removed. 396 of the 435 elisions come from those three files ([audit][AU] C1).

**Missing standard dimensions** ([evaluation note][EV] Q2):

- compile-time overhead (never measured);
- code or ISA size;
- per-stage vello timing (required by spec 005 FR-4 but not delivered);
- a third vendor or backend;
- an external Tint baseline;
- per-rule ablation;
- workgroup-size sensitivity.

## Tint shipped the narrower analysis first; the extra scope is where the bugs live

**The technique is not novel.** Every ingredient has published ancestors:

- interval abstract interpretation (Cousot & Cousot 1977);
- difference constraints on e-SSA ([ABCD, PLDI 2000][ABCD]);
- SSA index conditions with explicit loop-overflow handling (Würthinger 2007/2009);
- production passes in Go `prove`, LLVM IRCE/ConstraintElimination, HotSpot C2 and .NET RyuJIT ([related-work note][RW] Q1–Q2).

**Tint is the WGSL precedent.** Tint's `IntegerRangeAnalysis` began with commit `3bf3e9b20c` on 2024-12-19, "Implement range analysis on `LocalInvocationIndex`" ([Tint commit log][TINTLOG]). Chromium main sets `kWebGPUEnableRangeAnalysisForRobustness` to `FEATURE_ENABLED_BY_DEFAULT` ([finch features][FINCH], line 333), and the Chrome 141 post (2025-09-24) announced a progressive rollout ([Chrome 141][CHROME141]). The exact Chrome version where it became default is **UNKNOWN**, because the history page returned 403. Tint's scope, from reading its source ([related-work note][RW] Q3):

- Loops must match a rigid shape: a constant initializer, a ±1 update, and a body that starts with a load followed by a compare against a constant.
- There is no general branch refinement and no relational facts.
- Runtime-sized arrays are never elided ([robustness.cc][ROB]).

**Other WGSL compilers.** WebKit's WGSL compiler skips only constant indices ([WebKit BoundsCheck.cpp][WEBKIT]). wgpu#4360, which requests exactly this feature, has been open since 2021-09-15 ([wgpu#4360][W4360]). The paper cites none of these: grep finds 0 hits in any `.tex` ([status matrix note][RS] R1.8).

**The relational claim must shrink to what the code does.** The code has one `Relation {big, small, strict}` type (lines 731–756). It is used only by `related_difference` (764–778) to bound `a − b` in the `Subtract` case, with a constant of 0 or 1 and no transitive closure. That is "a minimal ABCD-style difference-constraint fact", not "relational analysis" ([related-work note][RW] Q1). Pentagons is the closest domain-level description; cite it only after reading the paper, since only its metadata has been checked.

**The central finding for the paper's framing.** The capabilities that exceed Tint are path-sensitive guard refinement, recurrences beyond ±1, and flexible loop starts. **Those are exactly where S1–S3 live** (inference from code; label HYPOTHESIS as a causal claim):

- S1 is a hole in refinement over loads. Tint avoids it by construction, because it requires the loop-variable load to be the first body instruction, immediately followed by the compare (`integer_range_analysis.h:166-181`).
- S3 is the in-body start rule. Tint requires a constant initializer block.
- For overrides, Tint asserts a constant workgroup size ([soundness note][SG] Q1–Q2).

So "strictly larger proof scope than Tint" is currently bought with soundness.

**What remains defensible**, in order ([related-work note][RW] Q5, adjusted by [confirmed results][CR]):

1. **The first range-based elision in Naga/wgpu.** The word "sound" must be dropped until S1–S3 are fixed.
2. **Broader scope than Tint on the documented axes**, stated as "by source inspection, not measured". It becomes a measured claim only after a head-to-head on the same corpus.
3. **A controlled per-GPU measurement of what elision recovers from WGSL's mandated robustness cost in a real application.** No peer-reviewed equivalent was found. The prior cost data points are informal: [gpuweb#1202][GPUWEB1202] reports 17–40% matmul regressions, and [Futhark][FUTHARK] reports a 4–6% geomean outside WebGPU. This carries "to our knowledge" and the unsound-binary caveat.
4. **An experience result.** The audit found six unsound root causes in a guard-refining analysis over a multi-front-end IR (GLSL `i++`, WGSL `&`, SPIR-V input), which argues for differential testing across front ends. This contribution got stronger today.

## The poster is due within about 24 hours and does not exist

**Logistics.**

- IICT requires "POSTERS A0 841 × 1189 MM · PORTRAIT". Posters uploaded by 28 September would be printed by the organizers; otherwise the author brings a printed copy ([Speakers Manual][MANUAL]).
- Deadlines that have passed: the upload (Sep 28), slides upload (Sep 30) and the teaser video (26 August on the website, 27 August in the recorded email) ([IICT][IICT]; [presentation note][PP] Q5).
- The workshop is on 2–3 October at IISc Bengaluru.
- No `poster/` directory exists. `find` found no poster or teaser file under `/home/bb` to depth 4, and one made elsewhere is UNKNOWN ([status matrix note][RS] O2–O3).
- Print lead time is **UNKNOWN** and is the critical path.
- The existing layout plan, `T7-POSTER.md`, is stale: it prescribes 74.1% / 81.9% as the headline ([presentation note][PP] Q1).

**Before the poster (next ~24 h): only work that changes what can be printed truthfully.** In order:

1. Correct the soundness wording in the records the poster will quote (S-9).
2. Build panels A–I (P-1 to P-7).
3. Run the forbidden-claims scrub (P-8).
4. Optional, only if they fit without endangering printing:
   - commit the six probes as red tests (S-1, hours);
   - add the one-line override guard (S-5);
   - compute SPIR-V size deltas from the saved `.spv` files (E-4, hours, no GPU).

**After the conference.** The S1/S2 fixes (1–2 days each), the S3 fix, the re-validation and re-measurement campaign (the confirmatory campaign alone took about 4 hours of wall clock), the statistics amendments, the Tint head-to-head, precision work, the testing infrastructure, and a v3 paper if one is needed. Whether IICT publishes proceedings is UNKNOWN ([presentation note][PP] Q5).

**Wording rules.** With S1–S3 open, the poster **must not claim soundness**. The soundness panel should say, in substance:

> The audit found six unsound root causes. Three were fixed with regression tests. Three are confirmed open at `19ffb4e`: stale-Load identity (including GLSL `i++`), `continue` edges into `continuing`, and the in-body initializer rule. Under wgpu's production `Restrict` policy the clamp on a genuinely out-of-bounds access is removed. The performance numbers below were measured on these binaries.

Numbers the poster must never print ([presentation note][PP] Q2):

- "0.0 ns", 7,568–8,192 ns, 29–31 ps and 105.03 / 111.03 ns;
- 74.1% without "20.4% excluding three matrix tests";
- "vello gets faster" without "as web content", because native vello runs `ShaderRuntimeChecks::unchecked()`;
- "fuzzed" (the sweeps are exhaustive enumeration);
- "verified on MSL/HLSL" (they are structurally tested only);
- the first-campaign vello numbers as headline.

## Review-point-to-action matrix: 48 points, none closed in paper or poster

All 48 points extracted from the four reviews, the scores and the organizer email are listed below. The source is `IICT_CONTEXT/02-REVIEWS.md` (RECORDED verbatim record; re-read by the writer), cross-checked against [status matrix note][RS] Q1.

**Status vocabulary.**

- **DONE**: code, artifacts and poster all reflect the point.
- **DICNIP**: DONE-IN-CODE-NOT-IN-PAPER. Code and/or artifacts close the point, but no poster or paper text does.
- **PARTIAL** or **MISSING**: as the words say.
- For praise, the evidence column also says whether the praised property still **HOLDS**.

**Tally:** DONE 2 (both need no action), DICNIP 18, PARTIAL 18, MISSING 9, UNKNOWN 1.

Action IDs refer to the implementation list in the next section.

| ID | Review point (type) | Status | Evidence (label; file:line / commit / artifact) | Action that closes it |
|---|---|---|---|---|
| R1.1 | Summary: "index safety analysis exploiting WGSL spec + loop induction variable analysis" (neutral) | DICNIP | Both parts exist, now as one interval analysis: builtin ranges `bounds_check_elision.rs:797-827`; abstract still shows the old two-analysis framing `v2.tex:104-125` (VERIFIED, [RS]) | P-3, V-2 |
| R1.2 | Praise: implemented in a production compiler | DICNIP (praise HOLDS-WITH-CAVEAT) | A fork, not upstream; no PR evidence (UNKNOWN); `sdata_manual` hack `back/spv/writer.rs:2824`; unsound at `19ffb4e` (VERIFIED, [RS], [CR]) | P-3 ("fork, not upstream"), S-6, S-2–S-4 |
| R1.3 | "More comprehensive evaluation would strengthen" | DICNIP (provisional: binaries unsound) | v4 7 kernels × 2 GPUs; realworld-2 confirmatory; census 729; corpus 295 (VERIFIED, [EV], [CR]) | P-5, S-8, E-3–E-8 |
| R1.4 | "Complete implementation of loop analysis would strengthen" | PARTIAL | Implemented through `86eb81c` … `19ffb4e`, 89 stress tests (VERIFIED, [RS]); S2, S3 and S1-loop are loop-rule unsoundness (VERIFIED executed, [CR]) | S-2, S-3, S-4, P-6 |
| R1.5 | Larger, more diverse benchmark suite | PARTIAL | Two laptop GPUs, Vulkan only; no AMD/Apple/Metal/D3D12 (RECORDED, [V4] "Honest limits") | P-5 (state limit), E-12 |
| R1.6 | Comparison with existing BCE techniques / prior work | PARTIAL | Literature collected; Tint capability table read from source ([RW] Q5); no head-to-head; no written section (VERIFIED) | R-1, R-3, P-7, E-5 |
| R1.7 | "Loop IV analysis appears to be work in progress" | PARTIAL | As R1.4 | S-2–S-4, P-6 |
| R1.8 | Novelty limited (score 0) | MISSING (positioning) | Tint IRA since 2024-12-19, default-on in Chromium main; wgpu#4360 (2021); 0 Tint hits in `.tex` (VERIFIED, [RW] Q3, [RS]) | R-3, P-7, R-1 |
| R1.9 | Evaluation lacks comparison with other approaches | PARTIAL | Internal baselines `no_bce` / `unchecked` / `bce_dup` exist (RECORDED [V4]); no external baseline | E-5, E-9, P-5 |
| R1.10 | Engineering 1 (implementation valued) | DICNIP (praise HOLDS) | 1,979-line analysis; 143 lib + 303 integration + 6 doc tests, 0 failed (`f006-full-test.log`, VERIFIED [RS]) | P-4 |
| R2.S1 | Praise: real deployed compiler (Naga → wgpu/Firefox) | DICNIP (HOLDS-WITH-CAVEAT) | Pass is in a fork; Firefox settings UNKNOWN; "used in Firefox" `v2.tex:100` uncited; wrong-default doc comment `naga/src/proc/index.rs:28-29` (RECORDED) | P-3, S-6, V-4, E-14 |
| R2.S2 | Praise: pipeline integration, "a genuine artifact" | DICNIP (HOLDS, strengthened) | `valid/function.rs:1943` → `FunctionInfo.safe_accesses` (`valid/analyzer.rs:254`) → `back/spv/index.rs:519-525`, `proc/index.rs:342-347`; MSL/HLSL structural tests pass; paper still says `safe_indices`, SPIR-V only (`v2.tex:284-292`) (VERIFIED [RS]) | P-4, S-6, T-6 |
| R2.S3 | Praise: spec-guarantee approach "clean, sound" | PARTIAL (builtin rule HOLDS; whole analysis NO LONGER HOLDS) | Checked multiply, product > 0 `bounds_check_elision.rs:801-806` (VERIFIED); S1–S3 open (VERIFIED executed [CR]); 3 earlier holes fixed (RECORDED/VERIFIED [RS]) | S-1–S-5, S-8, P-2 |
| R2.S4 | Praise: conservative gating (static wg size and length) | PARTIAL (HOLDS except override) | `argument_range` None without size `:801`; `IndexableLength::Known` `:619-624`; placeholder `[1;3]` `front/wgsl/lower/mod.rs:1644` gives a pre-override proof (`ov_workgroup.wgsl`, VERIFIED executed) | S-5, P-2 |
| R2.S5 | Praise: word counts + disassembly make zero-overhead checkable | DICNIP (HOLDS, stronger) | v4 `bce.spv` == `unchecked.spv` (same sha256; reduce_tree `f97b7fe6…`) (RECORDED [PP] Q3); paper uses non-production RZSW `v2.tex:186-205, 453-492` | P-6, E-4, V-2 |
| R2.W1 | Thin evaluation (one minimal shader + one matmul) | DICNIP (provisional) | v4 + realworld-2 + census (VERIFIED [CR], [EV]); binaries unsound | P-5, S-8 |
| R2.W2 | No real-world WGSL corpus | DICNIP | Census of 729 accesses over 9 projects (vello 440); vello + imageBlur benchmark (VERIFIED [EV]) | P-5 |
| R2.W3 | No hit-rate reporting | PARTIAL | Paper prints 74.1% without the 20.4% qualifier (`v2.tex:120-125`); census 118/729 not in paper; corpus study not re-run at `19ffb4e` (VERIFIED [RS]) | P-5, P-8, E-10, E-11 |
| R2.W4 | Timing methodology under-specified (samples, variance) | PARTIAL | v4 method and pre-registration 2 specified; confirmatory done (VERIFIED [CR]); open issues: slot-order bias, no multiplicity control, sub-margin GAIN/LOSS, no independence check (VERIFIED [EV] Q3) | P-5, E-1, E-2 |
| R2.W5 | "0.0 ns" reads as within-noise | DICNIP (paper contradicts artifacts) | Legacy ns SUPERSEDED/INVALID; paper still prints them `v2.tex:375-376, 387-390` (VERIFIED [RS]) | P-8, V-1 |
| R2.W6 | Missing related work (ABCD, Würthinger; 5 refs) | PARTIAL | Papers archived and read; Tint and wgpu#4360 not archived or cited (VERIFIED [RS], [RW]) | R-1, R-2, R-3, P-7 |
| R2.W7 | The novel kernel is narrow | PARTIAL | Operators `% / >> << & min max clamp select as` (`:544-592`); real-world yield 118/729; WGSL `&&` not refined (RECORDED) | P-6, A-1–A-8 |
| R2.W8 | Loop induction analysis not implemented | PARTIAL | Implemented but S2/S3 unsound (VERIFIED [CR]); diff-exec 8/8 class-A identical (VERIFIED [RS]) | S-2–S-4, P-6 |
| R2.W9 | Soundness corner: overridable workgroup size | PARTIAL | Pre-override proof confirmed (VERIFIED executed); production re-validates `back/pipeline_constants.rs:93-104, 285-294`; toggle lost `valid/mod.rs:645-647`; no test (VERIFIED [SG] Q1) | S-5, P-2, E-13 |
| R2.W10 | Soundness corner: syntactic-match fragility | MISSING (reopened) | Loop recognition and Load identity still syntactic; S1 confirmed incl. GLSL `i++` (VERIFIED [SG] Q2, [CR]); earlier "closed" status superseded | S-3, S-9, A-5, T-1, T-4 |
| R2.W11 | Soundness corner: Restrict interaction | PARTIAL | Proofs policy-independent, checked before `choose_policy` (`back/spv/index.rs:518-531`) (VERIFIED); under Restrict a wrong proof = UB, and S1–S3 delete `UMin` (VERIFIED executed) | P-2, S-6, S-7, T-5 |
| R2.W12 | Writing needs a pass | MISSING | 3 `\fbox` placeholders `v2.tex:181, 256, 273`; dangling sentence l.251-252; intro defects l.133-146 (VERIFIED [PP] Q1) | P-1, V-1–V-4 |
| R2.Sc | Scores: Novelty 1, Engineering 1, Evaluation 0 | PARTIAL | Evaluation answered provisionally; positioning missing | P-5, P-7, R-3 |
| R3.1 | "Template patterns" framing | PARTIAL | Index expressions are now compositional transfer functions; loop recurrences are still templates (VERIFIED [PP] Q3) | P-3, P-6 |
| R3.2 | Algorithms "could have been presented better" | MISSING | No pseudocode matching `19ffb4e` in any deliverable; a 30-line draft exists in [PP] Q3 | P-6, V-2 |
| R3.3 | Work in progress | PARTIAL | Loop analysis extended, but 3 open soundness bugs; paper lists done work as future (`v2.tex:544-563`) | P-2, P-5 |
| R3.4 | Evaluation "quite limited" | DICNIP (provisional) | As R1.3 | P-5, S-8 |
| R3.5 | "Unclear if the selected patterns cover enough opportunities" | DICNIP | 118/729 proved, ceiling ≈44%; cost-weighted recovery ≈0.990 Intel / ≈0.478 NVIDIA (first campaign, exploratory) ([EV] Q4) | P-5, E-6, E-7, E-10 |
| R3.6 | Novelty 0; Industry-practice track | MISSING (positioning) | As R1.8 | R-3, P-7 |
| R4.1 | Static-analysis description "sketchy" (Engineering 0) | MISSING | §3.5 is a bullet list; figures are placeholders (VERIFIED [RS]) | P-6, V-2 |
| R4.Q1 | What index expressions are considered? | DICNIP | Any 32-bit integer expression over literals, constants, locals, `let`, builtins, `+ − × / % << >> &`, `min max clamp select`, casts (`:512-600`, VERIFIED [PP] Q3) | P-6 |
| R4.Q2 | Only constant indices? | DICNIP | No: constants were already skipped upstream (`proc/index.rs:335-347`); every proof is for a runtime index (VERIFIED [RS]) | P-3 (bold, first block), P-6 |
| R4.Q3 | How are variable values obtained? | PARTIAL | Builtin spec ranges, path-sensitive refinement, recurrence closed forms (`refine_range` `:905-920`); refinement over loads is unsound when a store intervenes (S1) | P-6, S-3 |
| R4.Q4 | Only loop IVs, no other variables? | DICNIP | Any value with a derivable interval, e.g. `if (i < 8u) { t[i] }`; relation `lid ≥ off`; WGSL `&&` gap (VERIFIED [PP] Q3) | P-6, A-4 |
| R4.Q5 | What is "loop induction variable analysis"? | DICNIP | Closed-form header intervals for `i ± c`, `i·c` / `<<k`, `i/c` / `>>c`; ⊤ widening otherwise, no fixpoint (VERIFIED [PP] Q3); S2/S3 affect soundness | P-6, S-2, S-4 |
| R4.Q6 | Identifying recurrences? | DICNIP | `recurrence_ranges` `:1331-1374`; eq-exit since `2ffe1b7` (false at `8757d7d`); tests `top_guard_equality_exit_elides`, `recurrence_requires_reachable_exit_and_no_wrap` pass (VERIFIED [RS]) | P-6 |
| R4.Sc | Novelty 1, Engineering 0, prefers Talk | MISSING | Engineering 0 attributed to sketchiness (RECORDED `02-REVIEWS.md`) | P-6, P-4 |
| O1 | Accepted as poster | DONE (no action) | `02-REVIEWS.md` Decision (RECORDED) | — |
| O2 | "Address all the review comments … in the final poster" (binding) | MISSING | No poster on disk; `poster/` absent (VERIFIED [RS], [PP]) | P-1–P-8 |
| O3 | Optional 90-s teaser video | UNKNOWN (deadline passed) | No artifact; 26 Aug (site) vs 27 Aug (email) (VERIFIED [PP] Q5) | S-9 (record as not made / UNKNOWN) |
| X1 | Evaluation = 0 from all four reviewers | DICNIP (provisional) | As R1.3 / R2.W1–W5 | P-5, S-8 |
| X2 | Writing clarity 2 from all four | MISSING | As R2.W12 | P-1, V-4 |
| X3 | Relevance 3 from all four | DONE (HOLDS; acknowledge) | Motivation: gpuweb#1202, wgpu#4360 open since 2021 (VERIFIED [RW]) | P-3 (motivation line) |

## One prioritized implementation list, soundness first

**Tier ordering.** Tier 0 is soundness, then the poster, evaluation, related work, analysis precision, testing, and finally paper v3.

**Columns.**

- **Window**: "Before poster" means the item fits in the next ~24 h and changes what can be printed. "After" means after the conference.
- **Effort**: copied from the notes; "not estimated" where the notes give none.

**Timing caveat.** Tier 0 is first in priority, but most of it cannot land and be re-measured before the poster. The poster therefore carries S1–S3 as open.

| ID | Action | Effort | Window | Review points answered |
|---|---|---|---|---|
| **Tier 0: soundness (P0)** | | | | |
| S-1 | Commit the six probes as red tests in `bounds_check_stress.rs`, each with a must-elide neighbour. Add the untested traces: S1 relation key, S1 `continuing`/`break if` variant, S2 trace 2 ([SG] Q2, Q4, Q6) | hours | Before poster (optional) / After | R2.S3, R2.W10, R2.W11, R1.4 |
| S-2 | Fix S3: drop or restrict the in-body initializer rule (`single_update` l.1059-1065; `loop_range` l.428-438); re-run the suite | not estimated (rule removal) | After | R1.4, R1.7, R2.W8, R2.S3 |
| S-3 | Fix S1: per-local store generation bumped on Store / reset / loop entry / Call; map `Load(x)` to `Local(x)` in target, key, comparison, `loop_guard` and `break_if` only when the generation matches; loop matcher requires no store between the guard load and the `If` | ≈1–2 days | After | R2.W10, R2.S3, R2.W11, R4.Q3 |
| S-4 | Fix S2: join the state at every `Continue` that targets the loop (excluding nested loops, as in `may_continue` l.1243-1256) with the fall-through state before analysing `continuing` | 1–2 days | After | R1.4, R1.7, R2.W8, R2.S3, R4.Q5 |
| S-5 | Override: treat `workgroup_size_overrides.is_some()` as unknown size; add pinning tests (a) pre-override `safe_accesses` empty, post-`process_overrides` correct for lengths 1 and 64, and (b) elision-off plus 2 entry points stays off; pass the caller's elision flag into `process_overrides` ([SG] Q1) | ≈1 line + tests | Before poster (optional) / After | R2.W9, R2.S4 |
| S-6 | Remove latent hazards: unpopulated index-keyed `safe_indices`; `sdata_manual` hack (`spv/writer.rs:2824`, breaks WGSL zero-init); wrong-default doc comment (`proc/index.rs:28-29`) | not estimated (small) | After | R2.W11, R1.2, R2.S1, R2.S2 |
| S-7 | Establish Metal/D3D semantics after `discard_fragment()` / `discard;`; if execution continues, stop treating `Kill` as a terminator for those backends | UNKNOWN | After | R2.W11, R2.S3 |
| S-8 | After S-2–S-5: full test suite; census vs 118; production screen vs vello's 186 → 84 clamps; re-run v4 and realworld-2 on the fixed commit; mark `19ffb4e` numbers SUPERSEDED where they change | suite + census CPU; the realworld campaign took ~4 h wall clock | After | R1.3, R2.W1, R2.W4, X1, R2.S3 |
| S-9 | Correct the records (Rule 5, running code wins): ledger "two unsound cases … both fixed"; `03-REVIEW-RESPONSE-STATUS.md` rows 7 and 8; audit C7 "no unsound elision"; `T7-POSTER.md` headline; `05-REPOS-AND-REFERENCES.md:60` (mislabels `IICT 2026-3-1.pdf`, another author's paper, as a venue doc); teaser status; session log | hours | **Before poster** | R2.S3, R2.W9, R2.W10, O2, O3 |
| **Tier 1: poster-critical (next ~24 h)** | | | | |
| P-1 | Build the A0 portrait (841 × 1189 mm) poster from panels A–I in [PP] Q4, with an artifact path under every number; print it yourself (the upload deadline has passed) | not estimated; print lead time UNKNOWN | **Before poster** | O2, X2, R2.W12 + all DICNIP rows |
| P-2 | Soundness and limits panel: the six-root-cause wording above; the override sentence ("production wgpu path re-validates; pre-override proof confirmed; guard pending"); the evidence stack (89 stress tests, diff-exec 8/8, exhaustive interval sweeps); threats to validity | part of P-1 | **Before poster** | R2.S3, R2.S4, R2.W9, R2.W10, R2.W11, R3.3, R1.4, R1.7 |
| P-3 | Headline and problem panel: "interval-domain range analysis for runtime-varying indices"; **NOT constant indices**; per-backend policy table (Vulkan index Restrict, buffer Unchecked with robustBufferAccess2); motivation (gpuweb#1202, wgpu#4360); "fork of wgpu/Naga, not upstream" | part of P-1 | **Before poster** | R1.1, R3.1, R4.Q2, X3, R1.2, R2.S1 |
| P-4 | Pipeline panel: redraw Fig. 1 (vector, page 2 of `static-analysis-naga-iict.pdf`) as front ends → validator `elide_bounds_checks` → `safe_accesses` → SPIR-V/MSL/HLSL | part of P-1 | **Before poster** | R2.S2, R1.10, R4.1, R4.Sc |
| P-5 | Evaluation panel: confirmatory rule-v2 results per GPU with verdict counts, "~4 of 76 labels expected by chance", Intel crash and slot-order caveats, and the unsound-binary caveat; v4 mechanism bars with `bce/unchecked`; coverage funnel 729 → ≤320 → 78 → 105 → 118; 74.1% only beside 20.4%; methods box (n, rounds, control, GPUs); scope lines (synthetic kernels; vello as web content; 2 laptop GPUs, Vulkan) | part of P-1 | **Before poster** | R1.3, R1.5, R1.9, R2.W1–W4, R3.4, R3.5, X1, R2.Sc |
| P-6 | Algorithm panel: rule table (IR construct → abstract effect → code lines); the 30-line pseudocode; reduce_tree worked example (`UMin` 4 → 0, `bce.sha256 == unchecked.sha256`); R4 Q&A box with six one-line answers; state plainly "loop recurrences are templates; no fixpoint; ⊤ widening" | part of P-1 | **Before poster** | R3.1, R3.2, R4.1, R4.Q1–Q6, R4.Sc, R2.S5, R2.W7, R2.W8 |
| P-7 | Related-work strip: Tint IRA (2024-12-19, default-on in Chromium main), WebKit (constants only), wgpu#4360, ABCD, Würthinger, Go `prove`, LLVM IRCE; capability table labelled "by source inspection, not measured"; relational fact described as minimal; novelty claims 1–4 without "sound" | part of P-1 | **Before poster** | R1.6, R1.8, R2.W6, R3.6, R2.Sc |
| P-8 | Forbidden-claims scrub before printing: 0.0 ns, 7,568–8,192 ns, 29–31 ps, 105/111 ns, bare 74.1%, "sound", "fuzzed", "verified on MSL/HLSL", "vello gets faster", first-campaign headline | ~minutes per pass | **Before poster** | R2.W5, R2.W3, R2.S3 |
| **Tier 2: evaluation** | | | | |
| E-1 | Secondary, labelled analyses of the confirmatory data (the primary rule stays unchanged): Lakens four-outcome labels + TOST 90% CI; Holm/BH or expected-by-chance count; report `e_i` beside `d_i`; mixed model `log t ~ slot + position + (1\|run/round)`; ACF/lag independence check; minimum-estimator check; total frame time beside the geomean | not estimated (CPU only) | After (the chance count fits before poster) | R2.W4, R2.W5 |
| E-2 | Harness fixes for the next campaign: Williams or random slot order; fix the `ticks > 0` panic on small scenes; log `clocks_event_reasons.*`; clock locking (root UNKNOWN); extend the `T975` table beyond df 12; size N by dimensioning | not estimated | After (before S-8's re-run) | R2.W4 |
| E-3 | Compile-time overhead of parse + validate + analysis + emit, elision on/off, 98 census files + 20 vello shaders, N ≥ 10 | ~0.5 day, CPU | After | R1.3, R1.9 |
| E-4 | SPIR-V word and instruction deltas per slot from the saved `.spv` files; confirm `bce == unchecked` identity | hours, no runs | Before poster (optional) / After | R2.W1, R2.S5 |
| E-5 | Static head-to-head with Tint IRA on the same corpus (constant-length arrays), with per-feature attribution | 1–2 days (Dawn build; availability UNKNOWN) | After | R1.6, R1.8, R1.9 |
| E-6 | Per-rule ablation: record the discharging rule in the census JSONL; disable one rule at a time (includes the ABCD-style ablation of refinement vs relation) | 1 day, CPU | After | R3.5, R2.W7 |
| E-7 | Per-stage vello timing (spec 005 FR-4, not delivered) | 1 day code + one campaign | After | R3.5, R2.W4 |
| E-8 | Intel ISA diff (`INTEL_DEBUG=cs`) for matmul_tiled, scan_shared and `image_extend_modes` | 1 day + short GPU | After | R3.5, R2.W1 |
| E-9 | Comparison against driver robustness: (a) buffer Restrict + elision, (b) buffer Unchecked + elision on `index`, (c) all Restrict, no elision | not assessed | After | R1.9 |
| E-10 | Coverage at four levels (per access with a per-project cluster bootstrap; per entry point 28/147; per production clamp; cost-weighted with CIs); instrumented dynamic coverage | 1–2 days for instrumentation | After | R2.W3, R3.5 |
| E-11 | Re-run the corpus hit-rate study at the fixed commit; re-probe `for (…; i != N; …)` | not estimated | After | R2.W3 |
| E-12 | Third vendor or backend (AMD RADV or Metal) | 1–3 days + hardware | After | R1.5 |
| E-13 | Workgroup-size sensitivity at 64/128/256/overridden sizes | 0.5 day + short campaign | After | R2.W9 |
| E-14 | Firefox WebGPU trace | high; probably out of scope | After | R2.S1 |
| **Tier 3: related work and novelty** | | | | |
| R-1 | Archive Tint `integer_range_analysis.{h,cc}`, `robustness.cc`, the Chrome 141 post, wgpu#4360, gpuweb#1202 and WebKit `BoundsCheck.cpp` into `references/raw/` with access dates; add them to `CITATION_MANIFEST.md` | hours | After (URLs suffice on the poster) | R1.6, R1.8, R2.W6 |
| R-2 | Read the metadata-only papers before quoting mechanisms (Suzuki & Ishihata, Gupta, Kolte & Wolfe, Qian et al., Nguyen & Irigoin, Pentagons, Octagons, Venet & Brat); check V8/JSC and Tint's other passes; read SafeRace in full before stating a race-compatibility property | not estimated | After | R2.W6, R1.6 |
| R-3 | Write the related-work section and repositioning: the four defensible claims, "sound" removed until S-8 passes, relational fact scoped to ABCD-style c ∈ {0,1} | not estimated | After | R1.8, R3.6, R2.Sc, R2.W6 |
| **Tier 4: analysis precision (only after S-2–S-4)** | | | | |
| A-1 | Call-aware invalidation: reset only locals passed by pointer | ½–1 day; ≤22 accesses (HYPOTHESIS) | After | R2.W7, R3.5 |
| A-2 | Value numbering of pure expressions (reuses S1's generation counter) | 2–4 days; ≤20 | After | R2.W7, R3.5 |
| A-3 | Loop fixpoint with widening/narrowing | ~1 week; ~10–15 (estimate) | After | R2.W7, R4.Q5 |
| A-4 | `&&` / `\|\|` temp-bool predicate recovery (WGSL) | 2–3 days; subset of 59 | After | R4.Q4, R2.W7 |
| A-5 | Shape generalisation: range-valued bounds, non-exact starts, `LogicalNot` guards, commutative updates | 1–2 days; small | After | R2.W10, R2.W7 |
| A-6 | Struct-wrapped builtins, `subgroup_invocation_id` | small; 0–3 | After | R2.W7 |
| A-7 | Interprocedural facts | high; ≤78–79 (one file) | After | R2.W7 |
| A-8 | Check dedup / symbolic `arrayLength` (mainly Metal) | medium–high; ≤139 / ≤20 | After | R2.W7, R3.5 |
| **Tier 5: testing** | | | | |
| T-1 | IR-level differential oracle: an interpreter for the integer/control subset plus random structured programs biased to risky shapes; assert every proven access stays in bounds | 1–2 weeks (highest value) | After | R2.W10, R2.S3 |
| T-2 | "Checked-elision" debug policy that counts runtime violations of proven accesses on all GPU and real-world shaders | not estimated | After | R2.S3, R2.W11 |
| T-3 | CTS `webgpu:shader,execution,robust_access:*` + `shader,execution` with elision on (no selector in `cts_runner/test.lst`) | not estimated | After | R2.S3 |
| T-4 | Front-end fuzzing (GLSL, spirv-opt output; SPIR-V phi → local stale-load exposure UNKNOWN) | not estimated | After | R2.W10 |
| T-5 | Policy sweep of every must-retain test under Restrict / RZSW / Unchecked | not estimated | After | R2.W11 |
| T-6 | MSL/HLSL execution evidence | needs a Metal/D3D device | After | R2.S2 |
| **Tier 6: paper v3 (only if needed; proceedings UNKNOWN)** | | | | |
| V-1 | Delete invalid numbers; fix Table 3 denominators ("8/504 = 14.6%" is arithmetically false); explain the webgpu-samples 2 → 1 regression; update stale test counts | not estimated | After | R2.W5, R2.W3, R2.W12 |
| V-2 | Rewrite §3 from the fixed commit (domain, rules, pseudocode, worked example); rename §3.2 "Loop recurrences"; regenerate Fig. 4 under Restrict; dump reduce_tree proofs at `2ffe1b7` vs `19ffb4e` to settle the epoch-0.2 attribution | not estimated | After | R3.2, R4.1, R2.S5, R1.1 |
| V-3 | Soundness section + threats to validity (internal, construct, conclusion, external; material in [PP] Q5) | not estimated | After | R2.S3, R1.5, R2.W4 |
| V-4 | Writing pass: dangling sentence l.251-252, intro l.133-146, placeholders, citation for the Firefox claim | not estimated | After | R2.W12, X2, R2.S1 |

**Outcome branches (AGENTS.md Rule 3).** Each branch row names its source. Rows marked "writer" were drafted for this report and have no earlier note.

| Item | Success → next | Normal / partial → next | Bad → next | Disaster → next | Source |
|---|---|---|---|---|---|
| S-1 | Red at `19ffb4e`, green after fix, suite green → land | A trace was wrong (lowering differs) → record, keep as pin | Fix loses real-world elisions → re-run census vs 118 | Red test hits a measured benchmark kernel → stop, downgrade soundness wording | [SG] Q6 (S-1 red at `19ffb4e` already happened: [CR]) |
| S-2–S-4 | All six probes retain their clamp; suite green; census still 118 → go to S-8 | Some legitimate proofs lost (census < 118) → report the new count; old count SUPERSEDED | A must-elide neighbour breaks broadly → narrow the fix (e.g. restrict rather than drop S3) and re-test | A fix exposes another unsound shape → new DEVIATION, add a probe, keep the poster wording at "unsound" | writer, extending [SG] Q6 |
| S-5 | Test (a) and (b) fail before the guard, pass after; wgpu path unchanged | Only (a) fixed; toggle propagation deferred → record | Guard removes proofs on non-override modules → the guard is wrong; fix the condition | A backend emits code from pre-override info in practice → DEVIATION; treat as S-class bug | writer, from [SG] Q1 |
| S-8 | Re-measured effects inside prior CIs → headline stands, relabelled to the fixed commit | Fewer clamps removed, smaller gains → publish the new numbers, `19ffb4e` SUPERSEDED | Any benchmark proof depended on S1–S3 → that gain was measured on unsound code; retract it | Negative control CI excludes 0 on re-run → harness invalid; stop | writer; disaster row from [RW] Q6 E2 |
| P-1 / P-8 | Every review row has a poster block with an artifact path → print | Some rows shown as "open" on the poster → acceptable and honest | Any number not traceable to an artifact → pull it before printing | Poster cannot be printed in time → record as DEVIATION and tell the organizers (writer) | [RS] Q5; disaster row writer |
| E-1 / E-2 | Mixed-model position effect negligible; conclusions unchanged → report as sensitivity | Position effect material on Intel → report `e_i` as co-primary, re-run with Williams order | Many GAIN labels become "practically equivalent" → headline becomes suite-level estimate only | Independence fails badly (strong ACF) → switch to block bootstrap; re-derive every CI | writer, from [EV] Q3 |
| E-3 + E-5 | Overhead < a few % and the Tint comparison completes → two new panels | Tint cannot run some files → report per-file coverage of the comparison | Compile time superlinear on some shader → a finding; profile it | Tint build fails → record the error verbatim; fall back to qualitative comparison | [EV] Q5 |
| E-5 (proof counts) | Naga-fork ⊇ Tint on ≥95% of Tint's proofs + extras → "superset + N extra" | Roughly equal counts, extras in few shaders → novelty rests on claims 1 and 3 | Tint proves accesses the fork does not → file as gaps, scope the claim | Tint rejects a fork proof that is actually unsafe → DEVIATION, soundness fix, re-run the suite | [RW] Q6 E1 |
| E-9 | Win persists under buffer Unchecked → elision matters beyond buffers | Win vanishes under (b) → narrow the claim to non-robustness2 devices, Metal, D3D | robustness2 unavailable or inconsistent → report (a) vs (c) only | (same as bad) | [RW] Q6 E3 |
| T-1 | Oracle finds nothing after S-2–S-4 across N programs → cite it as evidence (still not a proof) | Oracle finds only precision misses → feed A-items | Oracle finds a new unsound shape → S-class fix + red test | Oracle and analysis disagree on IR semantics → fix the interpreter first; no soundness claim until resolved | writer |

## Conclusion

Today's results do more than add three bugs to a list. They change which contribution the paper can defend. The source-level comparison shows that the features beyond Tint and the confirmed unsoundness come from the same design choice: refining guards over loads without identity checks, and accepting loop shapes looser than Tint's rigid pattern. Tint's narrowness is part of how it stays sound. That makes "we prove more than Tint" the weakest claim until S-2 to S-5 land and the S-8 re-measurement shows how much scope survives. Three claims are stronger today:

- the controlled measurement: +7.01% on Intel, while NVIDIA is essentially neutral end-to-end;
- the coverage funnel with its 44% ceiling;
- the experience result that flow-layer rules which are sound for one front end's lowering break under another's, as GLSL `i++` shows.

The evaluation methodology has matured: pre-registration, a negative control, and a harness fix verified by the pilot. The analysis it measures has not kept pace. That inverts where this project's risk used to lie. The next month should go into the flow-layer oracle (T-1) and the fixes, not into more GPU campaigns on `19ffb4e`.

For the poster, honesty is the strongest answer to all four reviewers. A panel that says "unsound at `19ffb4e`, three root causes, here are the probes" answers R2's three corner cases more convincingly than any soundness claim could. It also shows directly the rigor the Evaluation = 0 scores asked for.

[CR]: ../research_notes/Review%20driven%20improvement%20plan/confirmed_results_2026-10-01.md
[SG]: ../research_notes/Review%20driven%20improvement%20plan/analysis_soundness_gaps.md
[EV]: ../research_notes/Review%20driven%20improvement%20plan/evaluation_methodology.md
[RW]: ../research_notes/Review%20driven%20improvement%20plan/related_work_novelty.md
[PP]: ../research_notes/Review%20driven%20improvement%20plan/paper_poster_presentation.md
[RS]: ../research_notes/Review%20driven%20improvement%20plan/review_point_status.md
[PR]: ../research_notes/Review%20driven%20improvement%20plan/probes/
[AU]: Bounds%20check%20elision%20project%20audit.md
[V4]: Benchmark%20v4%20results.md
[NV2]: ../bounds-check-testing-framework/runs-realworld/summary-v2-nvidia-realworld-2-19ffb4e.json
[IN2]: ../bounds-check-testing-framework/runs-realworld/summary-v2-intel-realworld-2-19ffb4e.json
[LOG2]: ../bounds-check-testing-framework/runs-realworld/campaign-20260930T182304Z.log
[CHROME141]: https://developer.chrome.com/blog/new-in-webgpu-141
[TINTLOG]: https://github.com/google/dawn/commits/main/src/tint/lang/core/ir/analysis/integer_range_analysis.cc
[ROB]: https://raw.githubusercontent.com/google/dawn/main/src/tint/lang/core/ir/transform/robustness.cc
[FINCH]: https://chromium.googlesource.com/chromium/src/+/main/gpu/config/gpu_finch_features.cc
[W4360]: https://github.com/gfx-rs/wgpu/issues/4360
[GPUWEB1202]: https://github.com/gpuweb/gpuweb/issues/1202
[WEBKIT]: https://raw.githubusercontent.com/WebKit/WebKit/main/Source/WebGPU/WGSL/BoundsCheck.cpp
[FUTHARK]: https://www.futhark-lang.org/publications/hlpp20.pdf
[ABCD]: https://doi.org/10.1145/349299.349342
[MANUAL]: https://compilertech.org/assets/docs/IICT-2026-Speakers-Manual.pdf
[IICT]: https://compilertech.org/
