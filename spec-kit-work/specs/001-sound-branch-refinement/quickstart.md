# Quickstart: verify feature 001

```bash
cd wgpu
git switch bce/sound-branch-refinement
cargo test -p naga --test naga bounds_check_stress   # new regression tests
cargo test -p naga                                    # full suite
cargo clippy -p naga --tests && cargo fmt --check -p naga
```

Census before/after (tool + corpora in `research_notes/Real world shader elision gaps/`):
rebuild `tools/access-census.rs` against this branch's naga and compare `safe` counts per
corpus with the `BASE_*` files.

Expected: GLSL + SPIR-V repros not proven safe; WGSL two-variable-comparison case proven safe;
real-world elided ≥ 100; naga tests ≥ 435.
