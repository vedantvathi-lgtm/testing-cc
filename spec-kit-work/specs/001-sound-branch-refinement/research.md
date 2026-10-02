# Phase 0 Research: Sound branch refinement

## R1 — What does `None` from `refine()` mean today, and where does it leak?

- **Decision**: `None` must mean only "refined range is empty ⇒ edge infeasible".
- **Evidence (VERIFIED 2026-09-28, `bounds_check_elision.rs` @ `83a90e0`)**: the `If` handler
  (lines ~221–250) maps `None` to "branch not taken"; if both are `None` it returns
  `falls_through: false`, ending analysis of the enclosing block. Inside `refine`, `?` is
  applied to: `negate_comparison(op)`, `swap_comparison(op)`,
  `self.expression_range(..)`, `.exact()`, `self.local_full_range(local)`, and
  `refine_range(..)`. Only the last one can legitimately signal an empty range.
- **Rationale**: conflating "unknown" with "impossible" is unsound whenever it happens on only
  one edge (conjunction true-edge / disjunction false-edge), and imprecise when it happens on
  both.
- **Alternatives considered**:
  - A tri-state enum (`Infeasible | Unchanged | Refined(Ranges)`): clearer, but a larger diff
    touching the `If` handler; deferred as a possible follow-up refactor.
  - Making `refine_range` never return `None`: loses real infeasibility (spec US2-AS3).

## R2 — Does `refine_range` returning `None` remain sound?

- **Decision**: keep it as the sole infeasibility signal.
- **Rationale**: `current` is a sound over-approximation of the local's value on entry to the
  edge, so an empty intersection with the condition genuinely means no execution takes the
  edge. `refine_range`'s catch-all `_ => None` arm is unreachable after the fix because
  non-comparison operators are filtered out first.

## R3 — Which front ends produce `Binary LogicalAnd/LogicalOr`?

- **Finding (VERIFIED)**: WGSL lowers runtime `&&`/`||` to control flow
  (`front/wgsl/lower/mod.rs:2564-2591`); GLSL emits `Binary LogicalAnd` (probe
  `logs/audit-20260927-UNSOUND-logicaland-probe.rs`: `ir_has_LogicalAnd=true`); SPIR-V maps
  `OpLogicalAnd` to `Binary LogicalAnd` (`front/spv/next_block.rs:1836-1838`).
- **Test strategy**: GLSL source test; SPIR-V test built by round-tripping the GLSL module
  through naga's SPIR-V writer (elision off) and parsing it back with the SPIR-V front end —
  keeps the test self-contained with no binary fixtures.

## R4 — Expected measurement (prototype, RECORDED 2026-09-27)

`research_notes/Real world shader elision gaps/census/F1only_*.jsonl` vs `BASE_*`:
webgpu-samples 1→2, wgpu 2→2, vello 68→94, bevy 4→4, rerun 3→3, misc 0→0, naga tests 435→435.
