use crate::compile::ShaderSpec;

/// A real-world workload measured slot by slot. Slots are indexed like `compile::SLOTS`.
pub trait Workload {
    fn name(&self) -> String;
    /// Where the shaders and inputs come from (upstream repo, commit, file), for the result file.
    fn provenance(&self) -> serde_json::Value;
    fn shaders(&self) -> Vec<ShaderSpec>;
    /// `modules[slot][shader]`, shaders in the order `shaders()` returned them.
    fn setup(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, modules: &[Vec<wgpu::ShaderModule>]);
    /// Record `reps` repetitions of the workload for `slot`. The first pass must write timestamp
    /// 0 at its beginning and the last pass timestamp 1 at its end.
    fn record(&self, slot: usize, encoder: &mut wgpu::CommandEncoder, queries: &wgpu::QuerySet, reps: u32);
    /// Run the workload once for `slot` from a reset state and return its output bytes.
    fn output(&self, device: &wgpu::Device, queue: &wgpu::Queue, slot: usize) -> Vec<u8>;
    /// For workloads with a known, elision-independent race: `(element_bytes, mask)` where
    /// `mask[k]` says whether output element `k` must be identical across slots. `None` = all.
    fn oracle_mask(&self) -> Option<(usize, Vec<bool>)> {
        None
    }
    /// For workloads whose upstream implementation is not bit-deterministic: every distinct output
    /// an independent reference produced on this adapter (e.g. the upstream renderer's own images
    /// over repeated renders; may be empty if none exist for this adapter). `None` = the workload
    /// is deterministic and slots are compared byte for byte.
    fn reference_outputs(&self, _adapter: &str) -> Option<Vec<Vec<u8>>> {
        None
    }
}

pub fn read_buffer(device: &wgpu::Device, buffer: &wgpu::Buffer) -> Vec<u8> {
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| ());
    device.poll(wgpu::PollType::wait_indefinitely()).expect("poll");
    let out = slice.get_mapped_range().expect("map").to_vec();
    buffer.unmap();
    out
}
