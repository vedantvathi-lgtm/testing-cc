
@group(0) @binding(0) var<storage, read> input: array<u32>;
@group(0) @binding(1) var<storage, read_write> output: array<u32>;

var<workgroup> tile_a: array<array<u32, 16>, 16>;
var<workgroup> tile_b: array<array<u32, 16>, 16>;
@compute @workgroup_size(16, 16)
fn main(@builtin(local_invocation_id) lid: vec3<u32>, @builtin(workgroup_id) wid: vec3<u32>) {
    let row = wid.y * 16u + lid.y;
    let col = wid.x * 16u + lid.x;
    var acc = 0u;
    for (var t = 0u; t < 32u; t = t + 1u) {
        tile_a[lid.y][lid.x] = input[row * 512u + t * 16u + lid.x];
        tile_b[lid.y][lid.x] = input[262144u + (t * 16u + lid.y) * 512u + col];
        workgroupBarrier();
        for (var k = 0u; k < 16u; k = k + 1u) {
            acc = acc + tile_a[lid.y][k] * tile_b[k][lid.x];
        }
        workgroupBarrier();
    }
    output[row * 512u + col] = acc;
}
