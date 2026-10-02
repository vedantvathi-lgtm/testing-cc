//! Replay what wgpu 29 does with ShaderSource::SpirV on Vulkan (wgpu/src/backend/wgpu_core.rs:1048,
//! wgpu-core/src/device/resource.rs:2388, wgpu-hal/src/vulkan/adapter.rs spv::Options):
//! spv-in -> Validator (default: elision ON) -> spv-out(index=Restrict, buffer=<arg>, force_loop_bounding).
use naga::back::spv;
use naga::proc::{BoundsCheckPolicies, BoundsCheckPolicy};
use naga::valid::{Capabilities, ValidationFlags, Validator};
use rspirv::binary::Disassemble;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let bytes = std::fs::read(&a[1]).unwrap();
    let buffer = if a[2] == "unchecked" { BoundsCheckPolicy::Unchecked } else { BoundsCheckPolicy::Restrict };
    let opts = naga::front::spv::Options { adjust_coordinate_space: false, strict_capabilities: true, block_ctx_dump_prefix: None };
    let module = naga::front::spv::parse_u8_slice(&bytes, &opts).unwrap();
    let info = Validator::new(ValidationFlags::all(), Capabilities::all()).validate(&module).unwrap();
    let safe: usize = (0..module.entry_points.len()).map(|i| info.get_entry_point(i).safe_accesses.len()).sum::<usize>()
        + module.functions.iter().map(|(h, _)| info[h].safe_accesses.len()).sum::<usize>();
    let options = spv::Options {
        lang_version: (1, 6),
        bounds_check_policies: BoundsCheckPolicies { index: BoundsCheckPolicy::Restrict, buffer, ..Default::default() },
        force_loop_bounding: true,
        ..Default::default()
    };
    let words = spv::write_vec(&module, &info, &options, None).unwrap();
    let dis = rspirv::dr::load_words(words.clone()).unwrap().disassemble();
    let umin = dis.lines().filter(|l| l.contains("OpExtInst") && l.contains("UMin")).count();
    println!("input={} buffer_policy={} words={} proofs_in_wgpu={} OpULessThan={} UMin(clamp)={} OpBranchConditional={} OpSelect={}",
        a[1].rsplit('/').next().unwrap(), a[2], words.len(), safe, dis.matches("OpULessThan").count(), umin,
        dis.matches("OpBranchConditional").count(), dis.matches("OpSelect").count());
    std::fs::write(format!("{}.{}.executed.spvasm", a[1], a[2]), dis).unwrap();
}
