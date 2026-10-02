# Tasks: Sound branch refinement for bounds-check elision

**Input**: `specs/001-sound-branch-refinement/` (spec, plan, research, data-model, contracts, quickstart)
**Tests**: REQUIRED (Constitution III — test-first, paired must-elide / must-retain)
**Paths**: relative to `../wgpu` unless stated.

## Phase 1: Setup

- [x] T001 Create branch `bce/sound-branch-refinement` from `bounds-check-elision` @ `83a90e0` in `../wgpu`; record that the pre-existing uncommitted edit in `naga/tests/naga/bounds_check_stress.rs` is carried along
- [x] T002 Record baseline: `cargo test -p naga` counts (expect 142 / 272 / 6) to `../IICT_CONTEXT/logs/f001-baseline-test.log`

## Phase 2: Foundational

- [x] T003 Add GLSL and SPIR-V-roundtrip proof helpers (`glsl_access_proofs`, `spv_roundtrip_access_proofs`) next to `access_proofs` in `naga/tests/naga/bounds_check_stress.rs` (they scan all functions, since the GLSL front end wraps `main`)

## Phase 3: Tests first — all new tests written and observed red (US1 + US2)

- [x] T004 [US1] Test `glsl_and_branch_assignment_retains_check`: GLSL `if (a < b && b > 0) { i = 100u; } t[i]` → proof `false`, in `naga/tests/naga/bounds_check_stress.rs`
- [x] T005 [US1] Test `glsl_or_else_assignment_retains_check`: `if (a < b || b > 0) {} else { i = 100u; } t[i]` → `false`
- [x] T006 [US1] Test `spv_input_and_branch_assignment_retains_check`: T004's module round-tripped through SPIR-V → `false`
- [x] T007 [US1] Test `glsl_and_branch_safe_assignment_elides`: `i = 3u` in the branch → `true` (no over-correction)
- [x] T008 [US1] Test `glsl_and_modellable_part_still_refines` (analysis A2): `if (i < 4u && a < b) { t[i] = 1.0; }` with `i` from a runtime load, `t` len 8 → `true`
- [x] T009 [P] [US2] Test `two_variable_comparison_does_not_stop_analysis`: WGSL `if a < b {}` then `sh[local_id.x]` (len 256, wg 256) → `true`
- [x] T010 [P] [US2] Test `two_variable_comparison_if_else_does_not_stop_analysis`: `if a < b { c = a; } else { c = b; }` then same access → `true`
- [x] T011 [P] [US2] Test `infeasible_branch_still_recognised`: `var i = 5u; if i < 3u { i = 100u; }` then `t[i]` (len 8) → `true`
- [x] T012 Run the new tests on unfixed code; expected RED: T004, T005, T006, T008, T009, T010; expected GREEN: T007, T011 (guards against over-correction). Save to `../IICT_CONTEXT/logs/f001-red.log`; any other outcome is a DEVIATION

## Phase 4: Implementation (US1 + US2)

- [x] T013 [US1] Implement the fix in `refine()` in `naga/src/proc/bounds_check_elision.rs` (only an empty refined range returns `None`; every unmodellable case returns facts unchanged); add a doc comment stating the `None` contract
- [x] T014 [US1] Re-run T004–T011 → all green; save to `../IICT_CONTEXT/logs/f001-green.log`

## Phase 5: User Story 3 — measured before/after (P3)

- [x] T015 [US3] Rebuild the census tool against this branch; run on all corpora; write per-segment BASE vs F001 table to `../research_notes/Real world shader elision gaps/census/F001_summary.txt`
- [x] T016 [US3] List every access newly proven safe vs BASE and review each for range correctness (FR-008 / SC-005) in the same summary

## Phase 6: Polish & gates

- [x] T017 Full `cargo test -p naga` (SC-002), `cargo clippy -p naga --tests`, `cargo fmt --check -p naga`; save logs
- [x] T018 Update `../IICT_CONTEXT/04-EVIDENCE-LEDGER.md` (soundness claim re-scoped) and append `../IICT_CONTEXT/08-SESSION-LOG.md`; mark outcome branch
- [x] T019 Review the full diff (reviewer pass) and prepare a commit message; do NOT commit without explicit user request

## Dependencies

T001 → T002 → T003 → (T004–T011) → T012 (red) → T013 (fix) → T014 (green) → T015 → T016 → T017 → T018 → T019.

## Parallel example

T009, T010, T011 are independent test functions in one file; they can be drafted together.

## Implementation strategy

MVP = Phases 3–4 (closes the soundness hole and restores precision). Phase 5 measures;
Phase 6 gates everything.

## Execution record (2026-09-28)

- T012 red run deviated from the prediction in a useful way — see DEVIATIONs D1–D2 below.
- **D1** `DEVIATION: expected SPIR-V test to compare only t[i]; observed spv-in turns constant buffer
  indexing into Access (3 proofs [true,true,true]); consequence: test narrowed to workgroup-array
  accesses (helper workgroup_access_proofs); next: none — re-run red showed [true] vs [false] as intended.`
- **D2** `DEVIATION: expected WGSL input to be sound (ledger claim); observed WGSL non-short-circuit
  boolean '&' reaches refine() as Binary And, whose false edge was treated as infeasible → t[i]
  proven safe, SPIR-V check removed (1→0); evidence: IICT_CONTEXT/logs/f001-UNSOUND-wgsl-amp.log;
  consequence: soundness claim for WGSL was false before this fix; two WGSL tests added
  (wgsl_non_short_circuit_and_else_assignment_retains_check, wgsl_bool_equality_else_assignment_retains_check); next: fixed by T013.`
- **D3** `DEVIATION: expected clippy --tests clean at 83a90e0 (Sep-20 log); observed 4 pre-existing
  'wildcard matches only a single variant' warnings in the T5-A property sweeps
  (bounds_check_elision.rs tests, lines 1337-1361 at 83a90e0); consequence: not introduced by this
  feature, left untouched (out of scope); next: separate cleanup task.`
- Outcome branch: **NORMAL** (snapshot spv-shadow.info.ron changed; 9 new proofs reviewed in range),
  otherwise SUCCESS criteria met: 142/282/6 green, fmt clean, real-world 78→105, naga tests 435→435, 0 lost.
