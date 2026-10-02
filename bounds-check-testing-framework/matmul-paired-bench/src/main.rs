// Rigorized matmul-style GPU benchmark: fixes two methodology gaps found in
// matmul_bench.rs:
//   1. Configs were timed in BLOCKED sequence (500 trials of A, then 500 of B, ...)
//      so thermal/clock drift over ~2000 total trials could bias later configs.
//      Fix: round-robin interleave one trial per config per round.
//   2. Only point deltas of separately-computed medians were reported, no
//      confidence interval on the derived "bounds check overhead" estimator.
//      Fix: record per-round paired samples and bootstrap a 95% CI on the
//      median of (retained_checked - retained_unchecked) - (elided_checked - elided_unchecked).

use ash::vk;
use naga::back::spv;
use naga::front::wgsl;
use naga::proc::{BoundsCheckPolicies, BoundsCheckPolicy};
use naga::valid::{Capabilities, ValidationFlags, Validator};
use std::ffi::CStr;

fn compile_spv(source: &str, policy: BoundsCheckPolicy) -> Vec<u32> {
    let module = wgsl::parse_str(source).expect("WGSL parse");
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .expect("validation");
    let opts = spv::Options {
        bounds_check_policies: BoundsCheckPolicies {
            index: policy,
            buffer: policy,
            ..Default::default()
        },
        ..Default::default()
    };
    spv::write_vec(&module, &info, &opts, None).expect("SPIR-V codegen")
}

const WGSL_ELIDED: &str = r#"
var<workgroup> A: array<f32, 1024>;
var<workgroup> B: array<f32, 1024>;

@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32) {
    A[lid] = f32(lid);
    B[lid] = f32(lid);
    workgroupBarrier();

    var sum = 0.0;
    for (var k = 0u; k < 1024u; k = k + 1u) {
        sum += A[lid] * B[k];
    }

    A[lid] = sum;
}
"#;

const WGSL_RETAINED: &str = r#"
var<workgroup> A: array<f32, 1024>;
var<workgroup> B: array<f32, 1024>;

@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32) {
    A[lid] = f32(lid);
    B[lid] = f32(lid);
    workgroupBarrier();

    var sum = 0.0;
    for (var k = 0u; k < 1024u; k = k + 1u) {
        sum += A[lid] * B[k + 1u];
    }

    A[lid] = sum;
}
"#;

struct VkCtx {
    _entry: ash::Entry,
    instance: ash::Instance,
    device: ash::Device,
    _physical_device: vk::PhysicalDevice,
    _queue_family_index: u32,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
    timestamp_period: f32,
}

impl VkCtx {
    unsafe fn new() -> Self {
        let entry = ash::Entry::load().expect("ash entry");
        let app_info = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_3);
        let instance = entry
            .create_instance(&vk::InstanceCreateInfo::default().application_info(&app_info), None)
            .expect("instance");

        let phys_devices = instance.enumerate_physical_devices().expect("physical devices");
        let (physical_device, queue_family_index) = phys_devices
            .iter()
            .find_map(|&pd| {
                let props = instance.get_physical_device_properties(pd);
                let name = CStr::from_ptr(props.device_name.as_ptr());
                eprintln!("  Device: {:?}", name);
                instance
                    .get_physical_device_queue_family_properties(pd)
                    .iter()
                    .position(|qf| qf.queue_flags.contains(vk::QueueFlags::COMPUTE))
                    .map(|idx| (pd, idx as u32))
            })
            .expect("no compute-capable device");

        let phys_props = instance.get_physical_device_properties(physical_device);
        let timestamp_period = phys_props.limits.timestamp_period;

        let queue_priorities = [1.0f32];
        let queue_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family_index)
            .queue_priorities(&queue_priorities);

        let device_features = vk::PhysicalDeviceFeatures::default();
        let device = instance
            .create_device(
                physical_device,
                &vk::DeviceCreateInfo::default()
                    .queue_create_infos(std::slice::from_ref(&queue_info))
                    .enabled_features(&device_features),
                None,
            )
            .expect("device");

        let queue = device.get_device_queue(queue_family_index, 0);
        let command_pool = device
            .create_command_pool(
                &vk::CommandPoolCreateInfo::default()
                    .queue_family_index(queue_family_index)
                    .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
                None,
            )
            .expect("command pool");

        VkCtx {
            _entry: entry,
            instance,
            device,
            _physical_device: physical_device,
            _queue_family_index: queue_family_index,
            queue,
            command_pool,
            timestamp_period,
        }
    }

    unsafe fn create_shader_module(&self, code: &[u32]) -> vk::ShaderModule {
        self.device
            .create_shader_module(&vk::ShaderModuleCreateInfo::default().code(code), None)
            .expect("shader module")
    }

    unsafe fn create_compute_pipeline(&self, sm: vk::ShaderModule) -> (vk::Pipeline, vk::PipelineLayout) {
        let pl = self
            .device
            .create_pipeline_layout(&vk::PipelineLayoutCreateInfo::default(), None)
            .expect("pipeline layout");
        let stage_name = CStr::from_bytes_with_nul(b"main\0").unwrap();
        let stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(sm)
            .name(stage_name);
        let pipeline = self
            .device
            .create_compute_pipelines(
                vk::PipelineCache::null(),
                std::slice::from_ref(&vk::ComputePipelineCreateInfo::default().stage(stage).layout(pl)),
                None,
            )
            .expect("compute pipeline")[0];
        (pipeline, pl)
    }

    unsafe fn run_trial(&self, spv: &[u32]) -> f64 {
        let sm = self.create_shader_module(spv);
        let (pipeline, pl) = self.create_compute_pipeline(sm);
        let qpool = self
            .device
            .create_query_pool(
                &vk::QueryPoolCreateInfo::default()
                    .query_type(vk::QueryType::TIMESTAMP)
                    .query_count(2),
                None,
            )
            .expect("query pool");

        let cmd_buf = self
            .device
            .allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(self.command_pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1),
            )
            .expect("cmd buf")[0];

        self.device.reset_query_pool(qpool, 0, 2);
        self.device
            .begin_command_buffer(
                cmd_buf,
                &vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )
            .expect("begin cmd");
        self.device
            .cmd_write_timestamp(cmd_buf, vk::PipelineStageFlags::TOP_OF_PIPE, qpool, 0);
        self.device
            .cmd_bind_pipeline(cmd_buf, vk::PipelineBindPoint::COMPUTE, pipeline);
        self.device.cmd_dispatch(cmd_buf, 1, 1, 1);
        self.device
            .cmd_write_timestamp(cmd_buf, vk::PipelineStageFlags::BOTTOM_OF_PIPE, qpool, 1);
        self.device.end_command_buffer(cmd_buf).expect("end cmd");

        let fence = self.device.create_fence(&vk::FenceCreateInfo::default(), None).expect("fence");
        let submit_info = vk::SubmitInfo::default().command_buffers(std::slice::from_ref(&cmd_buf));
        self.device
            .queue_submit(self.queue, std::slice::from_ref(&submit_info), fence)
            .expect("submit");
        self.device
            .wait_for_fences(std::slice::from_ref(&fence), true, 5_000_000_000)
            .expect("wait");

        let mut ts = [0u64; 2];
        self.device
            .get_query_pool_results(qpool, 0, &mut ts, vk::QueryResultFlags::WAIT | vk::QueryResultFlags::TYPE_64)
            .expect("get query results");
        let delta_ns = ts[1].saturating_sub(ts[0]) as f64 * f64::from(self.timestamp_period);

        self.device.destroy_fence(fence, None);
        self.device.free_command_buffers(self.command_pool, std::slice::from_ref(&cmd_buf));
        self.device.destroy_query_pool(qpool, None);
        self.device.destroy_pipeline(pipeline, None);
        self.device.destroy_pipeline_layout(pl, None);
        self.device.destroy_shader_module(sm, None);

        delta_ns
    }

    unsafe fn destroy(&self) {
        self.device.destroy_command_pool(self.command_pool, None);
        self.device.destroy_device(None);
        self.instance.destroy_instance(None);
    }
}

fn median(data: &[f64]) -> f64 {
    let mut sorted = data.to_vec();
    sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 0 {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    }
}

fn stddev(data: &[f64]) -> f64 {
    let n = data.len() as f64;
    if n == 0.0 {
        return 0.0;
    }
    let mean = data.iter().sum::<f64>() / n;
    (data.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n).sqrt()
}

/// Tiny deterministic xorshift64* PRNG — avoids pulling in a `rand` dependency
/// for what is just bootstrap resampling.
struct Xorshift64(u64);
impl Xorshift64 {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn next_index(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// Bootstrap a 95% CI on the median of `samples` via B resamples with replacement.
fn bootstrap_median_ci(samples: &[f64], b: usize, seed: u64) -> (f64, f64, f64) {
    let mut rng = Xorshift64(seed | 1);
    let n = samples.len();
    let mut medians = Vec::with_capacity(b);
    for _ in 0..b {
        let resample: Vec<f64> = (0..n).map(|_| samples[rng.next_index(n)]).collect();
        medians.push(median(&resample));
    }
    medians.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
    let lo = medians[(0.025 * b as f64) as usize];
    let hi = medians[((0.975 * b as f64) as usize).min(b - 1)];
    (median(samples), lo, hi)
}

fn main() {
    let num_rounds: u32 = std::env::var("ROUNDS").ok().and_then(|s| s.parse().ok()).unwrap_or(500);
    let warmup_rounds: u32 = 20;

    eprintln!("=== Matmul Bounds-Check Benchmark (PAIRED / INTERLEAVED) ===");
    eprintln!("  Single dispatch per trial, workgroup_size(256), 1024-iter inner loop");
    eprintln!("  Rounds: {num_rounds} (one trial of EACH of 4 configs per round, round-robin)");

    eprintln!("\nCompiling shaders...");
    let spv_elided_checked = compile_spv(WGSL_ELIDED, BoundsCheckPolicy::ReadZeroSkipWrite);
    let spv_elided_unchecked = compile_spv(WGSL_ELIDED, BoundsCheckPolicy::Unchecked);
    let spv_retained_checked = compile_spv(WGSL_RETAINED, BoundsCheckPolicy::ReadZeroSkipWrite);
    let spv_retained_unchecked = compile_spv(WGSL_RETAINED, BoundsCheckPolicy::Unchecked);

    eprintln!("\nSPIR-V word counts:");
    eprintln!("  Elided   (checked)   {:>5} words", spv_elided_checked.len());
    eprintln!("  Elided   (unchecked) {:>5} words", spv_elided_unchecked.len());
    eprintln!("  Retained (checked)   {:>5} words", spv_retained_checked.len());
    eprintln!("  Retained (unchecked) {:>5} words", spv_retained_unchecked.len());

    let ctx = unsafe { VkCtx::new() };

    eprintln!("\nWarming up ({warmup_rounds} rounds x 4 configs)...");
    for _ in 0..warmup_rounds {
        unsafe {
            ctx.run_trial(&spv_elided_checked);
            ctx.run_trial(&spv_elided_unchecked);
            ctx.run_trial(&spv_retained_checked);
            ctx.run_trial(&spv_retained_unchecked);
        }
    }

    // Round-robin interleaved timing: one trial of each config per round, in
    // ROTATING order across rounds (round % 4 determines starting config) to
    // additionally cancel out any fixed within-round ordering bias.
    let mut t_ec = Vec::with_capacity(num_rounds as usize);
    let mut t_eu = Vec::with_capacity(num_rounds as usize);
    let mut t_rc = Vec::with_capacity(num_rounds as usize);
    let mut t_ru = Vec::with_capacity(num_rounds as usize);

    eprintln!("\nRunning {num_rounds} interleaved rounds...");
    for round in 0..num_rounds {
        let order = round % 4;
        // Rotate the 4 measurements' order each round; still measure all 4.
        let mut results = [0.0f64; 4];
        for slot in 0..4 {
            let which = (order + slot) % 4;
            let val = unsafe {
                match which {
                    0 => ctx.run_trial(&spv_elided_checked),
                    1 => ctx.run_trial(&spv_elided_unchecked),
                    2 => ctx.run_trial(&spv_retained_checked),
                    _ => ctx.run_trial(&spv_retained_unchecked),
                }
            };
            results[which as usize] = val;
        }
        t_ec.push(results[0]);
        t_eu.push(results[1]);
        t_rc.push(results[2]);
        t_ru.push(results[3]);
        if round % 100 == 0 {
            eprintln!("  round {round}/{num_rounds}");
        }
    }

    println!("\n=== PER-CONFIG SUMMARY (interleaved, {num_rounds} rounds) ===");
    println!("{:<25} {:>12} {:>12}", "Config", "Median (ns)", "StdDev (ns)");
    println!("{:-<25} {:-<12} {:-<12}", "", "", "");
    println!("{:<25} {:>12.1} {:>12.1}", "Elided   (checked)", median(&t_ec), stddev(&t_ec));
    println!("{:<25} {:>12.1} {:>12.1}", "Elided   (unchecked)", median(&t_eu), stddev(&t_eu));
    println!("{:<25} {:>12.1} {:>12.1}", "Retained (checked)", median(&t_rc), stddev(&t_rc));
    println!("{:<25} {:>12.1} {:>12.1}", "Retained (unchecked)", median(&t_ru), stddev(&t_ru));

    // Per-round paired deltas — this is the key fix: computed per-round, not
    // from separately-measured medians, so the estimator itself has a
    // sampling distribution we can bootstrap.
    let elided_overhead: Vec<f64> = t_ec.iter().zip(&t_eu).map(|(c, u)| c - u).collect();
    let retained_overhead: Vec<f64> = t_rc.iter().zip(&t_ru).map(|(c, u)| c - u).collect();
    let bounds_check_cost: Vec<f64> = retained_overhead
        .iter()
        .zip(&elided_overhead)
        .map(|(r, e)| r - e)
        .collect();

    let (m_elided_oh, lo_e, hi_e) = bootstrap_median_ci(&elided_overhead, 2000, 12345);
    let (m_retained_oh, lo_r, hi_r) = bootstrap_median_ci(&retained_overhead, 2000, 67890);
    let (m_bc_cost, lo_bc, hi_bc) = bootstrap_median_ci(&bounds_check_cost, 2000, 424242);

    println!("\n=== PAIRED DELTAS WITH BOOTSTRAP 95% CI (2000 resamples) ===");
    println!(
        "Elided:   checked - unchecked     median={:>8.1} ns   95% CI [{:.1}, {:.1}]",
        m_elided_oh, lo_e, hi_e
    );
    println!(
        "Retained: checked - unchecked     median={:>8.1} ns   95% CI [{:.1}, {:.1}]",
        m_retained_oh, lo_r, hi_r
    );
    println!(
        "Bounds check overhead (retained-elided delta of deltas):"
    );
    println!(
        "                                   median={:>8.1} ns   95% CI [{:.1}, {:.1}]",
        m_bc_cost, lo_bc, hi_bc
    );
    let ci_excludes_zero = lo_bc > 0.0 || hi_bc < 0.0;
    println!(
        "\n95% CI {} zero -> effect is {}",
        if ci_excludes_zero { "EXCLUDES" } else { "INCLUDES" },
        if ci_excludes_zero { "statistically distinguishable from noise" } else { "NOT distinguishable from zero at 95% confidence" }
    );

    unsafe { ctx.destroy(); }
    eprintln!("\nDone.");
}
