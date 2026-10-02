# T1 — Certify the current tree

> ✅ EXECUTED 2026-09-13: SUCCESS. Toolchain restored (rustup 1.93.1, exact pin).
> Results: 137+272+6 tests pass, 0 failures; 58/58 bounds_check_stress + 2/2
> elision_assertions executed; clippy exit 0 (1 pre-existing upstream warning in
> a `tests/naga/*` file, outside elision-owned files — counted, not fixed);
> fmt exit 0. Logs: `IICT_CONTEXT/logs/t1-{cargo-test-20260913-full,clippy-20260913,fmt-20260913}.log`.
> Drift found vs this file's text: elision_assertions has **2** tests, not 3.

**Objective:** prove the committed state (`wgpu` @ `8757d7d`) actually builds and passes
its full suite, so every later task stands on verified ground.

## Gates (run all; abort task on failure unless branch says otherwise)

```bash
cd /home/bb/Projects/Dave-Sonar-WGPU--master
git -C wgpu status --short    # expect: ONLY '?? .repo-map.json' (pre-existing untracked
                              # tooling artifact — NOT yours: do not delete, do not add,
                              # do not commit it). Anything else = investigate before proceeding.
git -C wgpu log --oneline -1               # expect: 8757d7d ...
git -C wgpu branch --show-current          # expect: bounds-check-elision
df -h . | tail -1                          # expect: >10G available
cargo --version && rustc --version         # expect: toolchain present
```

Note (VERIFIED 2026-09-13): `wgpu/` and `naga/` both carry exactly one untracked
file, `.repo-map.json` (a code-graph tool artifact predating this task). The clean
tree check above is written to tolerate it — do not "clean it up."

## Steps

```bash
mkdir -p IICT_CONTEXT/logs
cd wgpu
cargo test -p naga 2>&1 | tee ../IICT_CONTEXT/logs/t1-cargo-test-$(date +%Y%m%d-%H%M).log
cargo clippy -p naga --all-targets 2>&1 | tee ../IICT_CONTEXT/logs/t1-clippy-$(date +%Y%m%d-%H%M).log
cargo fmt -p naga -- --check 2>&1 | tee ../IICT_CONTEXT/logs/t1-fmt-$(date +%Y%m%d-%H%M).log
```

Note: `cargo test -p naga` runs the full naga suite including
`bounds_check_stress.rs` (58 tests) and `elision_assertions.rs` (3 tests) via
`tests/naga/main.rs`. The suite is large; allow 10–30 min on first build.

## Verification (expected)

- Test summary: all pass. Prior recorded state: "272 naga tests + 58 stress +
  3 elision-assertion tests pass" (`BOUNDS_CHECK_ELISION_TODO.md` §4 note,
  `RECORDED` from Aug 28 commit session). Accept the count you actually see —
  copy the exact number from the log into your task log; do not assume 272.
- clippy: zero warnings in files owned by this project
  (`bounds_check_elision.rs`, `valid/analyzer.rs`, `valid/function.rs`,
  `proc/index.rs`, `back/spv/*`). Pre-existing upstream warnings elsewhere:
  count them, do not fix them (scope).
- fmt: clean (exit 0).

## Outcome branches

- **SUCCESS** — all green. Record exact pass counts + artifact paths in
  `08-SESSION-LOG.md`; mark this task done; T2 is now unlocked.
- **NORMAL** — flaky test: same test fails once, passes on immediate re-run.
  Record both runs (paths), proceed to T2 with a note naming the flaky test.
- **BAD** — genuine failure(s) in elision-owned files (list above) or >3 failures
  anywhere. DO NOT proceed to T2. Write DEVIATION; create branch
  `fix/t1-<short-desc>` in `wgpu/`, fix minimally, re-run until green, then
  record fix summary (do NOT commit unless explicitly instructed by the user).
- **DISASTER** — the tree does not compile at `8757d7d`. Record full error to
  `IICT_CONTEXT/logs/t1-build-failure.log`; update `04-EVIDENCE-LEDGER.md`:
  the "272 tests green" claim downgrades to INVALID pending fix; STOP all tasks.

## Anti-hallucination rules for this task

- Do not report "tests pass" without pasting the summary line into your log.
- Do not fix pre-existing upstream failures outside the elision files.
- Do not skip clippy/fmt because tests passed.

## Deliverables

- `IICT_CONTEXT/logs/t1-*.log` (3 files), session-log entry, task tick.
