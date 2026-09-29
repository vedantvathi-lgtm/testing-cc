@group(0) @binding(0) var<storage, read_write> output: array<f32, 8>;
var<workgroup> tile: array<f32, 8>;
@compute @workgroup_size(8)
fn main(@builtin(local_invocation_index) lid: u32) {
    var acc = 0.0;
    var i = 0u;
    loop {
        if (i == 8u) { break; }
        // The top-of-loop `==` exit bounds i to [0, 7] (`!=` refinement trims the
        // range endpoint; closed the 2026-09-27 gap in wgpu 2ffe1b7).
        tile[i] = f32(i);    // EXPECT: ELIDED
        workgroupBarrier();
        acc += tile[i];      // EXPECT: ELIDED
        i = i + 1u;
    }
    output[lid] = acc;       // EXPECT: ELIDED
}
