# WGSL corpus audit

Run date: 2026-08-27

Authoritative artifacts:

- `audit-20260827-project-v4.jsonl`
- `audit-20260827-project-v4.log`
- `audit-20260827-external-v4.jsonl`
- `audit-20260827-external-v4.log`

Older `v2`, `v3`, and pre-`v4` files are development artifacts and must not be
cited. These results measure static coverage and SPIR-V diagnostics, not GPU
performance.

## Method

Each WGSL input is parsed and validated twice from the same Naga module: once
with bounds-check elision enabled and once disabled. Both versions are emitted
under `ReadZeroSkipWrite`.

- `access_expressions` counts Naga `Expression::Access` nodes.
- `runtime_accesses` excludes indices that Naga's backend resolves directly as
  a literal, scalar zero value, or scalar constant.
- `elided` counts only exact access handles present in
  `FunctionInfo.safe_accesses`.
- Inputs are deduplicated by SHA-256 after line-ending normalization only.
  Comments, whitespace, and token boundaries remain significant.
- `spv_uless_*` is an aggregate `OpULessThan` diagnostic. It is not labeled as
  an exact bounds-check count because modules may contain unrelated unsigned
  comparisons.
- Code-generation errors and panic messages are retained per enabled/disabled
  side in each JSON record.

## Results

| Corpus | Files / unique | Parsed and validated | Codegen failures | Runtime accesses | Elided | Rate | Compute runtime | Compute elided | Compute rate |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Naga project fixtures | 164 / 153 | 153 | 12 | 587 | 435 | 74.11% | 42 | 15 | 35.71% |
| External wgpu corpus | 85 / 82 | 76 | 6 | 79 | 2 | 2.53% | 42 | 2 | 4.76% |

The project-fixture result is not representative of external WGSL. Its runtime
accesses are dominated by constructed local-loop tests: 437 are classified as
`shape_loop_local`, with 431 proved safe. The external corpus remains the
appropriate conservative headline.

## External shape distribution

| Shape | Runtime accesses | Elided |
|---|---:|---:|
| Builtin | 4 | 0 |
| Affine | 0 | 0 |
| Loop local | 6 | 2 |
| Unsupported/other | 69 | 0 |

The classifier is descriptive, not a proof system. `Unsupported` means the
index does not match one of the reporting categories; it does not mean the
program or access is invalid.

## Code-generation failures

The external failures are not all BCE defects:

- Bindless benchmark input: SPIR-V wrapper-struct inference validation error.
- Mesh shader inputs: requested SPIR-V version not enabled by the corpus
  writer configuration.
- External-texture inputs: unsupported SPIR-V backend paths panic with
  `not implemented`.

Both BCE-enabled and BCE-disabled sides fail identically for these records.
They remain visible in the artifacts rather than being counted as successful
code generation.

## Interpretation

1. The implemented patterns cover constructed recurrence fixtures well.
2. Natural external coverage remains small: 2 of 79 runtime accesses.
3. The two external elisions are loop-local cases; direct builtin and affine
   patterns did not fire in this corpus.
4. Aggregate SPIR-V diagnostics differ by two `OpULessThan` instructions in
   the external run, matching the two exact safe-access proofs, but this is
   still not access-level structural attribution.
5. No runtime-performance claim follows from this study.

## Reproduce

```bash
cargo test
cargo run -- audit-20260827-project-v4 ../wgpu/naga/tests/in/wgsl \
  >results/audit-20260827-project-v4.jsonl \
  2>results/audit-20260827-project-v4.log
cargo run -- audit-20260827-external-v4 \
  /home/bb/Projects/wgpu/benches \
  /home/bb/Projects/wgpu/examples \
  /home/bb/Projects/wgpu/tests \
  >results/audit-20260827-external-v4.jsonl \
  2>results/audit-20260827-external-v4.log
```
