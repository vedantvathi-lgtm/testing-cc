//! Benchmark kernels. All arithmetic is u32 (wrapping) so every configuration's output
//! can be compared exactly against a CPU reference.

pub const WG: u32 = 256;
pub const NWG: u32 = 2048;
pub const TOTAL: usize = (WG * NWG) as usize;
const MATMUL_N: usize = 512;

pub struct Kernel {
    pub name: &'static str,
    /// What the kernel stands for, in one line (goes into the report).
    pub models: &'static str,
    pub wgsl: &'static str,
    pub workgroup_size: (u32, u32, u32),
    pub dispatch: (u32, u32, u32),
    pub input_words: usize,
    pub output_words: usize,
    pub reference: fn(&[u32]) -> Vec<u32>,
}

const BINDINGS: &str = "
@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;
";

pub fn all() -> Vec<Kernel> {
    vec![
        Kernel {
            name: "empty",
            models: "timing floor: one store per thread, no array indexing to elide",
            wgsl: concat_bindings(EMPTY),
            workgroup_size: (WG, 1, 1),
            dispatch: (NWG, 1, 1),
            input_words: TOTAL,
            output_words: TOTAL,
            reference: |input| (0..TOTAL as u32).map(|i| i ^ input[0]).collect(),
        },
        Kernel {
            name: "scan_shared",
            models: "v3 kernel at full occupancy: tile[lid] + 256-iteration scan of workgroup memory",
            wgsl: concat_bindings(SCAN_SHARED),
            workgroup_size: (WG, 1, 1),
            dispatch: (NWG, 1, 1),
            input_words: TOTAL,
            output_words: TOTAL,
            reference: reference_scan_shared,
        },
        Kernel {
            name: "matmul_tiled",
            models: "16x16 shared-memory tiled matmul, N=512 (tile loops over k)",
            wgsl: concat_bindings(MATMUL_TILED),
            workgroup_size: (16, 16, 1),
            dispatch: (32, 32, 1),
            input_words: 2 * MATMUL_N * MATMUL_N,
            output_words: MATMUL_N * MATMUL_N,
            reference: reference_matmul,
        },
        Kernel {
            name: "reduce_tree",
            models: "workgroup tree reduction s[lid] += s[lid + stride], stride >>= 1",
            wgsl: concat_bindings(REDUCE_TREE),
            workgroup_size: (WG, 1, 1),
            dispatch: (NWG, 1, 1),
            input_words: TOTAL,
            output_words: NWG as usize,
            reference: reference_reduce,
        },
        Kernel {
            name: "prefix_scan_hs",
            models: "Hillis-Steele inclusive scan: s[lid - off], off *= 2",
            wgsl: concat_bindings(PREFIX_SCAN_HS),
            workgroup_size: (WG, 1, 1),
            dispatch: (NWG, 1, 1),
            input_words: TOTAL,
            output_words: TOTAL,
            reference: reference_prefix_scan,
        },
        Kernel {
            name: "vello_binning",
            models: "vello binning.wgsl shape: two-variable `if x0 == x1`, then per-slice bitmap loops",
            wgsl: concat_bindings(VELLO_BINNING),
            workgroup_size: (WG, 1, 1),
            dispatch: (NWG, 1, 1),
            input_words: TOTAL,
            output_words: TOTAL,
            reference: reference_vello_binning,
        },
        Kernel {
            name: "private_array",
            models: "vello fine.wgsl shape: per-thread `var area: array<_, N>` updated in nested loops",
            wgsl: concat_bindings(PRIVATE_ARRAY),
            workgroup_size: (WG, 1, 1),
            dispatch: (NWG, 1, 1),
            input_words: TOTAL,
            output_words: TOTAL,
            reference: reference_private_array,
        },
    ]
}

fn concat_bindings(body: &'static str) -> &'static str {
    Box::leak(format!("{BINDINGS}{body}").into_boxed_str())
}

const EMPTY: &str = "
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) g: vec3<u32>) {
    output[g.x] = g.x ^ input[0];
}
";

const SCAN_SHARED: &str = "
var<workgroup> tile: array<u32, 256>;
@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32, @builtin(workgroup_id) wid: vec3<u32>) {
    let base = wid.x * 256u;
    tile[lid] = input[base + lid];
    workgroupBarrier();
    var sum = 0u;
    for (var k = 0u; k < 256u; k = k + 1u) {
        sum = sum + (tile[k] ^ k);
    }
    output[base + lid] = sum + lid;
}
";

fn reference_scan_shared(input: &[u32]) -> Vec<u32> {
    let mut out = vec![0; TOTAL];
    for wg in 0..NWG as usize {
        let base = wg * WG as usize;
        let sum = (0..WG as usize).fold(0u32, |s, k| s.wrapping_add(input[base + k] ^ k as u32));
        for lid in 0..WG as usize {
            out[base + lid] = sum.wrapping_add(lid as u32);
        }
    }
    out
}

const MATMUL_TILED: &str = "
var<workgroup> tile_a: array<array<u32, 16>, 16>;
var<workgroup> tile_b: array<array<u32, 16>, 16>;
@compute @workgroup_size(16, 16)
fn main(@builtin(local_invocation_id) lid: vec3<u32>, @builtin(workgroup_id) wid: vec3<u32>) {
    let row = wid.y * 16u + lid.y;
    let col = wid.x * 16u + lid.x;
    var acc = 0u;
    for (var t = 0u; t < 32u; t = t + 1u) {
        tile_a[lid.y][lid.x] = input[row * 512u + t * 16u + lid.x];
        tile_b[lid.y][lid.x] = input[262144u + (t * 16u + lid.y) * 512u + col];
        workgroupBarrier();
        for (var k = 0u; k < 16u; k = k + 1u) {
            acc = acc + tile_a[lid.y][k] * tile_b[k][lid.x];
        }
        workgroupBarrier();
    }
    output[row * 512u + col] = acc;
}
";

fn reference_matmul(input: &[u32]) -> Vec<u32> {
    let n = MATMUL_N;
    let (a, b) = input.split_at(n * n);
    let mut out = vec![0u32; n * n];
    for r in 0..n {
        for k in 0..n {
            let av = a[r * n + k];
            let brow = &b[k * n..k * n + n];
            let orow = &mut out[r * n..r * n + n];
            for c in 0..n {
                orow[c] = orow[c].wrapping_add(av.wrapping_mul(brow[c]));
            }
        }
    }
    out
}

const REDUCE_TREE: &str = "
var<workgroup> s: array<u32, 256>;
@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32, @builtin(workgroup_id) wid: vec3<u32>) {
    s[lid] = input[wid.x * 256u + lid];
    workgroupBarrier();
    for (var stride = 128u; stride > 0u; stride = stride >> 1u) {
        if (lid < stride) {
            s[lid] = s[lid] + s[lid + stride];
        }
        workgroupBarrier();
    }
    if (lid == 0u) {
        output[wid.x] = s[0];
    }
}
";

fn reference_reduce(input: &[u32]) -> Vec<u32> {
    input
        .chunks(WG as usize)
        .map(|c| c.iter().fold(0u32, |s, v| s.wrapping_add(*v)))
        .collect()
}

const PREFIX_SCAN_HS: &str = "
var<workgroup> s: array<u32, 256>;
@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32, @builtin(workgroup_id) wid: vec3<u32>) {
    let gi = wid.x * 256u + lid;
    s[lid] = input[gi];
    workgroupBarrier();
    for (var off = 1u; off < 256u; off = off * 2u) {
        var t = 0u;
        if (lid >= off) {
            t = s[lid - off];
        }
        workgroupBarrier();
        s[lid] = s[lid] + t;
        workgroupBarrier();
    }
    output[gi] = s[lid];
}
";

fn reference_prefix_scan(input: &[u32]) -> Vec<u32> {
    let mut out = vec![0; TOTAL];
    for (wg, chunk) in input.chunks(WG as usize).enumerate() {
        let mut acc = 0u32;
        for (lid, v) in chunk.iter().enumerate() {
            acc = acc.wrapping_add(*v);
            out[wg * WG as usize + lid] = acc;
        }
    }
    out
}

const VELLO_BINNING: &str = "
var<workgroup> bitmaps: array<array<atomic<u32>, 256>, 8>;
@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32, @builtin(workgroup_id) wid: vec3<u32>) {
    let gi = wid.x * 256u + lid;
    let a = input[gi];
    let b = input[gi ^ 1u];
    var x0 = a & 15u;
    var x1 = b & 15u;
    if (x0 == x1) {
        x1 = x0 + 1u;
    }
    for (var i = 0u; i < 8u; i = i + 1u) {
        atomicStore(&bitmaps[i][lid], 0u);
    }
    workgroupBarrier();
    let slice = (a >> 4u) & 7u;
    let bin = (a >> 7u) & 255u;
    atomicOr(&bitmaps[slice][bin], 1u << (lid & 31u));
    workgroupBarrier();
    var count = 0u;
    for (var i = 0u; i < 8u; i = i + 1u) {
        count = count + countOneBits(atomicLoad(&bitmaps[i][lid]));
    }
    output[gi] = count * 16u + x1;
}
";

fn reference_vello_binning(input: &[u32]) -> Vec<u32> {
    let mut out = vec![0; TOTAL];
    for wg in 0..NWG as usize {
        let base = wg * WG as usize;
        let mut bitmaps = [[0u32; 256]; 8];
        for lid in 0..WG as usize {
            let a = input[base + lid];
            bitmaps[((a >> 4) & 7) as usize][((a >> 7) & 255) as usize] |= 1 << (lid & 31);
        }
        for lid in 0..WG as usize {
            let gi = base + lid;
            let (a, b) = (input[gi], input[gi ^ 1]);
            let (x0, mut x1) = (a & 15, b & 15);
            if x0 == x1 {
                x1 = x0 + 1;
            }
            let count: u32 = (0..8).map(|i| bitmaps[i][lid].count_ones()).sum();
            out[gi] = count.wrapping_mul(16).wrapping_add(x1);
        }
    }
    out
}

const PRIVATE_ARRAY: &str = "
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) g: vec3<u32>) {
    let gi = g.x;
    var acc: array<u32, 16>;
    for (var i = 0u; i < 16u; i = i + 1u) {
        acc[i] = input[(gi * 16u + i) & 524287u];
    }
    for (var r = 0u; r < 64u; r = r + 1u) {
        for (var i = 0u; i < 16u; i = i + 1u) {
            acc[i] = acc[i] * 1664525u + 1013904223u + r;
        }
    }
    var s = 0u;
    for (var i = 0u; i < 16u; i = i + 1u) {
        s = s ^ acc[i];
    }
    output[gi] = s;
}
";

fn reference_private_array(input: &[u32]) -> Vec<u32> {
    (0..TOTAL)
        .map(|gi| {
            let mut acc = [0u32; 16];
            for (i, slot) in acc.iter_mut().enumerate() {
                *slot = input[(gi * 16 + i) & 524287];
            }
            for r in 0..64u32 {
                for slot in acc.iter_mut() {
                    *slot = slot.wrapping_mul(1664525).wrapping_add(1013904223).wrapping_add(r);
                }
            }
            acc.iter().fold(0, |s, v| s ^ v)
        })
        .collect()
}
