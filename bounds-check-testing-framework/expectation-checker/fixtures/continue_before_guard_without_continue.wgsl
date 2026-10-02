// Neighbour of continue_before_guard without the continue: the guard bounds i.
@group(0) @binding(0) var<storage, read> xs: array<u32, 4>;
var<workgroup> t: array<u32, 9>;
@compute @workgroup_size(1)
fn main() {
    var i = 0u;
    loop {
        t[i] = 1u;                     // EXPECT: ELIDED
        if (i >= 8u) { break; }
        continuing { i = i + 1u; }
    }
}
