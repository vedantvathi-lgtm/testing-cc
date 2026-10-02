@group(0) @binding(0) var<storage, read_write> output: array<f32, 8>;
var<workgroup> tile: array<f32, 8>;
@compute @workgroup_size(8)
fn main(@builtin(local_invocation_index) lid: u32) {
    var acc = 0.0;
    for (var i = 0u; i < 8u; i = i + 1u) {
        tile[i] = f32(i);   // EXPECT: ELIDED
        workgroupBarrier();
        acc += tile[i];     // EXPECT: ELIDED
    }
    output[lid] = acc;      // EXPECT: ELIDED
}
