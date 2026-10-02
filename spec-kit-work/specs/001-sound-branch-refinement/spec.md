# Feature Specification: Sound branch refinement for bounds-check elision

**Feature Branch**: `001-sound-branch-refinement` (spec directory; code branch chosen at plan time)
**Created**: 2026-09-28
**Status**: Draft
**Input**: User description: "Continue and finish the unfinished work … use spec-kit … you be the
reviewer and the answerer and start the work." Selected as feature 001 because the 2026-09-27
audit found a soundness defect (evidence: `../IICT_CONTEXT/logs/audit-20260927-UNSOUND-logicaland-glsl.log`).

## Background (evidence, per Constitution I)

- VERIFIED 2026-09-27: when a shader branches on a comparison whose other side is not a known
  constant (e.g. `if a < b`), the elision analysis treats the branch as impossible instead of
  "nothing learned".
- Consequence A (all front ends): after such a branch, the analysis stops proving anything for
  the rest of the function. Real code loses elisions; in vello 26 provably safe accesses were
  missed (68 vs 94 with the prototype fix).
- Consequence B (GLSL input, likely SPIR-V input): for `if (a < b && c) { i = 100; }` only the
  true branch is treated as impossible, so the assignment is ignored and a later `t[i]` on an
  8-element array loses its bounds check. WGSL input is not affected because its front end
  lowers `&&` to nested branches.

## Clarifications

### Session 2026-09-28

Questions asked by the spec-kit clarify step; answered by Claude acting as answerer (per user
instruction), each answer backed by a check made this session.

- Q: Should SPIR-V input be verified in this feature, or left UNKNOWN? → A: Verify it.
  Evidence (VERIFIED): naga's SPIR-V front end maps `OpLogicalAnd` directly to the same
  conjunction operator as GLSL (`wgpu/naga/src/front/spv/next_block.rs:1836-1838`), so SPIR-V
  input reaches the same defect. A SPIR-V-input regression test is added (FR-009).
- Q: Can GLSL/SPIR-V regression tests run in the ordinary test suite, or do they need special
  build options? → A: Ordinary suite. Evidence (VERIFIED): the naga test build depends on
  `naga-test`, which enables the GLSL and SPIR-V input features (`wgpu/naga-test/Cargo.toml:21,23`),
  and existing snapshot tests already parse both inputs (`tests/naga/snapshots.rs:604,631`).
- Q: Where should the code change be made, given `wgpu` has a pre-existing uncommitted edit? →
  A: A new branch created from the current `bounds-check-elision` tip (`83a90e0`); the
  pre-existing one-line clippy fix in the stress-test file is carried along untouched and noted
  in the review. No commit or push without an explicit request (Constitution V).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - No unsafe elision on any shader input language (Priority: P1)

A native application compiles GLSL (or SPIR-V) shaders through naga with bounds-check elision
enabled. A branch assigns an out-of-range value to an index variable under a condition that
compares two runtime values combined with `&&`. The emitted code MUST still check that index.

**Why this priority**: an unsound elision is a memory-safety hole in a safety feature; the
constitution makes soundness non-negotiable.

**Independent Test**: compile the recorded GLSL repro with elision on and off; the access is
reported unproven and the emitted check count is identical in both builds.

**Acceptance Scenarios**:

1. **Given** the GLSL repro `if (a < b && b > 0) { i = 100u; } t[i] = 1.0;` with `t` of
   length 8, **When** it is validated with elision enabled, **Then** `t[i]` is not in the
   proven-safe set and one bounds check is emitted.
2. **Given** the same shader written with `||` and the assignment in the `else` branch,
   **When** validated, **Then** `t[i]` is not proven safe.
3. **Given** a GLSL shader where both branches keep `i` in range (e.g. `i = 3u`), **When**
   validated, **Then** `t[i]` is proven safe (the fix must not over-correct).

---

### User Story 2 - Analysis continues after comparisons it cannot model (Priority: P2)

A WGSL shader author writes `if x0 == x1 { y1 = y0; }` (two runtime values) early in a compute
kernel and later indexes a workgroup array with `local_invocation_id.x`. The later access MUST
still be proven safe.

**Why this priority**: this is the largest measured real-world gain available (vello +26), and
it is the same defect as Story 1.

**Independent Test**: compile a minimal shader with a two-variable comparison followed by
`sh[local_id.x]`; the access is proven safe.

**Acceptance Scenarios**:

1. **Given** `if a < b {}` (empty body) followed by `sh[local_id.x]` on a 256-element array
   with workgroup size 256, **When** validated, **Then** the access is proven safe.
2. **Given** `if a < b { c = a; } else { c = b; }` followed by the same access, **Then** the
   access is proven safe.
3. **Given** a genuinely infeasible branch (e.g. `var i = 5u; if i < 3u { i = 100u; }`
   followed by `t[i]`, length 8), **Then** the access is still proven safe (true
   infeasibility is still recognised).

---

### User Story 3 - Measured before/after on real-world shaders (Priority: P3)

A researcher preparing the IICT poster needs the before/after effect of the fix on each corpus
segment, with no loss on the project corpus.

**Why this priority**: required to state the improvement honestly (Constitution IV), but the
fix is valuable even without it.

**Independent Test**: re-run the per-access census on all corpora with the committed and fixed
builds and compare elided counts per segment.

**Acceptance Scenarios**:

1. **Given** the recorded census corpora, **When** the fixed build is measured, **Then** a
   per-segment table (webgpu-samples, wgpu, vello, bevy, rerun, misc, naga tests) is produced
   with committed and fixed counts.
2. **Given** the naga test corpus, **Then** its elided count is not lower than before (435).

### Edge Cases

- Comparisons where neither side is a tracked local (e.g. `if f(x) < g(y)`): no refinement,
  analysis continues.
- Non-comparison boolean conditions (`if flag`, `if a & b` on booleans): no refinement.
- Comparison of a local against a value whose range is known but not a single constant:
  treated as "cannot model" in this feature (range-valued bounds are a separate feature).
- Refinement that yields an empty range (e.g. `u < 0u` for unsigned `u`): still recognised as
  an impossible branch.
- `!` negation around an unmodellable comparison: no refinement either way.
- Nested `&&`/`||` chains mixing modellable and unmodellable comparisons: modellable parts
  still refine; unmodellable parts contribute nothing.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The analysis MUST treat a branch as impossible only when the facts it knows,
  combined with the branch condition, admit no value at all.
- **FR-002**: When a branch condition cannot be modelled (unknown operand, non-comparison
  operator, unknown bound, no tracked variable), the analysis MUST continue with the facts
  unchanged on that branch.
- **FR-003**: For a conjunction on its true branch (and a disjunction on its false branch), each
  part MUST contribute refinement only if it is modellable; an unmodellable part MUST NOT make
  the branch impossible.
- **FR-004**: Accesses reachable after a branch whose condition was unmodellable MUST continue
  to be analysed.
- **FR-005**: The GLSL soundness repro MUST be a permanent regression test, run in the normal
  test suite.
- **FR-006**: The WGSL precision case (two-variable comparison followed by a builtin-indexed
  access) MUST be a permanent regression test.
- **FR-007**: Existing tests (unit, integration, 58 stress tests, 5 interval property sweeps)
  MUST continue to pass unchanged.
- **FR-008**: The change MUST NOT add new elisions that are not justified by a proof valid on
  every path (Constitution II); any accesses newly proven safe on the census corpora MUST be
  listed and checked.
- **FR-009**: A SPIR-V-input variant of the soundness repro (a module containing a
  conjunction produced from SPIR-V input) MUST be a permanent regression test.

### Key Entities

- **Branch fact set**: the known value ranges of local integer variables on one side of a
  branch; "impossible" is a distinct state from "nothing learned".
- **Proven-safe access**: an array access whose index is proven in range on every path; the
  output consumed by the code generators.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 0 unsound elisions on the recorded GLSL repro and its `||`/else variant
  (previously 1).
- **SC-002**: 100% of pre-existing tests pass (at the time of writing 142 unit + 272
  integration + 6 doc tests).
- **SC-003**: Real-world elided accesses across the census corpora rise from 78 to at least
  100 (prototype measured 105), with no corpus segment decreasing.
- **SC-004**: The naga test corpus keeps at least 435 elided accesses.
- **SC-005**: Every access newly proven safe on the census corpora is reviewed and recorded as
  in range.

## Assumptions

- SPIR-V input shares the GLSL behaviour (same conjunction operator; see Clarifications) and
  is verified by FR-009.
- Range-valued bounds, new operators, and `!=` endpoint trimming are out of scope (separate
  features 002+).
- The census tool and corpora in `../research_notes/Real world shader elision gaps/` are the
  measurement baseline; committed-analysis numbers there are the "before" values.
- Code changes go to the naga fork `../wgpu` on a new branch; no commit or push without an
  explicit request (Constitution V).
