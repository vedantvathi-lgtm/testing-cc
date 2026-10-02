@group(0) @binding(0) var<storage, read_write> output: array<f32, 8>;
@compute @workgroup_size(8)
fn main(@builtin(local_invocation_index) lid: u32) {
    var i: i32 = -4;
    loop {
        if (i < 4) { } else { break; }
        if (i >= 0) {
            output[i] = f32(i);  // EXPECT: ELIDED (refinement proves [0,3])
        }
        i = i + 1;
    }
}
