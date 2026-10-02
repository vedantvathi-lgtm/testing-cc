@group(0) @binding(0) var<storage, read> xs: array<u32, 4>;
var<workgroup> t: array<u32, 4>;
@compute @workgroup_size(1) fn main() {
  var i = xs[0];
  let small = i < 4u;
  i = xs[1];
  if (small) { t[i] = 1u; }
}
