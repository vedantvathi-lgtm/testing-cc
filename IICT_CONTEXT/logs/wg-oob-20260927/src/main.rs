//! wg-oob: does Naga's own bounds check protect workgroup memory?
//! See PLAN.md (predictions written before running).

use naga::back::spv;
use naga::proc::{BoundsCheckPolicies, BoundsCheckPolicy};
use naga::valid::{Capabilities, ValidationFlags, Validator};
use rspirv::binary::Disassemble;
use sha2::{Digest, Sha256};
use std::borrow::Cow;

const WGSL: &str = r#"
@group(0) @binding(0) var<storage, read_write> out: array<u32, 24>;
@group(0) @binding(1) var<storage, read> params: array<u32, 4>;
var<workgroup> a: array<u32, 8>;
var<workgroup> b: array<u32, 8>;

@compute @workgroup_size(8)
fn main(@builtin(local_invocation_index) lid: u32) {
    a[lid] = 100u + lid;
    b[lid] = 200u + lid;
    workgroupBarrier();
    let off = params[0];
    if (lid == 0u) {
        a[off] = 999u;              // OOB write when off = 8
    }
    workgroupBarrier();
    out[lid] = a[lid + off];        // OOB reads a[8..15]
    out[8u + lid] = a[lid];
    out[16u + lid] = b[lid];
}
"#;

const OFF: u32 = 8;
const OUT_WORDS: usize = 24;
const REPEATS: usize = 5;

struct Compiled {
    words: Vec<u32>,
    uless: usize,
    umin: usize,
    safe: usize,
    sha: String,
}

fn count(words: &[u32]) -> (usize, usize) {
    let text = rspirv::dr::load_words(words.to_vec()).unwrap().disassemble();
    (text.matches("OpULessThan").count(), text.matches("UMin").count())
}

fn compile(policy: BoundsCheckPolicy, elision: bool) -> Compiled {
    let module = naga::front::wgsl::parse_str(WGSL).expect("parse");
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    validator.set_bounds_check_elision(elision);
    let info = validator.validate(&module).expect("validate");
    let safe = info.get_entry_point(0).safe_accesses.len();
    let options = spv::Options {
        bounds_check_policies: BoundsCheckPolicies {
            index: policy,
            // Buffer accesses here are all in-bounds; keep them fixed across arms.
            buffer: BoundsCheckPolicy::ReadZeroSkipWrite,
            ..Default::default()
        },
        ..Default::default()
    };
    let pipeline = spv::PipelineOptions {
        shader_stage: naga::ShaderStage::Compute,
        entry_point: "main".into(),
    };
    let words = spv::write_vec(&module, &info, &options, Some(&pipeline)).expect("spv");
    let (uless, umin) = count(&words);
    let sha = format!("{:x}", Sha256::digest(bytemuck::cast_slice(&words)));
    Compiled { words, uless, umin, safe, sha }
}

/// Replicates what wgpu-core + wgpu-hal(Vulkan) do with ShaderSource::SpirV:
/// naga spv-in -> validate (fork default: elision on) -> spv-out with hal's index policy.
fn relower(words: &[u32], index: BoundsCheckPolicy) -> (usize, usize) {
    let module = naga::front::spv::Frontend::new(words.iter().cloned(), &Default::default())
        .parse()
        .expect("spv-in");
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .expect("revalidate");
    let options = spv::Options {
        bounds_check_policies: BoundsCheckPolicies {
            index,
            buffer: BoundsCheckPolicy::Unchecked,
            ..Default::default()
        },
        ..Default::default()
    };
    let pipeline = spv::PipelineOptions {
        shader_stage: naga::ShaderStage::Compute,
        entry_point: "main".into(),
    };
    count(&spv::write_vec(&module, &info, &options, Some(&pipeline)).expect("re-spv"))
}

struct Ctx {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

fn device_new() -> (Ctx, wgpu::AdapterInfo) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
        ..Default::default()
    }))
    .expect("adapter");
    let info = adapter.get_info();
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("wg-oob"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::Performance,
        trace: wgpu::Trace::Off,
    }))
    .expect("device");
    (Ctx { device, queue }, info)
}

fn run(ctx: &Ctx, words: &[u32], trusted_unchecked: bool) -> Vec<u32> {
    let device = &ctx.device;
    let desc = wgpu::ShaderModuleDescriptor {
        label: Some("wg-oob"),
        source: wgpu::ShaderSource::SpirV(Cow::Owned(words.to_vec())),
    };
    let module = if trusted_unchecked {
        // SAFETY: test-only; the point is to execute exactly the checks Naga emitted.
        unsafe { device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::unchecked()) }
    } else {
        device.create_shader_module(desc)
    };
    let out_bytes = (OUT_WORDS * 4) as u64;
    let out = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("out"),
        size: out_bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let params = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("params"),
        size: 16,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    ctx.queue.write_buffer(&params, 0, bytemuck::cast_slice(&[OFF, 0, 0, 0]));
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: out_bytes,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let entry = |binding, read_only| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[entry(0, false), entry(1, true)],
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: out.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: params.as_entire_binding() },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&out, 0, &readback, 0, out_bytes);
    ctx.queue.submit([encoder.finish()]);
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| ());
    device.poll(wgpu::PollType::wait_indefinitely()).expect("poll");
    let words: Vec<u32> = bytemuck::cast_slice(&slice.get_mapped_range().expect("map")).to_vec();
    readback.unmap();
    words
}

fn main() {
    let files: Vec<String> = std::env::args().skip(1).collect();
    if !files.is_empty() {
        // Replay mode: what does wgpu actually execute for a saved .spv?
        for path in files {
            let bytes = std::fs::read(&path).expect("read spv");
            let words: Vec<u32> = bytemuck::cast_slice(&bytes).to_vec();
            let (u, m) = count(&words);
            let (ru, rm) = relower(&words, BoundsCheckPolicy::Restrict);
            println!("{path}\n   as saved: ULessThan={u} UMin={m} | after wgpu re-lowering (index=Restrict): ULessThan={ru} UMin={rm}");
        }
        return;
    }
    let (ctx, info) = device_new();
    println!("adapter: {} | driver: {} {}", info.name, info.driver, info.driver_info);
    println!("wgpu fork HEAD: see git; OFF={OFF}; repeats per arm={REPEATS}\n");

    use BoundsCheckPolicy::*;
    let arms: [(&str, BoundsCheckPolicy, bool, bool); 5] = [
        ("A Unchecked            (trusted)", Unchecked, false, true),
        ("B RZSW, elision off    (trusted)", ReadZeroSkipWrite, false, true),
        ("C Restrict, elision off(trusted)", Restrict, false, true),
        ("D RZSW, elision on     (trusted)", ReadZeroSkipWrite, true, true),
        ("E Unchecked SPIR-V, plain create_shader_module", Unchecked, false, false),
    ];
    for (name, policy, elision, trusted) in arms {
        let c = compile(policy, elision);
        let (re_u_unc, re_min_unc) = relower(&c.words, Unchecked);
        let (re_u_res, re_min_res) = relower(&c.words, Restrict);
        println!("== {name}");
        println!(
            "   naga out: OpULessThan={} UMin={} safe_accesses={} words={} sha256={}",
            c.uless, c.umin, c.safe, c.words.len(), &c.sha[..16]
        );
        println!(
            "   after wgpu re-lowering: hal index=Unchecked -> ULessThan={re_u_unc} UMin={re_min_unc} | hal index=Restrict -> ULessThan={re_u_res} UMin={re_min_res}"
        );
        let mut runs = Vec::new();
        for _ in 0..REPEATS {
            runs.push(run(&ctx, &c.words, trusted));
        }
        let stable = runs.iter().all(|r| r == &runs[0]);
        let r = &runs[0];
        println!("   out[0..8]  a[lid+off] (OOB read) = {:?}", &r[0..8]);
        println!("   out[8..16] a[lid]                = {:?}", &r[8..16]);
        println!("   out[16..24] b[lid]               = {:?}", &r[16..24]);
        println!("   deterministic across {REPEATS} runs: {stable}\n");
    }
}
