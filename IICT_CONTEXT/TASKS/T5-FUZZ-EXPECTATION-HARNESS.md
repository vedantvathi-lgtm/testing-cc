# T5 — Interval-domain fuzz + Go-style expectation harness (hardening)

**Objective:** (a) property-test the interval domain's internal invariants;
(b) make every future analysis change self-verifying via inline-annotated fixtures
(the Go `test/prove.go` pattern).

## Part A — interval property tests

**Steps:**
1. Read `wgpu/naga/src/proc/bounds_check_elision.rs` to inventory the interval ops
   (add/sub/mul? joins, refinements, widening). List them in your session log first —
   this inventory is a deliverable.
2. Create `wgpu/naga/src/proc/bounds_check_elision_property_tests.rs` (or a
   `tests/`-side file if the module's internals aren't exported; adapt, don't force).
   If a fuzzing dep is impractical, a deterministic exhaustive small-domain sweep is
   acceptable: iterate all boundary intervals over {0,1,u32::MAX-1,u32::MAX,
   i32::MIN/MAX, len±1...} × ops, since the domain is small.
3. Invariants to assert (the actual math):
   - **No unsound endpoint ever accepted**: an op on intervals that could overflow/
     underflow must yield "unknown" (None/conservative), never a narrowed interval.
     Test by brute force on tiny bit-widths if the implementation permits, else on
     the boundary set above.
   - **Joins never shrink**: join(a,b) ⊇ a and ⊇ b (as runtime value sets).
   - **Refinement never widens**: refined interval ⊆ input interval under the same
     branch condition.
   - **Monotone recurrences terminate**: the recurrence solver never loops forever
     on cyclic inputs (fuzz a few thousand random CFG-ish op sequences if feasible;
     at minimum, hand-built worst cases).
4. Any property violation = potential soundness bug → same severity handling as T3 BAD.

**Branches:** SUCCESS (all invariants hold, artifact = test file + run log) /
NORMAL (some invariants untestable due to module privacy — record which and why;
test the rest) / BAD (violation found — stop, minimize, report, ledger downgrade) /
DISASTER n/a.

## Part B — Go-style inline-expectation harness

**Model:** Go's `test/prove.go` annotates each case with the expected proof outcome
inline; a runner compiles and checks. We mirror it for WGSL.

**Steps:**
1. Create `bounds-check-testing-framework/expectation-checker/`:
   - `fixtures/*.wgsl` where each expected-elision site carries a marker comment:
     `// EXPECT: ELIDED` or `// EXPECT: RETAINED` on the access line.
   - `checker.py` (or Rust bin — prefer Python, zero build cost): for each fixture,
     compile via the naga CLI-equivalent path (simplest: a tiny Rust helper bin in
     the same crate family using the same path-dep API as `paired-vulkan-v3` that
     dumps SPIR-V text per policy/elision flag; Python then greps) — for each
     marked access, determine elided/retained from the disassembly and compare
     against the marker. Output a pass/fail table with fixture+line numbers.
2. Seed fixtures: port the 58 stress cases' patterns + the T3 boundary set into
   marker-comment form (this creates the permanent, readable regression corpus).
3. Wire a README note: "run this after ANY analysis change".

**Branches:** SUCCESS (table green; harness documented) / NORMAL (some patterns not
mechanically checkable from disassembly — e.g., multi-dimension ambiguity; mark
those fixtures `MANUAL` and document the human check) / BAD (a marker says ELIDED
but disassembly retains, or vice versa, without a stale-marker explanation: drift
or bug — investigate, same severity ladder as T3) / DISASTER n/a.

## Gates

- T1 green. CPU-only; runs parallel to T2/T3 without conflicts (different repos/
  directories; T5 Part B reads wgpu via a separate helper bin — coordinate only if
  T3 is concurrently modifying `diff-exec/`; don't run two cargo builds of the same
  crate simultaneously).

## Anti-hallucination rules

- The invariant list must be derived from reading the actual implementation, and
  the inventory (function names + line refs) goes in the session log BEFORE writing
  tests.
- Do not report fuzz "clean" without stating the actual domain coverage exercised.

## Deliverables

- Property-test file + inventory; `expectation-checker/` crate + fixtures +
  pass/fail table artifact; session-log entry; TODO §4 fuzz item status update.
