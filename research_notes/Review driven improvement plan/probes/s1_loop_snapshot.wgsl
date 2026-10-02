var<workgroup> t9: array<u32, 9>;
@compute @workgroup_size(1) fn main() {
  var i = 0u;
  loop { t9[i] = 1u; let done = i >= 9u; i = i + 1u; if (done) { break; } }
}
