// CONTROL (must FAIL): tile[lid] with len == workgroup size is always proven.
var<workgroup> tile: array<f32, 64>;
@compute @workgroup_size(64)
fn main(@builtin(local_invocation_index) lid: u32) {
    tile[lid] = 1.0;     // EXPECT: RETAINED
}
