@group(0) @binding(0) var<storage, read_write> output: array<f32, 8>;
@compute @workgroup_size(8)
fn main(@builtin(local_invocation_index) lid: u32) {
    var i = lid;
    if (i < 4u) {
        i = i + 8u;        // mutation invalidates the proof
    }
    output[i] = 1.0;       // EXPECT: RETAINED
}
