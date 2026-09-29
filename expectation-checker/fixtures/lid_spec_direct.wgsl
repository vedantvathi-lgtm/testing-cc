@group(0) @binding(0) var<storage, read_write> output: array<f32, 64>;
var<workgroup> tile: array<f32, 64>;
@compute @workgroup_size(64)
fn main(@builtin(local_invocation_index) lid: u32) {
    tile[lid] = f32(lid);          // EXPECT: ELIDED
    workgroupBarrier();
    let v = tile[lid];             // EXPECT: ELIDED
    output[lid] = v * 2.0;         // EXPECT: ELIDED
}
