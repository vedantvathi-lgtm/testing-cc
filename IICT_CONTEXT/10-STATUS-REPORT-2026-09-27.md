# Status report — IICT 2026 paper #55, as of 2026-09-27

> Bounds-check elision in the Naga WGSL compiler. Accepted as a poster (4× weak accept),
> IICT Oct 2–3 2026, which is **5–6 days from this report**.
> Written under AGENTS.md. Labels: VERIFIED = checked in this session (2026-09-27),
> RECORDED = from a project document (path cited) and not re-checked here.
> This session's evidence is in `IICT_CONTEXT/logs/audit-20260927-*`.

---

## 1. What the project is (one paragraph)

WGSL/WebGPU requires every GPU memory access to be bounds-safe, so Naga (the compiler
behind wgpu and Firefox) inserts a runtime check on every array/buffer access
(`ReadZeroSkipWrite` / `Restrict` policies). This project adds a static analysis that
proves some accesses in-bounds at compile time and emits them without the check. Proven
accesses compile to the same code as `Unchecked`; every access that isn't proven keeps
its check. Implementation: `wgpu/` fork, branch `bounds-check-elision`.

## 2. Original (submitted) paper vs. now

Submitted version: `~/Downloads/IICT-2026_paper_55.pdf` (7 pp, Aug 25 timestamp; VERIFIED, text extracted this session).

| Area | Submitted paper claimed | Current state | Label / evidence |
|---|---|---|---|
| Builtin analysis | `data[local_invocation_index]` is safe when array length ≥ workgroup-size product | Unchanged idea, now expressed as intervals, and affine forms (`A*lid+B`) are also handled | VERIFIED code: `wgpu/naga/src/proc/bounds_check_elision.rs` |
| Loop analysis | Syntactic pattern match of `if i < N {} else {break;}`, `break if i >= N` / `> N`. Reviewers R1–R3 believed it was unimplemented; R4 called it sketchy | **Replaced** by an interval-domain rewrite: signed and unsigned intervals, arithmetic rejected if an endpoint could overflow, path-sensitive refinement at branches, conservative joins, invalidation on mutation, monotone recurrences, equality exits, affine indices | VERIFIED commit `8757d7d` in `git -C wgpu log`; RECORDED feature list in `BOUNDS_CHECK_ELISION_TODO.md` §3 |
| Proof granularity | Function-wide `safe_indices: HashSet` | Per-access `FunctionInfo.safe_accesses` | VERIFIED `src/back/spv/index.rs:522`, `src/proc/index.rs:344` |
| Backends | "Currently targeting Vulkan" | SPIR-V, MSL and HLSL all consult the proof set (`access_needs_check`). **Only SPIR-V has been tested** | VERIFIED code paths `src/back/msl/writer.rs:832`, `src/back/hlsl/writer.rs:3961`; no MSL/HLSL test exists |
| Soundness testing | Not discussed (R2 raised it) | 58 adversarial stress tests (was 24), 2 structural assertions, 5 exhaustive interval property sweeps (`83a90e0`). Full suite **142 lib + 272 integration + 6 doc, 0 fail** | VERIFIED `logs/audit-20260927-cargo-test-naga.log` (at `83a90e0` + uncommitted stress-file clippy fix) |
| Known soundness hole | — | A signed-IV negative-start hole was found (Aug 25) and closed by the rewrite. `diff-exec` shows the guarded variant elides and the unguarded variant retains its check | RECORDED `04-EVIDENCE-LEDGER.md`; `diff-exec` JSON |
| Execution on hardware | None | Differential execution: 8/8 Class-A fixtures produce byte-identical outputs (checked vs elided) on an RTX 4060. **See caveats in §4** | VERIFIED JSON `bounds-check-testing-framework/diff-exec/results/diff-exec-20260920T173202Z.json` |
| Evaluation breadth | One minimal shader + one matmul | Corpus study on 295 deduplicated programs. Project corpus: 1.24% → **74.1%** overall, **81.9%** in compute. External corpora: **1–2.5%**. `OpULessThan` count 505 → 134 (−73%) | VERIFIED numbers copied from `corpus_hit_rate/results/RESULTS.md` lines 24, 28, 34, 41 |
| Timing | "0.0 ns" elided; 7,568–8,192 ns retained; ~29–31 ps/check | Legacy numbers are **SUPERSEDED/INVALID** for headline use. New schema-v3 paired runs with a negative control: **105.03 ns [101.53–106.60]** and **111.03 ns [108.44–112.42]** per dispatch (retained-check overhead, fixed-array-scan kernel). Control CIs contain 0 and `valid_for_claims=true` in both runs. The two headline CIs **do not overlap** (recorded DEVIATION) | VERIFIED values read from `runs/20260913T173908Z…/fixed-array-scan.json` and `runs/20260913T174017Z…/fixed-array-scan.json` |
| Related work | 5 references (specs + Muchnick) | References archived (ABCD, W07, W09, Cousot '77, IRCE RFC, GSOHC, Go prove/loopbce, LLVM SCEV/IRCE, Readyset) plus a related-work research note. **Not yet in the paper** | RECORDED `references/REFERENCE_STATUS.md`, `research_notes/related-work-bce.md` |
| R4 "constant indices only?" | Unclear in the paper | Answered: the contribution is **runtime-varying** indices. Constant folding already existed upstream | RECORDED `02-REVIEWS.md` answer table |

Other improvements: a no-hallucination protocol (`AGENTS.md`), an evidence ledger, a gated
task plan (`TASKS/T0–T8`), and legacy benchmark JSON frozen after audit
(`TEST_BENCHMARK_AUDIT.md`).

## 3. Implemented vs. still to do (by task)

| Task | Status | Evidence |
|---|---|---|
| T1 certify tree | DONE; re-verified today | today's log above |
| T2 GPU schema-v3 rerun | DONE for **one kernel** (fixed-array-scan). The multi-kernel sweep (matmul / reduction / prefix-scan) is **not run** | run dirs above |
| T3 differential execution | DONE, 10 fixtures (8 A + 2 B). The 58 stress fixtures are **not** encoded; caveats in §4 | diff-exec JSON |
| T4 corner cases + MSL/HLSL | **NOT STARTED.** There is no overridable-workgroup-size test (grep found no `override` in either test file) and no MSL/HLSL test | VERIFIED by grep |
| T5-A interval property sweeps | DONE, committed `83a90e0` | VERIFIED |
| T5-B expectation checker | **STARTED, BROKEN, UNLOGGED.** See §4.1 | VERIFIED |
| T6 paper v3 | **NOT STARTED.** v3.tex doesn't exist. v2 still has: 3 figure placeholders (`\fbox` at v2.tex:181, 256, 273); banned legacy timing (v2.tex:375, 390); §3.2 describing the old pattern matcher; 5 bibitems; no related-work section; no threats-to-validity section | VERIFIED grep of `iict2026_bounds_check_elision_v2.tex` |
| T7 poster | **NOT STARTED.** No `poster/` directory. This is the organizer's binding requirement | VERIFIED `ls` |
| T8 hygiene | NOT STARTED. The `sdata_manual` hack is still present. The Track-3 barrier back-edge bug is unfixed (out of paper scope) | VERIFIED `src/back/spv/writer.rs:2824` |
| 90-s teaser video | UNKNOWN (deadline was Aug 27) | `02-REVIEWS.md` |

## 4. Findings from this session's audit (new; not in any earlier record)

### 4.1 `DEVIATION`: the T5-B expectation checker counts the wrong SPIR-V opcode

`expectation-checker/checker.py` sets `OP_ULESS = 166`. In the SPIR-V grammar, 166 is
`OpLogicalOr`; `OpULessThan` is **176** (`spirv-0.4.0+sdk-1.4.341.0/autogen_spirv.rs:3762, 3772`,
VERIFIED). `OP_ACCESS_CHAIN = 61` is also wrong (it's 65; 61 is `OpLoad`). The same file has
dead code: `disassemble()` returns a class object instead of lines.

- Recorded run `results/expect-20260920T182427Z.json`: 2 pass / 4 fail, with **0 checks
  counted in both arms for every fixture**. The 2 "passes" are therefore vacuous.
- Re-run with opcode 176 (scratch copy; your file is unchanged): **4 pass / 2 fail**.
  Evidence: `logs/audit-20260927-expect-opcode176.json`, script `logs/audit-20260927-checker-opcode176.py`.
- The 2 remaining failures:
  - `lid_spec_direct`: a **fixture bug**. Line 7 has two accesses under one marker. All 3 checks are correctly elided (0 vs 3).
  - `eq_exit_loop`: a **real coverage gap**, described in §4.2.

`DEVIATION: expected T5-B checker to measure OpULessThan; observed it counts OpLogicalOr (166)
→ all counts 0; evidence: logs/audit-20260927-expect-opcode{166-rerun,176}.json; consequence:
the Sep-20 checker result is INVALID; next: set 176, fix the marker count on lid_spec_direct
line 7, decide the eq-exit expectation (§4.2), then log T5-B.`

### 4.2 `DEVIATION`: equality-exit elision only works in the `continuing { break if i == N }` shape

Variant compiles (`logs/audit-20260927-eqexit-variants.log`, VERIFIED), `OpULessThan` enabled/disabled:

| Loop shape (array len 8) | enabled | disabled | Elided? |
|---|---|---|---|
| `if (i == 8u) { break; }` at top, i++ in body (± barrier) | 1 | 1 | **no** |
| `if (i >= 8u) { break; }` at top | 0 | 1 | yes |
| `continuing { i++; break if i == 8u; }` | 0 | 1 | yes |
| `for (var i=0u; i<8u; i++)` (loop condition is itself 1 `OpULessThan`) | 1 | 2 | yes |
| `for (var i=0u; i!=8u; i++)` | 1 | 1 | **no** |

Root cause (VERIFIED by reading the code): with a guard at the top of the loop, `loop_range`
joins the exit value into the header range, giving `[0,8]` (`bounds_check_elision.rs:393-394`).
The `!= 8` refinement on the taken edge would narrow this to `[0,7]`, but `refine_range`'s
`NotEqual` arm only rules out an exact singleton and never trims an endpoint (`:619-622`).
The check is therefore **kept**. This is **sound (conservative)**, not a safety bug.

It does contradict two written claims:
- Paper v2 §3.5 (v2.tex, "Monotone recurrences" bullet) gives
  `if i == N { break; }` as the example of an accepted equality exit. The code does **not**
  elide that exact shape. The only stress tests for it (`bounds_check_stress.rs:1005, 1024`)
  use the `break if` form.
- `diff-exec`'s `eq_exit_loop` fixture (`expected_elision: true`) passes structurally only
  because `output[lid]` elides. Its rule is "any check removed" (`diff-exec/src/main.rs:468`).
  Both `tile[i]` accesses keep their checks (`safe_accesses_enabled 1 / dynamic 3`). So
  equality-exit elision has **not** been executionally demonstrated.

Candidate fix (small, sound): in `refine_range`, when `bound == upper`, set upper to `bound-1`;
when `bound == lower`, set lower to `bound+1`. Then add a stress test for the top-guard shape.

### 4.3 Caveats on "8/8 byte-identical"

- In 2 of the 8 Class-A fixtures (`mutation_after_guard_must_retain`,
  `unguarded_oob_write_must_retain`), the enabled and disabled SPIR-V hashes are **equal**.
  The same binary is compared with itself, so identical output is guaranteed. Those two are
  structural tests, not execution tests. Executionally meaningful: **6**, and one of those
  (`eq_exit_loop`) only exercises the lid access.
- The honest phrasing is: "outputs identical on 6 fixtures whose binaries differ, plus 2
  must-retain fixtures verified structurally."

### 4.4 Uncommitted / unpreserved state (risk)

- `bounds-check-testing-framework/`: `diff-exec/`, `expectation-checker/` and both T2 `runs/`
  dirs are **untracked**. The root-repo commit `625d225` only committed docs. The harness
  that produced the headline correctness evidence is not in any git history.
- `wgpu/`: the T1 clippy fix in `naga/tests/naga/bounds_check_stress.rs` is still uncommitted.
- `08-SESSION-LOG.md`'s T5-A entry is uncommitted, and it says the sweeps are uncommitted
  (`M bounds_check_elision.rs`). They were actually committed as `83a90e0`.

### 4.5 Record drift

- `~/Projects/naga` (third checkout, listed in `AGENTS.md` Rule 1 and `05-REPOS…`) **does
  not exist** (VERIFIED `cd` failure). Its branch `feat/elision-hardening-and-soundness-tests`
  may only exist in `~/Archive/*.tar.zst` (UNKNOWN, not checked).
- The docs place the `sdata_manual` special case in `back/spv/index.rs`. It is actually in
  `back/spv/writer.rs:2824`. What it does: it skips workgroup zero-initialization for a
  variable with that name. Any user shader with a workgroup variable called `sdata_manual`
  gets memory that isn't zero-initialized, which WGSL doesn't allow. It must be removed
  before any upstream PR.
- `BOUNDS_CHECK_ELISION_TODO.md` §5: "Run headline paired GPU measurements under schema v3"
  is unchecked with "not yet run", but T2 ran it (single kernel). §6 still lists every paper
  item as open, which is accurate.

## 5. Claims that are safe for the poster today, and ones that are not

**Safe** (artifact-backed): non-constant index elision is the contribution; the interval-domain
design; 74.1% / 81.9% on project shaders vs 1–2.5% external (state both); `OpULessThan`
505 → 134; retained-check overhead 105.03 / 111.03 ns per dispatch on one kernel, one GPU,
with the non-overlap disclosed; 0 test failures; must-retain cases retain; execution-identical
outputs on 6 differing-binary fixtures.

**Not safe:** "0.0 ns" / 7,568 ns / 29 ps per check (superseded); "eq-exits such as
`if i == N {break;}`" (false for that shape); "verified across SPIR-V/MSL/HLSL" (only SPIR-V
tested); any speedup on real workloads; any claim beyond RTX 4060 + Vulkan; "fuzzed"
(the sweeps are exhaustive boundary enumeration, not fuzzing).

## 6. Recommended order for the next 5 days

Each item has branches in `07-MASTER-PLAN.md` / `TASKS/`.

1. **T7 poster** (binding, not started). Build it as the claims→evidence matrix from §5 and
   `03-REVIEW-RESPONSE-STATUS.md`. Success: every R1–R4 point has a block. Partial: some
   points marked "open" on the poster. Bad: the poster claims something from §5's
   "not safe" list, so it must be fixed before printing.
2. **Commit the unpreserved evidence** (§4.4): `diff-exec/` and `runs/` in the framework repo,
   and the stress-file fix. This is your call under Rule 1.
3. **Fix T5-B** (§4.1): about 3 one-line edits, then re-run and log.
4. **Eq-exit precision fix + regression test** (§4.2). Or, if time is short, correct the v2
   wording and the poster instead. Either way, stop claiming the top-guard shape.
5. **T4-A override pinning test**: small, and closes an R2 corner case by name.
6. **T6 paper v3** (related work, §3, figures, numbers) can follow the poster. It isn't needed
   for the Oct 2–3 poster.
