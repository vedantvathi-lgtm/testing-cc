// Hillis-Steele scan: `lid >= off` makes `lid - off` non-negative (relational fact),
// and off = 1, 2, ..., 128 (geometric loop variable). Proven since wgpu 19ffb4e.
var<workgroup> s: array<u32, 256>;
@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32) {
    for (var off = 1u; off < 256u; off = off * 2u) {
        var t = 0u;
        if (lid >= off) {
            t = s[lid - off];          // EXPECT: ELIDED
        }
        workgroupBarrier();
        s[lid] = s[lid] + t;           // EXPECT: ELIDED
        workgroupBarrier();
    }
}
