
@group(0) @binding(0) var<storage, read_write> output: array<f32, 256>;
var<workgroup> tile: array<f32, 256>;

@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32) {
    tile[lid] = f32(lid + 1u);
    workgroupBarrier();

    var sum = 0.0;
    for (var k = 0u; k < 256u; k = k + 1u) {
        sum += tile[k];
    }
    output[lid] = sum;
}
