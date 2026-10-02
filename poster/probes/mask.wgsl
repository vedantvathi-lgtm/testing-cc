@group(0) @binding(0) var<storage, read_write> data: array<u32>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) g: vec3<u32>) {
    var lut = array<u32, 8>(1u, 2u, 3u, 4u, 5u, 6u, 7u, 8u);
    data[g.x] = lut[data[g.x] & 7u];
}
