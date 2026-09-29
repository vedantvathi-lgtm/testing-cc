adapters seen: ['NVIDIA GeForce RTX 4060 Laptop GPU']; reporting: NVIDIA; wgpu rev: 19ffb4e

| kernel | runs | BCE changes binary | proofs (BCE) | clamps no_bce → bce | no_bce / bce (geomean) | per-run min–max | control bce_dup/bce max dev (strict CI∋1) | bce / unchecked (residual) | no_bce / unchecked (all checks) | verdict |
|---|---|---|---|---|---|---|---|---|---|---|
| empty | 1 | no | 0/1 | 0 → 0 | **0.9987** (-0.13%) | 0.9987–0.9987 | 0.120% (0/1) | 1.0000 (-0.00%) | 1.0013 (+0.13%) | NO_PROOF (control) |
| matmul_tiled | 1 | yes | 8/11 | 8 → 0 | **1.1079** (+10.79%) | 1.1079–1.1079 | 0.007% (0/1) | 0.9999 (-0.01%) | 1.1079 (+10.79%) | RESOLVED |
| reduce_tree | 1 | yes | 4/6 | 4 → 0 | **1.0127** (+1.27%) | 1.0127–1.0127 | 0.016% (1/1) | 1.0000 (-0.00%) | 1.0128 (+1.28%) | RESOLVED |
| vello_binning | 1 | yes | 6/9 | 6 → 0 | **1.1484** (+14.84%) | 1.1484–1.1484 | 0.086% (0/1) | 0.9984 (-0.16%) | 1.1461 (+14.61%) | RESOLVED |

written runs-v4/summary-nvidia-19ffb4e.json
