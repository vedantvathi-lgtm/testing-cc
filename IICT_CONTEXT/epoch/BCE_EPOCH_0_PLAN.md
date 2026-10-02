# BCE EPOCH 0 — PLAN

## BOUNDS-CHECK ELISION IN NAGA / wgpu: FROM THE WebGPU SAFETY CONTRACT TO SOUND, MEASURED ELISION

**Style sources:**
- `~/Documents/Notes/COMPILER_GOD_MASTER_PROMPT_v3.1.md`: Prajwal principle, WHY-first, two-layer rule, surface-detection test, backtracking.
- `~/Documents/Notes/TRUE_GPU_GOD.md`: Part V section template and 8-layer GPU compiler trace.
- Project-epoch precedent: `~/Documents/Notes/HM_epoch_0_section_0.1–0.7.md`.

**File naming:** `BCE_epoch_0_section_0.N.md`. BCE = bounds-check elision, the project prefix, the way HM = Hail Mary.

**Audience:** you, from zero. Every term is bolded and defined with an analogy the first time it appears.

**Evidence rule (AGENTS.md):** every number, commit, file:line and claim in every section is copied from an artifact and carries VERIFIED / RECORDED / REPORTED / UNKNOWN. Nothing is recalled from memory. Wrong turns are taught as *what was believed, what was true, how it was found*. They are not hidden.

---

## Section template (applied to every section, no deviation)

Taken from TRUE_GPU_GOD Part V, as used in the HM epoch:

1. `# [EPOCH 0 — SECTION 0.N]: TITLE`
2. ⛔ **DO NOT SKIP**: which later sections break without this one, and why.
3. Prerequisites
4. THE WHY: WHY 1 Existence, WHY 2 Mechanical, WHY 3 Design Decision, WHY 4 Historical.
5. What Is This? (analogy first)
6. How It Actually Works — the 8-Layer Trace, adapted to this project (below).
7. The Causation Chain, with an explicit legality/soundness proof.
8. ASCII diagrams.
9. Code / IR example: every line commented, real tool output from this repo.
10. Surface Detection Self-Audit, FM1–FM6 PASS/FAIL.
11. Correctness and Testing Dimension.
12. Common Bugs and Misconceptions (5–10).
13. Integrated Research: only papers verified in `references/` or `CITATION_MANIFEST.md`.
14. Connection to the Real Project (exact paths/commits).
15. Resources.
16. Note-Taking Instructions.
17. Mini-Project: runnable in this repo, with expected output.
18. Mastery Checklist: WHY / Trace / Causation / Counterfactual questions.
19. Teach-Back.
20. Interview Fireproofing.

### The 8-layer trace, adapted to this project

| Layer | Name | What it covers |
|---|---|---|
| A | WGSL source | what the programmer writes, e.g. `t[i]` |
| B | Front end | WGSL (or GLSL / SPIR-V) → **Naga IR** (arenas, handles, `Expression::Access`, `Emit`) |
| C | Validator / analyzer | types, uniformity, and the **elision analysis** that produces `FunctionInfo.safe_accesses` |
| D | Policy decision | `BoundsCheckPolicy` per address space; `access_needs_check` consumes the proof set |
| E | Backend emission | SPIR-V `OpULessThan` + branch, or `UMin` clamp; MSL `metal::min`; HLSL `min` |
| F | wgpu layers | wgpu → wgpu-core (validation) → wgpu-hal (per-backend Naga options: Restrict / Unchecked / robustBufferAccess2, `force_loop_bounding`) |
| G | Driver | pipeline creation compiles SPIR-V / MSL / HLSL to machine ISA; driver-side robustness |
| H | Hardware and measurement | what the GPU executes, and how its cost is timed (timestamp queries, per-round ratios) |

**Running example**, used across sections: the tree reduction `s[lid + stride]` under `if (lid < stride)`.
- It is unproven on the submitted paper's analysis.
- It is unproven after the Aug 28 rewrite; its leftover check measured +1.20% (NVIDIA) and +7.30% (Intel).
- It is proven after wgpu `2ffe1b7`; the leftover check goes to 1.0000 (final value from the pending campaign).

---

## The sections (13)

### 0.1 — WHAT A WebGPU PROGRAM IS: THE SAFETY CONTRACT, AND wgpu AS A BACKEND-AGNOSTIC LAYER

- **Covers:**
  - Why a browser lets untrusted web pages run GPU code at all.
  - WebGPU's promise that no shader can read or write outside its own buffers.
  - The stack that keeps that promise: WGSL → Naga → SPIR-V / MSL / HLSL / GLSL → driver → GPU.
  - Why wgpu is "backend-agnostic": one API, four native backends (Vulkan, Metal, D3D12, GL). wgpu-core validates; wgpu-hal speaks each native API.
  - Firefox ships wgpu + Naga; Chrome ships Dawn + Tint.
  - Why CUDA / Vulkan have no such promise, and therefore no bounds checks to elide.
- **Evidence:** WGSL/WebGPU specs in `references/`; `wgpu-core/src/device/resource.rs`; `wgpu-hal/src/vulkan/adapter.rs`.
- **Mini-project:** trace one WGSL compute shader from `create_shader_module` to the SPIR-V wgpu hands the driver.
- **Why first:** every later section is "how the compiler keeps this promise more cheaply".

### 0.2 — INSIDE NAGA: FRONT ENDS, THE IR, THE VALIDATOR, AND THE BACK ENDS

- **Covers:**
  - Naga IR from first principles: arenas and handles, `Expression` vs `Statement`, `Emit` ranges, `LocalVariable` + `Load` / `Store`, `Access` vs `AccessIndex`, `FunctionArgument` builtins.
  - How the WGSL front end lowers `for`, `&&` (into control flow) and `var` / `let`.
  - How the GLSL and SPIR-V front ends differ: they keep `LogicalAnd`. That difference is at the root of the soundness bug in 0.8.
  - The validator and analyzer: `ModuleInfo`, `FunctionInfo`, uniformity.
  - The four back ends.
- **Evidence:** `naga/src/ir/mod.rs`, `front/wgsl/lower/mod.rs:2564`, `front/spv/next_block.rs:1836`, `valid/analyzer.rs`.
- **Mini-project:** dump the IR of a 6-line shader and label every handle.

### 0.3 — BOUNDS CHECKS: WHAT THE SPEC REQUIRES, WHAT NAGA EMITS, WHAT THEY COST

- **Covers:**
  - WGSL out-of-bounds semantics.
  - The three `BoundsCheckPolicy` values (ReadZeroSkipWrite, Restrict, Unchecked) and the exact SPIR-V each produces (`OpULessThan` + branch + `OpPhi`, versus `OpExtInst UMin`).
  - `IndexableLength` and runtime-sized arrays.
  - Driver robustness (`robustBufferAccess` / `robustBufferAccess2`) and **which policy wgpu really uses on each backend** (Vulkan / Metal / DX12 / GLES table).
  - Why a check inside a hot loop costs so much more than its one instruction: dependency, blocked address folding, driver behaviour.
- **Evidence:** `proc/index.rs`, `back/spv/index.rs:522`, the wgpu-hal policy table, `spirv-dis` output from `runs-v4/*/artifacts/*.spv`.
- **Mini-project:** compile one access under each policy and diff the SPIR-V.

### 0.4 — PROVING AN INDEX SAFE: INTERVALS, ABSTRACT INTERPRETATION, AND PRIOR ART

- **Covers:**
  - Static analysis from zero: abstract values, the interval lattice, join, soundness vs precision, "unknown must mean keep the check".
  - Overflow-checked endpoint arithmetic.
  - Path-sensitivity from branch conditions.
  - Loop induction variables and recurrences.
  - Relational facts: difference constraints, as in ABCD.
  - Prior art, each read from the archived sources: Cousot & Cousot POPL'77; ABCD (PLDI 2000); Würthinger et al. (2007 / 2009); LLVM SCEV + IRCE; Go `prove` / `loopbce`; Tint `IntegerRangeAnalysis` (Chrome 141).
  - wgpu#4360, which has requested this feature since 2021.
- **Evidence:** `references/`, `research_notes/related-work-bce.md`, the audit §4.
- **Mini-project:** hand-run an interval analysis on a 10-line loop.

### 0.5 — THE ORIGINAL PAPER: WHAT WAS CLAIMED, WHAT THE REVIEWERS SAID, WHAT WAS ACTUALLY TRUE

- **Covers:**
  - The submitted paper (#55): a spec-guarantee builtin rule plus syntactic loop-pattern matching; SPIR-V word counts; "0.0 ns" and 7,568 ns timing.
  - The 4 reviews: all "weak accept"; Experimental Evaluation = 0 from all four; R4's six questions ("constant indices only?").
  - The July audit (`a.md`): v1 described an interval analysis that did not exist.
  - The August reviews-driven response.
- **Taught as:** how claims outrun code, and why the project's evidence rules exist.
- **Evidence:** the submitted PDF, `IICT_CONTEXT/02-REVIEWS.md`, `a.md`, ledger anti-claims.

### 0.6 — THE INTERVAL-DOMAIN REWRITE: HOW NAGA'S ELISION ANALYSIS WORKS

- **Covers:**
  - A walk through `bounds_check_elision.rs` as of `8757d7d`: `IntRange` (U32 / I32), `add` / `sub` / `mul` with checked endpoints, `refine_range`, `join_ranges` / `join_values`, `Ranges` (locals) vs `Values` (expressions).
  - `analyze_block` / `analyze_loop`, mutation invalidation, `loop_range` + `recurrence_ranges` + equality exits, affine builtins.
  - Per-access proofs (`safe_accesses`) and how the SPIR-V / MSL / HLSL back ends consume them.
  - What the rewrite answered for each reviewer.
- **Evidence:** the source file with line citations; the stress suite; `TODO` §3.
- **Mini-project:** run `access_proofs` on 5 shaders and predict each result before running it.

### 0.7 — MEASURING COVERAGE HONESTLY: CORPORA, CONCENTRATION, AND THE REAL-WORLD CENSUS

- **Covers:**
  - The corpus study's "74.1% / 81.9%", and the discovery that **91% of it came from 3 near-duplicate matrix tests** (20.4% without them).
  - Building the per-access census tool and validating it against the corpus harness.
  - Adding real projects: vello (compute), bevy (moved to WESL), rerun, webgpu-samples.
  - The anatomy of the misses: 46% runtime-sized buffers bounded only on the host (unprovable by any shader compiler), vertex / instance indices, helper-function parameters, operators.
- **Evidence:** `logs/audit-20260927-corpus-concentration.log`, `research_notes/Real world shader elision gaps/`.
- **Mini-project:** re-run the census and reproduce the category table.

### 0.8 — THE SOUNDNESS BUGS: "UNKNOWN IS NOT IMPOSSIBLE" AND "A CONTINUE BEFORE THE GUARD"

- **Covers:**
  - How two unsound elisions were found: by bisecting a vello precision miss, and by questioning a rule's assumption.
  - Bug 1, `refine()` treating "cannot model" as "infeasible":
    - WGSL boolean `&`;
    - GLSL / SPIR-V `&&`;
    - why WGSL-only tests could never see it.
  - Bug 2, a `continue` skipping the loop guard.
  - The soundness argument for each fix.
  - Red → green regression tests.
  - On-device differential execution.
  - The lesson: every assumption in a sound analysis is a proof obligation.
- **Evidence:** wgpu `9c4615b`, `5365d84`; `logs/f001-*`, `fix-continue-*`, `audit-20260927-UNSOUND-*`.
- **Mini-project:** reproduce bug 1 on the parent commit and watch the check disappear from the SPIR-V.

### 0.9 — THE VERIFICATION MACHINERY: WHAT EACH TEST PROVES AND WHAT IT CANNOT

- **Covers:**
  - Stress tests asserting on the proof set, not on text.
  - Exhaustive interval property sweeps.
  - `diff-exec`: elided vs checked binaries executed on the GPU. Class A / B fixtures, and the "identical-hash" caveat.
  - The Go-style expectation checker:
    - the opcode-166 bug (`OpLogicalOr` counted instead of `OpULessThan`);
    - the redesign to per-access proofs;
    - must-fail control fixtures.
  - The census.
  - Why vacuous passes happen and how controls prevent them.
- **Evidence:** `bounds_check_stress.rs`, `diff-exec/`, `expectation-checker/`, their results JSON.

### 0.10 — BENCHMARKING I: WHY THE FIRST NUMBERS WERE WRONG

- **Covers:**
  - Benchmarking discipline from zero, built on your benchmarking notes: variance bands (1–2%), geometric mean, one variable at a time, controls, noise, system state.
  - The v3 harness and its audit, 5 PASS / 4 PARTIAL / 10 FAIL.
  - The central finding: **the GPU did not run the binaries the report described**. wgpu re-parses `ShaderSource::SpirV`, re-validates it and re-emits it with Restrict and loop bounding. It was found by replaying wgpu's pipeline.
  - The measured variant was ReadZeroSkipWrite on one workgroup, not production wgpu.
- **Evidence:** `spec-kit-work/specs/003-benchmark-suite-spec-check/{audit.md,evidence/}`.

### 0.11 — BENCHMARKING II: THE v4 HARNESS AND WHAT THE GPU REALLY PAYS

- **Covers the design:**
  - passthrough SPIR-V, so what runs is what is recorded;
  - wgpu's production Vulkan options;
  - 7 kernels at full occupancy with exact CPU references;
  - 4 slots per round: no_bce / bce / bce_dup / unchecked;
  - per-round ratios, and why they cancel a mid-run clock step;
  - steady-state warm-up;
  - noise gate before and after each run, provenance gate, telemetry.
- **Covers the results:**
  - NVIDIA +10.8% to +15.8% on 3 kernels; Intel differs (+11.6% on 1 kernel).
  - Drivers sometimes remove checks themselves.
  - Residual costs of unproven checks.
- **Covers the incidents:** thermal limits, a GPU job starting mid-run, a stale-binary provenance hole.
- **Evidence:** `reports/Benchmark v4 results.md`, `runs-v4/summary-*.json`.

### 0.12 — CLOSING THE MEASURED GAPS: FEATURES 005 AND 006

- **Covers:** each new rule, driven by a measured residual:
  - operators (`%`, `/`, `>>`, `&`, `min` / `max` / `clamp`, `select`, conversions);
  - range-valued guard bounds;
  - refining builtins and values, not only locals;
  - shrinking and geometric loop variables with a wrap check;
  - `!=` endpoint trimming (equality exits);
  - relational facts (`a ≥ b ⇒ a − b ≥ 0`) with compiler-enforced invalidation.
  - Before / after per kernel and GPU: the leftover-check costs of reduce, scan and binning going to ~1.0.
  - The census: 78 → 105 → 118 of 729.
- **Evidence:** wgpu `2ffe1b7`, `19ffb4e`; the final 2026-09-29 campaign (`summary-*-19ffb4e.json`, **pending: numbers inserted only after the campaign finishes**).

### 0.13 — WHERE THIS STANDS: NOVELTY, LIMITS, AND WHAT COMES NEXT

- **Covers:**
  - An honest position against Tint's range analysis and wgpu#4360. What is new: guard refinement, relational facts, equality exits, measured per-GPU evaluation. What is not new: the interval idea itself.
  - Limits that no shader compiler can cross: host-bounded buffers, draw-dependent vertex indices.
  - Next steps, each with a measured upper bound:
    - symbolic `arrayLength` facts;
    - check deduplication (matters on Metal);
    - loop versioning / IRCE-style hoisting;
    - interprocedural ranges (78 of 79 cases in one bevy file).
  - The path to an upstream PR (remove the `sdata_manual` hack; commit hygiene).
  - What the poster may and may not claim.
- **Evidence:** the audit reports, the ledger anti-claims, `reports/`.

---

## Order and dependencies

```
0.1 contract ─► 0.2 Naga ─► 0.3 checks ─► 0.4 theory ─► 0.6 rewrite ─► 0.7 coverage
                                 │                          │               │
                                 │          0.5 paper ──────┘               ▼
                                 │     (the claims vs. truth story)    0.8 soundness bugs
                                 │                                          │
                                 └──────────────► 0.10 bench I ─► 0.11 bench II ─► 0.12 gaps closed ─► 0.13 standing
                                                         ▲
                                         0.9 verification machinery (used by 0.8, 0.10–0.12)
```

- **CONNECTIONS BOXES** after 0.5 and after 0.10, following the master's rule of one every 5 sections.
- **Backtracking:**
  - after 0.8, revisit 0.6's soundness argument and mark exactly which assumption each bug broke;
  - after 0.12, revisit 0.7's miss categories with the post-005/006 numbers.

**Generation:** one section per request, as in the HM epoch. Section 0.12 is written last, so it quotes the final campaign numbers from artifacts.
