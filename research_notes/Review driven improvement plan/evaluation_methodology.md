# Evaluation methodology: audit, gaps and improvements for the Naga bounds-check elision evaluation

Written 2026-10-01 by a read-only research agent. No build, cargo, GPU program or git operation was run. The spec-005 confirmatory campaign (harness `realworld-2`) was running during this session. Its results are **not** used anywhere below. One exploratory script also read the `realworld-2` run directories by accident. Those rows were discarded and are not reported.

Evidence labels follow `AGENTS.md` Rule 0:
- **VERIFIED**: checked this session by reading the file or running a read-only query over existing JSON.
- **RECORDED**: stated in a project artifact; cited, but not re-derived.
- **REPORTED**: stated by the task brief or a document, with no artifact found.
- **UNKNOWN**: nobody knows.

Local sources are given as repository-relative paths.

---

## Q1. How do the current evaluation artifacts compare with the reviewers' complaints and with benchmarking best practice?

### Takeaway
The evaluation has moved far past what the reviewers scored, which was "one minimal shader + one matmul, 0.0 ns". There is now:
- a 729-access real-world census with a feature-by-feature ablation;
- a 7-kernel synthetic suite on 2 GPUs, with an in-round identical-binary control and an unchecked upper bound;
- a 38-workload real-world benchmark (vello's 34 scenes plus imageBlur).

The remaining weaknesses are these:
1. The statistical decision rules. The v4 fixed margin, the worst-run control rule, and some properties of rule v2.
2. An unbalanced slot-order design.
3. Missing standard dimensions: compile time, code/ISA size, per-stage vello timing, more vendors and backends, and an external baseline (Tint).
4. A coverage metric that is static and dominated by vello.

### Cited Findings

**Reviewer complaints (RECORDED)**
- All four reviewers scored Experimental Evaluation = 0 ([IICT_CONTEXT/02-REVIEWS.md](IICT_CONTEXT/02-REVIEWS.md):32).
- R2: "thin evaluation (one minimal shader + one matmul; no real corpus; no hit-rate reporting); timing methodology under-specified (sample count, variance); exact '0.0 ns' delta reads as within-noise" (02-REVIEWS.md:54-56).
- R1: "larger, more diverse benchmark suite; comparison with existing bounds-check-elimination techniques" (02-REVIEWS.md:43-44).
- R3: "without extensive experiments, it is unclear if the selected patterns cover enough optimisation opportunities" (02-REVIEWS.md:65-66).

**Coverage (hit rate)**
- The census covers 729 real-world runtime accesses. Elided: BASE 78, F001 105, F005 118. vello contributes 440 accesses (68 → 94 → 98 elided). All changed proofs were reviewed, with LOST = 0 (VERIFIED: [research_notes/Real world shader elision gaps/census/F001_summary.txt](research_notes/Real%20world%20shader%20elision%20gaps/census/F001_summary.txt), [F005_summary.txt](research_notes/Real%20world%20shader%20elision%20gaps/census/F005_summary.txt)).
- F006 (wgpu `19ffb4e`) still proves 118/729. It closed the benchmark kernels' last clamps but proved no additional real-world access (RECORDED: [reports/Benchmark v4 results.md](reports/Benchmark%20v4%20results.md), addendum "Limits unchanged").
- The structural ceiling is "at most ~44% (320 of 729)" provable in principle:
  - 46% of accesses index runtime-sized storage buffers;
  - 7% use vertex/instance indices;
  - 3% hit binding arrays.

  Source: RECORDED, [reports/Real world shader elision gaps.md](reports/Real%20world%20shader%20elision%20gaps.md), Summary §5.
- Corpus concentration in the older project-corpus figure:
  - all files: 587 runtime accesses, 435 elided, rate 74.1%;
  - without the `mat_cx*` files: 191 runtime accesses, 39 elided, rate 20.4%.

  Source: VERIFIED, [IICT_CONTEXT/logs/audit-20260927-corpus-concentration.log](IICT_CONTEXT/logs/audit-20260927-corpus-concentration.log).

**Synthetic benchmark v4 (RECORDED, [reports/Benchmark v4 results.md](reports/Benchmark%20v4%20results.md))**
- Method: passthrough SPIR-V (the executed words are hashed); wgpu-hal production Vulkan policy; 4 rotating slots (`no_bce`, `bce`, `bce_dup`, `unchecked`); 7 kernels at full occupancy; 10 processes per kernel on NVIDIA and 5 on Intel; exact u32 CPU reference.
- NVIDIA gains on `19ffb4e`:

  | kernel | no_bce/bce |
  |---|---|
  | scan_shared | 1.1579 |
  | matmul_tiled | 1.1080 |
  | vello_binning | 1.1522 |
  | reduce_tree | 1.0131 |
  | prefix_scan_hs | 1.0086 |

- Intel gains: scan_shared 1.1162, reduce_tree 1.0733, prefix_scan_hs 1.0365. matmul, vello_binning and private_array are ~1.0000 on Intel.
- The report states two caveats itself:
  - The ±0.5% margin "was declared after that first run and before any kernel result was inspected". Strict control-CI-contains-1 failed on the empty kernel because of "a 0.13% bias between identical pipelines".
  - "These kernels were also the ones used to *find* the gaps that 005/006 close... not an unbiased sample".
- DEVIATION recorded in the report: the Intel matmul control was at 1.0100 in one run, started 12 s after the NVIDIA campaign.

**Real-world benchmark, realworld-1 (RECORDED, [reports/Real-world benchmark results.md](reports/Real-world%20benchmark%20results.md))**
- Step 0 screen: 147 entry points compiled, 28 change their binary.
- vello benchmark set: UMin 197 → 95. Bounds clamps 186 → 84, so 102 removed.
- Suite results (34 scenes, equal weight):

  | GPU | elision gain (geomean) | cost of all checks | bce/unchecked | RESOLVED gains | RESOLVED losses | not resolvable | oracle-flagged |
  |---|---|---|---|---|---|---|---|
  | NVIDIA | 1.0071 | 1.0149 | 1.0054 | 4 | 0 | 26 | 4 |
  | Intel | 1.0671 | 1.0678 | 1.0006 | 21 | 5 | 1 | 7 |

- Intel per-scene range: +34.27% to −5.30%.
- Scope limits:
  - Native vello runs `ShaderRuntimeChecks::unchecked()`, so `no_bce` models vello as web content.
  - Firefox's exact settings are UNKNOWN.

**Spec 005 pre-registration 2 (RECORDED, [spec-kit-work/specs/005-real-world-benchmark/spec.md](spec-kit-work/specs/005-real-world-benchmark/spec.md), committed `98bd514` per the analyze_v2.py docstring)**
- Diagnosis of realworld-1 on NVIDIA:
  - "~1 in 5 samples of any slot is 20-40% slower";
  - "per-round standard deviation of log(bce_dup/bce) for identical binaries is ~15%";
  - graphics clock "495-2610 MHz", utilisation "48% median".
- Proposed fixes: submit all 4 slots in one batch, and adopt verdict rule v2.

**Spec requirement not delivered (VERIFIED)**
- Spec 005 FR-4 says "Report per stage where the workload has stages (vello), plus end-to-end".
- A grep for "stage" in the report, `analyze_realworld.py` and `make_tables.py` returns 0 matches. The report is end-to-end only.
- This is a docs-versus-spec gap.

**Environment and telemetry (VERIFIED)**
- Telemetry queries `timestamp,pstate,clocks.gr,clocks.mem,temperature.gpu,power.draw,utilization.gpu` every 200 ms ([bounds-check-testing-framework/realworld-v1/src/main.rs](bounds-check-testing-framework/realworld-v1/src/main.rs):365; `paired-vulkan-v4/src/main.rs`:388).
- It does **not** record throttle reasons (`clocks_event_reasons.*`).
- One realworld-1 run's telemetry shows P5 at 495 MHz and P3 at 1890 MHz within the same second (`runs-realworld/20260929T215005Z-vello:07-fill_types/artifacts/telemetry.csv`).
- The brief's "SW Power Cap 55 W, SW Thermal Slowdown observed" was **not found in any repo artifact** searched (IICT_CONTEXT, research_notes, runs-realworld diag/sizing). It stays REPORTED.

**Best-practice checklist, against the literature**
- Report variation and effect-size CIs. Kalibera & Jones found that 71 of 90 execution-time papers "failed to provide any measure of variation", and recommend: "Report performance changes with effect size confidence intervals" ([Kalibera & Jones, ISMM 2013](https://kar.kent.ac.uk/33611/45/p63-kaliber.pdf), §1 and §4). The project now reports per-run bootstrap CIs and across-run min–max (v4) or t-intervals (rule v2). This is met.
- Show upper performance bounds ([Hoefler & Belli, SC'15, Rule 11](https://htor.inf.ethz.ch/publications/img/hoefler-scientific-benchmarking.pdf)). This is met by the `unchecked` slot.
- Document all varying factors and the complete setup (Hoefler & Belli, Rule 9). Largely met by `meta.json`, but throttle reasons and clock locking are missing.
- Do not assume normality without diagnostic checking (Hoefler & Belli, Rule 6), and check independence of iterations with ACF and lag plots ([Kalibera & Jones](https://kar.kent.ac.uk/33611/45/p63-kaliber.pdf), §6). **Not met.** A grep for `autocorr|acf|lag` in both harnesses and the analysis scripts finds no independence or normality diagnostic (VERIFIED). The within-run bootstrap resamples rounds as if they were i.i.d. (`realworld-v1/src/main.rs`:46-56, VERIFIED).
- Avoid summarising ratios; use the geometric mean only when costs are not available (Hoefler & Belli, Rule 4). **Partially met.** The suite summaries are geomeans of per-scene ratios, even though per-scene absolute frame times exist (`median_ns_per_rep` in each `result.json`).
- Measurement bias and setup randomisation. Mytkowicz et al. found that "none of the papers with experimental results adequately consider measurement bias" among 133 papers. They propose "setup randomization" and "causal analysis" ([Mytkowicz et al., ASPLOS 2009](https://users.cs.northwestern.edu/~robby/courses/322-2013-spring/mytkowicz-wrong-data.pdf), §1). The `bce_dup` identical-binary slot is a direct measurement-bias detector, and it did detect one: the 0.13% pipeline-identity bias in v4. Setup randomisation beyond slot rotation (for example pipeline creation order or allocation order) is not done.

### Inferences
- For the reviewers' specific points, R2's "0.0 ns / within-noise" and "sample count, variance" are answered at the method level by v4 and spec 005. R2's "no real corpus / hit-rate" and R3's "coverage" are answered by the census. R1's "comparison with existing approaches" is **not** answered (see Q2).
- The biggest residual threat to a reviewer's reading is not missing effort. It is that the headline numbers are split: +0.71% suite on NVIDIA versus +6.71% on Intel. On top of that, the decision rule changed between campaigns. A reviewer will ask which rule is primary and why. The pre-registration answers this, but only if the confirmatory campaign is reported under rule v2 exactly as registered.

### Gaps
- The v4 per-kernel CIs across runs are not re-derived here. This note did not recompute v4 summaries.

---

## Q2. Which standard evaluation dimensions are missing, and what would each cost and produce?

### Takeaway
Seven standard dimensions are missing or only partial:
1. compile-time overhead: never measured;
2. SPIR-V/ISA size: SPIR-V is available offline, ISA is never inspected;
3. per-stage vello timing: required by FR-4, not delivered;
4. additional vendors and backends;
5. an external baseline (Tint's range analysis);
6. per-rule ablation: only per-feature exists;
7. workgroup-size sensitivity.

Items 1, 2, 5 and 6 need no GPU time or only offline GPU-free tooling. They are the cheapest answers to R1 and R3.

### Cited Findings
- **Compile-time overhead: no measurement exists (VERIFIED).**
  - A grep across `IICT_CONTEXT`, `reports`, `research_notes`, `spec-kit-work/specs`, the TODO and the paper tex finds no compile-time or analysis-time measurement.
  - Spec 001's plan lists "**Performance Goals**: no measurable compile-time change" ([spec-kit-work/specs/001-sound-branch-refinement/plan.md](spec-kit-work/specs/001-sound-branch-refinement/plan.md):27). That is a goal, not a measurement.
- **Code size.**
  - The old paper reports SPIR-V words for the minimal shader, e.g. 399 = 399 words ([iict2026_bounds_check_elision_v2.tex](iict2026_bounds_check_elision_v2.tex):346-357, VERIFIED).
  - Every v4 and realworld run saves each slot's executed `.spv` (`runs-realworld/<run>/artifacts/*.{no_bce,bce,bce_dup,unchecked}.spv`, VERIFIED listing). So a word and instruction count is computable post hoc with no new runs.
  - Reports currently give clamp (UMin) counts only.
- **ISA inspection tools.**
  - Intel/Mesa: the `INTEL_DEBUG` flag `cs` dumps compute shader assembly ([Mesa environment variables](https://docs.mesa3d.org/envvars.html), via search excerpt).
  - NVIDIA: Nsight Graphics' Shader Profiler supports Vulkan and has a Disassembly (formerly SASS) view. Per-instruction sample association is in Pro builds ([Nsight Graphics Shader Profiler docs](https://docs.nvidia.com/nsight-graphics/UserGuide/shader-profiler.html), via search excerpt).
  - Either tool could explain the per-driver differences. Examples: Intel 0% on matmul_tiled and vello_binning in v4, but +34.27% on `image_extend_modes (bilinear)`; NVIDIA `private_array` at 0.9997 ("driver removes clamps itself").
- **Per-stage vello breakdown.** Required by spec 005 FR-4, not delivered (VERIFIED, Q1).
- **Vendors and backends.**
  - Only NVIDIA RTX 4060 Laptop (driver 580.178.04) and Intel RPL-S iGPU (Mesa 26.2.2), on Vulkan (RECORDED, v4 report).
  - "Metal also checks buffers, so it may gain more (UNKNOWN)" (RECORDED, realworld report, Deviations and limits).
- **External baseline.**
  - Tint's `IntegerRangeAnalysis` (Dawn; Chrome 141) predates this work and is uncited in the project (RECORDED, [IICT_CONTEXT/04-EVIDENCE-LEDGER.md](IICT_CONTEXT/04-EVIDENCE-LEDGER.md):69).
  - Epoch plan §0.4 lists Tint as prior art to read, and §"honest position" calls for a comparison ([IICT_CONTEXT/epoch/BCE_EPOCH_0_PLAN.md](IICT_CONTEXT/epoch/BCE_EPOCH_0_PLAN.md):108, 215). No section 0.4 file exists yet (VERIFIED directory listing).
  - A web search confirms that Dawn's log contains a change enabling integer range analysis in the robustness transform for `Access` ([dawn log](https://dawn.googlesource.com/dawn/+log/chromium/7151..chromium/7204?n=1000), search excerpt only; commit not read).
- **Ablation.** A per-feature ablation already exists but is not framed as one:
  - census: BASE → F001 → F005 → F006 per corpus (VERIFIED, summaries above);
  - v4: `83a90e0` → `19ffb4e` per kernel, where "The residual measured on `83a90e0` predicted the gain on `19ffb4e`", e.g. Intel `reduce_tree` leftover 1.0730, then gain 1.0733 (RECORDED, v4 addendum).
  - Per-*rule* attribution (which proof rule discharged which access) is not recorded in the census schema. Its fields are file, line, indexed type and index tree (VERIFIED, summary rows).
- **Baselines already present.** `no_bce` is production clamp-everything (Restrict for arrays). The buffer policy is `Unchecked` under robustBufferAccess2, which is driver robustness. `unchecked` is the no-check bound (RECORDED, spec 004 Q2/Q3; realworld report, Method).
- **Upper bounds.** "If possible, show upper performance bounds to facilitate interpretability" ([Hoefler & Belli SC'15, Rule 11](https://htor.inf.ethz.ch/publications/img/hoefler-scientific-benchmarking.pdf)). This is already satisfied by `unchecked`.

### Inferences

Improvement list, ordered by value per unit of cost. Everything marked "no GPU" must still wait until the campaign ends, because CPU load trips the noise gate (`loadavg` below 2).

| # | Improvement | Reviewer point | Cost | Result it would produce |
|---|---|---|---|---|
| E1 | **Compile-time overhead.** Time naga parse + validate + analysis + SPIR-V emit with elision on/off for all 98 census files and the 20 vello shaders. Use N ≥ 10 process repetitions and report median plus CI of the ratio. | Standard compiler-paper dimension; R1 "comparison" | ~0.5 day, CPU only | "Analysis adds X% (CI) to naga compile time on real shaders", or a flagged outlier if some shader blows up (worklist/fixpoint cost) |
| E2 | **Code size from saved artifacts.** SPIR-V words and instruction counts per slot per shader, from existing `.spv` files. | R2 "thin" (cheap extra dimension) | Hours; no new runs | Table of Δwords per shader; confirms `bce == unchecked` byte-identity where claimed |
| E3 | **ISA diff on Intel** (`INTEL_DEBUG=cs`) for 2–3 kernels: matmul_tiled (0% gain), scan_shared (+11.6%), `image_extend_modes`. | R3 "unclear which patterns matter"; explains cross-GPU variance | 1 day; needs short GPU use *after* the campaign | Mechanistic explanation of "driver hides clamp" versus "clamp stays in hot loop"; turns an anomaly into a finding |
| E4 | **Tint comparison (static).** Run Tint's robustness transform with range analysis on the same 98 files and count remaining clamps per access or per entry point versus naga `19ffb4e`. | R1 "comparison with existing approaches"; novelty | 1–2 days (build Dawn/Tint; map WGSL features) | Head-to-head proof counts on an identical corpus. This is the strongest single answer to R1 |
| E5 | **Per-stage vello timing** (FR-4). Per-dispatch timestamps are already possible in the batched harness. | R3 coverage; explains which of the 16 dispatches carry the gain | 1 day code + one campaign | Attributes the Intel +6.7% to specific shaders (e.g. fine vs. coarse) |
| E6 | **Per-rule ablation.** Record the proof rule in the census JSONL; turn off one rule at a time. | R3 "selected patterns cover enough" | 1 day; CPU only | Table: rule → accesses proven → (via E5) time recovered |
| E7 | **Third vendor / backend.** AMD via RADV (Vulkan), or Metal on Apple silicon. | R1 "diverse"; external validity | Hardware access; 1–3 days | Tests whether the NVIDIA-vs-Intel split generalises |
| E8 | **Workgroup-size sensitivity.** Re-run 2 kernels at 64/128/256/overridden sizes. | R2 "overridable workgroup size" soundness corner | 0.5 day + short campaign | Shows proofs track `@workgroup_size` and that overrides are handled |
| E9 | **Browser trace.** Firefox WebGPU running vello or a WebGPU sample. | Realism ("no_bce models Firefox" is UNKNOWN) | High; Firefox build with patched naga | Real-app evidence; probably out of scope for a poster |

### Gaps
- Whether Tint's range analysis can be run standalone on the census WGSL files (for example through the `tint` CLI with robustness enabled) was not verified this session.
- Whether root is available for `nvidia-smi --lock-gpu-clocks` is UNKNOWN (spec 005 "Bad" branch already says so).

---

## Q3. What statistical protocol fits this setup? Is a fixed 0.5% margin with a worst-run control rule appropriate, and is rule v2 sound?

### Takeaway
- **The v4 rule is not a good primary rule.** Its margin is fixed, and it was chosen after seeing control data. The worst-run control criterion is a maximum over N, so it becomes *harder* to pass as more runs are added.
- **Rule v2 is much closer to the literature.** It is a two-level design: per-process summaries, then a t-interval across processes. It pre-registers an equivalence bound, in line with Kalibera & Jones, Georges et al. and Lakens.
- **Rule v2 has five fixable weaknesses:**
  - no multiplicity control across 76 tests;
  - statistically significant but trivial effects are labelled GAIN/LOSS;
  - a 3:1 slot-order imbalance that the control correction does not cancel;
  - no independence check;
  - a margin that is not a justified smallest effect size of interest.

### Cited Findings

**Literature**
- Kalibera & Jones (ISMM 2013) [link](https://kar.kent.ac.uk/33611/45/p63-kaliber.pdf):
  - Experiments are multi-level (iterations within executions within builds). Repetition is "most needed where most uncertainty arises".
  - "Adding repetition at the highest level will always increase the precision". The multi-level interval "is the same as if we had used a single-level interval for the means of all data from all but the highest level" (§9).
  - A dimensioning experiment should choose the lower-level counts (§9.1–9.3).
  - The ratio of means gets a Fieller-type interval (§10.1).
  - Visual CI-overlap tests are "rather conservative" (§4).
  - Significance tests "confuse sample size and practical relevance".
  - For SPEC CPU they tabulate executions needed for 95% CI half-widths of 0.5–2%, and "would still run 5 executions... to get the confidence interval estimate" (§11).
- Georges, Buytaert & Eeckhout (OOPSLA 2007) [link](https://dri.es/files/oopsla07-georges.pdf):
  - The 'best of 30' method "does not control non-determinism" and led to wrong conclusions in their GC example (§1).
  - For steady state: run p VM invocations, detect steady state per invocation with a CoV threshold (0.01 or 0.02), take the per-invocation mean of k iterations, then compute a CI across the p invocation means (§4.2).
  - Use Student t with fewer than 30 measurements (§4.1).
  - For more than two alternatives: ANOVA plus Tukey HSD (§3.4).
- Chen & Revels (arXiv:1608.04295, 2016) [link](https://arxiv.org/html/1608.04295v1):
  - They model timing noise as nonnegative additive delays and recommend the **minimum** as estimator, because "choosing the smallest timing measurement will choose the sample with the smallest magnitude of error". Its across-trial distribution was "unimodal in all cases we have observed".
  - Their regression threshold in Julia CI was a 30% increase of the minimum.
- Lakens (2017), *Soc. Psychol. Personal. Sci.* 8(4):355–362 [link](https://pmc.ncbi.nlm.nih.gov/articles/PMC5502906/):
  - TOST tests H01: Δ ≤ −ΔL and H02: Δ ≥ ΔU. At α = .05 the matching interval is a **90% CI**.
  - Bounds should be the smallest effect size of interest and "should be prespecified". If no practical boundary exists, use the smallest effect one has power to detect.
  - Four outcomes: equivalent-and-not-different, different-and-not-equivalent, **different-and-equivalent**, undetermined.
- Hoefler & Belli (SC'15) [link](https://htor.inf.ethz.ch/publications/img/hoefler-scientific-benchmarking.pdf):
  - Rule 5: for nondeterministic data, report CIs.
  - Rule 6: do not assume normality without diagnostic checking.
  - Rule 7: compare with a sound test, e.g. non-overlapping CIs or ANOVA.
  - Rule 4: geometric mean only when costs or rates are unavailable.
- Mytkowicz et al. (ASPLOS 2009) [link](https://users.cs.northwestern.edu/~robby/courses/322-2013-spring/mytkowicz-wrong-data.pdf): measurement bias is "significant and commonplace". Remedies are setup randomisation and causal analysis.
- GPU clocks:
  - NVIDIA recommends locking core and memory clocks with `nvidia-smi --lock-gpu-clocks` / `--lock-memory-clocks` before measuring, because clock changes "can vary performance, introducing errors in measurements". This needs `sudo` on Linux ([NVIDIA, Advanced API Performance: SetStablePowerState](https://developer.nvidia.com/blog/advanced-api-performance-setstablepowerstate/)).
  - Nsight Compute locks clocks by default for reproducibility ([NVIDIA forum, staff answer](https://forums.developer.nvidia.com/t/nsight-compute-clock-speed-during-profiling/208646), search excerpt).
  - `nvidia-smi` exposes `clocks_event_reasons.sw_power_cap` and `.sw_thermal_slowdown` ([nvidia-smi(1) man page](https://man.archlinux.org/man/nvidia-smi.1.en), via search excerpt).

**Project facts about the rules (VERIFIED unless labelled)**
- **v4 rule** (spec 004 Q8, plus the margin in spec 005 "Expected outcomes"): resolved if the control is within ±0.5%, all runs agree in sign, and effect > max(2 × control deviation, 0.5%).
  - "Control deviation" is the worst run's (realworld report: "the in-round control deviated by 0.5% or more in some run").
  - The margin was fixed after the first empty-kernel run (RECORDED, v4 "Honest limits").
- **Rule v2** ([bounds-check-testing-framework/realworld-v1/analyze_v2.py](bounds-check-testing-framework/realworld-v1/analyze_v2.py)):
  - d_i = ln(median no_bce/bce) − ln(median bce_dup/bce);
  - two-sided 95% t-interval across runs;
  - checks run in the order GAIN → LOSS → EQUIVALENT (95% CI inside ±ln 1.005) → INCONCLUSIVE;
  - t-table `T975` is hard-coded only for df 1–12, so N > 13 runs would raise a `KeyError` (minor);
  - no multiplicity adjustment.
- **Post hoc rule v2 on realworld-1** (labelled post hoc per the spec). Source: `runs-realworld/summary-v2-{nvidia,intel}-realworld-1-19ffb4e.json`.

  | GPU | GAIN | LOSS | EQUIVALENT | INCONCLUSIVE | INVALID | vello control-corrected geomean |
  |---|---|---|---|---|---|---|
  | NVIDIA | 8 | 12 | 9 | 3 | 2 | 1.00358 over 32 scenes |
  | Intel | 22 | 7 | 0 | 1 | 4 | 1.07567 over 30 scenes |

  Examples of significant but sub-margin effects:
  - NVIDIA `robust_paths`: 0.9969 (CI 0.9941–0.9998), labelled LOSS;
  - NVIDIA `blur`: 1.0068 (1.0014–1.0124), labelled GAIN;
  - Intel `blur`: 1.0034 (1.0032–1.0035), labelled GAIN.

  Under the v4 rules all three blur/robust_paths cases above were "not resolvable" or small. The blur rows are in the realworld report table.
- **Slot-order imbalance (VERIFIED from code and data).**
  - Both harnesses rotate cyclically: `order = (r + j) % 4` (`realworld-v1/src/main.rs`:460, 477; `paired-vulkan-v4/src/main.rs`:441). The batched realworld-2 path still uses this code.
  - The four sequences are [nb,b,bd,u], [b,bd,u,nb], [bd,u,nb,b] and [u,nb,b,bd]. So `no_bce` precedes `bce` in 3 of 4 rounds, and `bce_dup` precedes `bce` in only 1 of 4.
  - The per-run record confirms it: `order_effect_ratio.n_first = 90, n_second = 30` of 120 rounds (`runs-realworld/20260929T215005Z-vello:07-fill_types/result.json`).
  - A cyclic Latin square balances *position*, not *pairwise precedence*. Because the effect numerator and the control numerator have opposite precedence imbalance relative to `bce`, any position effect enters e_i and c_i with opposite signs. The subtraction d = e − c therefore **adds** it instead of cancelling it.
- **Size of that order effect in existing data.** Read-only analysis this session: per run, median of the per-round log-ratio when X precedes `bce` minus when `bce` precedes X, in percentage points. Only realworld-1 and v4 were used.

  | harness | GPU | runs | effect term median (IQR) | control term median (IQR) |
  |---|---|---|---|---|
  | realworld-1 | Intel | 188 | −0.052 (−0.188…0.008) | +0.086 (0.040…0.196) |
  | realworld-1 | NVIDIA | 380 | −0.057 (−0.532…0.442) | −0.073 (−0.566…0.346) |
  | v4 | Intel | 70 | −0.009 | −0.001 |
  | v4 | NVIDIA | 140 | −0.002 | 0.003 |

  - Intel realworld-1 shows a consistent sign in the control term (IQR entirely > 0).
  - The v4 report separately recorded "up to ~0.8% higher" ratios depending on order for matmul (RECORDED).
  - The script was run ad hoc and not saved, so it needs re-running before being cited in a paper.

### Inferences

**Why the fixed margin plus worst-run rule is weak**
1. The margin was set after observing control data. Lakens requires prespecified bounds, so the v4 rule is exploratory.
2. The worst-run criterion is an extreme-order statistic. Its expected value grows with N, so adding runs lowers the chance of passing, which inverts the usual power relationship. This follows directly from the rule's definition; no source was found that analyses this exact rule.
3. A single margin cannot fit both a 0.12 ms vello frame and a long v4 dispatch. The realworld report says so itself ("too tight for them on this GPU").

**What is sound in rule v2**
- The process is the top-level experimental unit. There is a robust within-process summary (median of paired per-round log-ratios) and a t-interval across processes.
- This matches Georges §4.2 (per-invocation summary, then CI across invocations). It also matches Kalibera & Jones §9, where the CI over top-level means is the correct multi-level interval.
- Log-ratios make the ratio CI symmetric. This is a standard alternative to Fieller.
- Pre-registration before the confirmatory campaign is exactly Lakens' requirement.

**Recommended amendments.** Apply these as reported secondary analyses of the running campaign, labelled as such. Do not change the registered primary rule after seeing data.
1. **Four-outcome labels (Lakens).** Report "GAIN (practically equivalent)" when the CI excludes 0 but lies inside ±0.5%, e.g. NVIDIA `robust_paths` −0.31%. Also report the TOST 90% CI next to the 95% CI.
2. **Multiplicity.** With 38 workloads × 2 GPUs = 76 intervals, about 4 false GAIN/LOSS labels are expected under a global null at 95%. Report the count expected by chance. Alternatively, add a Holm or Benjamini–Hochberg adjusted column, or lead with suite-level estimates.
3. **Balanced order design.** Use a Williams design (for 4 treatments, 4 sequences in which every slot precedes every other equally often), or random permutations per round. At minimum, fit log(t) ~ slot + position + (1 | run/round) as a sensitivity analysis. This is a mixed-effects model in the spirit of Kalibera & Jones' random-effects levels. It removes position effects instead of relying on the control subtraction.
4. **Report e_i alone next to d_i.** The control correction adds control noise and, under the cyclic design, doubles position bias. Showing both is transparent.
5. **Independence diagnostic.** Report round-series ACF and lag plots for a sample of runs (Kalibera & Jones §6). If rounds are autocorrelated, switch the within-run bootstrap to a block bootstrap.
6. **Minimum-estimator robustness check (Chen & Revels).** For each run, compare the minima as well as the medians of each slot, e.g. min(no_bce)/min(bce). This fits the "1 in 5 samples 20–40% slower" additive-delay pattern. Caveat: DVFS clock changes are not purely additive delays, and Georges shows "best-of" can mislead. So this is a check, not a primary estimator.
7. **Clock control.** Lock clocks (`nvidia-smi --lock-gpu-clocks`, needs root) or record `clocks_event_reasons.*` in telemetry. With unlocked clocks, report the unlocked result as the "real laptop" condition and treat locked clocks as mechanism-isolating.
8. **Suite summary.** Report total frame time across scenes (sum of `median_ns_per_rep` per slot) and the distribution of per-scene effects, not only the geomean of ratios (Hoefler & Belli, Rule 4).
9. **Sample size by dimensioning.** Use the realworld-1 per-run variance of d_i to compute N per GPU for a target CI half-width (Kalibera & Jones §9.3). This replaces the fixed "10 NVIDIA / 5 Intel".

### Gaps
- No primary source was found that evaluates "worst-run control deviation" rules specifically. The criticism above is a derivation, not a citation.
- The magnitude of the position effect under the *batched* realworld-2 harness cannot be stated: those results were deliberately not used.
- Fieller versus log-t versus BCa bootstrap was not compared on this data.

---

## Q4. How should hit rate be reported, and can dynamic (execution-weighted) coverage be obtained?

### Takeaway
Report coverage at four levels: per access, per entry point / binary, per production clamp, and execution- or cost-weighted.

The project already has the data for a **cost-weighted** coverage metric that no reviewer has seen: the fraction of the measured cost of all checks that elision recovers. On the vello suite it is about 48% on NVIDIA and about 99% on Intel, computed from copied realworld-1 geomeans. A true execution-count-weighted metric needs per-site counters, which is moderate work.

### Cited Findings
- Per access: 118/729 real-world (16.2%), with vello at 98/440 (VERIFIED, F005 summary; F006 equal per the v4 addendum).
- Per entry point: 28 of 147 compiled entry points change their production binary (RECORDED, realworld report, Step 0).
- Per production clamp: vello bounds clamps 186 → 84, so 102 removed (54.8% of vello's bounds clamps; the percentage is computed here) (RECORDED, realworld report).
- Concentration:
  - vello is 440 of 729 accesses (60.4%, computed here);
  - the older project-corpus rate falls from 74.1% to 20.4% when the `mat_cx*` near-duplicates are removed (VERIFIED, concentration log).
- Ceiling: at most ~44% (320 of 729) provable in principle; 46% runtime-sized storage buffers (RECORDED, gaps report).
- Cost-weighted recovery, computed here as ln(gain)/ln(cost of all checks) from the realworld suite geomeans (RECORDED numbers):

  | GPU | gain | cost of all checks | recovered |
  |---|---|---|---|
  | NVIDIA | 1.0071 | 1.0149 | ≈ 0.478 |
  | Intel | 1.0671 | 1.0678 | ≈ 0.990 |

  - The per-scene `bce/unchecked` column gives the same information from the other side: Intel suite 1.0006, NVIDIA 1.0054.
  - In v4 on `19ffb4e`, the leftover cost is 0.9998–1.0004 on every kernel and GPU (RECORDED), i.e. about 100% recovered on the synthetic kernels.

### Inferences
- **Per-access rate:** report it both micro-averaged (118/729) and macro-averaged per project. Add a cluster (per-file or per-project) bootstrap CI so that the vello and `mat_cx*` concentration is visible, not hidden.
- **Per-entry-point:** 28/147 binaries changed. This is the number most relevant to runtime, because unchanged binaries cannot change time.
- **Per-clamp in production binaries:** 186 → 84 for vello. Extend the screen to all 28 changed entry points.
- **Cost-weighted:** fraction of all-checks cost recovered, with CI across runs; this is computable from existing `result.json` files.
- **Execution-count-weighted, true dynamic coverage.** The obvious method is instrumentation. Emit a variant in which each residual clamp site increments an atomic counter in an extra storage buffer, run one frame per scene, and weight each site by its execution count. That gives "X% of dynamically executed bounds checks were elided" per scene.
  - Cost: ~1–2 days in naga's SPIR-V backend or as a post-pass. This is an inference, not a validated tool.
  - An alternative without code changes is to use the per-instruction execution counters of the Nsight Graphics Shader Profiler on Vulkan compute ([Nsight Graphics docs](https://docs.nvidia.com/nsight-graphics/UserGuide/shader-profiler.html), search excerpt).
- **Answer to R3:** the cost-weighted metric, not the 16% static rate, is the right answer to "do the patterns cover enough opportunities". On Intel, elision recovers essentially all the measured check cost on vello even though it proves only 22% of vello's static accesses. That gap between the static and weighted metrics is itself a finding worth one figure.

### Gaps
- Whether naga or wgpu has an existing instrumentation hook for per-site counters was not investigated.
- Cost-weighted recovery with CIs per scene is not computed here. The ratios above are from suite geomeans only.

---

## Q5. What minimal additional experiments would most raise a reviewer's Experimental Evaluation score for a poster?

### Takeaway
For a poster, the bottleneck is presentation and rule discipline, not more GPU time.

Doable in roughly 3–4 working days, most of it GPU-free:
1. Report the confirmatory campaign under the pre-registered rule v2, with the four-outcome labels and a chance-count note.
2. Add compile-time overhead (E1).
3. Add code size from the saved artifacts (E2).
4. Add a static Tint comparison (E4).
5. Present coverage at four levels, including cost-weighted recovery (Q4).

### Cited Findings
- The loudest review signal was Experimental Evaluation = 0 from all four reviewers. R1 asked specifically for "comparison with existing bounds-check-elimination techniques" (RECORDED, 02-REVIEWS.md:32, 43-44).
- Tint's range analysis predates the work and is uncited (RECORDED, evidence ledger :69). No comparison artifact exists (VERIFIED, no epoch section 0.4 file).
- Kalibera & Jones recommend summary statements such as "we are 95% confident that system A is faster than system B by 5.5% ± 2.5%" (§4) ([link](https://kar.kent.ac.uk/33611/45/p63-kaliber.pdf)).

### Inferences

Priority order for a poster. Each step needs no new GPU campaign unless marked.
1. **(0 GPU) One results panel under rule v2:** per-GPU suite effect with CI, the verdict counts, the count expected by chance, and the four-outcome labels. Add the v4 kernels as the "mechanism" panel.
2. **(0 GPU) Coverage panel** at four levels (Q4), including the static-versus-cost-weighted contrast and the 44% ceiling.
3. **(CPU only, E1) Compile-time overhead** on vello plus the census files.
4. **(CPU only, E4) Tint versus naga proof counts** on the same 98 files, or at least on vello plus imageBlur.
5. **(0 GPU, E2) SPIR-V size deltas** from saved artifacts.
6. **(Short GPU run, E3) One Intel ISA diff** explaining "driver hides clamp" versus "clamp in hot loop".
7. **Stretch (GPU):** per-stage vello (E5), a Williams-balanced order re-run of a subset, and clock-locked NVIDIA if root is allowed.

Branch plan in the shape required by AGENTS.md Rule 3, for E1 + E4 together:

| Branch | What happens | Next action |
|---|---|---|
| Success | Overhead < a few % and Tint comparison completes | Two new poster panels |
| Partial | Tint cannot be run on some files | Report per-file coverage of the comparison, not a substitute |
| Bad | Compile time grows superlinearly on some shader | That is a finding; report it and profile |
| Disaster | Tint build fails | Record the error verbatim and fall back to a qualitative comparison from Tint source |

### Gaps
- Poster-venue expectations for evaluation depth (IICT 2026) were not researched.
- Effort estimates are this agent's judgement, not measured.
