| kernel | runs | BCE changes binary | proofs (BCE) | clamps no_bce → bce | no_bce / bce (geomean) | per-run min–max | control bce_dup/bce max dev (strict CI∋1) | bce / unchecked (residual) | no_bce / unchecked (all checks) | verdict |
|---|---|---|---|---|---|---|---|---|---|---|
| empty | 10 | no | 0/1 | 0 → 0 | **1.0013** (+0.13%) | 0.9997–1.0024 | 0.290% (6/10) | 1.0003 (+0.03%) | 0.9990 (-0.10%) | NO_PROOF (control) |
| scan_shared | 10 | yes | 2/4 | 2 → 0 | **1.1579** (+15.79%) | 1.1578–1.1579 | 0.007% (9/10) | 1.0000 (-0.00%) | 1.1578 (+15.78%) | RESOLVED |
| matmul_tiled | 10 | yes | 8/11 | 8 → 0 | **1.1080** (+10.80%) | 1.1079–1.1081 | 0.005% (10/10) | 1.0000 (-0.00%) | 1.1080 (+10.80%) | RESOLVED |
| reduce_tree | 10 | yes | 3/6 | 4 → 1 | **1.0009** (+0.09%) | 1.0007–1.0011 | 0.027% (9/10) | 1.0120 (+1.20%) | 1.0128 (+1.28%) | NOT_RESOLVABLE: effect <= max(2x control deviation, 0.5%) |
| prefix_scan_hs | 10 | yes | 4/7 | 5 → 1 | **1.0017** (+0.17%) | 1.0010–1.0030 | 0.252% (9/10) | 1.0071 (+0.71%) | 1.0089 (+0.89%) | NOT_RESOLVABLE: effect <= max(2x control deviation, 0.5%) |
| vello_binning | 10 | yes | 4/9 | 6 → 2 | **1.1388** (+13.88%) | 1.1376–1.1398 | 0.054% (9/10) | 1.0108 (+1.08%) | 1.1510 (+15.10%) | RESOLVED |
| private_array | 10 | yes | 4/6 | 4 → 0 | **0.9997** (-0.03%) | 0.9991–1.0005 | 0.103% (6/10) | 0.9997 (-0.03%) | 0.9995 (-0.05%) | NOT_RESOLVABLE: runs disagree in sign |

written runs-v4/summary.json
