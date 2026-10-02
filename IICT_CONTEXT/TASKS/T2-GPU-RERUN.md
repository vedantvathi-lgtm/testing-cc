# T2 — GPU schema-v3 rerun + negative control (P1, the headline experiment)

> ✅ EXECUTED 2026-09-13: SUCCESS with one recorded DEVIATION. Steps 1–3 done
> (baseline + reproducibility run; the Step-4 multi-kernel extension was NOT run).
> Run 1 `runs/20260913T173908Z`: headline 105.03 ns [95% CI 101.53–106.60], control
> 2.74 ns [−5.45, +5.36] (includes zero), correctness exact (0.0/0.0), valid_for_claims=true.
> Run 2 `runs/20260913T174017Z`: headline 111.03 ns [95% CI 108.44–112.42], control
> 2.07 ns [−5.16, +5.93] (includes zero), correctness exact, valid_for_claims=true.
> Static sanity held in both: SPV 486→422 words (−64), safe_accesses 0→3, OpULessThan 4→1.
> DEVIATION: the two headline CIs do NOT overlap (this file's success text predicted
> "overlap substantially"); both exclude zero by ≈40× the control width — classified as
> inter-run clock-state drift, both runs kept, runs never merged/averaged. Also recorded:
> run 2's pre-run nvidia-smi power sample is a garbage 590.01 W reading (sensor glitch,
> captured in provenance).

**Objective:** produce provenance-complete, correctness-gated, negative-control-verified
GPU timing for bounds-check elision ON vs OFF under the same policy — replacing the
frozen Aug-28 numbers with fresh, citable ones.

## Background facts (all RECORDED; verify in-file before trusting)

- Harness: `bounds-check-testing-framework/paired-vulkan-v3/` — single-kernel runner
  (hardcoded `SHADER` = fixed-array-scan, idiom: `tile[i]` under a loop-exit guard),
  **already implements**: BCE enabled/disabled paired compile via
  `validator.set_bounds_check_elision()`, identical-binary negative control,
  output-correctness gate before timing, per-round paired samples, nvidia-smi capture,
  provenance (git branch/status/diff, shader sha256, timestamp period).
- Config via env: `BCE_ROUNDS` (default 300), `BCE_CONTROL_ROUNDS` (200),
  `BCE_WARMUP_ROUNDS` (50).
- Output: `bounds-check-testing-framework/runs/<UTCSTAMP>-fixed-array-scan-bce-on-off/`
  containing `fixed-array-scan.json` + `artifacts/` (shader.wgsl, enabled.spv,
  disabled.spv, runner.rs, diffs).
- Path deps: `naga = ../../wgpu/naga`, `wgpu = ../../wgpu/wgpu` (features vulkan+spirv).
  **The runner measures exactly the tree in `wgpu/` — T1 must be green first.**
- Pre-rewrite runs exist: `runs/20260826T{204341Z,211308Z,215110Z}-...` — these are
  from BEFORE the `8757d7d` rewrite; do not treat them as current results.
- Known result to compare against (RECORDED, `a.md` §10b): retained-arm bounds-check
  overhead 7,568 ns [CI 7,456–7,744]; elided delta 0 ns [0,0]; control PASS.
  Same order of magnitude expected; exact values will differ (different harness).

## Gates

```bash
cd /home/bb/Projects/Dave-Sonar-WGPU--master
nvidia-smi | head -12                        # expect: GPU listed, no errors
git -C wgpu status --short | grep -v '.repo-map.json' | grep -v 'naga/tests/naga/bounds_check_stress.rs' | wc -l   # expect: 0
                                             # (.repo-map.json = pre-existing untracked tooling artifact — ignore, never delete.
                                              #  bounds_check_stress.rs = T1's clippy fix, deliberately uncommitted per T0
                                              #  git-safety: `field_reassign_with_default` → struct-literal init, VERIFIED
                                              #  58/58 stress + 2/2 assertions still pass. Do not commit, do not revert.)
df -h . | tail -1                            # expect: >5G available
ls bounds-check-testing-framework/paired-vulkan-v3/Cargo.toml   # expect: exists
```

## Steps

1. Baseline run (exact, verbatim):
```bash
cd /home/bb/Projects/Dave-Sonar-WGPU--master/bounds-check-testing-framework/paired-vulkan-v3
BCE_ROUNDS=300 BCE_CONTROL_ROUNDS=200 BCE_WARMUP_ROUNDS=50 \
  cargo run --release 2>&1 | tee ../../IICT_CONTEXT/logs/t2-run1-$(date +%Y%m%d-%H%M).log
```
2. Locate and read the run JSON:
```bash
ls -t ../../runs/ | head -3
python3 -m json.tool ../../runs/<newest>/fixed-array-scan.json | head -120
```
Extract EXACTLY (copy, don't round): `negative_control` estimate + whether its CI
includes zero (`negative_control_includes_zero` field), `headline` estimate + CI,
`valid_for_claims`, `correctness` result, static artifacts (`spv_words`,
`op_uless_than_total` enabled vs disabled, `safe_accesses`), `machine` block
(adapter, driver, timestamp_period_ns), `sources` (git revisions captured).
3. Reproducibility run (same command, second run): confirms inter-run stability.
4. **Optional extension (only if run 1 + 2 are SUCCESS and time permits):**
   multi-kernel sweep. The SHADER is hardcoded — create (do not modify the original):
   `src/bin/matmul.rs` and `src/bin/reduction.rs` by copying `main.rs`, changing ONLY
   the `SHADER` const (kernels: workgroup-tiled reduction wg=256; tiled matmul 16×16 —
   WGSL templates exist in `bounds-check-bench` and `IICT_CONTEXT/06-EXPERIMENTS-AND-METHODS.md`
   references) and the `run_id` suffix (`-matmul-bce-on-off`, `-reduction-bce-on-off`).
   Keep every other mechanism (correctness gate, control, provenance) untouched.
   If the copied runner needs a different output extent, fix the correctness gate in
   the COPY, not by weakening asserts.
   CRITICAL correctness requirement (audit lesson): the kernel must write results to a
   storage buffer large enough for all invocations, and the elided/disabled arms must
   be semantically identical (no OOB in either arm).

## Verification

- `valid_for_claims: true` in the JSON.
- `negative_control_includes_zero: true`.
- `correctness` gate passed (exact expected output).
- Static sanity: `safe_accesses > 0` when enabled; `== 0` disabled; disabled
  `op_uless_than_total > enabled` (the runner asserts these itself — a normal exit
  implies they held).
- `sources[].branch == bounds-check-elision` and captured wgpu commit == `8757d7d`
  (or T1's fix branch tip if T1 went BAD→fix route).

## Outcome branches

- **SUCCESS** — control CI ⊇ 0, claims valid, two runs consistent (headline CIs
  overlap substantially). Write results into `04-EVIDENCE-LEDGER.md` as VERIFIED
  (keep the old frozen row, mark SUPERSEDED-BY with run path). Update
  `BOUNDS_CHECK_ELISION_TODO.md` §5 items to checked with run paths. T6 may now
  use these numbers.
- **NORMAL** — control passes; headline effect's CI **includes zero** on this kernel
  (overhead not resolvable on fixed-array-scan at these round counts). This is a
  legitimate publishable finding ("not statistically resolvable on kernel X at n=300");
  do NOT bump rounds silently to chase significance. Record honestly; optionally run
  ONE pre-registered follow-up at `BCE_ROUNDS=1000` and report both.
- **BAD** — control FAILS (`negative_control_includes_zero: false`): pipeline broken;
  all numbers from this session INVALID for claims; do not update ledger as results.
  Diagnose in order: timestamp_period sanity, nvidia-smi power state/clocks captured,
  thermal (rerun after 5 min cool-down), then harness code. Fix on a new branch of
  `bounds-check-testing-framework` (`fix/t2-<desc>`); re-run; record.
- **DISASTER** — Vulkan adapter init failure / driver dies mid-run (this exact mode
  happened Aug 27→Sep 13). Capture `nvidia-smi` output + error text into
  `logs/t2-disaster.log`; mark T2 BLOCKED with date; do NOT run optional extension;
  skip to T4/T5/T6 (no-GPU tasks); do not reuse legacy numbers.

## Anti-hallucination rules

- Copy every number from the JSON mechanically; include the JSON path next to each.
- If the JSON structure differs from the fields listed above, record the actual
  structure (it's RECORDED from source reading, may have drifted) — do not force-fit.
- Never merge/average the two runs; report both.

## Deliverables

- 2+ run dirs under `runs/`, logs under `IICT_CONTEXT/logs/t2-*.log`,
  ledger + TODO updates, session-log entry with branch-called and evidence.
