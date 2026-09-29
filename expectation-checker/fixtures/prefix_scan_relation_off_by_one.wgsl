// Must-retain neighbour of prefix_scan_relation: lid - off - 1 underflows when lid == off.
var<workgroup> s: array<u32, 256>;
@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32) {
    for (var off = 1u; off < 256u; off = off * 2u) {
        var t = 0u;
        if (lid >= off) {
            t = s[lid - off - 1u];     // EXPECT: RETAINED
        }
        workgroupBarrier();
        s[lid] = s[lid] + t;           // EXPECT: ELIDED
        workgroupBarrier();
    }
}
