
@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;

var<workgroup> s: array<u32, 256>;
@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32, @builtin(workgroup_id) wid: vec3<u32>) {
    s[lid] = input[wid.x * 256u + lid];
    workgroupBarrier();
    for (var stride = 128u; stride > 0u; stride = stride >> 1u) {
        if (lid < stride) {
            s[lid] = s[lid] + s[lid + stride];
        }
        workgroupBarrier();
    }
    if (lid == 0u) {
        output[wid.x] = s[0];
    }
}
