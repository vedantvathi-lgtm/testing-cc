# Implementation Plan: Sound branch refinement for bounds-check elision

**Branch**: `001-sound-branch-refinement` (spec) · code: `bce/sound-branch-refinement` in `../wgpu`
| **Date**: 2026-09-28 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `specs/001-sound-branch-refinement/spec.md`

## Summary

`refine()` in `wgpu/naga/src/proc/bounds_check_elision.rs` returns `Option<Ranges>`, where
`None` means "this branch edge is infeasible". Several `?` operators inside it also turn
"cannot model this condition" into `None`. The fix gives `None` a single meaning — "the refined
range is empty" — and returns the facts unchanged for everything the analysis cannot model.
Regression tests cover the GLSL and SPIR-V soundness repros, the WGSL precision loss, and the
"true infeasibility still recognised" case. The prototype of this exact change (F1) was
measured on 2026-09-27: suite green, GLSL repro retains its check, real-world elisions 78 → 105.

## Technical Context

**Language/Version**: Rust 1.93 (pinned by `wgpu/rust-toolchain.toml`), crate `naga` (`no_std`)
**Primary Dependencies**: none new; tests use existing dev-deps (`rspirv`, `naga-test` which
enables `glsl-in` + `spv-in`)
**Storage**: N/A
**Testing**: `cargo test -p naga` (unit + `tests/naga` integration incl. `bounds_check_stress.rs`);
`cargo clippy -p naga --tests`; `cargo fmt --check -p naga`
**Target Platform**: compiler library (all wgpu backends consume the proof set)
**Project Type**: compiler analysis pass
**Performance Goals**: no measurable compile-time change (the fix removes early exits only in
unmodellable cases; analysis remains linear in statements)
**Constraints**: soundness non-negotiable (Constitution II); minimal, upstream-style diff
(Constitution V)
**Scale/Scope**: one function (`refine`, ~45 lines) + ~6 tests; call sites: only the
`Statement::If` handler (lines 227, 230) and `refine`'s own recursion (472–479) — VERIFIED by
grep, 2026-09-28

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design.*

| Principle | Status | How this plan satisfies it |
|---|---|---|
| I Evidence labels | PASS | Defect + prototype numbers cite `IICT_CONTEXT/logs/audit-20260927-*` and census files; new measurements saved as artifacts |
| II Soundness first | PASS | The change only *removes* false infeasibility; must-retain tests for GLSL `&&`, SPIR-V `&&`, `||`-else; must-elide test that true infeasibility is still used |
| III Test-first | PASS | Tasks order: write 4 failing tests → observe failure → implement → green |
| IV Measured, segmented | PASS | Census re-run per segment, before/after, newly elided accesses listed |
| V Repo safety | PASS | New branch in `../wgpu`; no commit/push without explicit request; `wgpu-upstream` untouched |

Post-design re-check: PASS (no new dependencies, no API change; `FunctionInfo.safe_accesses`
semantics unchanged — only fewer false negatives and no false positives).

## Outcome branches (AGENTS.md Rule 3)

- **SUCCESS**: new tests fail before / pass after; full suite green; clippy+fmt clean; census:
  real-world ≥ 100, no segment decreases, naga tests ≥ 435; every newly elided access reviewed
  in range. → Record in ledger + session log; offer commit to the user.
- **NORMAL**: all of the above but a snapshot file changes (emitted SPIR-V for an existing test
  shader loses a check) → inspect each snapshot diff; accept only if the newly elided access is
  provably in range; record.
- **BAD**: an existing must-retain stress test starts eliding, or the census shows an access
  newly elided that is not provably in range → revert the change, minimise, and treat as a
  second soundness defect (stop coverage work).
- **DISASTER**: WGSL soundness regression reproducible on device (diff-exec divergence) →
  stop; ledger claim "sound on WGSL" downgraded until root-caused.

## Project Structure

### Documentation (this feature)

```text
specs/001-sound-branch-refinement/
├── spec.md
├── plan.md              # this file
├── research.md          # Phase 0
├── data-model.md        # Phase 1
├── quickstart.md        # Phase 1
├── contracts/
│   └── proof-set-contract.md
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository: `../wgpu`)

```text
naga/src/proc/bounds_check_elision.rs   # refine(): the fix
naga/tests/naga/bounds_check_stress.rs  # new regression tests (GLSL, SPIR-V, WGSL)
```

**Structure Decision**: tests go in the existing stress-test file (it already hosts the
`access_proofs` helper and all elision regressions); no new files in the naga crate.

## Complexity Tracking

No constitution violations; nothing to justify.
