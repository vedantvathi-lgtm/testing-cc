@group(0) @binding(0) var<storage, read_write> output: array<f32, 8>;
var<workgroup> tile: array<f32, 8>;
@compute @workgroup_size(8)
fn main(@builtin(local_invocation_index) lid: u32) {
    var acc = 0.0;
    var i = 0u;
    loop {
        if (i == 9u) { break; }
        // Must-retain neighbour of eq_exit_loop: exiting at 9 lets i reach 8.
        tile[i] = f32(i);    // EXPECT: RETAINED
        workgroupBarrier();
        acc += tile[i];      // EXPECT: RETAINED
        i = i + 1u;
    }
    output[lid] = acc;       // EXPECT: ELIDED
}
