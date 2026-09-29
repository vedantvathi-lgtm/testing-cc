// CONTROL (must ERROR): the marked line contains no runtime access.
var<workgroup> tile: array<f32, 64>;
@compute @workgroup_size(64)
fn main(@builtin(local_invocation_index) lid: u32) {
    let x = lid;         // EXPECT: ELIDED
    tile[x] = 1.0;
}
