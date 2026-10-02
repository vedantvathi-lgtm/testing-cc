# Probe results (bce-epoch-tools against wgpu fork f73a687; same results at 19ffb4e)

Generated 2026-10-01T02:54:17Z by running: bce-epoch-tools ir <file>

## guard.wgsl
```
[13] Access { base: [11], index: [12] }
[19] Access { base: [17], index: [18] }
[20] Access { base: [10], index: [14] }
-- safe_accesses = []
```
## guard_var.wgsl
```
[13] Access { base: [11], index: [12] }
[21] Access { base: [19], index: [20] }
[23] Access { base: [10], index: [22] }   <= in safe_accesses
-- safe_accesses = [23]
```
## guard_builtin.wgsl
```
[16] Access { base: [14], index: [15] }
[17] Access { base: [10], index: [11] }
-- safe_accesses = []
```
## guard_let_arith.wgsl
```
[13] Access { base: [11], index: [12] }
[21] Access { base: [19], index: [20] }
[22] Access { base: [10], index: [16] }
-- safe_accesses = []
```
## guard_let2.wgsl
```
[13] Access { base: [11], index: [12] }
[19] Access { base: [17], index: [18] }
[20] Access { base: [10], index: [14] }
-- safe_accesses = []
```
## mask.wgsl
```
[13] Access { base: [11], index: [12] }
[16] Access { base: [14], index: [15] }
[20] Access { base: [10], index: [19] }   <= in safe_accesses
-- safe_accesses = [20]
```
## relation_prefix_scan.wgsl
```
[ 7] Access { base: [6], index: [0] }   <= in safe_accesses
[ 9] Access { base: [8], index: [5] }
[23] Access { base: [20], index: [22] }   <= in safe_accesses
[26] Access { base: [25], index: [0] }   <= in safe_accesses
[28] Access { base: [27], index: [0] }   <= in safe_accesses
[36] Access { base: [35], index: [5] }
[38] Access { base: [37], index: [0] }   <= in safe_accesses
-- safe_accesses = [7, 23, 26, 28, 38]
```
## recurrence_reduce_tree.wgsl
```
[ 3] Access { base: [2], index: [0] }   <= in safe_accesses
[ 9] Access { base: [4], index: [8] }
[19] Access { base: [18], index: [0] }   <= in safe_accesses
[21] Access { base: [20], index: [0] }   <= in safe_accesses
[26] Access { base: [23], index: [25] }   <= in safe_accesses
[36] Access { base: [34], index: [35] }
-- safe_accesses = [3, 19, 21, 26]
```
