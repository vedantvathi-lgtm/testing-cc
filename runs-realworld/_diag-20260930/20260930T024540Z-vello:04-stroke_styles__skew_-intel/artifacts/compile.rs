//! WGSL -> SPIR-V for one slot, with wgpu-hal's production Vulkan options (see
//! `wgpu/wgpu-hal/src/vulkan/adapter.rs:2802-2838`, robustBufferAccess2 case).

use naga::back::spv;
use naga::proc::{BoundsCheckPolicies, BoundsCheckPolicy};
use naga::valid::{Capabilities, ValidationFlags, Validator};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub const SLOTS: [&str; 4] = ["no_bce", "bce", "bce_dup", "unchecked"];

#[derive(Serialize, Clone)]
pub struct Static {
    pub shader: String,
    pub slot: String,
    pub bce: bool,
    pub index_policy: String,
    pub buffer_policy: String,
    pub spv_words: usize,
    pub spv_sha256: String,
    pub proven_accesses: usize,
    pub dynamic_accesses: usize,
    pub umin_clamps: usize,
}

pub struct ShaderSpec {
    pub label: &'static str,
    pub wgsl: String,
    pub entry: &'static str,
    pub stage: naga::ShaderStage,
}

const OP_EXT_INST: u32 = 12;
const GLSL_STD_450_UMIN: u32 = 38;

pub fn umin_count(words: &[u32]) -> usize {
    let (mut i, mut n) = (5, 0);
    while i < words.len() {
        let count = (words[i] >> 16) as usize;
        if words[i] & 0xffff == OP_EXT_INST && count >= 5 && words[i + 4] == GLSL_STD_450_UMIN {
            n += 1;
        }
        i += count.max(1);
    }
    n
}

pub fn sha256(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}

/// The binding map wgpu-hal Vulkan hands naga: within each group, the k-th binding in ascending
/// order becomes Vulkan binding k (wgpu-core sorts layout entries, `device/bgl.rs` `from_entries`;
/// wgpu-hal numbers them consecutively, `vulkan/device.rs:1417-1456`, and maps them at
/// `vulkan/device.rs:1611-1631`). Assumes each pipeline layout declares exactly the shader's bindings,
/// which every workload in this harness does.
fn vulkan_binding_map(module: &naga::Module) -> spv::BindingMap {
    let mut per_group: std::collections::BTreeMap<u32, Vec<u32>> = Default::default();
    for (_, var) in module.global_variables.iter() {
        if let Some(rb) = var.binding {
            per_group.entry(rb.group).or_default().push(rb.binding);
        }
    }
    let mut map = spv::BindingMap::default();
    for (group, mut bindings) in per_group {
        bindings.sort_unstable();
        bindings.dedup();
        for (k, binding) in bindings.into_iter().enumerate() {
            map.insert(
                naga::ResourceBinding { group, binding },
                spv::BindingInfo { descriptor_set: group, binding: k as u32, binding_array_size: None },
            );
        }
    }
    map
}

/// Returns the SPIR-V words and static facts for `spec` compiled for `slot` (index into SLOTS).
pub fn compile(spec: &ShaderSpec, slot: usize) -> (Vec<u32>, Static, [u32; 3]) {
    let (bce, index, buffer) = match slot {
        0 => (false, BoundsCheckPolicy::Restrict, BoundsCheckPolicy::Unchecked),
        1 | 2 => (true, BoundsCheckPolicy::Restrict, BoundsCheckPolicy::Unchecked),
        _ => (false, BoundsCheckPolicy::Unchecked, BoundsCheckPolicy::Unchecked),
    };
    let module = naga::front::wgsl::parse_str(&spec.wgsl).unwrap_or_else(|e| panic!("{}: {}", spec.label, e.emit_to_string(&spec.wgsl)));
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    validator.set_bounds_check_elision(bce);
    let info = validator.validate(&module).unwrap_or_else(|e| panic!("{}: {}", spec.label, e.emit_to_string(&spec.wgsl)));
    let ep_index = module.entry_points.iter().position(|ep| ep.name == spec.entry).expect("entry point");
    let ep = &module.entry_points[ep_index];
    let fi = info.get_entry_point(ep_index);
    let dynamic = ep.function.expressions.iter().filter(|(_, e)| matches!(e, naga::Expression::Access { .. })).count();
    let options = spv::Options {
        lang_version: (1, 6),
        bounds_check_policies: BoundsCheckPolicies { index, buffer, image_load: BoundsCheckPolicy::Unchecked, binding_array: BoundsCheckPolicy::Unchecked },
        zero_initialize_workgroup_memory: spv::ZeroInitializeWorkgroupMemoryMode::Native,
        force_loop_bounding: true,
        binding_map: vulkan_binding_map(&module),
        ..Default::default()
    };
    let pipeline = spv::PipelineOptions { shader_stage: spec.stage, entry_point: spec.entry.into() };
    let words = spv::write_vec(&module, &info, &options, Some(&pipeline)).expect("spv");
    let st = Static {
        shader: spec.label.into(),
        slot: SLOTS[slot].into(),
        bce,
        index_policy: format!("{index:?}"),
        buffer_policy: format!("{buffer:?}"),
        spv_words: words.len(),
        spv_sha256: sha256(bytemuck::cast_slice(&words)),
        proven_accesses: fi.safe_accesses.len(),
        dynamic_accesses: dynamic,
        umin_clamps: umin_count(&words),
    };
    (words, st, ep.workgroup_size)
}
