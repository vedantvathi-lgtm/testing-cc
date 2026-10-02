# T8 — Hygiene (follow-up-paper material; no deadline; independent of T2–T7)

**Objective:** dispose of the three known debts so they can't resurface as surprises.

## Part A — Track 3 back-edge bug (standalone `naga/` repo)

**Fact base (RECORDED, `a.md` §10d):** the WGUL post-barrier forward-scan in
`naga/src/back/spv/block.rs` is block-local; a hazardous workgroup write reached via
a loop back-edge is invisible to it, so a required post-barrier is elided — a real
data-race bug. Repro exists: `naga/examples/barrier_repro.rs` (commit `9d5a6b82`).
The correct fix is non-trivial (scan must account for back-edges, or post-barriers
must be unconditional in loops) — this task does NOT implement the fix; it locks the
bug in as a documented regression.

**Steps:**
1. Reproduce locally: build the standalone `naga/` and run the example (or convert
   it to a `cargo test` example-test); confirm the missing post-barrier in the SPIR-V
   disassembly (record the exact disassembly lines showing `OpBranch` where the
   barrier should be).
2. Add `naga/tests/` regression test asserting the CURRENT (buggy) behavior with a
   `#[ignore]` + comment `// KNOWN-BUG: back-edge hazard, see a.md §10d — flip when
   fixed`, so the moment someone fixes the scan, this test fails loudly and the fix
   is forced to update it. (Asserting current behavior keeps CI green while making
   the bug un-invisible.)

**Branches:** SUCCESS (bug reproduced + pinned test lands) / NORMAL (example no
longer reproduces — behavior changed in this checkout: STOP and investigate, that's
a finding either way) / BAD (cannot build standalone naga: record toolchain error,
leave Part A undone with reason) / DISASTER n/a.

## Part B — `sdata_manual` special-case (wgpu repo)

**Fact base (RECORDED from session analysis):** `wgpu/naga/src/back/spv/index.rs`
contains a debug special-case keyed on `var.name == Some("sdata_manual")` — a
benchmark hack that must not reach any upstream PR.

**Steps:**
1. `grep -rn "sdata_manual" wgpu/naga/src/` — locate every occurrence (RECORDED:
   `spv/index.rs`; verify, don't assume).
2. Trace who depends on it: `grep -rn "sdata_manual" wgpu/naga/ bounds-check-testing-framework/` —
   if benchmark shaders reference that variable name, removal changes their codegen.
3. Remove the hack on branch `fix/t8-remove-sdata-manual`; run T1's gate commands;
   re-run the affected benchmark compile step if any fixture used the name (expect:
   SPIR-V word counts may shift — record before/after for any affected artifact).
4. Do NOT commit unless explicitly instructed; leave the branch + summary.

**Branches:** SUCCESS (hack gone, suite green, no fixture depended on it) /
NORMAL (a fixture depended on the name: update that fixture's WGSL to not rely on
the special case, record the SPIR-V delta) / BAD (removal breaks stress tests:
the hack was load-bearing somewhere unexpected — revert branch, document, hand
back to reasoning agent) / DISASTER n/a.

## Part C — three-checkout consolidation map (documentation only)

**Fact base (RECORDED, `05-REPOS-AND-REFERENCES.md`):** elision work exists in three
places: `wgpu/` (primary, validator-level, current), `naga/` (backend-level ancestor),
`~/Projects/naga` (hardening branch). The paper describes only the wgpu variant.

**Steps:** write `IICT_CONTEXT/09-CHECKOUT-CONSOLIDATION.md`: one table — checkout,
branch, what analysis lives there, file paths, commit tips, recommendation
(keep `wgpu/` as canonical; archive/deprecate the others with dates; explicitly note
`wgpu-upstream/` pristine). No code changes.

## Gates

- Part A/B touch different repos — can run in either order. T1 green recommended
  before Part B (its verification reuses T1 gates).

## Anti-hallucination rules

- Do not "fix" the Track 3 scan as a side effect — the fix is a design decision
  reserved for the reasoning agent.
- Do not describe `sdata_manual`'s effect from memory — read the code path and
  quote the lines in your log.

## Deliverables

- Part A: test file + reproduction disassembly excerpt; Part B: branch + grep/removal
  record; Part C: `09-CHECKOUT-CONSOLIDATION.md`; session-log entry.
