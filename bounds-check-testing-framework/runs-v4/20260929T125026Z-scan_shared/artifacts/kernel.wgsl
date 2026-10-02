
@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;

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
