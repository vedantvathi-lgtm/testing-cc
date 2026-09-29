adapters seen: ['Intel(R) Graphics (RPL-S)', 'NVIDIA GeForce RTX 4060 Laptop GPU']; reporting: intel

| kernel | runs | BCE changes binary | proofs (BCE) | clamps no_bce → bce | no_bce / bce (geomean) | per-run min–max | control bce_dup/bce max dev (strict CI∋1) | bce / unchecked (residual) | no_bce / unchecked (all checks) | verdict |
|---|---|---|---|---|---|---|---|---|---|---|
| empty | 5 | no | 0/1 | 0 → 0 | **1.0000** (+0.00%) | 0.9999–1.0002 | 0.020% (5/5) | 1.0000 (+0.00%) | 1.0000 (+0.00%) | NO_PROOF (control) |
| scan_shared | 5 | yes | 2/4 | 2 → 0 | **1.1162** (+11.62%) | 1.1161–1.1163 | 0.006% (5/5) | 1.0000 (-0.00%) | 1.1161 (+11.61%) | RESOLVED |
| matmul_tiled | 5 | yes | 8/11 | 8 → 0 | **1.0000** (-0.00%) | 0.9994–1.0003 | 0.012% (5/5) | 1.0001 (+0.01%) | 1.0000 (-0.00%) | NOT_RESOLVABLE: runs disagree in sign |
| reduce_tree | 5 | yes | 3/6 | 4 → 1 | **1.0001** (+0.01%) | 0.9999–1.0007 | 0.028% (5/5) | 1.0730 (+7.30%) | 1.0733 (+7.33%) | NOT_RESOLVABLE: runs disagree in sign |
| prefix_scan_hs | 5 | yes | 4/7 | 5 → 1 | **1.0000** (+0.00%) | 0.9996–1.0005 | 0.041% (5/5) | 1.0365 (+3.65%) | 1.0370 (+3.70%) | NOT_RESOLVABLE: runs disagree in sign |
| vello_binning | 5 | yes | 4/9 | 6 → 2 | **0.9999** (-0.01%) | 0.9995–1.0002 | 0.016% (5/5) | 0.9999 (-0.01%) | 0.9999 (-0.01%) | NOT_RESOLVABLE: runs disagree in sign |
| private_array | 5 | yes | 4/6 | 4 → 0 | **1.0000** (+0.00%) | 1.0000–1.0000 | 0.001% (5/5) | 0.9995 (-0.05%) | 1.0000 (+0.00%) | NOT_RESOLVABLE: effect <= max(2x control deviation, 0.5%) |

written runs-v4/summary-intel.json
