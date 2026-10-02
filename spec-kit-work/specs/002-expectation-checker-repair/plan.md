# Implementation Plan: Repair the expectation checker

**Branch**: `002-expectation-checker-repair` | **Date**: 2026-09-28 | **Spec**: [spec.md](./spec.md)

## Summary
The Rust dump helper (`dump/src/main.rs`) gains a `--proofs` mode that prints, per runtime
`Access`, `{line, safe}` from `FunctionInfo.safe_accesses` and the expression span. The Python
checker maps `// EXPECT:` markers to accesses on the same line and judges each one; the
`OpULessThan` delta (opcode 176) stays as a secondary structural line.

## Technical Context
**Language**: Rust 1.93 (helper, path-dep on `../../../wgpu/naga`), Python 3 (checker)
**Testing**: the checker's own fixture suites (`fixtures/` must pass; `fixtures-control/` must fail)
**Constraints**: helper builds against the current checkout of `wgpu` (branch
`bce/sound-branch-refinement` at the time of writing)

## Constitution Check
I: results JSON records revision + time — PASS. II: n/a (no analysis change). III: verdicts from
the proof set — PASS. IV: n/a. V: files only in the untracked framework dir; no commits — PASS.

## Outcome branches
- SUCCESS: main suite all PASS, control suite all FAIL, exit codes 0 / non-zero respectively.
- NORMAL: a main fixture fails because the analysis behaves differently from the marker
  → verify behaviour independently (census tool), then fix the marker with evidence.
- BAD: a control fixture passes → the checker is still vacuous; fix before any use.
- DISASTER: n/a (tooling only).

## Structure
```text
bounds-check-testing-framework/expectation-checker/
├── checker.py            # rewritten judging logic
├── dump/src/main.rs      # + --proofs mode (JSON per access)
├── fixtures/             # corrected markers
└── fixtures-control/     # deliberately wrong markers (must FAIL)
```
