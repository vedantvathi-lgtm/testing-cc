@group(0) @binding(0) var<uniform> n: i32;
var<workgroup> sh: array<u32, 256>;
@compute @workgroup_size(256)
fn main(@builtin(local_invocation_id) local_id: vec3<u32>) {
  var a = n; var b = 0; var c = 0;
  if a == b { c = a; }
  sh[local_id.x] = 1u;
}
