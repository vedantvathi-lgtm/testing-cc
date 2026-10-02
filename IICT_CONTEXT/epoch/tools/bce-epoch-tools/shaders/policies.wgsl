@group(0) @binding(0) var<storage, read_write> buf: array<u32>;
var<workgroup> s: array<u32, 64>;
@compute @workgroup_size(64)
fn main(@builtin(local_invocation_index) lid: u32) {
    let j = buf[lid];
    s[lid] = j;
    s[j] = lid;
}
