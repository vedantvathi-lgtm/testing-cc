
@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;

var<workgroup> s: array<u32, 256>;
@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32, @builtin(workgroup_id) wid: vec3<u32>) {
    let gi = wid.x * 256u + lid;
    s[lid] = input[gi];
    workgroupBarrier();
    for (var off = 1u; off < 256u; off = off * 2u) {
        var t = 0u;
        if (lid >= off) {
            t = s[lid - off];
        }
        workgroupBarrier();
        s[lid] = s[lid] + t;
        workgroupBarrier();
    }
    output[gi] = s[lid];
}
