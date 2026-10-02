<!--
Sync Impact Report
- Version change: (template, unversioned) → 1.0.0
- Principles: all five newly defined (derived from ../AGENTS.md Rules 0–5 plus the
  2026-09-27 audit findings on analysis soundness)
- Added sections: Technical Constraints; Development Workflow & Quality Gates; Governance
- Removed sections: none
- Templates: plan/spec/tasks templates read this file at runtime; not modified
- Deferred TODOs: none
-->

# Naga Bounds-Check Elision (IICT 2026 #55) Constitution

## Core Principles

### I. Evidence-Labelled Claims (NON-NEGOTIABLE)

Every factual statement in a spec, plan, task, commit message, paper or poster MUST carry
provenance (file:line, commit hash, artifact path, or URL with access date) and one label:
VERIFIED, RECORDED, REPORTED, UNKNOWN, or SUPERSEDED/INVALID (definitions in `../AGENTS.md`
Rule 0). Numbers MUST be copied from artifacts, never recalled. A label MUST NOT be upgraded
without a new check. A result that differs from its prediction MUST be written as a
`DEVIATION:` line (expected / observed / evidence / consequence / next).

Rationale: this project previously published analyses that did not exist; the label system is
its failure-prevention mechanism.

### II. Soundness Before Coverage (NON-NEGOTIABLE)

The elision analysis MUST NOT mark an access safe unless its index is proven in bounds on
every execution path that reaches it. Specifically:
- "Cannot model / unknown" MUST mean "retain the check". It MUST NOT be encoded as
  "infeasible" or "unreachable" (the `refine()` defect found 2026-09-27).
- Every naga front end that feeds the validator (WGSL, GLSL, SPIR-V input) is in scope for
  soundness, not only WGSL.
- A change that increases elision MUST ship with must-retain tests for its nearest unsound
  neighbours (overflow, wrap, negative values, mutation, sibling branches).
- If any soundness fixture diverges, all coverage work stops until it is root-caused.

Rationale: a wrong elision is an out-of-bounds GPU write in a browser-facing compiler.

### III. Test-First With Paired Expectations

Code changes MUST follow red → green → refactor: the failing test is written and observed
failing before the implementation. Each new or changed elision rule MUST have at least one
must-elide and one must-retain test that assert on `FunctionInfo.safe_accesses` (the proof
set), not only on text of emitted code. Interval arithmetic changes MUST keep the exhaustive
boundary property sweeps green.

Rationale: text-grep structural checks have already produced vacuous passes in this project
(the opcode-166 checker).

### IV. Measured, Segmented, Gated Evidence

Coverage and performance claims MUST come from reproducible artifacts:
- Coverage is reported per corpus segment (project tests vs real-world projects), with
  concentration disclosed when a few files dominate a total.
- Performance claims MUST pass the gates in `../AUDIT_INVENTORY.md` (negative control with
  CI containing zero, provenance-complete runs); legacy numbers stay SUPERSEDED.
- Configurations MUST state the bounds-check policy and backend, since wgpu's production
  policy differs from the measured one.

Rationale: an unsegmented 74.1% headline hid that three files produced 91% of it.

### V. Repo Safety and Upstream Readiness

No destructive git operation, no push, and no commit without an explicit request.
`wgpu-upstream/` is read-only. Changes to the naga fork MUST be minimal, rustfmt- and
clippy-clean, match surrounding code style, and be reviewable as an upstream PR (no
special-cased identifiers such as the `sdata_manual` hack).

Rationale: several repos hold deliberate checkpoints; the end goal is an upstreamable change.

## Technical Constraints

- Implementation lives in `../wgpu/naga/src/proc/bounds_check_elision.rs` and its consumers;
  naga is `no_std` (no runtime environment access in library code).
- Toolchain: Rust 1.93 (pinned by `../wgpu/rust-toolchain.toml`).
- Interval domain: 32-bit signed and unsigned integers; all endpoint arithmetic is
  overflow-checked; unsupported operations yield "unknown", never a guessed range.
- Evaluation corpora: naga tests (project), webgpu-samples, wgpu upstream, vello, rerun, bevy
  (partial, WESL), per `../research_notes/Real world shader elision gaps/PROVENANCE.txt`.

## Development Workflow & Quality Gates

1. Specify → clarify (questions answered and recorded with evidence) → plan with SUCCESS /
   NORMAL / BAD / DISASTER outcome branches (`../AGENTS.md` Rule 3) → tasks → analyze →
   implement → review → record.
2. Gate before any claim of completion: `cargo test -p naga` green, clippy clean for touched
   files, `cargo fmt --check` clean, soundness regression tests green, real-world census
   re-run with before/after per corpus.
3. Every session appends to `../IICT_CONTEXT/08-SESSION-LOG.md` and updates
   `../IICT_CONTEXT/04-EVIDENCE-LEDGER.md` when a claim changes.

## Governance

`../AGENTS.md` takes precedence over this constitution; where they conflict, AGENTS.md wins
and this file is amended to match. Amendments use semantic versioning (MAJOR: principle
removed or redefined; MINOR: principle or section added; PATCH: wording). Every spec, plan
and review MUST include a constitution check against Principles I–V.

**Version**: 1.0.0 | **Ratified**: 2026-09-13 | **Last Amended**: 2026-09-28
