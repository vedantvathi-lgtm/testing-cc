# T0 — Executor Protocol (READ FIRST, non-negotiable)

You are the executing agent (DeepSeek V4.1) running pre-scoped tasks. The reasoning,
scoping, and branching are already done. Your job is disciplined execution + honest
recording. These rules override anything else you were told.

## Execution rules

1. **One task at a time, in order** (T1 → T2 → … unless a task says otherwise). Never
   start a task whose gates fail — record the gate failure and stop that task.
2. **Run every gate/verification command exactly as written.** If a command isn't
   listed but you need it, you may run read-only commands freely (cat, grep, git log,
   ls, cargo test). Anything that modifies files must be within the task's stated scope.
3. **Copy numbers from artifacts.** Never type a number from memory into any file.
   Every number you write must be followed by its artifact path in the same line/table.
4. **Evidence labels** (mandatory): `VERIFIED` (you ran it this session),
   `RECORDED` (existing doc/artifact you cite by path), `REPORTED` (unconfirmed —
   treat as unproven), `UNKNOWN`, `INVALID`. If you can't label it, you didn't earn it.
5. **DEVIATION format** — if anything breaks or differs from the task's expectation:
   ```
   DEVIATION: expected <X>; observed <Y>; evidence: <path/output>;
   consequence: <what this changes>; next: <exact action>.
   ```
   Write it into the task's log section AND `IICT_CONTEXT/08-SESSION-LOG.md`. Never
   reword a failure as a success. An expected-but-unobserved result is not a success.
6. **Outcome branches.** Every task lists SUCCESS / NORMAL / BAD / DISASTER. After
   running, state which branch occurred with evidence. If reality matches NO branch,
   that is automatically a DEVIATION — stop, record, and end the task. Do not improvise
   past an unlisted outcome.
7. **Git safety.** Repos: `wgpu/` (bounds-check-elision @ `8757d7d`), `naga/`
   (test/elision-soundness-and-repro @ `9d5a6b82`), `wgpu-upstream/` (**READ-ONLY,
   never touch**), plus framework/corpus repos. Never push. Never commit unless the
   task explicitly says to, and then only the files the task names. Never reset,
   stash, checkout-away, or clean anything you did not create. Check `git status`
   before and after each task and include it in the task log.
8. **No scope creep.** If you notice an unrelated bug/improvement, write it into
   `IICT_CONTEXT/08-SESSION-LOG.md` under "Observations" — do not fix it mid-task.
9. **Stop conditions (hard).** Stop and end the session log immediately if:
   (a) a gate fails and the task has no branch for it, (b) you would have to invent
   a number, URL, or file content, (c) a command would delete/overwrite data you
   didn't create, (d) two tasks would conflict on the same repo state.
10. **Session close-out.** After each task: append a dated section to
    `IICT_CONTEXT/08-SESSION-LOG.md` (did / verified / deviations / artifacts /
    next) and tick the task's checklist item in the task file itself with the
    artifact path. Update `IICT_CONTEXT/00-INDEX.md` "Current state" if it changed.

## Environment facts (VERIFIED 2026-09-13 — re-verify with the gate commands, don't trust this list blindly)

- Project root: `/home/bb/Projects/Dave-Sonar-WGPU--master`
- GPU: NVIDIA RTX 4060 Laptop, driver 580.178.04, `nvidia-smi` functional.
- Disk: ~22 GB free at session start. Builds may need several GB — gate checks disk.
- Network available. Rust toolchain assumed present (repos built recently).
- Key docs: `AGENTS.md` (protocol), `IICT_CONTEXT/00-INDEX.md` (knowledge base),
  `IICT_CONTEXT/07-MASTER-PLAN.md` (strategy the tasks implement).

## Task map

| Task | What | Depends on | Est. effort |
|---|---|---|---|
| T1 | Certify current tree (tests/clippy/fmt) | — | 10–30 min |
| T2 | GPU schema-v3 rerun + negative control (+ optional multi-kernel extension) | T1 | 1–3 h incl. builds |
| T3 | Differential execution testing (elided vs checked on device) | T1 | 3–6 h |
| T4 | Corner-case pinning test + MSL/HLSL verification | T1 | 1–3 h |
| T5 | Interval fuzz + Go-style expectation harness | T1 (parallel ok) | 3–5 h |
| T6 | Paper v3: §3 accuracy + related work + numbers swap | T1 (T2 for numbers) | 4–8 h |
| T7 | Poster claims→evidence matrix | T2, T6 ideally | 2–4 h |
| T8 | Hygiene: Track-3 bug test, sdata_manual removal, consolidation | independent | 2–4 h |
