# Specification Analysis Report — 001-sound-branch-refinement

Reviewer: Claude (acting reviewer). Date: 2026-09-28. Artifacts: spec.md, plan.md, tasks.md,
research.md, data-model.md, contracts/, constitution v1.0.0.

| ID | Category | Severity | Location | Summary | Resolution |
|----|----------|----------|----------|---------|------------|
| A1 | Constitution (III test-first) | HIGH | tasks.md T011–T014 | US2 tests were scheduled after the fix (T009), with T014 trying to prove "red" retroactively via a scratch build — violates red→green ordering | Fixed: US2 tests (T009–T011) moved before the red run T012; the fix is T013 |
| A2 | Coverage | MEDIUM | spec FR-003 vs tasks | FR-003 (partial refinement from modellable conjuncts) had no positive test — only must-retain tests | Fixed: added T008 `glsl_and_modellable_part_still_refines` (`if (i < 4u && a < b) { t[i] }` → proof `true`) |
| A3 | Consistency | LOW | plan.md Technical Context | Line numbers (227/230, 472–479) are for `83a90e0`; they shift after the edit | Accepted; plan states the commit |
| A4 | Ambiguity | LOW | spec SC-003 "no corpus segment decreasing" | Segment list must match census files | Covered: quickstart + T015 list the same 7 segments |
| A5 | Constitution (V) | INFO | tasks T001 | Switching the checked-out branch of `../wgpu` is a visible state change (reversible: `git switch bounds-check-elision`) | Accepted; recorded in session log |

Coverage summary: FR-001 → T004,T005,T011; FR-002 → T009,T010; FR-003 → T004,T006,T008;
FR-004 → T009,T010; FR-005 → T004; FR-006 → T009; FR-007 → T017; FR-008 → T016; FR-009 → T006.
All 9 FRs and 5 SCs mapped. No unmapped tasks. Constitution: no remaining violations.
