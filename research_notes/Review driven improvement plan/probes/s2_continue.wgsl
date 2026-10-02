@group(0) @binding(0) var<storage, read> xs: array<u32, 4>;
var<workgroup> t: array<u32, 8>;
@compute @workgroup_size(1) fn main() {
  var i = 8u;
  loop {
    if (i == 0u) { break; }
    if (xs[0] == 1u) { continue; }
    i = i - 1u;
    continuing { t[i] = 1u; }
  }
}
