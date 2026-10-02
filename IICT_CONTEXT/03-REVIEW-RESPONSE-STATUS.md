# Review comment → response status matrix

> Companion to `02-REVIEWS.md`. Status vocabulary: **DONE-VERIFIED** (artifact exists,
> re-checked 2026-09-13) / **DONE-RECORDED** (artifact exists, cite source) /
**PARTIAL** / **OPEN** / **BLOCKED** (was blocked; see notes). Every row answers:
which reviewer said it, what we did, where the proof lives.

| # | Reviewer | Comment | Status | Evidence / where it lives |
|---|---|---|---|---|
| 1 | R1,R2,R3 | "Loop induction analysis not implemented / WIP" | **DONE-VERIFIED** | `wgpu` commits `86eb81c` (loop-exit-bound extraction + stress suite) then `8757d7d` (full interval-domain rewrite). 24→58 stress tests; `BOUNDS_CHECK_ELISION_TODO.md` §3/§4 |
| 2 | R1,R2,R3 | "Evaluation limited; no evidence patterns fire in practice" (hit-rate) | **DONE-VERIFIED** | Corpus study `corpus_hit_rate/results/RESULTS.md` (commit `6ff062b`): 295 programs; project corpus 1.24%→**74.1%** overall / **81.9%** compute; external 1–2.5%; `OpULessThan` 505→134 |
| 3 | R1,R2 | "No comparison with existing BCE techniques" | **PARTIAL** | Literature collected + positioned internally (`LITERATURE_…IV.md`; ABCD/W07/W09/IRCE/GSOHC archived, see `references/`). **Paper v2 still lacks a written related-work section** — OPEN item P3 |
| 4 | R2 | "Timing methodology under-specified; 0.0 ns reads as within-noise" | **PARTIAL** | Corrected numbers with CIs + negative control exist (`a.md` §10b/§11: 7,568 ns [7,456–7,744]; elided 0 ns [0,0]; control PASS). But these predate the schema-v3 runner; **fresh provenance-complete rerun required before headline use** — OPEN item P1 (driver now works) |
| 5 | R2 | "No real-world WGSL corpus" | **DONE-VERIFIED** | Three corpora in the study: naga tests (153), wgpu upstream (76), webgpu-samples (66; re-cloned 2026-09-13 to `references/raw/webgpu-samples`) |
| 6 | R2 | "Related work on BCE absent" | **OPEN** (P3) | Same as #3; the writing task remains |
| 7 | R2 | "Soundness corner cases: overridable workgroup size" | **PARTIAL** | Verified by reading: gating reads concrete `ep.workgroup_size` from IR, override can't produce bogus product (recorded in session analysis). **No pinned regression test** — OPEN item P4 |
| 8 | R2 | "Soundness corner cases: syntactic-match fragility" | **DONE-VERIFIED** | The rewrite replaces syntactic matching with interval domain; signed-IV negative-start hole (found Aug 25 session) closed by mutation-invalidation + overflow-rejecting arithmetic; `BOUNDS_CHECK_ELISION_TODO.md` §3 |
| 9 | R2 | "Soundness corner cases: Restrict interaction" | **DONE-RECORDED** | Elision check precedes `choose_policy` (`back/spv/index.rs`) — proven-safe indices skip clamping under every policy; both policies covered in stress suite |
| 10 | R3 | "Algorithms could have been presented better" + R4 "sketchy" | **OPEN** (P2) | v2 §3 pseudocode/figure still don't match implemented code (`BOUNDS_CHECK_ELISION_TODO.md` §6 explicitly lists "Rewrite algorithms to match implemented code exactly") |
| 11 | R3 | "Template driven — coverage unclear" | **DONE-VERIFIED** | Same artifact as #2; plus shape-classification roadmap (69/79 external misses = runtime-sized storage arrays) |
| 12 | R4 | "What kind of index expressions? constant only?" | **DONE-VERIFIED** | Non-constant elision proven in code + corpus; poster must state the constant-vs-runtime distinction loudly (answer table in `02-REVIEWS.md`) |
| 13 | R4 | "Recurrences?" | **DONE-VERIFIED** | Monotone recurrence recognition + eq-exits in `8757d7d` (TODO §3 lines) |
| 14 | ALL | "Evaluation" (per-axis score) | **PARTIAL** | Corpus study answers breadth; missing: GPU rerun under v3 (#4), multi-kernel shapes, baseline positioning (#3) |
| 15 | Org | "Poster must address all review comments" | **OPEN** (P5) | No poster artifact exists on disk yet (UNKNOWN whether made elsewhere) |
| 16 | Org | Optional 90-s video due Aug 27 | **UNKNOWN** | No artifact on disk; treat as not done unless author says otherwise |
| 17 | R1 | "Larger/more diverse benchmark suite" | **OPEN** (P1b) | Multi-kernel sweep under `paired-vulkan-v3` designed (`FIXED_BENCHMARKING_SUITE.md`), not yet run |

## The five priorities derived from this matrix

- **P1** — GPU schema-v3 rerun + negative control (unblocked 2026-09-13; driver verified working). Feeds #4, #14, #17.
- **P2** — Rewrite paper §3 pseudocode + figure to match `8757d7d` implementation. Feeds #10, #12.
- **P3** — Related-work section (ABCD, W07/W09, IRCE, Go prove/loopbce, GSOHC; Readyset counterpoint). Feeds #3, #6.
- **P4** — Overridable-workgroup-size pinning test + MSL/HLSL behavioral verification. Feeds #7, #9.
- **P5** — Poster assembled as claims→evidence matrix over this file. Feeds #15.
