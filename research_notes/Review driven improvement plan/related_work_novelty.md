# Related work and honest novelty positioning for Naga bounds-check elision (answer to R1/R2/R3)

Session: 2026-10-01. Written under AGENTS.md Rule 0. Labels:
- `VERIFIED`: read in the primary source this session. A URL, file path or line is given, and web sources were accessed on 2026-10-01.
- `RECORDED`: in a project document, cited by path, not re-checked.
- `REPORTED`: stated somewhere and not confirmed.
- `UNKNOWN`: nobody has established it.

Scratch copies of the fetched sources (not project evidence) are in the session scratchpad. They are not committed. Nothing in the repo was changed except this file. No builds or GPU programs were run.

Fetch failures this session, recorded rather than worked around:
- DBLP API: connection reset on every query. Crossref API was used instead for bibliographic metadata.
- Chromium gitiles `+log` for `gpu/config/gpu_finch_features.cc`: HTTP 403 "Please sign in to view the history pages".
- `reeselevine.github.io/assets/pdf/saferace.pdf`: HTTP 404.
- ACM DL page for SafeRace: blocked for the scraper. The abstract came from Crossref instead.
- GitHub code-search API: HTTP 401, because it needs authentication.

---

## Q1. Classic literature: what each work contributed and what is relevant

### Takeaway
Every analysis ingredient in this project has a published ancestor:
- interval abstract domain (Cousot 1977);
- proving array index ranges safe (Suzuki & Ishihata 1977);
- flow-analysis check optimisation (Gupta 1993; Kolte & Wolfe 1995);
- difference constraints on SSA with guard-derived facts (ABCD 2000);
- conditions on SSA index expressions with explicit loop-variable overflow handling (Würthinger 2007/2009);
- weakly relational domains built for array bounds (Pentagons 2008/2010; Octagons 2006).

The project cannot claim a new analysis technique. At most it claims a new *placement* (Naga/WGSL, mandated-robustness semantics, multi-backend proof consumption) and a new *evaluation*.

### Cited Findings
- `VERIFIED` Cousot & Cousot, "Abstract Interpretation: A Unified Lattice Model…", POPL 1977, pp. 238–252. This is the lattice/fixpoint foundation of interval analysis. Local archive: `references/raw/papers/CousotCousot-POPL-77-ACM-p238--252-1977.pdf`. `RECORDED` as OCR-extracted in `references/extract/cousot-popl77/`; content cited per `research_notes/related-work-bce.md` row 9. [author PDF](https://www.di.ens.fr/~cousot/publications.www/CousotCousot-POPL-77-ACM-p238--252-1977.pdf)
- `VERIFIED` metadata (Crossref): Suzuki & Ishihata, "Implementation of an array bound checker", POPL '77, pp. 132–143, DOI 10.1145/512950.512963. Content not read this session. Cite bibliographically only; `CITATION_MANIFEST.md` has it as METADATA-ONLY. [DOI](https://doi.org/10.1145/512950.512963)
- `VERIFIED` metadata (Crossref) for the following. Content not read this session, so use for `\cite{}` only, not for quoted mechanism:
  - Gupta, "Optimizing array bound checks using flow analysis", ACM LOPLAS 1993, pp. 135–150, DOI 10.1145/176454.176507. [DOI](https://doi.org/10.1145/176454.176507)
  - Gupta, "A fresh look at optimizing array bound checking", PLDI 1990, SIGPLAN Notices pp. 272–282, DOI 10.1145/93548.93581. [DOI](https://doi.org/10.1145/93548.93581)
  - Kolte & Wolfe, "Elimination of redundant array subscript range checks", PLDI 1995, pp. 270–278, DOI 10.1145/207110.207160. [DOI](https://doi.org/10.1145/207110.207160)
  - Qian, Hendren & Verbrugge, "A Comprehensive Approach to Array Bounds Check Elimination for Java", CC 2002 (LNCS), pp. 325–341, DOI 10.1007/3-540-45937-5_23. [DOI](https://doi.org/10.1007/3-540-45937-5_23)
  - Nguyen & Irigoin, "Efficient and effective array bound checking", ACM TOPLAS 2005, pp. 527–570, DOI 10.1145/1065887.1065893. [DOI](https://doi.org/10.1145/1065887.1065893)
  - Logozzo & Fähndrich, "Pentagons", ACM SAC 2008, pp. 184–188, DOI 10.1145/1363686.1363736. Journal version: "Pentagons: A weakly relational abstract domain for the efficient validation of array accesses", Science of Computer Programming 2010, pp. 796–807, DOI 10.1016/j.scico.2009.04.004. [DOI](https://doi.org/10.1016/j.scico.2009.04.004)
  - Miné, "The octagon abstract domain", Higher-Order and Symbolic Computation 2006, pp. 31–100, DOI 10.1007/s10990-006-8609-1. [DOI](https://doi.org/10.1007/s10990-006-8609-1)
  - Venet & Brat, "Precise and efficient static array bound checking for large embedded C programs", PLDI 2004, pp. 231–242, DOI 10.1145/996841.996869. Surfaced by the search, same caveat. [DOI](https://doi.org/10.1145/996841.996869)
- `VERIFIED` ABCD (Bodík, Gupta, Sarkar, PLDI 2000, pp. 321–333, DOI 10.1145/349299.349342). Read in `references/extract/abcd/abcd.txt` p.1–2:
  - "ABCD works by adding a few edges to the SSA value graph and performing a simple traversal of the graph".
  - It "removes on average 45% of dynamic bound check instructions".
  - It needs "fewer than 10 simple analysis steps per bounds check".
  - It is "demand-driven".
  - Constraints are "difference constraints … of the form x − y ≤ c" (abcd.txt line ~106) on an "inequality graph" built from an "extended SSA (e-SSA)" form with π-assignments at branches (lines ~200–283).
  - Redundancy is proved by shortest-path reasoning (lines ~212–228).
  - ABCD also handles partially redundant checks through insertion (PRE).
  - [Crossref DOI](https://doi.org/10.1145/349299.349342)
- `VERIFIED` Würthinger, Wimmer, Mössenböck, PPPJ 2007 (DOI 10.1145/1294325.1294343). Abstract (`references/extract/wuerthinger07/wuerthinger07.txt` lines 16–30):
  - The algorithm works "on an intermediate representation in static single assignment form and maintains conditions for index expressions".
  - It "fully removes bounds checks if it can be proven that they never fail".
  - "Whenever possible, it moves bounds checks out of loops."
- `VERIFIED` Würthinger 2009 journal extension (`references/extract/wuerthinger09/wuerthinger09.txt`):
  - A hoisted check that fails "falls back to interpreted mode" (deoptimisation), abstract lines 17–21.
  - The evaluation reports "a speedup near to the theoretical maximum for … SciMark", and "array bounds checks do not have a significant impact on the performance of object-oriented applications" (DaCapo), lines 22–25.
  - §5.3 "Overflow of Loop Variable" (lines ~607–625): they must prove "not only that the added value is at least zero, but also that no overflow can occur". Because "proving the second part … is impossible in most cases", they rely on a deoptimising guard before the loop (`deoptimize if x > MAX − c + 1`).
  - §6.3 compares against the HotSpot server compiler (C2), which "groups checks where the same index instruction plus a constant offset and the same array are involved" (lines ~837–854).

### Inferences
- The project's per-access relational fact `a >= b` is a strict subset of ABCD's difference constraints. `VERIFIED` in `wgpu/naga/src/proc/bounds_check_elision.rs` @19ffb4e: `Relation {big, small, strict}` at lines 731–756; it is used only by `related_difference` (lines 764–778) to bound `a - b`, called from the `Subtract` case at line 533. That is a relation with constant 0 (or 1 when strict), with no transitive shortest-path closure. The paper should call it "a minimal difference-constraint fact (ABCD-style, c ∈ {0,1}, no transitive closure)", not "relational analysis" unqualified.
- The project's overflow rule is the static-only form of Würthinger's §5.3 problem. With no deoptimisation, a WGSL compiler must *prove* no-wrap statically or keep the check. That is a real, citable difference in setting: there is no speculation or deopt on GPUs.
- Pentagons (interval × strict upper bound `x < y`) is the closest *domain-level* description of what the project computes (intervals plus `big >= small` facts). Citing it would pre-empt "you reinvented a weakly relational domain". This is a content-level claim, so read the paper before quoting.

### Gaps
- Content of Suzuki & Ishihata, Gupta 1990/1993, Kolte & Wolfe, Qian et al., Nguyen & Irigoin, Pentagons, Octagons and Venet & Brat was **not read this session**, only metadata. Their mechanism descriptions must come from the papers themselves before any sentence goes beyond a bibliographic cite.

---

## Q2. Production compilers: how industrial BCE compares

### Takeaway
Industrial compilers go far beyond this project on the CPU side:
- Go `prove`: path-sensitive signed/unsigned limits plus relational posets.
- LLVM: IRCE loop splitting, LoopPredication guard widening, ConstraintElimination with Fourier–Motzkin over dominating conditions.
- HotSpot C2: pre/main/post iteration splitting.
- .NET RyuJIT: demand-driven range check on SSA with assertion merging.

In design the project is a small cousin of Go `prove`. It does no loop versioning or check hoisting (these need speculation or code duplication) and no general constraint solving.

### Cited Findings
- `VERIFIED` Go (`references/raw/go/go.git` @ `9834516`, `src/cmd/compile/internal/ssacompile/prove.go`):
  - A `relation` "set of possible relations between pairs of variables (v, w)". On a branch `v op w` "the set of relations is updated to exclude any relation not possible" (lines 45–64).
  - Separate signed and unsigned domains (lines 85–88), a `limitFact` per value (line 122), a `constDeltaAdd` for "non-over/underflowing additions like v = w + delta" (line 128), and orderings "like v < w" held in `orderS`/`orderU` posets (lines 137, 320).
  - It self-describes as "sound, but incomplete … performs no deduction or arithmetic" outside special cases (lines 149–150).
  - The per-value signed+unsigned `Limit` and overflow-rejecting `SafeAdd` are `RECORDED` in `research_notes/related-work-bce.md` row 4, and `loopbce` `(Phi min nxt)` induction detection in row 5.
- `VERIFIED` LLVM (`references/raw/llvm/llvm-project.git` @ `39e3e92`, 2026-08-26):
  - `InductiveRangeCheckElimination.cpp` header: it "splits a loop's iteration space into three disjoint ranges … such that the loop running in the middle loop provably does not need range checks".
  - `ConstraintElimination.cpp` header: "Eliminate conditions based on constraints collected from dominating conditions". Its `ConstraintSystem` eliminates "using Fourier–Motzkin elimination" (`include/llvm/Analysis/ConstraintSystem.h` line 73).
  - `LoopPredication.cpp` header: it "tries to convert loop variant range checks to loop invariant by widening checks across loop iterations" (`guard(i < len)` → `guard(n - 1 < len)`), then relies on unswitching.
  - SCEV is `RECORDED` as archived in `CITATION_MANIFEST.md`.
- `VERIFIED` .NET RyuJIT (`dotnet/runtime` main, `src/coreclr/jit/rangecheck.h`): for each bounds check it asks "the range of the index variable" and follows the SSA def chain through phis. A cyclic dependency marks the range "dependent", and it "merge[s] assertions from its pred block's edges for a phi argument", which gives e.g. an upper bound `a.len`. [source](https://raw.githubusercontent.com/dotnet/runtime/main/src/coreclr/jit/rangecheck.h)
- `VERIFIED` HotSpot C2 (`openjdk/jdk` master, `src/hotspot/share/opto/loopTransform.cpp`): it uses pre/main/post loops for range-check elimination ("RCE"); a comment at line ~1255 reads "We will actually perform iteration-splitting, a more powerful form of RCE". [source](https://raw.githubusercontent.com/openjdk/jdk/master/src/hotspot/share/opto/loopTransform.cpp)
- `RECORDED` CPU cost counterpoints:
  - Readyset: 651 checks removed, change "entirely explainable by measurement noise".
  - Rust Perf Book.
  - Both in `research_notes/related-work-bce.md` rows 7–8.

### Inferences
- Honest one-line positioning: "a path-sensitive interval + ordering-fact analysis in the style of Go's `prove` pass, restricted to Naga's IR and without loop versioning/predication (IRCE, LoopPredication, C2 RCE), which require code duplication or speculation."
- Why no IRCE/predication here is a legitimate design point, not only a limitation. WGSL robustness semantics (clamp or read-zero) make a hoisted, loop-invariant check awkward. A version-and-clamp scheme is possible in principle, but the project has not implemented or measured it. Status: `UNKNOWN`.

### Gaps
- V8 TurboFan/Turboshaft and JavaScriptCore (DFG/B3 integer range analysis) BCE were **not checked this session**. Do not cite them until a source file is read.
- LLVM ConstraintElimination's introduction date/author was not verified this session.

---

## Q3. GPU / WebGPU-specific prior work

### Takeaway
The most important prior work is **Tint's IntegerRangeAnalysis** in Chrome's WGSL compiler:
- It was built from Dec 2024 through 2025.
- It is wired into Tint's robustness transform behind a Dawn toggle.
- On Chromium `main` it is **enabled by default**.

It covers the same core pattern this project targets: local-invocation builtins from `@workgroup_size`, simple counted loops, and overflow-checked interval arithmetic, applied to constant-length arrays only. WebKit's WGSL compiler has no range analysis; it skips only constant indices. Naga upstream has only constant-index skipping (`RECORDED`). wgpu#4360 (2021) asks for exactly this feature and is still open.

GPU-architecture papers (GPUShield, cuCatch) and Futhark address GPU bounds checking in CUDA/OpenCL or functional-language settings, not WGSL.

### Cited Findings
**Tint IntegerRangeAnalysis (IRA), `VERIFIED`.** Source is `google/dawn` main, fetched 2026-10-01. The latest commit touching the `.cc` is `c43a057fa6` (2026-03-02).
- History (GitHub commits API on `src/tint/lang/core/ir/analysis/integer_range_analysis.cc`, 33 commits):
  - First commit `3bf3e9b20c` 2024-12-19, "Implement range analysis on `LocalInvocationIndex`".
  - `817f471a8e` 2025-01-13, `LocalInvocationId`.
  - Loop-control-variable work 2025-03-25 to 2025-05-04.
  - Operators added: Add 2025-05-09, Subtract 2025-05-10, Multiply 2025-05-16, Convert 2025-05-28, Divide 2025-06-06, `<<` and `>>` 2025-06-06, `min` 2025-07-16, `max` 2025-07-21, Modulo 2025-07-29.
  - [commits](https://github.com/google/dawn/commits/main/src/tint/lang/core/ir/analysis/integer_range_analysis.cc)
- Domain: `IntegerRangeInfo` is a variant of signed `int64` or unsigned `uint64` inclusive `[min_bound, max_bound]` (`integer_range_analysis.h`). [header](https://raw.githubusercontent.com/google/dawn/main/src/tint/lang/core/ir/analysis/integer_range_analysis.h)
- Handled values (`integer_range_analysis.cc`, `GetInfo(Value*)` ~lines 234–256): `Constant`, `FunctionParam` (builtins), `Var`/`Load` (loop control variables only), `Access` (a constant component of a function param), `Let`, `Binary`, `Convert`, `CoreBuiltinCall`. Anything else returns invalid, with `TODO(348701956): Support more instruction types`.
- Binary ops: Add, Subtract, Multiply, Divide, ShiftLeft, ShiftRight; Modulo is special-cased even when the LHS has no range (lines ~259–300). Others return invalid ("TODO … Support more binary operators").
- Builtins: only `min` and `max` (lines ~305–322).
- There is **no handling of `&`, `clamp`, `select`, or comparison-guarded refinement** in `GetInfo`. Absence was checked by reading the dispatch switch, not by exhaustive search of every file.
- Builtin ranges: `local_invocation_index` ∈ `[0, x·y·z − 1]` and `local_invocation_id` per component, from `func->WorkgroupSizeAsConst()` (lines ~814–830).
- Loops (`AnalyzeLoop`, lines ~323–390; header doc lines ~130–175). All of the following must hold:
  - the initializer block has exactly 2 instructions and sets the variable to a **constant**;
  - the continuing block has exactly 4 instructions and does `±1` ("add one or minus one");
  - the body starts `load; compare-with-constant; if` where the false block only contains `exit_loop`;
  - the variable is used only by loads, plus the store in the continuing block (lines ~555–565);
  - the comparison is `<, >, <=, >=` against a **constant integer** (lines ~585–600), and always-true/always-false comparisons are rejected.
  - The resulting range is attached to the `Var` for all its loads, which is flow-insensitive but sound under those restrictions.
- Robustness use (`src/tint/lang/core/ir/transform/robustness.{h,cc}`):
  - `RobustnessConfig::use_integer_range_analysis` defaults to `false` (robustness.h line 69).
  - `IndexMayOutOfBound` returns true "when `limit` is not a constant value". **Runtime-sized arrays are therefore never elided by Tint IRA.**
  - A signed range with `min_bound < 0` is rejected before comparing to the constant limit.
  - When the index may be out of bounds, the code is `b.Min(CastToU32(idx), limit)`, i.e. clamping.
  - [robustness.cc](https://raw.githubusercontent.com/google/dawn/main/src/tint/lang/core/ir/transform/robustness.cc)
- Enablement:
  - Dawn toggle `enable_integer_range_analysis_in_robustness` ("skip doing index clamping when the out of bound access cannot happen", crbug 348701956; `src/dawn/native/Toggles.cpp` ~line 751).
  - It is defaulted from the platform feature `kWebGPUEnableRangeAnalysisForRobustness` on Vulkan, Metal, D3D12, D3D11 and GL (`PhysicalDeviceVk.cpp` ~1360, `PhysicalDeviceMTL.mm` ~612, `PhysicalDeviceD3D12.cpp` ~992, `PhysicalDeviceD3D11.cpp` ~381, `PhysicalDeviceGL.cpp` ~519).
  - Chromium `gpu/config/gpu_finch_features.cc` on main: `BASE_FEATURE(kWebGPUEnableRangeAnalysisForRobustness, base::FEATURE_ENABLED_BY_DEFAULT);` (line 333), mapped in `gpu/command_buffer/service/dawn_platform.cc`. [finch features](https://chromium.googlesource.com/chromium/src/+/main/gpu/config/gpu_finch_features.cc)
- Chrome 141 "What's New in WebGPU" (page "Last updated 2025-09-24"): "The Chrome team is progressively rolling out a new integer range analysis for Tint … aims to improve efficiency by reducing the need for costly bounds checking and will soon be enabled by default across all platforms. See issue 348701956." [Chrome blog](https://developer.chrome.com/blog/new-in-webgpu-141)

**Other WGSL compilers and requests**
- WebKit WGSL, `VERIFIED` (`Source/WebGPU/WGSL/BoundsCheck.cpp`, latest commit `30e039895da8` 2026-03-09):
  - it returns early only `if (access.constantValue())`;
  - otherwise it rewrites the index to `__wgslMin(u32(index), size - 1)`;
  - no range analysis is present in that file.
  - [source](https://raw.githubusercontent.com/WebKit/WebKit/main/Source/WebGPU/WGSL/BoundsCheck.cpp)
- `VERIFIED` wgpu#4360 "Range analysis for bounds checks": kvark, 2021-09-15, **open** on 2026-10-01. Body: "`workgroup_memory[invocation_id]` - since we know the range of invocation IDs", spawned from gpuweb#1202. [issue](https://github.com/gfx-rs/wgpu/issues/4360)
- `VERIFIED` GitHub issue search on gfx-rs/wgpu for "range analysis" / "bounds check elision" / "elide bounds" returned 4 hits. Only #4360 is on topic, so no upstream wgpu PR implementing range-based elision was found (search API, 2026-10-01). Upstream Naga's constant-index-only skipping is `RECORDED` (`reports/Bounds check elision project audit.md` §4.1, `proc/index.rs:335` @ `688e4761c`).
- `VERIFIED` wgpu backend policy (fork @19ffb4e, `wgpu-hal/src/vulkan/adapter.rs` ~2814–2827):
  - `index: Restrict`;
  - `buffer:` `Unchecked` if `robust_buffer_access2`, else `Restrict`.
  - So on devices with robustness2, storage/uniform buffer accesses already get no Naga checks. Elision then matters for the `index` policy: local, workgroup, private and constant-length arrays.

**GPU bounds-checking papers**
- `VERIFIED` GPUShield (Lee et al., ISCA 2022, pp. 27–41, DOI 10.1145/3470496.3527420): a "hardware-software cooperative region-based bounds-checking mechanism". It reduces overhead "by utilizing compile-time bounds analysis". That analysis is an LLVM-IR pass that walks GEP operand trees and elides runtime checks when an index is statically bounded (e.g. from `get_global_id` maxima) (§5.3.2). The static analysis "costs a few tens of milliseconds of compilation time" (§8.3). [PDF](https://hparch.gatech.edu/papers/jaewon_2022_isca.pdf)
- `VERIFIED` (metadata + abstract snippet) cuCatch (Tarek Ibn Ziad et al., PACMPL/PLDI 2023, pp. 124–147, DOI 10.1145/3591225) is a CUDA memory-safety *debugging* tool. The PDF was not read this session. [DOI](https://doi.org/10.1145/3591225)
- `VERIFIED` Henriksen, "Bounds Checking on GPU", IJPP 2021, pp. 761–775, DOI 10.1007/s10766-021-00703-4. The preprint `futhark-lang.org/publications/hlpp20.pdf` was read:
  - Futhark's safety-checking of array indexing on GPUs shows "geometric mean overhead … 4% and 6% on two different GPUs" over 19 benchmarks;
  - it can "eliminate all failure checking entirely" for failure-tolerant kernels (§3.5);
  - the focus is failure signalling, not range proofs.
  - [PDF](https://www.futhark-lang.org/publications/hlpp20.pdf)
- `VERIFIED` (Crossref abstract) SafeRace (Levine et al., PACMPL 9 OOPSLA2, 2025, pp. 697–725, DOI 10.1145/3763075):
  - It identifies a WGSL "specification vulnerability … where code with data races can be compiled to intermediate representations in which an optimizing compiler could legitimately remove memory safety guardrails".
  - It proposes that "program slices contributing to memory indexing must be race free".
  - [DOI](https://doi.org/10.1145/3763075)
- `RECORDED` GSOHC (ECOOP 2025) is CPU–GPU synchronisation motion, **not BCE** (`research_notes/related-work-bce.md` row 6).
- **Doc conflict (Rule 5):** `LITERATURE_BOUNDS_CHECK_ELISION_AND_IV.md` §2 calls GSOHC the "nearest recent GPU-shader static-analysis work". The archived-source check (row 6) wins, so GSOHC must not be cited as BCE prior art.

### Inferences
- The strongest reviewer attack is "Tint already does this and ships it in Chrome". It is now documented from source, and it must be cited prominently. The pre-existing audit (`reports/Bounds check elision project audit.md` §4) reached the same conclusion on 2026-09-27; this session re-verified it from source and added:
  - the enablement chain;
  - the constant-limit restriction in the robustness transform;
  - the `min`/`max`/`%` additions (July 2025).
- SafeRace bears on soundness. Facts derived from values read from racy shared or storage memory could be invalidated by races. The project's analysis ranges come from literals, locals, builtins and constant expressions (see Q5), not from loaded buffer contents. A paper sentence should state this explicitly, with a code citation, as a SafeRace-compatible property. This inference was not checked against SafeRace's full text.

### Gaps
- **Exact Chrome version in which IRA became default: `UNKNOWN`.** The Chrome 141 post says "progressively rolling out … will soon be enabled by default". Chromium main shows `FEATURE_ENABLED_BY_DEFAULT`, but the history page returned 403.
- crbug 348701956 contents (and any perf numbers Google measured) were not read. issues.chromium.org is a JS app and was not attempted.
- Vulkan `robustBufferAccess` / `robustness2` cost studies: no peer-reviewed study was found. The only measured data found is gpuweb#1202 (Q4).
- Other GPU BCE papers ("Guardian", GPUArmor, CUSAFE): only search snippets were seen. Not verified, do not cite.

---

## Q4. Is there a published study measuring the cost of WebGPU bounds checks, or of elision in WGSL compilers?

### Takeaway
No peer-reviewed study measuring WebGPU/WGSL bounds-check cost or WGSL elision benefit was found.

The closest measured data is:
- an informal GitHub issue (gpuweb#1202, 2020): 17–40% regressions on a matmul with Dawn robustness;
- Chrome's blog, which gives no numbers;
- Futhark's non-WebGPU GPU study (4–6% geomean).

This is the most defensible opening for the paper's evaluation contribution. Absence is stated relative to this session's searches, not proven.

### Cited Findings
- `VERIFIED` gpuweb#1202 "Out-of-bounds clamping brings big perf regression on TFJS ML demos" (qjia7, 2020-11-04, open). MatMul 1024×1024, RBA enabled vs `--disable-dawn-robustness`:
  - Radeon Pro 555X 6.594 vs 5.454 ms;
  - Intel Iris Plus (ICL) 8.022 vs 5.137;
  - UHD630 36.98 vs 21.807;
  - GTX1080 Ti 0.626 vs 0.468;
  - "17%~40% perf regressions … specially on integrated GPUs".
  - [issue](https://github.com/gpuweb/gpuweb/issues/1202)
  - The numbers were copied from the issue body fetched via the GitHub API.
- `VERIFIED` Chrome 141 blog gives no quantitative results for IRA ([link](https://developer.chrome.com/blog/new-in-webgpu-141)).
- `VERIFIED` Futhark GPU bounds checking costs 4% and 6% geomean on two GPUs (non-WebGPU; [PDF](https://www.futhark-lang.org/publications/hlpp20.pdf)).
- Search `"WebGPU shader bounds checking overhead robustness performance evaluation paper"` (firecrawl, 2026-10-01) returned:
  - gpuweb#35 (spec discussion);
  - DarthShader (fuzzing);
  - SafeRace (memory safety);
  - unrelated WebGPU/Wasm papers.
  - No elision-cost study.
- `RECORDED` project measurements:
  - Real-world vello suite (34 scenes), from `reports/Real-world benchmark results.md` lines 144–181:
    - Intel RPL-S iGPU elision gain geomean 1.0671 (+6.71%) over all 34 scenes, 1.0848 (+8.48%) over 27 unflagged scenes;
    - NVIDIA RTX 4060 Laptop 1.0071 (+0.71%) all, 1.0027 (+0.27%) unflagged, against a "cost of all checks" of 1.0134.
  - Intel iGPU losses on some scenes (e.g. −5.30% `robust_paths`) coincide with `no_bce/unchecked` < 1 as well (line 177).
  - These numbers precede the confirmatory campaign pre-registered in commit `98bd514` (now running). Until it completes, they are **exploratory, not gate-passed headline numbers** (AGENTS.md Rule 2).

### Inferences
- The paper can honestly claim: "to our knowledge, the first per-GPU measurement of what range-based elision recovers of WGSL's mandated robustness cost in a real application". The qualifier "to our knowledge" is required, and gpuweb#1202 and Futhark must be cited as the prior cost measurements.
- The gpuweb#1202 pattern (integrated GPUs hurt most) matches the project's exploratory Intel-vs-NVIDIA asymmetry. The hardware generations differ, so this is consistency, not confirmation.

### Gaps
- ACM DL / Google Scholar full-text search for "WGSL" + "bounds check" was not possible (ACM blocked the scraper), so the absence claim rests on the web search and the sources above.

---

## Q5. What is plausibly novel, verified against Tint's source

### Takeaway
Candidate-by-candidate verdicts after reading Tint's source:
- (1) "First range-based elision in Naga/wgpu (Firefox's WGSL compiler)": **defensible**.
- (2) Guard refinement / relational facts beyond Tint: **defensible but narrow**, and must not be claimed novel against the literature (ABCD, Go `prove`, .NET).
- (3) Per-GPU real-application measurement: **defensible as a contribution** once the confirmatory run passes the gates.
- (4) Soundness bugs and the front-end-dependence lesson: **defensible as an experience/negative-result contribution**. It is not an analysis novelty.

The analysis *technique* is not novel.

### Cited Findings
The Naga side was checked in `wgpu/naga/src/proc/bounds_check_elision.rs` @ `19ffb4e` (1979 lines), `VERIFIED` by reading function signatures and the cited line ranges. Tint side as in Q3.

| Capability | This project (Naga @19ffb4e) | Tint IRA (dawn main) | Literature precedent |
|---|---|---|---|
| Signed + unsigned intervals, overflow-checked arithmetic | yes: `IntRange::{U32,I32}`, `checked_add/sub` (lines 58–107) | yes: int64/uint64 variant, overflow-rejecting ops | Go `Limit`+`SafeAdd` (RECORDED); Cousot 1977 |
| `@workgroup_size` builtin ranges | local_invocation_index / id (lines 798–828) | same two builtins (cc ~814–830) | GPUShield uses `get_global_id` maxima (LLVM) |
| + − × ÷ % << >> | yes (lines 525–561) | yes | Go, LLVM |
| `&`, `clamp`, `select`, `as` casts | yes (`bit_and` 1470, `Clamp` 582, `Select` 589, `As` 592) | `Convert` yes; `&`/clamp/select **no** (not in dispatch switch) | Go |
| min / max | yes | yes (July 2025) | Go |
| Path-sensitive refinement from arbitrary `if` guards | yes: `refine()` (635+), `refine_range` (905); non-exact bounds use the bound's interval (713–719) | **no**: ranges only from loop exit pattern; `Var` range is flow-insensitive | ABCD π-nodes; Go posets; .NET assertions |
| Ordering fact `a >= b` used for `a − b` | yes, subtraction only (533, 764–778) | **no** | ABCD difference constraints (general); Go `orderS/orderU` |
| Loop variable: constant init, ±1, constant bound | yes | yes (exact 2/4-instruction shapes) | Go `loopbce`; IRCE |
| Non-unit, shrinking (`>>`, `/`), geometric (`*`) recurrences, wrap checks, equality exits | yes: `growing_update` 1108, `shrinking_update` 1070, `geometric_range` 1149, `recurrence_ranges` 1331; tests `geometric_range_rejects_wrap_around` 1655, `recurrence_requires_reachable_exit_and_no_wrap` 1679 | **no**: "add one or minus one"; `<,>,<=,>=` only | SCEV AddRecs; Go `loopbce`; W09 §5.3 overflow |
| Mutation invalidation (`forget_local`, `mutated_locals`) | yes (40, 978) | handled by shape restriction (loads only + continuing store) | standard |
| Loop bound non-constant (e.g. `arrayLength`, uniform) | no: `exact_constant` for recurrence bounds (1310–1321) | no: constant RHS only | IRCE, ABCD, Go handle symbolic `len` |
| Runtime-sized arrays elided | **no**: requires `IndexableLength::Known` (619–622) | **no**: `IndexMayOutOfBound` true for non-constant limit | Java/Go/LLVM all handle symbolic length |
| Where proofs are consumed | validator → SPIR-V/MSL/HLSL back ends (per task brief; not re-read here) | robustness transform in core IR, all backends | — |
| Shipped | no (fork) | yes, default-on in Chromium main | — |

- `RECORDED` soundness findings (`IICT_CONTEXT/04-EVIDENCE-LEDGER.md`):
  - Line 71, 2026-09-27: GLSL `LogicalAnd` false edge treated as infeasible.
  - Line 73, 2026-09-28 correction: also WGSL non-short-circuit `&` and SPIR-V input; "elision was unsound on WGSL, GLSL and SPIR-V input" before the fix.
  - Line 79, 2026-09-29: a second WGSL-reachable unsound case (`continue` before a top-of-body guard), fixed in wgpu `5365d84`.
  - Required wording: "two unsound cases found by audit, both fixed with regression tests".

### Inferences
Defensible novelty statements, in order of strength:
- (1) **The first sound range-based bounds-check elision for Naga/wgpu** (the WGSL compiler used by Firefox and native wgpu).
  - Supported by: wgpu#4360 open since 2021, the search finding no upstream PR, and upstream constant-only skipping (RECORDED).
  - Framing: this is an engineering and placement contribution, "bringing Tint-class analysis to Naga".
- (2) **Strictly larger proof scope than Tint IRA on the documented axes.** The axes are path-sensitive guard refinement, the a≥b subtraction fact, non-unit/shrinking/geometric recurrences with equality exits and wrap proofs, and `&`/`clamp`/`select`.
  - This is VERIFIED from both sources.
  - Frame it as "beyond the WGSL state of the art", never "novel analysis". Every one of these has CPU precedent (ABCD, Go `prove`, SCEV).
  - To carry weight it needs a measured counterpart: how many *additional* accesses these features prove on a real corpus (Q6-E1). Without that it is a feature list.
- (3) **Measurement.** A per-GPU, real-application (vello), controlled measurement of elision gain vs cost-of-all-checks, with a negative control. No published WGSL equivalent was found (Q4).
  - This is currently the strongest novelty claim.
  - It is only headline-grade after the confirmatory campaign passes AUDIT_INVENTORY gates.
- (4) **Soundness lesson.** An analysis that is sound for the WGSL front end's lowering can be unsound for other Naga front ends or other boolean operators, because the IR admits `Binary And/LogicalAnd` conditions that WGSL `&&` never produces.
  - This is a real, citable experience result. Two bugs were found by audit and fixed with regression tests.
  - It argues for differential testing across front ends. Present it as a lesson, not a technique.

What is **not** novel, and must not be claimed:
- interval-domain BCE;
- overflow-checked interval arithmetic;
- guard-derived facts;
- induction-variable ranges;
- workgroup-size-derived builtin ranges (Tint already does this, and GPUShield does the analogous thing for OpenCL/CUDA);
- per-access proof granularity (Tint IRA is per access too).

### Gaps
- Whether Tint's *other* passes (e.g. value numbering, or Tint's DirectVariableAccess/const-eval) remove additional checks in ways that narrow gap (2): **UNKNOWN**. Only IRA and the robustness transform were read.
- Where the proofs are consumed in the SPIR-V/MSL/HLSL back ends was taken from the task brief and not re-read this session.
- No head-to-head count yet of accesses proven by Tint vs Naga-fork on the same shaders: **UNKNOWN**. That is experiment E1.

---

## Q6. Comparison experiments that would be credible (Rule 3 branches)

### Takeaway
R1 wants comparison with other approaches. Three comparisons are cheap and directly answer the reviewers:
- **E1:** a static head-to-head of proven-access counts against Tint IRA on the same WGSL corpus.
- **E2:** runtime against clamp-everything and unchecked, which already exists in the exploratory data as `no_bce` / `unchecked`.
- **E3:** against driver robustness (robustness2 / `buffer: Unchecked`).

A fourth, **E4**, runtime Naga-fork vs Tint on the same GPU, is attractive but confounded by whole-compiler differences.

### Cited Findings
- Available levers (`VERIFIED`):
  - Tint: `RobustnessConfig::use_integer_range_analysis` (robustness.h line 69); Dawn toggle `enable_integer_range_analysis_in_robustness` (Toggles.cpp ~751); `tintOptions.disable_integer_range_analysis` (ShaderModuleVk.cpp ~297, ShaderModuleMTL.mm ~315).
  - wgpu: per-policy `BoundsCheckPolicies` with `Restrict` / `ReadZeroSkipWrite` / `Unchecked` (`naga/src/proc/index.rs` lines 42–102); Vulkan selects `buffer: Unchecked` under robustness2 (adapter.rs ~2816).
- `RECORDED` the existing harness already reports `no_bce`, `bce`, `bce_dup` (identical-binary control) and `unchecked` slots (`reports/Real-world benchmark results.md` race table header and summary table).

### Inferences
Each experiment below has its expected branches written before running (Rule 3). None has been run.

**E1: static head-to-head with Tint IRA.** No GPU needed, so it is safe while the campaign runs, though it needs a Tint build.
- Method: compile the same WGSL corpus with Tint (SPIR-V writer, robustness on, IRA on vs off) and count `min`/clamp instructions removed. Compare with the Naga-fork count of `safe_accesses`, restricted to *constant-length* arrays, since neither elides runtime-sized ones. The corpus is the project corpus, `references/raw/webgpu-samples` (75 .wgsl) and vello.
- Report per-feature attribution: accesses proven only by guard refinement, only by non-unit recurrences, and so on.
- Branches:
  - *Success*: Naga-fork ⊇ Tint on ≥95% of accesses Tint proves, plus a non-trivial extra set. Next: headline "superset + N extra".
  - *Normal/partial*: roughly equal counts, with the extras concentrated in a few shaders. Next: report honestly; the novelty rests on (1) and (3).
  - *Bad*: Tint proves accesses Naga-fork does not, e.g. from IR-level `let` folding. Next: file them as gaps and fix or scope the claim.
  - *Disaster*: Tint's results expose a Naga-fork proof that Tint rejects *and* that is actually unsafe. Next: a DEVIATION entry, a soundness fix, and re-run of the suite.
- Confound: Tint IR vs Naga IR shape (Tint's loop pattern is 2/4-instruction exact). State it.

**E2: clamp-everything vs elision vs unchecked (runtime).** This is already the design of the running campaign.
- Present "gain recovered / cost of all checks" as the comparison metric.
- Branches:
  - *Success*: the Intel ratio stays near 1, as in the exploratory data. Next: headline.
  - *Partial*: significant on Intel only. Next: frame as GPU-dependent, consistent with gpuweb#1202's integrated-GPU pattern.
  - *Bad*: the CI of the gain includes 0 on both GPUs. Next: headline becomes proof coverage plus a null runtime result, engaging Readyset/W09 DaCapo.
  - *Disaster*: the negative control CI excludes 0. Next: the harness is invalid; stop and investigate.

**E3: vs driver robustness.**
- On Vulkan with robustness2, compare three configurations:
  - (a) `buffer: Restrict` + elision;
  - (b) `buffer: Unchecked` (driver handles it) + elision on `index` only;
  - (c) everything `Restrict`, no elision.
- This answers "why not rely on hardware?" and quantifies how much of the win comes from non-buffer (workgroup/private) arrays, where robustness2 does not help.
- Branches:
  - *Success*: elision's win persists under (b), showing it matters beyond buffers.
  - *Partial*: the win vanishes under (b), meaning it was mostly buffer accesses the driver already covers. That is still publishable, but it narrows the claim to non-robustness2 devices, Metal and D3D.
  - *Bad or disaster*: robustness2 is unavailable on the test GPUs, or the driver behaves inconsistently. Record it and fall back to reporting (a) vs (c) only.

**E4: Naga-fork vs Chrome/Dawn end-to-end runtime.** Not recommended as a headline.
- Different compilers, backends and pipelines confound it.
- If done, use Dawn's toggle to isolate the IRA-on/off delta *within* Tint and compare deltas, not absolute times.

**E5: ABCD-style ablation**, optional and cheap.
- Disable guard refinement and relation facts separately and recount proofs.
- This answers R2 by showing *which* classical ingredient buys what in WGSL code.

### Gaps
- Whether a Tint/Dawn build is available locally for E1: **UNKNOWN**, not checked. Building Dawn is heavy, and no build was run per the constraints.
- The effort and feasibility of E3's policy switching in the running harness was not assessed.

---

### Reviewer-facing summary (draft wording, each clause anchored above)

> Range-based bounds-check elimination is classical: interval abstract interpretation (Cousot & Cousot 1977), difference constraints on e-SSA (ABCD, PLDI 2000), SSA index conditions with loop-variable overflow handling (Würthinger et al. 2007/2009), and production passes in Go (`prove`), LLVM (IRCE, ConstraintElimination), HotSpot C2 and .NET. In WGSL, Chrome's Tint compiler added an integer range analysis (2024–25, default-on in current Chromium) that removes clamps for local-invocation-derived indices and simple counted loops into constant-length arrays. Naga, the WGSL compiler in Firefox and wgpu, had no such analysis (wgpu#4360, open since 2021). We contribute (i) a sound range-based elision for Naga, which (ii) proves more than Tint's analysis on the documented axes (path-sensitive guard refinement, ordering facts for subtraction, non-unit/shrinking/geometric recurrences with wrap and equality-exit proofs), measured head-to-head on the same shaders, (iii) the first controlled per-GPU measurement, to our knowledge, of how much of WGSL's mandated robustness cost elision recovers in a real application, and (iv) two soundness bugs found by audit that show how front-end-dependent IR shapes break guard-refinement assumptions.

Clause (ii)'s "measured head-to-head" becomes true only after E1 runs. Until then it must read "on the documented axes, by source inspection".
