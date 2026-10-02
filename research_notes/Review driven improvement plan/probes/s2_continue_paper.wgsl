var<workgroup> t: array<u32, 4>;
@compute @workgroup_size(1) fn main() {
  var i = 0u;
  loop {
    if (i >= 4u) { continue; }
    continuing { t[i] = 1u; i = i + 1u; break if i >= 10u; }
  }
}
