# IICT_CONTEXT — Index

> Knowledge base for IICT 2026 paper #55 ("Static Analysis for Bounds Check Elision
> in the Naga WGSL Compiler", accepted poster, IICT Oct 2–3 2026). Created 2026-09-13.
> **Read AGENTS.md first — it is binding on every agent.** Then load in order:

| File | Contains | Read when |
|---|---|---|
| `../AGENTS.md` | Binding no-hallucination / no-assumption protocol; evidence vocabulary; repo safety | Always, first |
| `01-CHRONOLOGY.md` | Full time-based event history with provenance labels + discrepancies register | Orientation |
| `02-REVIEWS.md` | The four IICT reviews (score grid + verbatim record), R4's questions + established answers | Any paper/poster/rebuttal work |
| `03-REVIEW-RESPONSE-STATUS.md` | Every reviewer comment → current response status → evidence path; the five priorities P1–P5 | Planning |
| `04-EVIDENCE-LEDGER.md` | Claim-by-claim status incl. explicit anti-claims (what must never be asserted) | Before writing any claim |
| `05-REPOS-AND-REFERENCES.md` | Repo map with verified commits; references archive state incl. what's missing | Any code/citation work |
| `06-EXPERIMENTS-AND-METHODS.md` | Measurement primitives; why legacy JSON is disqualified; standing results; evidence gates | Any experiment work |
| `07-MASTER-PLAN.md` | Gated steps with SUCCESS/NORMAL/BAD/DISASTER branches; decision rules | Executing work |
| `08-SESSION-LOG.md` | Append-only per-session record (did/verified/deviations/next) | Always update at session end |
| `TASKS/` (T0–T8) | **Executor-ready task files** — pre-scoped, with exact commands, gates, verification, and outcome branches, written for an executing agent (DeepSeek V4.1) to run verbatim. `TASKS/T0-EXECUTOR-PROTOCOL.md` is the binding executor rulebook | Executing work |
| `09-CHECKOUT-CONSOLIDATION.md` | (T8 Part C will create) three-checkout consolidation map | Repo hygiene |
| `10-STATUS-REPORT-2026-09-27.md` | Full status report: original paper vs now, done vs open, audit findings (T5-B opcode bug, eq-exit shape gap, diff-exec caveats) | Orientation; before poster work |

## Current state (2026-09-13, all VERIFIED this session)

- Driver **recovered** (580.178.04, verified) and Rust toolchain **RESTORED 2026-09-13**
  (rustup 1.29.1 + pinned 1.93.1 w/ clippy/rustfmt/wasm32, user-space, no sudo;
  Vulkan loader + NVIDIA ICD verified working). Full evidence: `IICT_CONTEXT/logs/t1-build-failure.log`
  (the absence) + `08-SESSION-LOG.md` (the restore).
- **T1 executed SUCCESS** (137+272+6 tests, 0 fail; 58/58 stress, 2/2 assertions;
  clippy 0 warnings after one in-scope fix — left uncommitted in
  `naga/tests/naga/bounds_check_stress.rs` per T0 git-safety; fmt clean).
  **T2 executed SUCCESS w/ 1 recorded DEVIATION**: two schema-v3 GPU runs,
  control ⊇ 0 both, `valid_for_claims=true` both; headline **105.03 ns [101.53–106.60]**
  and **111.03 ns [108.44–112.42]** per dispatch @ `8757d7d`; CIs do not overlap
  (inter-run clock-state drift, kept separate — see ledger). Frozen 7,568 ns figure
  marked SUPERSEDED-BY these runs (different harness/normalization; never merge).
- wgpu @ `8757d7d` + T1's clippy fix uncommitted; naga @ `9d5a6b82`; references
  archive complete except W07 / Cousot / IRCE-RFC (recorded missing, not substituted).
- Headline evidence today: corpus study 74.1%/81.9% (VERIFIED); fresh paired timing
  above (VERIFIED); legacy timing banned.
- Biggest open gaps: related-work + §3-accuracy in paper; differential execution +
  fuzzing (T3/T5); poster (T7). Optional: T2 Step-4 multi-kernel sweep.
- **Execution handoff:** pre-scoped tasks live in `TASKS/T0–T8` — an executing agent
  should read `TASKS/T0-EXECUTOR-PROTOCOL.md` first, then run T1 → T2 → (T3‖T4‖T5) →
  T6 → T7, with T8 anytime. Reasoning/scoping/branching is already done in those files;
  the executor's job is disciplined, honestly-logged execution.
