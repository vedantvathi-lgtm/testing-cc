# Bounds-Check Elision Completion Checklist

Updated: 2026-09-13 (§2/§5 corrections + reference addendum; see note below)

> **Correction note (2026-09-13).** §2 checkboxes were unticked although the work was
> substantially complete (references/ archive existed since 2026-08-27 with
> REFERENCE_STATUS.md + SHA256SUMS). Checkboxes are updated only to verified reality:
> everything below is verified 2026-09-13, with three recorded exceptions (Würthinger
> 2007 PDF, Cousot PDF, IRCE RFC thread — see `references/REFERENCE_STATUS.md`
> addendum and `CITATION_MANIFEST.md`). §5: the NVIDIA driver is **working again**
> (verified 2026-09-13, 580.178.04); the BLOCKED notes below are stale — the rerun is
> unblocked but NOT yet executed. Full context: `IICT_CONTEXT/00-INDEX.md`.

## 1. Audit and evidence baseline

- [x] Inventory WGPU and standalone Naga branches and commits.
- [x] Inspect stashes and unreachable checkpoint commits.
- [x] Confirm checkpoint commits contain duplicate trees only.
- [x] Separate the current WGPU implementation from the older Naga/barrier lineage.
- [x] Run the existing 25-test bounds-check stress baseline.
- [x] Add access-specific validator assertions alongside minimal backend smoke tests.
- [x] Add adversarial proof-scope tests: before guard, sibling branch, after loop, mutation after guard, calls, switches, and hidden updates.
- [x] Complete benchmark source, result-schema, timing, statistics, correctness, and provenance audit.
- [x] Freeze legacy benchmark results as non-headline historical evidence.

## 2. References — complete except three recorded gaps (2026-09-13)

- [x] Archive WGSL, WebGPU, Vulkan/SPIR-V, and wgpu issue references.
- [x] Archive ABCD, Wuerthinger 2007/2009, Cousot, IRCE, and GSOHC papers.
      (W07 PDF: TLS failure, metadata-only; Cousot: no verified URL, metadata-only;
      W09/ABCD/GSOHC archived. Details: CITATION_MANIFEST.md.)
- [x] Clone sparse source references for Go `prove`/`loopbce` and LLVM SCEV/IRCE.
      (Go @ 9834516 incl. moved ssacompile/loopbce.go + test oracles; LLVM @ 39e3e92e
      incl. IRCE .cpp + tests; verified 2026-09-13.)
- [x] Archive Readyset and Rust Performance Book measurement counterpoints.
- [x] Create a verified citation manifest with URLs, versions, access dates, and implementation lessons.
      (CITATION_MANIFEST.md, 2026-09-13.)

## 3. Sound analysis

- [x] Replace function-wide loop-index marking with access-specific proof results.
- [x] Implement signed and unsigned bounded interval types.
- [x] Reject arithmetic whose endpoints can overflow or underflow.
- [x] Implement path-sensitive condition refinement and conservative joins.
- [x] Implement local-variable mutation invalidation.
- [x] Recognize monotone loop recurrences with explicit start, step, and exit.
- [x] Support equality exits only when reachability and no-wrap conditions are proven.
- [x] Support affine builtin and induction-variable expressions.
- [x] Keep runtime-sized and unsupported cases checked.

## 4. Validation

- [x] Run unit, Naga integration, snapshot, formatter, and clippy checks.
  (272 naga tests + 58 stress + 3 elision-assertion tests pass; touched files clippy-clean; rustfmt clean; committed as `8757d7d` on `bounds-check-elision`.)
- [x] Add differential execution for elided versus checked policies.
  (2026-09-20: `bounds-check-testing-framework/diff-exec/` — 8/8 Class-A fixtures
  byte-identical on device, 2 Class-B policy fixtures per semantics, per-access
  partial-elision verified, determinism PASS across two runs. Ledger updated.)
- [ ] Verify SPIR-V, MSL, and HLSL behavior separately.
- [ ] Fuzz interval operations and adversarial WGSL expression shapes.
  (2026-09-28: NOT fuzzing, but T5 delivered: Part A exhaustive boundary sweeps (`83a90e0`) + Part B per-access expectation checker with a must-fail control suite (`bounds-check-testing-framework/expectation-checker/`, main 7/7 PASS, control 3/3 FAIL). Randomised fuzzing still open.)

## 5. Corpus and benchmarks

- [x] Deduplicate corpus inputs by normalized content hash.
- [x] Classify dynamic index expression shapes and base types.
- [x] Count emitted checks separately from Naga `Access` expressions.
  (v5 harness counts emitted `OpULessThan` via rspirv disassembly: 505 → 134 on naga tests, −73%.)
- [x] Report project-authored and external corpora separately.
- [x] Final v5 corpus run after interval rewrite: project corpus 74.1% overall / 81.9% compute; external 1–2.5%. See corpus_hit_rate/results/RESULTS.md (commit `6ff062b`).
- [x] Build a provenance-complete schema-v3 benchmark runner.
  (`paired-vulkan-v3` harness complete with meta.json provenance; see testing-framework commit `dd4f5bb`.)
- [x] Run negative controls before headline experiments. (EXECUTED 2026-09-13 after toolchain restore: two schema-v3 runs, control CI ⊇ 0 in both, `valid_for_claims=true`; headline 105.03 ns [101.53–106.60] and 111.03 ns [108.44–112.42] per dispatch @ `8757d7d`. NOTE: the two CIs do NOT overlap — recorded as DEVIATION in `IICT_CONTEXT/08-SESSION-LOG.md`; runs `runs/20260913T173908Z` + `runs/20260913T174017Z`.)
- [ ] Run headline paired GPU measurements under schema v3. (UNBLOCKED 2026-09-13: same; not yet run.)
- [ ] Validate kernel outputs, timestamps, device state, and statistical estimators.
- [ ] Persist raw samples, SPIR-V words, compiler revisions, and environment metadata.

## 6. Paper

- [x] Draft revision v2 (`iict2026_bounds_check_elision_v2.tex`) with: interval-domain soundness rewrite section, adversarial test description, corpus hit-rate study with before/after table, SPIR-V OpULessThan cross-check, updated abstract and progress section. Compiles to 8-page PDF.
- [ ] Regenerate the three lost figure PDFs (were in /tmp; placeholders in v2).
- [ ] Rewrite algorithms to match implemented code exactly (full §3 pseudocode).
- [ ] Replace the inaccurate interval-analysis figure.
- [ ] Replace unsupported timing and instruction-count claims (needs GPU re-run under schema v3).
- [ ] Add verified related work and normative citations (ABCD, Würthinger, IRCE, Go prove/loopbce, Readyset).
- [ ] Add threats to validity, negative results, and residual limitations.
- [ ] Regenerate the final PDF and audit every table, figure, citation, and numerical claim.
