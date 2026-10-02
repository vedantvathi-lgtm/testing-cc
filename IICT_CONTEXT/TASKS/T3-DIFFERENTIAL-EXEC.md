# T3 — Differential execution testing (elided vs checked, on device)

**Objective:** close the project's biggest testing gap (`a.md` §11 gap #6): every
correctness claim so far is structural (SPIR-V text). This task executes elided and
non-elided code on the GPU and diffs outputs byte-for-byte.

## Design (fixed; do not redesign)

- New crate: `bounds-check-testing-framework/diff-exec/` (new directory; nothing
  existing is modified).
- Deps: identical pattern to `paired-vulkan-v3/Cargo.toml` (path deps into
  `../../wgpu/naga` + `../../wgpu/wgpu`, features `wgsl-in spv-out` /
  `spirv std vulkan`; plus `pollster`, `serde`, `serde_json`, `sha2`).
- Mechanics per fixture:
  1. WGSL fixture (list below) parsed + validated twice: once
     `validator.set_bounds_check_elision(true)`, once `false` (same API as
     `paired-vulkan-v3/src/main.rs:145` — read that file first).
  2. Compile both to SPIR-V; assert they DIFFER in `OpULessThan` count where the
     fixture is expected to elide (sanity that the toggle matters), and are IDENTICAL
     where the fixture is expected to retain (sanity that elision isn't overfiring
     structurally).
  3. Run BOTH binaries on the device with identical input buffers.
  4. Read back outputs; require byte-identical (assert + record sha256 of both).
- Fixture classes (create under `diff-exec/fixtures/`):
  - **Class A — soundness (equality REQUIRED):** the 58 existing stress shaders
    (copy from `wgpu/naga/tests/in/bounds-check-elision-*.wgsl` and the inline
    shaders in `wgpu/naga/tests/naga/bounds_check_stress.rs` — extract or re-encode
    the same patterns); boundary indices (len-1, len, len+1 writes where legal),
    u32-wrap candidates (`lid - 1u` at lid=0 guarded/un-guarded), signed i32 IV with
    negative start (must RETAIN; execute to confirm identical outputs), eq-exit loops,
    affine indices (`i+1u`, `i*2u`), nested loops, multiple IVs.
  - **Class B — policy-semantics (difference REQUIRED, NOT a bug):** intentional
    OOB accesses where ReadZeroSkipWrite must return 0/drop writes and elision-
    enabled/disabled must agree with policy semantics — label clearly so no one
    reads these as failures.
- Output: `diff-exec/results/<stamp>/report.json` — per fixture: name, class,
  elided? (observed), outputs identical?, sha256 pair, plus environment block
  (copy the provenance pattern from `paired-vulkan-v3`).

## Gates

- T1 green. GPU functional (same gate commands as T2).

## Steps

1. Read `paired-vulkan-v3/src/main.rs` end-to-end once — reuse its device setup,
   buffer plumbing, and provenance helpers verbatim where possible.
2. Create the crate + fixtures + fixture manifest.
3. `cargo run --release -- --manifest fixtures/manifest.json` (shape the CLI as you
   like; keep it deterministic and file-driven).
4. Full run; then re-run once to confirm determinism of the harness itself.

## Verification

- Every Class-A fixture: outputs byte-identical AND structural expectation matches
  (elides iff stress-suite says it should).
- Every Class-B fixture: documented, expected divergence only (policy semantics),
  identical between enabled/disabled within the same policy.

## Outcome branches

- **SUCCESS** — all Class A identical. This upgrades the paper's safety claim from
  structural to executional: add one sentence to `04-EVIDENCE-LEDGER.md` (with run
  path) and tick TODO §4 "differential execution".
- **NORMAL** — all identical, but some fixtures elide where the stress suite
  predicted retention (or vice versa): analysis coverage changed since tests were
  written (possible post-`8757d7d` drift). Do NOT hide: record each, cross-check
  against the analysis source, and file the drift as findings. If the prediction
  table is stale, updating the TESTS' expectations is allowed on a branch with the
  evidence recorded; changing the ANALYSIS is NOT this task.
- **BAD** — any Class-A fixture diverges on device: **real soundness bug**. STOP
  other tasks. Minimize the WGSL repro; write a permanent regression test in
  `wgpu/naga/tests/naga/bounds_check_stress.rs` (new file content added on branch
  `fix/t3-<desc>`); file full write-up in `08-SESSION-LOG.md`; elision claims
  downgrade in `04-EVIDENCE-LEDGER.md` until fixed.
- **DISASTER** — systematic divergence across a whole class (soundness argument of
  the rewrite itself broken): full stop; top-severity write-up; all elision claims
  INVALID; hand back to reasoning agent.

## Anti-hallucination rules

- The "58 stress tests" figure is RECORDED — count the actual fixtures you encode
  and write that number.
- Do not claim "byte-identical" without the sha256 pair in the report.
- Class-B divergences must never be summarized as "all tests passed" without the
  class distinction stated.

## Deliverables

- `diff-exec/` crate + fixtures + manifest + two run reports,
  ledger/TODO updates, session-log entry.
