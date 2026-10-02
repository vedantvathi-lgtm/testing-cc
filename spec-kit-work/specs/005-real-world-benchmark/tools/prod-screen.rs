//! Production-binary screen: which real-world entry points get a DIFFERENT SPIR-V binary when
//! bounds-check elision is on, under wgpu-hal's Vulkan options (index Restrict, buffer Unchecked
//! [robustBufferAccess2 case], image_load Unchecked, force_loop_bounding, SPIR-V 1.6, native zero-init).
//! Usage: bce-prod-screen <corpus> <file.wgsl>...   → one JSON line per entry point on stdout.
use naga::back::spv;
use naga::proc::{BoundsCheckPolicies, BoundsCheckPolicy};
use naga::valid::{Capabilities, ValidationFlags, Validator};
use sha2::{Digest, Sha256};

const GLSL_STD_450_UMIN: u32 = 38;
const OP_EXT_INST: u32 = 12;

fn umin_count(words: &[u32]) -> usize {
    let mut i = 5; // skip header
    let mut n = 0;
    while i < words.len() {
        let count = (words[i] >> 16) as usize;
        let op = words[i] & 0xffff;
        if op == OP_EXT_INST && count >= 5 && words[i + 4] == GLSL_STD_450_UMIN {
            n += 1;
        }
        i += count.max(1);
    }
    n
}

fn words(module: &naga::Module, elision: bool, ep: &naga::EntryPoint) -> Result<Vec<u32>, String> {
    let mut v = Validator::new(ValidationFlags::all(), Capabilities::all());
    v.set_bounds_check_elision(elision);
    let info = v.validate(module).map_err(|e| format!("validate: {e}"))?;
    let options = spv::Options {
        lang_version: (1, 6),
        bounds_check_policies: BoundsCheckPolicies {
            index: BoundsCheckPolicy::Restrict,
            buffer: BoundsCheckPolicy::Unchecked,
            image_load: BoundsCheckPolicy::Unchecked,
            binding_array: BoundsCheckPolicy::Unchecked,
        },
        zero_initialize_workgroup_memory: spv::ZeroInitializeWorkgroupMemoryMode::Native,
        force_loop_bounding: true,
        ..Default::default()
    };
    let po = spv::PipelineOptions { shader_stage: ep.stage, entry_point: ep.name.clone() };
    spv::write_vec(module, &info, &options, Some(&po)).map_err(|e| format!("spv: {e}"))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let corpus = &args[1];
    for path in &args[2..] {
        let src = match std::fs::read_to_string(path) { Ok(s) => s, Err(e) => { eprintln!("READ_FAIL {path} {e}"); continue } };
        let module = match naga::front::wgsl::parse_str(&src) { Ok(m) => m, Err(e) => { eprintln!("PARSE_FAIL {path} {}", e.emit_to_string(&src).lines().next().unwrap_or("")); continue } };
        for ep in &module.entry_points {
            let (off, on) = match (words(&module, false, ep), words(&module, true, ep)) {
                (Ok(a), Ok(b)) => (a, b),
                (Err(e), _) | (_, Err(e)) => { eprintln!("COMPILE_FAIL {path} {} {e}", ep.name); continue }
            };
            let h = |w: &[u32]| { let b: Vec<u8> = w.iter().flat_map(|x| x.to_le_bytes()).collect(); format!("{:x}", Sha256::digest(&b)) };
            println!("{}", serde_json::json!({
                "corpus": corpus, "file": path, "entry": ep.name, "stage": format!("{:?}", ep.stage),
                "workgroup_size": ep.workgroup_size,
                "umin_off": umin_count(&off), "umin_on": umin_count(&on),
                "words_off": off.len(), "words_on": on.len(),
                "binary_changes": h(&off) != h(&on),
            }));
        }
    }
}
