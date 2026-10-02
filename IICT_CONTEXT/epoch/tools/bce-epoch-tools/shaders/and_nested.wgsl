var<workgroup> s: array<u32, 128>;
@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32) {
    if (lid < 128u) { if (lid > 3u) { s[lid] = 1u; } }
}
