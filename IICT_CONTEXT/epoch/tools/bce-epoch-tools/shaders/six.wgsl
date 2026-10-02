var<workgroup> s: array<u32, 256>;
@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32) {
    var i = lid;
    if (i < 128u) { s[i + 128u] = s[i]; }
}
