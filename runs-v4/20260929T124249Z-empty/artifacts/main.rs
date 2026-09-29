//! paired-vulkan-4: bounds-check-elision benchmark that times exactly the SPIR-V it records.
//!
//! One process = one kernel = one independent run. Per round, four slots run in a rotating
//! order: `no_bce` (wgpu's production Vulkan policy without elision), `bce` (with elision),
//! `bce_dup` (identical words, separate pipeline: in-round negative control) and `unchecked`
//! (no bounds checks: upper bound on any possible gain). Shaders are submitted with
//! `create_shader_module_passthrough`, so the driver receives these exact words; the
//! SPIR-V is emitted with the options wgpu-hal uses for Vulkan (index: Restrict,
//! buffer: Unchecked with robustBufferAccess2, force_loop_bounding, native workgroup
//! zero-init, SPIR-V 1.6). See spec-kit-work/specs/004-benchmark-harness-v4/spec.md.

mod kernels;

use kernels::Kernel;
use naga::back::spv;
use naga::proc::{BoundsCheckPolicies, BoundsCheckPolicy};
use naga::valid::{Capabilities, ValidationFlags, Validator};
use rspirv::binary::Disassemble;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const HARNESS_VERSION: &str = "paired-vulkan-4";
const SLOTS: [&str; 4] = ["no_bce", "bce", "bce_dup", "unchecked"];
const BOOTSTRAP: usize = 10_000;
const TARGET_SAMPLE_NS: f64 = 8_000_000.0;

#[derive(Serialize, Clone)]
struct Static {
    slot: String,
    bce: bool,
    index_policy: String,
    buffer_policy: String,
    spv_words: usize,
    spv_sha256: String,
    proven_accesses: usize,
    dynamic_accesses: usize,
    op_u_less_than: usize,
    umin_clamps: usize,
}

#[derive(Serialize, Clone)]
struct Round {
    round: usize,
    order: Vec<String>,
    /// ns per dispatch, indexed like SLOTS
    ns: [f64; 4],
}

#[derive(Serialize)]
struct Estimate {
    median: f64,
    ci95_lo: f64,
    ci95_hi: f64,
    /// median as a percentage of the `bce` slot median
    median_pct_of_bce: f64,
}

#[derive(Serialize)]
struct Correctness {
    slot: String,
    before_mismatches: usize,
    after_mismatches: usize,
}

fn words_for(kernel: &Kernel, bce: bool, index: BoundsCheckPolicy, buffer: BoundsCheckPolicy, slot: &str) -> (Vec<u32>, Static) {
    let module = naga::front::wgsl::parse_str(kernel.wgsl).unwrap_or_else(|e| panic!("{}: {}", kernel.name, e.emit_to_string(kernel.wgsl)));
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    validator.set_bounds_check_elision(bce);
    let info = validator.validate(&module).unwrap_or_else(|e| panic!("{}: {}", kernel.name, e.emit_to_string(kernel.wgsl)));
    let ep = &module.entry_points[0].function;
    let fi = info.get_entry_point(0);
    let dynamic = ep.expressions.iter().filter(|(_, e)| matches!(e, naga::Expression::Access { .. })).count();
    let options = spv::Options {
        lang_version: (1, 6),
        bounds_check_policies: BoundsCheckPolicies { index, buffer, ..Default::default() },
        zero_initialize_workgroup_memory: spv::ZeroInitializeWorkgroupMemoryMode::Native,
        force_loop_bounding: true,
        ..Default::default()
    };
    let pipeline = spv::PipelineOptions { shader_stage: naga::ShaderStage::Compute, entry_point: "main".into() };
    let words = spv::write_vec(&module, &info, &options, Some(&pipeline)).expect("spv");
    let dis = rspirv::dr::load_words(words.clone()).expect("valid spv").disassemble();
    let st = Static {
        slot: slot.into(),
        bce,
        index_policy: format!("{index:?}"),
        buffer_policy: format!("{buffer:?}"),
        spv_words: words.len(),
        spv_sha256: sha256(bytemuck::cast_slice(&words)),
        proven_accesses: fi.safe_accesses.len(),
        dynamic_accesses: dynamic,
        op_u_less_than: dis.matches("OpULessThan").count(),
        umin_clamps: dis.lines().filter(|l| l.contains("OpExtInst") && l.contains("UMin")).count(),
    };
    (words, st)
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipelines: Vec<wgpu::ComputePipeline>,
    bind_group: wgpu::BindGroup,
    output: wgpu::Buffer,
    readback: wgpu::Buffer,
    query_set: wgpu::QuerySet,
    query_resolve: wgpu::Buffer,
    query_readback: wgpu::Buffer,
    period: f32,
    dispatch: (u32, u32, u32),
    output_bytes: u64,
}

impl Gpu {
    fn new(kernel: &Kernel, words: &[Vec<u32>; 4], input: &[u32]) -> Result<(Self, wgpu::AdapterInfo), String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        // BCE_ADAPTER=<substring> selects an adapter by name (e.g. "Intel" for the iGPU);
        // default is the high-performance adapter.
        let adapter = match std::env::var("BCE_ADAPTER") {
            Ok(want) => pollster::block_on(instance.enumerate_adapters(wgpu::Backends::VULKAN))
                .into_iter()
                .find(|a| a.get_info().name.to_lowercase().contains(&want.to_lowercase()))
                .ok_or(format!("no adapter matching {want:?}"))?,
            Err(_) => pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            }))
            .map_err(|e| format!("adapter: {e}"))?,
        };
        let needed = wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::PASSTHROUGH_SHADERS;
        if !adapter.features().contains(needed) {
            return Err(format!("adapter lacks {:?}", needed - adapter.features()));
        }
        let info = adapter.get_info();
        let limits = wgpu::Limits { max_storage_buffer_binding_size: adapter.limits().max_storage_buffer_binding_size, ..wgpu::Limits::default() };
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("bce-v4"),
            required_features: needed,
            required_limits: limits,
            experimental_features: unsafe { wgpu::ExperimentalFeatures::enabled() },
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .map_err(|e| format!("device: {e}"))?;
        let output_bytes = (kernel.output_words * 4) as u64;
        use wgpu::util::DeviceExt;
        let input_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("input"),
            contents: bytemuck::cast_slice(input),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("output"),
            size: output_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: output_bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let entry = |binding, read_only| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only }, has_dynamic_offset: false, min_binding_size: None },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("layout"), entries: &[entry(0, true), entry(1, false)] });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bg"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: input_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: output.as_entire_binding() },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("pl"), bind_group_layouts: &[Some(&layout)], immediate_size: 0 });
        let pipelines = words
            .iter()
            .zip(SLOTS)
            .map(|(w, slot)| {
                let module = unsafe {
                    device.create_shader_module_passthrough(wgpu::ShaderModuleDescriptorPassthrough {
                        label: Some(slot),
                        entry_points: Cow::Owned(vec![wgpu::PassthroughShaderEntryPoint { name: "main".into(), workgroup_size: kernel.workgroup_size }]),
                        spirv: Some(Cow::Owned(w.clone())),
                        ..Default::default()
                    })
                };
                device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some(slot),
                    layout: Some(&pipeline_layout),
                    module: &module,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    cache: None,
                })
            })
            .collect();
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor { label: Some("ts"), ty: wgpu::QueryType::Timestamp, count: 2 });
        let query_resolve = device.create_buffer(&wgpu::BufferDescriptor { label: Some("qr"), size: 16, usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false });
        let query_readback = device.create_buffer(&wgpu::BufferDescriptor { label: Some("qrb"), size: 16, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
        let period = queue.get_timestamp_period();
        Ok((Gpu { device, queue, pipelines, bind_group, output, readback, query_set, query_resolve, query_readback, period, dispatch: kernel.dispatch, output_bytes }, info))
    }

    fn time(&self, slot: usize, dispatches: u32) -> f64 {
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites { query_set: &self.query_set, beginning_of_pass_write_index: Some(0), end_of_pass_write_index: Some(1) }),
            });
            pass.set_pipeline(&self.pipelines[slot]);
            pass.set_bind_group(0, &self.bind_group, &[]);
            for _ in 0..dispatches {
                pass.dispatch_workgroups(self.dispatch.0, self.dispatch.1, self.dispatch.2);
            }
        }
        enc.resolve_query_set(&self.query_set, 0..2, &self.query_resolve, 0);
        enc.copy_buffer_to_buffer(&self.query_resolve, 0, &self.query_readback, 0, 16);
        self.queue.submit([enc.finish()]);
        let slice = self.query_readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| ());
        self.device.poll(wgpu::PollType::wait_indefinitely()).expect("poll");
        let ticks = {
            let m = slice.get_mapped_range().expect("map");
            let t: &[u64] = bytemuck::cast_slice(&m);
            t[1].checked_sub(t[0]).expect("non-monotonic timestamps")
        };
        self.query_readback.unmap();
        assert!(ticks > 0);
        ticks as f64 * f64::from(self.period) / f64::from(dispatches)
    }

    fn run_and_read(&self, slot: usize) -> Vec<u32> {
        let mut enc = self.device.create_command_encoder(&Default::default());
        enc.clear_buffer(&self.output, 0, None);
        {
            let mut pass = enc.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipelines[slot]);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(self.dispatch.0, self.dispatch.1, self.dispatch.2);
        }
        enc.copy_buffer_to_buffer(&self.output, 0, &self.readback, 0, self.output_bytes);
        self.queue.submit([enc.finish()]);
        let slice = self.readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| ());
        self.device.poll(wgpu::PollType::wait_indefinitely()).expect("poll");
        let out = bytemuck::cast_slice::<u8, u32>(&slice.get_mapped_range().expect("map")).to_vec();
        self.readback.unmap();
        out
    }
}

fn median(v: &[f64]) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    let n = s.len();
    if n % 2 == 1 { s[n / 2] } else { (s[n / 2 - 1] + s[n / 2]) / 2.0 }
}

fn bootstrap(values: &[f64], seed: u64, bce_median: f64) -> Estimate {
    let mut x = seed | 1;
    let mut next = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let mut meds: Vec<f64> = (0..BOOTSTRAP)
        .map(|_| median(&(0..values.len()).map(|_| values[next() as usize % values.len()]).collect::<Vec<_>>()))
        .collect();
    meds.sort_by(f64::total_cmp);
    let m = median(values);
    Estimate { median: m, ci95_lo: meds[BOOTSTRAP * 25 / 1000], ci95_hi: meds[BOOTSTRAP * 975 / 1000], median_pct_of_bce: 100.0 * m / bce_median }
}

fn sha256(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}

fn sh(program: &str, args: &[&str]) -> String {
    Command::new(program).args(args).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default()
}

fn read(path: &str) -> String {
    std::fs::read_to_string(path).map(|s| s.trim().to_string()).unwrap_or_default()
}

fn git(path: &Path, args: &[&str]) -> String {
    Command::new("git").args(args).current_dir(path).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default()
}

/// sha256 of the raw `git diff HEAD -- naga/src` bytes (same as `git diff ... | sha256sum`).
fn naga_diff_sha(wgpu: &Path) -> String {
    let out = Command::new("git").args(["diff", "HEAD", "--", "naga/src"]).current_dir(wgpu).output().map(|o| o.stdout).unwrap_or_default();
    sha256(&out)
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn main() {
    let kernel_name = std::env::args().nth(1).expect("usage: bce-paired-vulkan-v4 <kernel>|--list");
    let kernels = kernels::all();
    if kernel_name == "--list" {
        for k in &kernels {
            println!("{}", k.name);
        }
        return;
    }
    let kernel = kernels.into_iter().find(|k| k.name == kernel_name).expect("unknown kernel");
    let warmup = env_usize("BCE_WARMUP_ROUNDS", 30);
    let rounds = env_usize("BCE_ROUNDS", 120);

    let utc_start = sh("date", &["-u", "+%Y%m%dT%H%M%SZ"]);
    let framework = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    let project = framework.parent().unwrap().to_path_buf();
    let adapter_tag = std::env::var("BCE_ADAPTER").map(|a| format!("-{}", a.to_lowercase())).unwrap_or_default();
    let run_dir = framework.join("runs-v4").join(format!("{utc_start}-{}{adapter_tag}", kernel.name));
    let art = run_dir.join("artifacts");
    std::fs::create_dir_all(&art).unwrap();

    // Provenance gate: the binary must have been built from the source it reports.
    // run_campaign.sh builds with BCE_BUILD_WGPU_REV / BCE_BUILD_NAGA_DIFF_SHA set.
    let runtime_rev = git(&project.join("wgpu"), &["rev-parse", "HEAD"]);
    let runtime_diff = naga_diff_sha(&project.join("wgpu"));
    let build_rev = option_env!("BCE_BUILD_WGPU_REV").unwrap_or("unknown");
    let build_diff = option_env!("BCE_BUILD_NAGA_DIFF_SHA").unwrap_or("unknown");
    if build_rev != runtime_rev || build_diff != runtime_diff {
        std::fs::write(
            run_dir.join("environment-blocked.txt"),
            format!("provenance mismatch: built from wgpu {build_rev} (naga/src diff {build_diff}), source now {runtime_rev} (diff {runtime_diff}); rebuild with run_campaign.sh\n"),
        )
        .unwrap();
        eprintln!("refusing to run: binary was not built from the current wgpu source");
        std::process::exit(4);
    }

    // Noise gate (benchmarking notes §11): no other GPU compute work may be running.
    let compute_apps = sh("nvidia-smi", &["--query-compute-apps=pid,process_name", "--format=csv,noheader"]);
    if !compute_apps.is_empty() {
        std::fs::write(run_dir.join("environment-blocked.txt"), format!("other GPU compute apps present:\n{compute_apps}\n")).unwrap();
        eprintln!("refusing to run: other GPU compute apps present");
        std::process::exit(3);
    }

    // Deterministic input.
    let mut x: u32 = 0x9E37_79B9;
    let input: Vec<u32> = (0..kernel.input_words)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x
        })
        .collect();
    let expected = (kernel.reference)(&input);

    let (index, buffer) = (BoundsCheckPolicy::Restrict, BoundsCheckPolicy::Unchecked);
    let (w_no, s_no) = words_for(&kernel, false, index, buffer, "no_bce");
    let (w_bce, s_bce) = words_for(&kernel, true, index, buffer, "bce");
    let (w_dup, mut s_dup) = words_for(&kernel, true, index, buffer, "bce_dup");
    s_dup.slot = "bce_dup".into();
    let (w_un, s_un) = words_for(&kernel, false, BoundsCheckPolicy::Unchecked, BoundsCheckPolicy::Unchecked, "unchecked");
    assert_eq!(w_bce, w_dup, "bce and bce_dup must be identical words");
    let statics = vec![s_no.clone(), s_bce.clone(), s_dup, s_un];
    for (w, slot) in [&w_no, &w_bce, &w_dup, &w_un].iter().zip(SLOTS) {
        std::fs::write(art.join(format!("{slot}.spv")), bytemuck::cast_slice::<u32, u8>(w)).unwrap();
    }
    std::fs::write(art.join("kernel.wgsl"), kernel.wgsl).unwrap();
    std::fs::write(art.join("main.rs"), include_str!("main.rs")).unwrap();
    std::fs::write(art.join("kernels.rs"), include_str!("kernels.rs")).unwrap();
    let bce_changes_binary = w_no != w_bce;

    // Continuous GPU telemetry for the whole run (notes §7, audit BR-13).
    let telemetry_file = std::fs::File::create(art.join("telemetry.csv")).unwrap();
    let mut telemetry = Command::new("nvidia-smi")
        .args(["--query-gpu=timestamp,pstate,clocks.gr,clocks.mem,temperature.gpu,power.draw,utilization.gpu", "--format=csv,noheader", "-lms", "200"])
        .stdout(Stdio::from(telemetry_file))
        .spawn()
        .ok();

    let words = [w_no, w_bce, w_dup, w_un];
    let (gpu, adapter) = match Gpu::new(&kernel, &words, &input) {
        Ok(v) => v,
        Err(e) => {
            if let Some(t) = telemetry.as_mut() {
                let _ = t.kill();
            }
            std::fs::write(run_dir.join("environment-blocked.txt"), e.clone()).unwrap();
            eprintln!("environment blocked: {e}");
            std::process::exit(3);
        }
    };

    let check = |slot: usize| gpu.run_and_read(slot).iter().zip(&expected).filter(|(a, b)| a != b).count();
    let before: Vec<usize> = (0..4).map(check).collect();

    let probe = gpu.time(0, 3);
    let dispatches = (TARGET_SAMPLE_NS / probe).round().clamp(1.0, 4000.0) as u32;

    // Warm-up until steady state (notes §7): >= `warmup` rounds AND >= 5 s AND the last two
    // 10-round windows of the no_bce slot within 0.5%; capped at 60 s.
    let mut warm = Vec::new();
    let warm_start = std::time::Instant::now();
    let mut r = 0;
    loop {
        for j in 0..4 {
            let slot = (r + j) % 4;
            let t = gpu.time(slot, dispatches);
            if slot == 0 {
                warm.push(t);
            }
        }
        r += 1;
        let elapsed = warm_start.elapsed().as_secs_f64();
        let n = warm.len();
        let settled = n >= 20 && {
            let (a, b) = (median(&warm[n - 20..n - 10]), median(&warm[n - 10..]));
            ((b - a) / a).abs() < 0.005
        };
        if (r >= warmup && elapsed >= 5.0 && settled) || elapsed >= 60.0 {
            break;
        }
    }
    let warmup_rounds_done = r;
    let warmup_seconds = warm_start.elapsed().as_secs_f64();
    let measure_start_unix_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
    let mut data = Vec::with_capacity(rounds);
    for r in 0..rounds {
        let order: Vec<usize> = (0..4).map(|j| (r + j) % 4).collect();
        let mut ns = [0.0; 4];
        for &slot in &order {
            ns[slot] = gpu.time(slot, dispatches);
        }
        data.push(Round { round: r, order: order.iter().map(|&s| SLOTS[s].to_string()).collect(), ns });
    }
    let measure_end_unix_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
    // Noise gate, second half: another GPU compute process that appeared *during*
    // the measurement invalidates the run (our own process is excluded).
    let own_pid = std::process::id().to_string();
    let compute_apps_after: Vec<String> = sh("nvidia-smi", &["--query-compute-apps=pid,process_name", "--format=csv,noheader"])
        .lines()
        .filter(|line| !line.trim().is_empty() && line.split(',').next().map(str::trim) != Some(own_pid.as_str()))
        .map(str::to_string)
        .collect();
    let quiet_during_run = compute_apps_after.is_empty();
    let after: Vec<usize> = (0..4).map(check).collect();
    if let Some(t) = telemetry.as_mut() {
        let _ = t.kill();
    }

    let col = |s: usize| data.iter().map(|r| r.ns[s]).collect::<Vec<_>>();
    let diff = |a: usize, b: usize| data.iter().map(|r| r.ns[a] - r.ns[b]).collect::<Vec<_>>();
    let bce_med = median(&col(1));
    let medians: Vec<f64> = (0..4).map(|s| median(&col(s))).collect();
    let ratio = |a: usize, b: usize| data.iter().map(|r| r.ns[a] / r.ns[b]).collect::<Vec<_>>();
    // Per-round ratios cancel clock-state shifts that hit all four slots of a round.
    let effect_ratio = bootstrap(&ratio(0, 1), 0x5A71, 1.0);
    let control_ratio = bootstrap(&ratio(2, 1), 0x5A72, 1.0);
    let residual_ratio = bootstrap(&ratio(1, 3), 0x5A73, 1.0);
    let all_checks_ratio = bootstrap(&ratio(0, 3), 0x5A74, 1.0);
    let effect = bootstrap(&diff(0, 1), 0xB0CE, bce_med);
    let control = bootstrap(&diff(2, 1), 0xC0C0, bce_med);
    let unchecked_gap = bootstrap(&diff(1, 3), 0x0C0C, bce_med);
    let checks_total = bootstrap(&diff(0, 3), 0xABCD, bce_med);
    // Order effect: effect split by whether no_bce ran before bce inside the round.
    let pos = |r: &Round, s: &str| r.order.iter().position(|o| o == s).unwrap();
    let (mut first, mut second) = (Vec::new(), Vec::new());
    for r in &data {
        let d = r.ns[0] / r.ns[1];
        if pos(r, "no_bce") < pos(r, "bce") { first.push(d) } else { second.push(d) }
    }
    let n = warm.len();
    let warmup_last_window_change_pct = if n >= 20 {
        100.0 * (median(&warm[n - 10..]) - median(&warm[n - 20..n - 10])) / median(&warm[n - 20..n - 10])
    } else {
        f64::NAN
    };
    let correctness: Vec<Correctness> = (0..4).map(|s| Correctness { slot: SLOTS[s].into(), before_mismatches: before[s], after_mismatches: after[s] }).collect();
    let correct = before.iter().chain(&after).all(|&m| m == 0);
    let control_ok = control_ratio.ci95_lo <= 1.0 && control_ratio.ci95_hi >= 1.0;

    let meta = serde_json::json!({
        "schema": "bce-meta-2",
        "harness_version": HARNESS_VERSION,
        "utc_start": utc_start,
        "utc_end": sh("date", &["-u", "+%Y%m%dT%H%M%SZ"]),
        "sources": {
            "wgpu": {"rev": git(&project.join("wgpu"), &["rev-parse", "HEAD"]), "branch": git(&project.join("wgpu"), &["branch", "--show-current"]),
                      "dirty_files": git(&project.join("wgpu"), &["status", "--porcelain", "--", "naga/src", "wgpu", "wgpu-core", "wgpu-hal"]),
                      "naga_src_diff_sha256": runtime_diff},
            "framework": {"rev": git(&framework, &["rev-parse", "HEAD"]), "dirty": !git(&framework, &["status", "--porcelain"]).is_empty()},
            "rustc": sh("rustc", &["-V"]), "cargo": sh("cargo", &["-V"]),
            "build": {"wgpu_rev": build_rev, "naga_src_diff_sha256": build_diff},
        },
        "machine": {
            "adapter": adapter.name, "backend": format!("{:?}", adapter.backend), "driver": adapter.driver, "driver_info": adapter.driver_info,
            "timestamp_period_ns": gpu.period,
            "cpu_model": sh("sh", &["-c", "grep -m1 'model name' /proc/cpuinfo | cut -d: -f2"]),
            "cpu_governor": read("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor"),
            "mem_total": sh("sh", &["-c", "grep MemTotal /proc/meminfo"]),
            "ac_online": read("/sys/class/power_supply/ADP0/online"),
            "kernel": sh("uname", &["-srvm"]),
            "loadavg": read("/proc/loadavg"),
            "gpu_display_active": sh("nvidia-smi", &["--query-gpu=display_active", "--format=csv,noheader"]),
            "gpu_compute_apps_before": compute_apps,
        },
    });
    std::fs::write(run_dir.join("meta.json"), serde_json::to_vec_pretty(&meta).unwrap()).unwrap();

    let result = serde_json::json!({
        "schema": "bce-result-4",
        "harness_version": HARNESS_VERSION,
        "kernel": kernel.name,
        "models": kernel.models,
        "dispatch": kernel.dispatch,
        "workgroup_size": kernel.workgroup_size,
        "dispatches_per_sample": dispatches,
        "warmup_rounds": warmup,
        "rounds": rounds,
        "slots": SLOTS,
        "statics": statics,
        "bce_changes_binary": bce_changes_binary,
        "median_ns_per_dispatch": {"no_bce": medians[0], "bce": medians[1], "bce_dup": medians[2], "unchecked": medians[3]},
        "ratio_no_bce_over_bce": effect_ratio,
        "ratio_control_bce_dup_over_bce": control_ratio,
        "ratio_residual_bce_over_unchecked": residual_ratio,
        "ratio_all_checks_no_bce_over_unchecked": all_checks_ratio,
        "effect_no_bce_minus_bce": effect,
        "control_bce_dup_minus_bce": control,
        "residual_bce_minus_unchecked": unchecked_gap,
        "all_checks_no_bce_minus_unchecked": checks_total,
        "order_effect_ratio": {"no_bce_first_median": median(&first), "bce_first_median": median(&second), "n_first": first.len(), "n_second": second.len()},
        "warmup_rounds_done": warmup_rounds_done,
        "warmup_seconds": warmup_seconds,
        "warmup_last_window_change_pct": warmup_last_window_change_pct,
        "measure_window_unix_ms": [measure_start_unix_ms, measure_end_unix_ms],
        "correctness": correctness,
        "gpu_compute_apps_after": compute_apps_after,
        "quiet_during_run": quiet_during_run,
        "valid": correct && control_ok && quiet_during_run,
        "rounds_raw": data,
    });
    std::fs::write(run_dir.join("result.json"), serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    println!(
        "{} valid={} bce_changes_binary={} bce={:.0}ns no_bce/bce={:.4} [{:.4},{:.4}] control={:.4} [{:.4},{:.4}] bce/unchecked={:.4} warmup={}r/{:.1}s -> {}",
        kernel.name, correct && control_ok && quiet_during_run, bce_changes_binary, medians[1], effect_ratio.median, effect_ratio.ci95_lo, effect_ratio.ci95_hi,
        control_ratio.median, control_ratio.ci95_lo, control_ratio.ci95_hi, residual_ratio.median, warmup_rounds_done, warmup_seconds, run_dir.display()
    );
}
