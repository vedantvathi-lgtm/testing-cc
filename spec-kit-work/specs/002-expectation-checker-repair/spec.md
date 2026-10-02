# Feature Specification: Repair the Go-style expectation checker (T5 Part B)

**Feature Branch**: `002-expectation-checker-repair`
**Created**: 2026-09-28
**Status**: Draft
**Input**: "Continue and finish the unfinished work." T5 Part B was started in a Freebuff session
that ended mid-turn ("Your free session ran out while this turn was running", Freebuff thread
`fd3abeaa…` seq26, 2026-09-20 23:55). The partial checker in
`../bounds-check-testing-framework/expectation-checker/` was never logged.

## Background (evidence)

- VERIFIED 2026-09-27: the checker counts SPIR-V opcode 166 (`OpLogicalOr`) instead of 176
  (`OpULessThan`); every count is 0, so its recorded 2 PASS results are vacuous
  (`../IICT_CONTEXT/logs/audit-20260927-expect-opcode{166-rerun,176}.json`).
- With the opcode corrected: 4 pass / 2 fail. `lid_spec_direct` fails because one marked line
  holds two accesses (fixture defect). `eq_exit_loop` fails because the analysis does not elide
  top-guard equality exits (a known, sound precision gap).
- The checker attributes counts to source lines heuristically; per-access truth is available
  from the compiler's proof set.

## Clarifications

### Session 2026-09-28 (answered by Claude as answerer)

- Q: Should expectations be checked against emitted-code counts or the compiler's proof set? →
  A: Proof set, per access, keyed by source line (Constitution III: assert on
  `safe_accesses`); the emitted `OpULessThan` delta is kept as a secondary structural check.
  Evidence: naga records a source span per expression (used by the census tool, which matched
  the corpus harness exactly).
- Q: How should known precision gaps (top-guard `==` exit) be expressed? → A: As `RETAINED`
  with a `KNOWN-GAP` note, so the suite documents current behaviour and flips when the gap is
  fixed; never as `ELIDED` expected-to-fail.
- Q: How do we prove the checker can fail? → A: A separate control fixture set with deliberately
  wrong expectations; the checker MUST report every control fixture as FAIL.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Trustworthy per-access expectations (Priority: P1)

A developer changing the analysis runs one command and learns, per annotated access, whether
the compiler still proves (or refuses to prove) what the annotation says.

**Independent Test**: run the checker on `fixtures/`; every annotated access is reported with its
line, expectation and actual proof status; the run passes.

**Acceptance Scenarios**:
1. **Given** a line with two annotated accesses, **When** checked, **Then** both are evaluated
   (no heuristic attribution).
2. **Given** an `ELIDED` marker on an access the compiler does not prove, **Then** the fixture
   FAILS with the line number.
3. **Given** a `RETAINED` marker on an access the compiler proves, **Then** it FAILS.

### User Story 2 - The checker cannot pass vacuously (Priority: P1)

**Independent Test**: run on `fixtures-control/` (deliberately wrong markers) → every fixture
FAILS; a marker on a line with no access is an ERROR, not a pass.

### Edge Cases
- Marker on a line with no runtime access → ERROR.
- Accesses on a marked line with a constant index → ignored (not runtime accesses).
- `MANUAL` markers → reported, never pass/fail.
- Shader fails to compile → UNCOMPILABLE, counted as failure of the run.

## Requirements *(mandatory)*

- **FR-001**: Expectations MUST be evaluated per runtime access using the compiler's proof set
  and the access's source line.
- **FR-002**: A marker line with no runtime access MUST be an ERROR.
- **FR-003**: The emitted-check delta MUST use the correct opcode (`OpULessThan` = 176) and be
  reported as a secondary check.
- **FR-004**: The control fixture set MUST produce FAIL for 100% of its fixtures.
- **FR-005**: The checker's exit status MUST be non-zero on any FAIL, ERROR or UNCOMPILABLE.
- **FR-006**: Results MUST be written as JSON with harness version, compiler revision and UTC time.

## Success Criteria *(mandatory)*

- **SC-001**: Main fixtures: 100% of annotated accesses match (after fixture corrections).
- **SC-002**: Control fixtures: 100% reported FAIL.
- **SC-003**: Zero accesses attributed heuristically (every verdict names one access).

## Assumptions
- The checker stays in the (untracked) testing-framework repo; committing it is the user's call.
- Fixture corrections (splitting the two-access line; eq-exit markers → RETAINED + KNOWN-GAP)
  reflect the analysis's verified behaviour, not wishes.
