# Bounds-Check Elision Benchmark Results

**Hardware:** NVIDIA GeForce RTX 4060 Laptop GPU · Vulkan · Driver 580.159.04
**Peak bandwidth:** 196.7 GB/s (GPU-timed sustained copy kernel, 20 passes)
**Date:** 2026-07-08
**Total measurements:** 84 configurations across 5 kernels × 3 policies × 4 sizes

---

## reduction/local

| Size   | Unchecked (µs) | RZSW (µs)    | Restrict (µs)  | RZSW Δ     | Restrict Δ   | RZSW d   | Restrict d |
|:------:|:--------------:|:------------:|:--------------:|:----------:|:------------:|:--------:|:----------:|
| 64K    | 3.953          | 3.527        | 3.942          | -10.66%†   | -0.20%†      | -1.8     | -0.5       |
| 1M     | 42.965         | 33.080       | 43.253         | -23.01%†   | +0.67%†      | -168.1   | +4.2       |
| 10M    | 435.315        | 364.420      | 433.720        | -16.29%†   | -0.37%†      | -40.1    | -0.9       |
| 16M    | 688.659        | 590.771      | 696.132        | -14.21%†   | +1.09%†      | -38.7    | +3.2       |

## reduction/storage

| Size   | Unchecked (µs) | RZSW (µs)    | Restrict (µs)  | RZSW Δ     | Restrict Δ   | RZSW d   | Restrict d |
|:------:|:--------------:|:------------:|:--------------:|:----------:|:------------:|:--------:|:----------:|
| 64K    | 2.927          | 3.142        | 3.021          | +7.36%†    | +3.24%†      | +10.1    | +4.7       |
| 1M     | 27.701         | 31.199       | 29.086         | +12.62%†   | +5.00%†      | +30.2    | +7.9       |
| 10M    | 397.387        | 434.157      | 421.766        | +9.28%†    | +6.13%†      | +1.2     | +17.4      |
| 16M    | 640.470        | 695.226      | 676.768        | +8.55%†    | +5.67%†      | +18.6    | +12.5      |

## prefix_scan/local

| Size   | Unchecked (µs) | RZSW (µs)    | Restrict (µs)  | RZSW Δ     | Restrict Δ   | RZSW d   | Restrict d |
|:------:|:--------------:|:------------:|:--------------:|:----------:|:------------:|:--------:|:----------:|
| 64K    | 3.960          | 4.155        | 4.148          | +4.90%†    | +4.77%†      | +18.4    | +11.2      |
| 1M     | 45.352         | 48.528       | 48.469         | +7.01%†    | +6.87%†      | +20.6    | +28.2      |
| 10M    | 519.451        | 533.330      | 527.750        | +2.67%†    | +1.59%†      | +4.1     | +2.9       |
| 16M    | 791.475        | 851.630      | 842.696        | +7.60%†    | +6.47%†      | +16.0    | +14.7      |

## prefix_scan/storage

| Size   | Unchecked (µs) | RZSW (µs)    | Restrict (µs)  | RZSW Δ     | Restrict Δ   | RZSW d   | Restrict d |
|:------:|:--------------:|:------------:|:--------------:|:----------:|:------------:|:--------:|:----------:|
| 64K    | 3.938          | 4.617        | 4.153          | +17.26%†   | +5.47%†      | +35.6    | +18.5      |
| 1M     | 48.615         | 59.834       | 52.244         | +23.08%†   | +7.46%†      | +25.3    | +9.6       |
| 10M    | 587.629        | 706.730      | 627.930        | +20.28%†   | +6.86%†      | +34.4    | +12.0      |
| 16M    | 939.896        | 1130.156     | 1003.503       | +20.24%†   | +6.77%†      | +43.2    | +17.6      |

## tiled_matmul/local

| Size   | Unchecked (µs) | RZSW (µs)    | Restrict (µs)  | RZSW Δ     | Restrict Δ   | RZSW d   | Restrict d |
|:------:|:--------------:|:------------:|:--------------:|:----------:|:------------:|:--------:|:----------:|
| 64K    | 45.267         | 46.238       | 45.774         | +2.15%†    | +1.12%†      | +4.3     | +2.1       |
| 1M     | 2673.010       | 2709.384     | 2697.559       | +1.36%†    | +0.92%†      | +4.0     | +2.3       |
| 10M    | 110846.400     | 112950.720   | 111971.872     | +1.90%†    | +1.03%†      | +3.6     | +2.0       |
| 16M    | 222466.112     | 226424.064   | 224092.608     | +1.78%†    | +0.73%†      | +1.3     | +2.2       |

## track3b_test/local

| Size   | Unchecked (µs) | RZSW (µs)    | Restrict (µs)  | RZSW Δ     | Restrict Δ   | RZSW d   | Restrict d |
|:------:|:--------------:|:------------:|:--------------:|:----------:|:------------:|:--------:|:----------:|
| 64K    | 2.456          | 2.551        | 2.632          | +3.84%†    | +7.16%†      | +5.1     | +8.2       |
| 1M     | 20.885         | 21.486       | 22.398         | +2.88%†    | +7.25%†      | +35.7    | +135.6     |
| 10M    | 250.793        | 257.642      | 271.019        | +2.73%†    | +8.07%†      | +5.4     | +16.0      |
| 16M    | 401.469        | 412.329      | 434.052        | +2.70%†    | +8.11%†      | +5.6     | +15.1      |

## wgul_reduce/local

| Size   | Unchecked (µs) | RZSW (µs)    | Restrict (µs)  | RZSW Δ     | Restrict Δ   | RZSW d   | Restrict d |
|:------:|:--------------:|:------------:|:--------------:|:----------:|:------------:|:--------:|:----------:|
| 64K    | 4.546          | 4.482        | 4.565          | -1.41%†    | +0.42%†      | -5.3     | +1.6       |
| 1M     | 50.484         | 50.695       | 50.995         | +0.42%†    | +1.01%†      | +26.2    | +75.7      |
| 10M    | 529.557        | 558.682      | 536.051        | +5.50%†    | +1.23%†      | +13.2    | +2.7       |
| 16M    | 847.745        | 891.895      | 857.817        | +5.21%†    | +1.18%†      | +10.3    | +2.5       |

---

## Key Findings

† All comparisons significant at Bonferroni-corrected α = 0.000893 (56 comparisons, 100k permutation samples).

1. **ReadZeroSkipWrite accelerates reduction/local** — up to 23% faster than Unchecked (Cohen's d = -168.1), best at 1M. The elision reduces register pressure from dead branches.
2. **ReadZeroSkipWrite hurts prefix_scan/storage** — up to 23% slower at 1M. Array-length queries (`OpArrayLength`) and zero-fill dominate runtime.
3. **Restrict policy adds -0.4% to +8.1% overhead** — consistent, small, and statistically significant across all kernels. No elision benefit, no pathological harm.
4. **tiled_matmul is most resilient** — overhead capped at +2.15% for either policy.

---

### Legend

| Symbol | Meaning |
|--------|---------|
| **RZSW** | ReadZeroSkipWrite (bounds check elided; zero-initialized on OOB access) |
| **Restrict** | Bounds check restricted via naga `Restrict` policy |
| **Δ** | Relative change vs Unchecked baseline (negative = faster) |
| **d** | Cohen's d effect size (|d| ≥ 0.8 = large) |
| **†** | Significant at Bonferroni-corrected threshold |
| **Peak BW** | 196.7 GB/s (live GPU-timed measurement) |