var<workgroup> t: array<u32, 9>;
@compute @workgroup_size(1) fn main() {
  var i = 8u;
  loop { t[i] = 1u; i = 100u; if (i == 0u) { break; } i = i - 1u; }
}
