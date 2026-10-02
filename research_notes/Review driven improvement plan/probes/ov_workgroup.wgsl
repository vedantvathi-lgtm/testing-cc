override WG: u32 = 64;
var<workgroup> a: array<u32, 1>;
@compute @workgroup_size(WG) fn main(@builtin(local_invocation_id) l: vec3u) { a[l.x] = 1u; }
