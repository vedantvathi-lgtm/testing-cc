// A `continue` before the exit test skips it while the update still runs: i can pass 8.
@group(0) @binding(0) var<storage, read> xs: array<u32, 4>;
var<workgroup> t: array<u32, 9>;
@compute @workgroup_size(1)
fn main() {
    var i = 0u;
    loop {
        t[i] = 1u;                     // EXPECT: RETAINED
        if (xs[0] == 1u) { continue; }
        if (i >= 8u) { break; }
        continuing { i = i + 1u; }
    }
}
