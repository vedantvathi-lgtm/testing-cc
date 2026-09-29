// Runtime-sized storage: compile-time analysis can NEVER prove these.
@group(0) @binding(0) var<storage, read_write> output: array<f32>;
@compute @workgroup_size(8)
fn main(@builtin(local_invocation_index) lid: u32) {
    var i = lid * 3u;      // (no access on this line; the MANUAL marker belongs on the access below)
    if (i < arrayLength(&output)) {
        output[i] = 1.0;   // EXPECT: MANUAL (dominating guard exists but analysis is conservative)
    }
}
