
@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;

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
