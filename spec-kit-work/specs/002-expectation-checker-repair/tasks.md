# Tasks: Repair the expectation checker

Paths relative to `../bounds-check-testing-framework/expectation-checker/`.

## Phase 1: Tests first (US2 control suite, US1 corrected fixtures)
- [x] T001 [US2] Create `fixtures-control/` with 3 deliberately wrong fixtures: ELIDED on an unguarded OOB access; RETAINED on a spec-guaranteed `tile[lid]`; a marker on a line with no access
- [x] T002 [US1] Correct `fixtures/lid_spec_direct.wgsl`: one access per marked line
- [x] T003 [US1] Correct `fixtures/eq_exit_loop.wgsl`: `tile[i]` markers → `RETAINED (KNOWN-GAP: top-guard == exit)`; keep `output[lid]` ELIDED on its own line
- [x] T004 Run the OLD checker (opcode fixed only) on both suites and record which verdicts are heuristic / vacuous → `../../IICT_CONTEXT/logs/f002-before.txt`

## Phase 2: Implementation
- [x] T005 [US1] Add `--proofs` mode to `dump/src/main.rs`: JSON lines `{line, safe, index}` for every runtime `Access` (constant indices excluded, as in the corpus harness)
- [x] T006 [US1] Rewrite judging in `checker.py`: per-access verdicts from `--proofs`; ERROR for marker lines without accesses; opcode 176 delta as secondary; JSON output with revision/time; non-zero exit on FAIL/ERROR/UNCOMPILABLE
- [x] T007 [US1] Run main suite → all PASS; save `results/` JSON + `../../IICT_CONTEXT/logs/f002-main.txt`
- [x] T008 [US2] Run control suite → all FAIL/ERROR, non-zero exit; save `../../IICT_CONTEXT/logs/f002-control.txt`

## Phase 3: Record
- [x] T009 Ledger + session log; mark T5 Part B done in `../../BOUNDS_CHECK_ELISION_TODO.md` only if T007 and T008 both meet the success branch

## Execution record (2026-09-28)
- T004 (old logic, opcode fixed): main 6 PASS; control c3 (marker with no access) PASSED vacuously → the heuristic defect the redesign removes (`IICT_CONTEXT/logs/f002-before.txt`).
- T007 first run: `DEVIATION: expected main suite all PASS; observed ERROR on runtime_sized_storage.wgsl line 5 (MANUAL marker on a declaration line, no access); evidence: first block of f002 run; consequence: fixture defect exposed by FR-002; next: marker moved into a plain comment (no expectation changed).` Also fixed own bug: compiler revision path (printed "@").
- Final: main 7/7 PASS exit 0 (`logs/f002-main.txt`), control 3/3 FAIL/ERROR exit 1 (`logs/f002-control.txt`). Outcome: SUCCESS.
- Original checker preserved at `IICT_CONTEXT/logs/f002-checker-original-20260920.py`.
