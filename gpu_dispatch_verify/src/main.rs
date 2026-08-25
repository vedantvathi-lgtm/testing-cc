use std::time::Instant;
use wgpu::util::DeviceExt;

const WG_SIZE:    u32 = 256;
const N_ELEMENTS: u32 = WG_SIZE * 256; // sum = 2147418112, fits in f32

const WGSL_MANUAL: &str = r#"
struct Params { n: u32 }
@group(0) @binding(0) var<storage, read>       input:  array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<uniform>             params: Params;
var<workgroup> sdata: array<f32, 256>;
@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32,
        @builtin(workgroup_id)           wid: vec3<u32>) {
    let gid = wid.x * 256u + lid;
    sdata[lid] = select(0.0, input[gid], gid < params.n);
    workgroupBarrier();
    var stride: u32 = 128u;
    loop {
        if lid < stride { sdata[lid] += sdata[lid + stride]; }
        workgroupBarrier();
        stride >>= 1u;
        continuing { break if stride == 0u; }
    }
    if lid == 0u { output[wid.x] = sdata[0]; }
}
"#;

const WGSL_PARTIAL: &str = r#"
@group(0) @binding(0) var<storage, read>       partials: array<f32>;
@group(0) @binding(1) var<storage, read_write> output:   array<f32>;
var<workgroup> sdata: array<f32, 256>;
@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32) {
    let n = arrayLength(&partials);
    sdata[lid] = select(0.0, partials[lid], lid < n);
    workgroupBarrier();
    var stride: u32 = 128u;
    loop {
        if lid < stride { sdata[lid] += sdata[lid + stride]; }
        workgroupBarrier();
        stride >>= 1u;
        continuing { break if stride == 0u; }
    }
    if lid == 0u { output[0] = sdata[0]; }
}
"#;

fn main() { pollster::block_on(run()); }

async fn run() {
    let instance = wgpu::Instance::new(
        wgpu::InstanceDescriptor::new_without_display_handle()
    );
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })
        .await
        .expect("no adapter");

    let info = adapter.get_info();
    println!("══════════════════════════════════════════════════════");
    println!("ADAPTER: {} ({:?})", info.name, info.backend);
    println!("Driver:  {}", info.driver);
    println!("Device:  {:?}", info.device_type);
    println!("══════════════════════════════════════════════════════\n");

    let features     = adapter.features();
    let has_subgroup = features.contains(wgpu::Features::SUBGROUP);
    let sg_min       = info.subgroup_min_size;
    let sg_max       = info.subgroup_max_size;

    println!("── CLAIM 5/7: Subgroup feature detection ─────────────");
    println!("  SUBGROUP feature:  {has_subgroup}");
    println!("  subgroup_min_size: {sg_min}");
    println!("  subgroup_max_size: {sg_max}");
    if has_subgroup {
        if sg_min == sg_max {
            println!("  → Fixed-width = {sg_min}");
            let r = if sg_min == 32 { "CONFIRMED ✓" } else { "WRONG — not 32" };
            println!("  CLAIM 5 (NVIDIA fixed-32): {r}");
            println!("  CLAIM 7 (AMD variable):    N/A");
        } else {
            println!("  → Variable [{sg_min},{sg_max}]");
            println!("  CLAIM 7 (AMD variable-size): CONFIRMED ✓");
            println!("  CLAIM 5 (NVIDIA fixed-32):   N/A");
        }
    } else {
        println!("  CLAIM 5: SUBGROUP not exposed on this adapter");
    }
    println!();

    let want = if has_subgroup { wgpu::Features::SUBGROUP } else { wgpu::Features::empty() };
    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("verify"),
        required_features: want,
        required_limits: wgpu::Limits::default(),
        memory_hints: wgpu::MemoryHints::default(),
        trace: wgpu::Trace::Off,
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
    }).await.expect("device");

    let m1 = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("m1"), source: wgpu::ShaderSource::Wgsl(WGSL_MANUAL.into()),
    });
    let m2 = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("m2"), source: wgpu::ShaderSource::Wgsl(WGSL_PARTIAL.into()),
    });

    let num_groups = (N_ELEMENTS + WG_SIZE - 1) / WG_SIZE;
    let input_data: Vec<f32> = (0..N_ELEMENTS).map(|x| x as f32).collect();
    let expected: f64        = (0u64..N_ELEMENTS as u64).map(|x| x as f64).sum();

    let buf_in = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("in"), contents: bytemuck::cast_slice(&input_data),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let buf_part = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("part"), size: (num_groups * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let buf_out = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("out"), size: 4,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let buf_rb = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("rb"), size: 4,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let params: [u32; 1] = [N_ELEMENTS];
    let buf_params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("params"), contents: bytemuck::cast_slice(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let bgl1 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bgl1"), entries: &[bgl_ro(0), bgl_rw(1), bgl_uni(2)],
    });
    let bgl2 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bgl2"), entries: &[bgl_ro(0), bgl_rw(1)],
    });
    let pl1 = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("pl1"), bind_group_layouts: &[Some(&bgl1)], immediate_size: 0,
    });
    let pl2 = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("pl2"), bind_group_layouts: &[Some(&bgl2)], immediate_size: 0,
    });
    let pipe1 = mkpipe(&device, &pl1, &m1, "main", "p1");
    let pipe2 = mkpipe(&device, &pl2, &m2, "main", "p2");
    let bg1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bg1"), layout: &bgl1,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: buf_in.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: buf_part.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: buf_params.as_entire_binding() },
        ],
    });
    let bg2 = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bg2"), layout: &bgl2,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: buf_part.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: buf_out.as_entire_binding() },
        ],
    });

    println!("── Manual Reduction: Correctness ─────────────────────");
    twopass(&device, &queue, &pipe1, &bg1, num_groups, &pipe2, &bg2);
    let got = rb_f32(&device, &queue, &buf_out, &buf_rb).await;
    let err = (got as f64 - expected).abs() / expected;
    println!("  N={N_ELEMENTS}  expected={expected:.0}  got={got:.0}  err={err:.2e}");
    println!("  Result: {}", if err < 1e-3 { "PASS ✓" } else { "FAIL ✗" });
    println!();

    println!("── CLAIM 6: Timing (host-side, 50 runs, 5 warmup) ────");
    for _ in 0..5 { twopass(&device, &queue, &pipe1, &bg1, num_groups, &pipe2, &bg2); }
    let mut ts: Vec<f64> = (0..50).map(|_| {
        let t = Instant::now();
        twopass(&device, &queue, &pipe1, &bg1, num_groups, &pipe2, &bg2);
        t.elapsed().as_secs_f64() * 1e6
    }).collect();
    ts.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean   = ts.iter().sum::<f64>() / 50.0;
    let stddev = (ts.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / 50.0).sqrt();
    println!("  Median={:.1}µs  Mean={:.1}±{:.1}µs  Min={:.1}  Max={:.1}",
        ts[25], mean, stddev, ts[0], ts[49]);
    println!("  Subgroup ratio: NOT MEASURABLE (#5555 blocks WGSL frontend)");
    println!();

    println!("── CLAIM 4b: Subgroup via naga IR injection ──────────");
    match build_subgroup_spv() {
        Ok(spv) => {
            println!("  Naga IR→SPIR-V: OK ({} words)", spv.len());
            if device.features().contains(wgpu::Features::SUBGROUP) {
                let sg_mod = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("sg"), source: wgpu::ShaderSource::SpirV(std::borrow::Cow::Owned(spv)),
                });
                let bgl_sg = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("sg_bgl"), entries: &[bgl_rw(0)],
                });
                let pl_sg = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("sg_pl"), bind_group_layouts: &[Some(&bgl_sg)], immediate_size: 0,
                });
                let pipe_sg = mkpipe(&device, &pl_sg, &sg_mod, "main", "sg");
                let buf_sg = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("sg_out"), size: 4,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                });
                let buf_sg_rb = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("sg_rb"), size: 4,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let bg_sg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("sg_bg"), layout: &bgl_sg,
                    entries: &[wgpu::BindGroupEntry { binding: 0, resource: buf_sg.as_entire_binding() }],
                });
                let mut enc = device.create_command_encoder(&Default::default());
                { let mut p = enc.begin_compute_pass(&Default::default());
                  p.set_pipeline(&pipe_sg); p.set_bind_group(0, &bg_sg, &[]);
                  p.dispatch_workgroups(1, 1, 1); }
                enc.copy_buffer_to_buffer(&buf_sg, 0, &buf_sg_rb, 0, 4);
                queue.submit(std::iter::once(enc.finish()));
                let result = rb_f32(&device, &queue, &buf_sg, &buf_sg_rb).await;
                let exp_sg = (0..sg_min).map(|x| x as f32).sum::<f32>();
                println!("  Dispatch: SUCCESS");
                println!("  subgroupAdd(lid) over {sg_min} threads: expected={exp_sg:.0} got={result:.0}");
                let ok = (result - exp_sg).abs() < 2.0;
                println!("  CLAIM 4b: {}", if ok { "IR INJECTION WORKS ✓" } else { "WRONG RESULT ✗" });
            } else {
                println!("  SUBGROUP not on device — SPIR-V built OK, dispatch SKIPPED");
                println!("  CLAIM 4b: IR generation PASS; GPU dispatch N/A");
            }
        }
        Err(e) => println!("  CLAIM 4b: FAILED — {e}"),
    }

    println!("\n══════════════════════════════════════════════════════");
    println!("DONE");
}

fn build_subgroup_spv() -> Result<Vec<u32>, String> {
    use naga::*;
    use naga::ir::{MemoryDecorations, StorageAccess};

    let mut module = Module::default();
    let u32_ty = module.types.insert(Type { name: None, inner: TypeInner::Scalar(Scalar::U32) }, Span::UNDEFINED);
    let f32_ty = module.types.insert(Type { name: None, inner: TypeInner::Scalar(Scalar::F32) }, Span::UNDEFINED);
    let arr_f32 = module.types.insert(Type {
        name: None,
        inner: TypeInner::Array { base: f32_ty, size: ArraySize::Dynamic, stride: 4 },
    }, Span::UNDEFINED);
    let output_var = module.global_variables.append(GlobalVariable {
        name: Some("output".into()),
        space: AddressSpace::Storage { access: StorageAccess::LOAD | StorageAccess::STORE },
        binding: Some(ResourceBinding { group: 0, binding: 0 }),
        ty: arr_f32, init: None, memory_decorations: MemoryDecorations::empty(),
    }, Span::UNDEFINED);

    let mut exprs = Arena::new();

    // Emit group A: expressions needed before the subgroup op
    let lid     = exprs.append(Expression::FunctionArgument(0), Span::UNDEFINED); // index 0, pre-populated
    let lid_f32 = exprs.append(Expression::As { expr: lid, kind: ScalarKind::Float, convert: Some(4) }, Span::UNDEFINED); // index 1
    let emit_a  = exprs.range_from(1); // covers [1..2) = [lid_f32] only, captured before sg_res

    // sg_res: result of SubgroupCollectiveOperation — must NOT be emitted
    let sg_res  = exprs.append(Expression::SubgroupOperationResult { ty: f32_ty }, Span::UNDEFINED); // index 2

    // Emit group B: expressions needed after the subgroup op
    let b_start = exprs.len(); // = 3
    let zero_u  = exprs.append(Expression::Literal(Literal::U32(0)), Span::UNDEFINED);
    let zero_u2 = exprs.append(Expression::Literal(Literal::U32(0)), Span::UNDEFINED);
    let cond    = exprs.append(Expression::Binary { op: BinaryOperator::Equal, left: lid, right: zero_u2 }, Span::UNDEFINED);
    let gv      = exprs.append(Expression::GlobalVariable(output_var), Span::UNDEFINED);
    let ptr     = exprs.append(Expression::Access { base: gv, index: zero_u }, Span::UNDEFINED);
    let emit_b  = exprs.range_from(b_start); // covers [3..8) = [zero_u..ptr]

    let mut body = Block::new();
    body.push(Statement::Emit(emit_a), Span::UNDEFINED);  // emit lid_f32
    body.push(Statement::SubgroupCollectiveOperation {
        op: SubgroupOperation::Add,
        collective_op: CollectiveOperation::Reduce,
        argument: lid_f32, result: sg_res,
    }, Span::UNDEFINED);
    body.push(Statement::Emit(emit_b), Span::UNDEFINED);  // emit post-subgroup exprs
    let mut accept = Block::new();
    accept.push(Statement::Store { pointer: ptr, value: sg_res }, Span::UNDEFINED);
    body.push(Statement::If { condition: cond, accept, reject: Block::new() }, Span::UNDEFINED);
    body.push(Statement::Return { value: None }, Span::UNDEFINED);

    let function = Function {
        name: Some("main".into()),
        arguments: vec![FunctionArgument {
            name: Some("lid".into()), ty: u32_ty,
            binding: Some(Binding::BuiltIn(BuiltIn::LocalInvocationIndex)),
        }],
        result: None, local_variables: Arena::new(), expressions: exprs,
        named_expressions: Default::default(), body, diagnostic_filter_leaf: None,
    };
    module.entry_points.push(EntryPoint {
        name: "main".into(), stage: ShaderStage::Compute,
        early_depth_test: None, workgroup_size: [32, 1, 1], workgroup_size_overrides: None,
        function, mesh_info: None, task_payload: None, incoming_ray_payload: None,
    });

    let info = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::SUBGROUP)
        .subgroup_stages(naga::valid::ShaderStages::COMPUTE)
        .subgroup_operations(naga::valid::SubgroupOperationSet::all())
        .validate(&module)
        .map_err(|e| format!("validate: {e:?}"))?;

    naga::back::spv::write_vec(&module, &info,
        &naga::back::spv::Options { lang_version: (1, 3), ..Default::default() },
        Some(&naga::back::spv::PipelineOptions { shader_stage: ShaderStage::Compute, entry_point: "main".into() }),
    ).map_err(|e| format!("spv: {e:?}"))
}

fn twopass(device: &wgpu::Device, queue: &wgpu::Queue,
    p1: &wgpu::ComputePipeline, bg1: &wgpu::BindGroup, groups: u32,
    p2: &wgpu::ComputePipeline, bg2: &wgpu::BindGroup) {
    let mut enc = device.create_command_encoder(&Default::default());
    { let mut p = enc.begin_compute_pass(&Default::default());
      p.set_pipeline(p1); p.set_bind_group(0, bg1, &[]); p.dispatch_workgroups(groups, 1, 1); }
    { let mut p = enc.begin_compute_pass(&Default::default());
      p.set_pipeline(p2); p.set_bind_group(0, bg2, &[]); p.dispatch_workgroups(1, 1, 1); }
    queue.submit(std::iter::once(enc.finish()));
    device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).ok();
}

async fn rb_f32(device: &wgpu::Device, queue: &wgpu::Queue, src: &wgpu::Buffer, rb: &wgpu::Buffer) -> f32 {
    let mut enc = device.create_command_encoder(&Default::default());
    enc.copy_buffer_to_buffer(src, 0, rb, 0, 4);
    queue.submit(std::iter::once(enc.finish()));
    rb.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).ok();
    let view = rb.slice(..).get_mapped_range().unwrap();
    let v = bytemuck::cast_slice::<u8, f32>(&*view)[0];
    drop(view); rb.unmap(); v
}

fn mkpipe(device: &wgpu::Device, layout: &wgpu::PipelineLayout,
    module: &wgpu::ShaderModule, ep: &str, label: &str) -> wgpu::ComputePipeline {
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(label), layout: Some(layout), module,
        entry_point: Some(ep), compilation_options: Default::default(), cache: None,
    })
}

fn bgl_ro(b: u32) -> wgpu::BindGroupLayoutEntry { wgpu::BindGroupLayoutEntry { binding: b,
    visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer {
    ty: wgpu::BufferBindingType::Storage { read_only: true },
    has_dynamic_offset: false, min_binding_size: None }, count: None } }
fn bgl_rw(b: u32) -> wgpu::BindGroupLayoutEntry { wgpu::BindGroupLayoutEntry { binding: b,
    visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer {
    ty: wgpu::BufferBindingType::Storage { read_only: false },
    has_dynamic_offset: false, min_binding_size: None }, count: None } }
fn bgl_uni(b: u32) -> wgpu::BindGroupLayoutEntry { wgpu::BindGroupLayoutEntry { binding: b,
    visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer {
    ty: wgpu::BufferBindingType::Uniform,
    has_dynamic_offset: false, min_binding_size: None }, count: None } }
