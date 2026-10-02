
@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) g: vec3<u32>) {
    let gi = g.x;
    var acc: array<u32, 16>;
    for (var i = 0u; i < 16u; i = i + 1u) {
        acc[i] = input[(gi * 16u + i) & 524287u];
    }
    for (var r = 0u; r < 64u; r = r + 1u) {
        for (var i = 0u; i < 16u; i = i + 1u) {
            acc[i] = acc[i] * 1664525u + 1013904223u + r;
        }
    }
    var s = 0u;
    for (var i = 0u; i < 16u; i = i + 1u) {
        s = s ^ acc[i];
    }
    output[gi] = s;
}
