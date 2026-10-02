# T4 — Corner-case pinning (overridable workgroup size) + MSL/HLSL verification

**Objective:** close R2's named corner cases with permanent tests; verify the
cross-backend proof consumption that the paper claims (and v1 falsely claimed —
this task makes the claim *tested*).

## Part A — overridable workgroup_size pinning test

**Why:** R2 flagged "overridable workgroup size" as an undiscussed soundness corner.
Recorded analysis: the gating reads the entry point's concrete `workgroup_size` from
the IR, so an override cannot produce a bogus product — but no test pins this.

**Steps:**
1. Read `wgpu/naga/src/proc/bounds_check_elision.rs` + `valid/analyzer.rs` to locate
   where `workgroup_size` is read (RECORDED: from `ep.workgroup_size`).
2. Add test(s) to `wgpu/naga/tests/naga/bounds_check_stress.rs` (append; do not
   reorder existing tests) covering:
   - entry point with explicit `@workgroup_size(4, 4, 4)` vs implicit/default,
   - 1×1×64 (product 64) with arrays of len 63 (retain) and 64 (elide) and 128 (elide),
   - a second entry point in the same module with a DIFFERENT workgroup size than
     the one containing the access (proves per-entry-point, not module-global, sizing).
   Expected elision/retention per the exact-equality/≥ semantics you read in the
   source — write the expectation FROM THE SOURCE, then run.

**Branches:**
- **SUCCESS** — new tests pass: R2 corner case #1 closed with named tests. Tick TODO.
- **NORMAL** — an expectation you derived from reading is wrong (test reveals actual
  semantics differ): the test was wrong, not necessarily the code. Re-read source,
  fix the TEST expectation on a branch, record what the code actually does in the
  session log (this is valuable spec documentation).
- **BAD** — a genuinely unsafe elision under an override shape (e.g., elides with a
  mismatched product): soundness bug → same handling as T3 BAD (stop, minimize,
  regression test, ledger downgrade).
- **DISASTER** — n/a.

## Part B — MSL/HLSL verification

**Why:** TODO §4 "Verify SPIR-V, MSL, and HLSL behavior separately" is open; paper
v2 says the analyses target Vulkan but the proofs are consumed by all three backends.

**Steps:**
1. Structural (this machine can do all of it): extend `elision_assertions.rs` or a
   new `wgpu/naga/tests/naga/` test file: compile each Class-A fixture from T3 (or
   the stress shaders) to MSL and HLSL with elision enabled vs disabled; assert:
   - enabled MSL output lacks the clamp/guard for proven accesses that SPIR-V elides
     (match backend-appropriate patterns — read the MSL/HLSL writer's check-emission
     code first; do NOT assume SPIR-V opcodes; MSL clamp shape differs);
   - disabled outputs retain guards;
   - enabled==disabled byte-identical for retain-fixtures.
2. Behavioral (only if a Metal/D3D device existed — it does NOT on this Linux box):
   skip and record "structural-only; no Metal/D3D device available" — explicitly,
   in the report and ledger. Do not simulate or claim behavioral coverage.

**Branches:**
- **SUCCESS** — structural cross-backend equivalence proven for all fixtures; TODO
  §4 backend item ticked with "structural-only" caveat recorded.
- **NORMAL** — MSL/HLSL guard patterns too dissimilar for clean assertions on some
  fixtures: assert on the strictest subset that is well-defined; document which
  fixtures are structurally assertable; do not weaken to "compiles without error".
- **BAD** — a backend elides where SPIR-V does not (or vice versa) on the same
  fixture: backend inconsistency bug → branch fix + session-log write-up + ledger
  note (cross-backend claim now *tested*, found wanting, until fixed).
- **DISASTER** — n/a.

## Gates

- T1 green. No GPU needed for Part B structural (it's pure compilation).
  Part A uses the CPU test suite only.

## Anti-hallucination rules

- Backend guard-emission shapes must be read from `naga/src/back/msl/writer.rs` and
  `naga/src/back/hlsl/writer.rs` (grep for bounds-check emission), never assumed.
- Record the actual test count added; do not inflate.

## Deliverables

- New/extended test files (on the existing branch, uncommitted, per git-safety rules
  — or committed ONLY if user explicitly instructs), session-log entry, TODO ticks.
