use naga::back::spv;
use naga::proc::{BoundsCheckPolicies, BoundsCheckPolicy};
use naga::valid::{Capabilities, ValidationFlags, Validator};
use rspirv::binary::Disassemble;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::process::Command;

const HARNESS_VERSION: &str = "paired-vulkan-3";
const ARRAY_LEN: usize = 256;
const OUTPUT_BYTES: u64 = (ARRAY_LEN * size_of::<f32>()) as u64;
const POLICY: BoundsCheckPolicy = BoundsCheckPolicy::ReadZeroSkipWrite;
const BOOTSTRAP_SAMPLES: usize = 10_000;
const DEFAULT_ROUNDS: usize = 300;
const DEFAULT_CONTROL_ROUNDS: usize = 200;
const DEFAULT_WARMUP_ROUNDS: usize = 50;

const SHADER: &str = r#"
@group(0) @binding(0) var<storage, read_write> output: array<f32, 256>;
var<workgroup> tile: array<f32, 256>;

@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32) {
    tile[lid] = f32(lid + 1u);
    workgroupBarrier();

    var sum = 0.0;
    for (var k = 0u; k < 256u; k = k + 1u) {
        sum += tile[k];
    }
    output[lid] = sum;
}
"#;

#[derive(Clone, Serialize)]
struct SourceState {
    path: String,
    revision: String,
    branch: String,
    dirty: bool,
    diff_sha256: String,
}

#[derive(Serialize)]
struct StaticArtifact {
    bce_enabled: bool,
    spv_words: usize,
    spv_sha256: String,
    safe_accesses: usize,
    dynamic_accesses: usize,
    op_uless_than_total: usize,
    emitted_checks_by_unchecked_delta: usize,
}

#[derive(Clone, Serialize)]
struct PairedSample {
    round: usize,
    order: String,
    first_ns_per_dispatch: f64,
    second_ns_per_dispatch: f64,
    enabled_ns_per_dispatch: f64,
    disabled_ns_per_dispatch: f64,
    disabled_minus_enabled_ns: f64,
}

#[derive(Clone, Serialize)]
struct ControlSample {
    round: usize,
    order: String,
    a_ns_per_dispatch: f64,
    b_ns_per_dispatch: f64,
    a_minus_b_ns: f64,
}

#[derive(Serialize)]
struct Estimate {
    median: f64,
    ci95_lo: f64,
    ci95_hi: f64,
    bootstrap_samples: usize,
    seed: u64,
}

#[derive(Serialize)]
struct CorrectnessResult {
    expected: f32,
    enabled_max_abs_error: f32,
    disabled_max_abs_error: f32,
    passed: bool,
}

#[derive(Serialize)]
struct Report {
    schema_version: String,
    harness_version: String,
    utc_start: String,
    utc_end: String,
    sources: Vec<SourceState>,
    machine: Machine,
    shader_sha256: String,
    bounds_policy: String,
    dispatches_per_sample: u32,
    warmup_rounds: usize,
    headline_rounds: usize,
    control_rounds: usize,
    static_artifacts: Vec<StaticArtifact>,
    correctness: CorrectnessResult,
    negative_control: Estimate,
    negative_control_includes_zero: bool,
    headline: Estimate,
    valid_for_claims: bool,
    control_samples: Vec<ControlSample>,
    paired_samples: Vec<PairedSample>,
}

#[derive(Serialize)]
struct Machine {
    adapter: String,
    backend: String,
    driver: String,
    driver_info: String,
    timestamp_period_ns: f32,
    os: String,
    architecture: String,
    kernel: String,
    nvidia_smi_before: String,
    nvidia_smi_after: String,
}

struct Compiled {
    words: Vec<u32>,
    static_artifact: StaticArtifact,
}

fn compile(bce_enabled: bool, unchecked_uless: usize) -> Compiled {
    let module = naga::front::wgsl::parse_str(SHADER).expect("WGSL parse");
    let dynamic_accesses = module.entry_points[0]
        .function
        .expressions
        .iter()
        .filter(|(_, expression)| matches!(expression, naga::Expression::Access { .. }))
        .count();
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    validator.set_bounds_check_elision(bce_enabled);
    let info = validator.validate(&module).expect("validation");
    let safe_accesses = info.get_entry_point(0).safe_accesses.len();
    let options = spv::Options {
        bounds_check_policies: BoundsCheckPolicies {
            index: POLICY,
            buffer: POLICY,
            ..Default::default()
        },
        ..Default::default()
    };
    let pipeline_options = spv::PipelineOptions {
        shader_stage: naga::ShaderStage::Compute,
        entry_point: "main".into(),
    };
    let words =
        spv::write_vec(&module, &info, &options, Some(&pipeline_options)).expect("SPIR-V codegen");
    let disassembly = rspirv::dr::load_words(words.clone())
        .expect("valid SPIR-V")
        .disassemble();
    let op_uless_than_total = disassembly.matches("OpULessThan").count();
    Compiled {
        static_artifact: StaticArtifact {
            bce_enabled,
            spv_words: words.len(),
            spv_sha256: sha256(bytemuck::cast_slice(&words)),
            safe_accesses,
            dynamic_accesses,
            op_uless_than_total,
            emitted_checks_by_unchecked_delta: op_uless_than_total.saturating_sub(unchecked_uless),
        },
        words,
    }
}

fn unchecked_uless_count() -> usize {
    let module = naga::front::wgsl::parse_str(SHADER).expect("WGSL parse");
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    validator.set_bounds_check_elision(false);
    let info = validator.validate(&module).expect("validation");
    let options = spv::Options {
        bounds_check_policies: BoundsCheckPolicies {
            index: BoundsCheckPolicy::Unchecked,
            buffer: BoundsCheckPolicy::Unchecked,
            ..Default::default()
        },
        ..Default::default()
    };
    let words = spv::write_vec(&module, &info, &options, None).expect("SPIR-V codegen");
    rspirv::dr::load_words(words)
        .expect("valid SPIR-V")
        .disassemble()
        .matches("OpULessThan")
        .count()
}

struct GpuRunner {
    device: wgpu::Device,
    queue: wgpu::Queue,
    enabled: wgpu::ComputePipeline,
    disabled: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    output: wgpu::Buffer,
    output_readback: wgpu::Buffer,
    query_set: wgpu::QuerySet,
    query_resolve: wgpu::Buffer,
    query_readback: wgpu::Buffer,
    timestamp_period: f32,
}

impl GpuRunner {
    fn new(enabled_words: &[u32], disabled_words: &[u32]) -> (Self, wgpu::AdapterInfo) {
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
        .expect("Vulkan adapter");
        assert!(
            adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY),
            "adapter lacks timestamp queries"
        );
        let adapter_info = adapter.get_info();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("bce-paired-v3"),
            required_features: wgpu::Features::TIMESTAMP_QUERY,
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .expect("device");
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("output"),
            size: OUTPUT_BYTES,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let output_readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("output-readback"),
            size: OUTPUT_BYTES,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bind-group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: output.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline-layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let make_pipeline = |label, words: &[u32]| {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::SpirV(Cow::Owned(words.to_vec())),
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                module: &module,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };
        let enabled = make_pipeline("bce-enabled", enabled_words);
        let disabled = make_pipeline("bce-disabled", disabled_words);
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: 2,
        });
        let query_resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("query-resolve"),
            size: 16,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let query_readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("query-readback"),
            size: 16,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let timestamp_period = queue.get_timestamp_period();
        (
            Self {
                device,
                queue,
                enabled,
                disabled,
                bind_group,
                output,
                output_readback,
                query_set,
                query_resolve,
                query_readback,
                timestamp_period,
            },
            adapter_info,
        )
    }

    fn pipeline(&self, enabled: bool) -> &wgpu::ComputePipeline {
        if enabled {
            &self.enabled
        } else {
            &self.disabled
        }
    }

    fn observe(&self, enabled: bool, dispatches: u32) -> f64 {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("timed"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("timed-pass"),
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: &self.query_set,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: Some(1),
                }),
            });
            pass.set_pipeline(self.pipeline(enabled));
            pass.set_bind_group(0, &self.bind_group, &[]);
            for _ in 0..dispatches {
                pass.dispatch_workgroups(1, 1, 1);
            }
        }
        encoder.resolve_query_set(&self.query_set, 0..2, &self.query_resolve, 0);
        encoder.copy_buffer_to_buffer(&self.query_resolve, 0, &self.query_readback, 0, 16);
        self.queue.submit([encoder.finish()]);
        let slice = self.query_readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| ());
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let mapped = slice.get_mapped_range().expect("mapped timestamps");
        let timestamps: &[u64] = bytemuck::cast_slice(&mapped);
        let ticks = timestamps[1]
            .checked_sub(timestamps[0])
            .expect("non-monotonic GPU timestamps");
        assert!(ticks > 0, "zero GPU timestamp delta");
        drop(mapped);
        self.query_readback.unmap();
        ticks as f64 * f64::from(self.timestamp_period) / f64::from(dispatches)
    }

    fn validate_output(&self, enabled: bool) -> f32 {
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(self.pipeline(enabled));
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&self.output, 0, &self.output_readback, 0, OUTPUT_BYTES);
        self.queue.submit([encoder.finish()]);
        let slice = self.output_readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| ());
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let mapped = slice.get_mapped_range().expect("mapped output");
        let output: &[f32] = bytemuck::cast_slice(&mapped);
        let expected = expected_output();
        let max_error = output
            .iter()
            .map(|value| (value - expected).abs())
            .fold(0.0f32, f32::max);
        drop(mapped);
        self.output_readback.unmap();
        max_error
    }
}

fn expected_output() -> f32 {
    (1..=ARRAY_LEN).map(|value| value as f32).sum()
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    match sorted.len() {
        0 => f64::NAN,
        n if n % 2 == 1 => sorted[n / 2],
        n => (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0,
    }
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

fn estimate(values: &[f64], seed: u64) -> Estimate {
    assert!(!values.is_empty());
    let mut rng = Rng(seed | 1);
    let mut medians = Vec::with_capacity(BOOTSTRAP_SAMPLES);
    for _ in 0..BOOTSTRAP_SAMPLES {
        let sample: Vec<_> = (0..values.len())
            .map(|_| values[rng.next() as usize % values.len()])
            .collect();
        medians.push(median(&sample));
    }
    medians.sort_by(f64::total_cmp);
    Estimate {
        median: median(values),
        ci95_lo: medians[(BOOTSTRAP_SAMPLES as f64 * 0.025) as usize],
        ci95_hi: medians[((BOOTSTRAP_SAMPLES as f64 * 0.975) as usize).min(BOOTSTRAP_SAMPLES - 1)],
        bootstrap_samples: BOOTSTRAP_SAMPLES,
        seed,
    }
}

fn command_output(program: &str, args: &[&str], cwd: Option<&Path>) -> String {
    let mut command = Command::new(program);
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_default()
}

fn source_state(path: &Path, artifacts: &Path, name: &str) -> SourceState {
    let revision = command_output("git", &["rev-parse", "HEAD"], Some(path));
    let branch = command_output("git", &["branch", "--show-current"], Some(path));
    let status = command_output("git", &["status", "--porcelain"], Some(path));
    let diff = Command::new("git")
        .args(["diff", "--binary", "HEAD"])
        .current_dir(path)
        .output()
        .expect("git diff")
        .stdout;
    std::fs::write(artifacts.join(format!("{name}.diff")), &diff).expect("write diff");
    SourceState {
        path: path.display().to_string(),
        revision,
        branch,
        dirty: !status.is_empty(),
        diff_sha256: sha256(&diff),
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn utc_now() -> String {
    command_output("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"], None)
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn main() {
    let utc_start = utc_now();
    let rounds = env_usize("BCE_ROUNDS", DEFAULT_ROUNDS);
    let control_rounds = env_usize("BCE_CONTROL_ROUNDS", DEFAULT_CONTROL_ROUNDS);
    let warmup_rounds = env_usize("BCE_WARMUP_ROUNDS", DEFAULT_WARMUP_ROUNDS);
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let framework = workspace.parent().expect("framework root");
    let project = framework.parent().expect("project root");
    let wgpu_root = project.join("wgpu");
    let run_id = format!(
        "{}-fixed-array-scan-bce-on-off",
        utc_start.replace([':', '-'], "")
    );
    let run_dir = framework.join("runs").join(run_id);
    let artifacts_dir = run_dir.join("artifacts");
    std::fs::create_dir_all(&artifacts_dir).expect("create run directory");
    std::fs::write(artifacts_dir.join("shader.wgsl"), SHADER).expect("write shader");
    std::fs::write(artifacts_dir.join("runner.rs"), include_str!("main.rs"))
        .expect("write runner source");

    let unchecked_uless = unchecked_uless_count();
    let enabled = compile(true, unchecked_uless);
    let disabled = compile(false, unchecked_uless);
    std::fs::write(
        artifacts_dir.join("enabled.spv"),
        bytemuck::cast_slice(&enabled.words),
    )
    .expect("write enabled SPIR-V");
    std::fs::write(
        artifacts_dir.join("disabled.spv"),
        bytemuck::cast_slice(&disabled.words),
    )
    .expect("write disabled SPIR-V");

    assert!(enabled.static_artifact.safe_accesses > 0);
    assert_eq!(disabled.static_artifact.safe_accesses, 0);
    assert!(
        disabled.static_artifact.emitted_checks_by_unchecked_delta
            > enabled.static_artifact.emitted_checks_by_unchecked_delta,
        "BCE toggle did not change emitted checks"
    );

    let sources = vec![
        source_state(&wgpu_root, &artifacts_dir, "wgpu"),
        source_state(framework, &artifacts_dir, "benchmark-framework"),
    ];
    let nvidia_smi_before = command_output(
        "nvidia-smi",
        &[
            "--query-gpu=name,driver_version,pstate,temperature.gpu,power.draw,clocks.gr,clocks.mem",
            "--format=csv,noheader",
        ],
        None,
    );
    let (runner, adapter_info) = GpuRunner::new(&enabled.words, &disabled.words);
    let enabled_error = runner.validate_output(true);
    let disabled_error = runner.validate_output(false);
    let correctness = CorrectnessResult {
        expected: expected_output(),
        enabled_max_abs_error: enabled_error,
        disabled_max_abs_error: disabled_error,
        passed: enabled_error == 0.0 && disabled_error == 0.0,
    };
    assert!(correctness.passed, "GPU output validation failed");

    let probe_dispatches = 10;
    let probe_ns = runner.observe(false, probe_dispatches);
    let dispatches_per_sample = (5_000_000.0 / probe_ns).round().clamp(1.0, 10_000.0) as u32;

    for round in 0..warmup_rounds {
        if round % 2 == 0 {
            runner.observe(true, dispatches_per_sample);
            runner.observe(false, dispatches_per_sample);
        } else {
            runner.observe(false, dispatches_per_sample);
            runner.observe(true, dispatches_per_sample);
        }
    }

    let mut control_samples = Vec::with_capacity(control_rounds);
    for round in 0..control_rounds {
        let a_first = round % 2 == 0;
        let (a, b) = if a_first {
            (
                runner.observe(true, dispatches_per_sample),
                runner.observe(true, dispatches_per_sample),
            )
        } else {
            let b = runner.observe(true, dispatches_per_sample);
            let a = runner.observe(true, dispatches_per_sample);
            (a, b)
        };
        control_samples.push(ControlSample {
            round,
            order: if a_first { "AB" } else { "BA" }.into(),
            a_ns_per_dispatch: a,
            b_ns_per_dispatch: b,
            a_minus_b_ns: a - b,
        });
    }
    let control_deltas: Vec<_> = control_samples
        .iter()
        .map(|sample| sample.a_minus_b_ns)
        .collect();
    let negative_control = estimate(&control_deltas, 0x434f_4e54_524f_4c);
    let negative_control_includes_zero =
        negative_control.ci95_lo <= 0.0 && negative_control.ci95_hi >= 0.0;

    let mut paired_samples = Vec::with_capacity(rounds);
    for round in 0..rounds {
        let enabled_first = round % 2 == 0;
        let (enabled_ns, disabled_ns, first, second) = if enabled_first {
            let enabled_ns = runner.observe(true, dispatches_per_sample);
            let disabled_ns = runner.observe(false, dispatches_per_sample);
            (enabled_ns, disabled_ns, enabled_ns, disabled_ns)
        } else {
            let disabled_ns = runner.observe(false, dispatches_per_sample);
            let enabled_ns = runner.observe(true, dispatches_per_sample);
            (enabled_ns, disabled_ns, disabled_ns, enabled_ns)
        };
        paired_samples.push(PairedSample {
            round,
            order: if enabled_first {
                "enabled-disabled"
            } else {
                "disabled-enabled"
            }
            .into(),
            first_ns_per_dispatch: first,
            second_ns_per_dispatch: second,
            enabled_ns_per_dispatch: enabled_ns,
            disabled_ns_per_dispatch: disabled_ns,
            disabled_minus_enabled_ns: disabled_ns - enabled_ns,
        });
    }
    let headline_deltas: Vec<_> = paired_samples
        .iter()
        .map(|sample| sample.disabled_minus_enabled_ns)
        .collect();
    let headline = estimate(&headline_deltas, 0x4243_455f_5633);
    let nvidia_smi_after = command_output(
        "nvidia-smi",
        &[
            "--query-gpu=name,driver_version,pstate,temperature.gpu,power.draw,clocks.gr,clocks.mem",
            "--format=csv,noheader",
        ],
        None,
    );
    let machine = Machine {
        adapter: adapter_info.name,
        backend: format!("{:?}", adapter_info.backend),
        driver: adapter_info.driver,
        driver_info: adapter_info.driver_info,
        timestamp_period_ns: runner.timestamp_period,
        os: std::env::consts::OS.into(),
        architecture: std::env::consts::ARCH.into(),
        kernel: command_output("uname", &["-srvo"], None),
        nvidia_smi_before,
        nvidia_smi_after,
    };
    let valid_for_claims = correctness.passed && negative_control_includes_zero;
    let report = Report {
        schema_version: "3.0.0".into(),
        harness_version: HARNESS_VERSION.into(),
        utc_start,
        utc_end: utc_now(),
        sources,
        machine,
        shader_sha256: sha256(SHADER.as_bytes()),
        bounds_policy: "ReadZeroSkipWrite".into(),
        dispatches_per_sample,
        warmup_rounds,
        headline_rounds: rounds,
        control_rounds,
        static_artifacts: vec![enabled.static_artifact, disabled.static_artifact],
        correctness,
        negative_control,
        negative_control_includes_zero,
        headline,
        valid_for_claims,
        control_samples,
        paired_samples,
    };
    let report_path = run_dir.join("fixed-array-scan.json");
    std::fs::write(
        &report_path,
        serde_json::to_vec_pretty(&report).expect("serialize report"),
    )
    .expect("write report");
    println!("{}", report_path.display());
    println!(
        "valid_for_claims={} negative_control_includes_zero={} headline_median_ns={:.3} ci95=[{:.3},{:.3}]",
        report.valid_for_claims,
        report.negative_control_includes_zero,
        report.headline.median,
        report.headline.ci95_lo,
        report.headline.ci95_hi
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_uses_average_for_even_samples() {
        assert_eq!(median(&[1.0, 4.0, 2.0, 3.0]), 2.5);
    }

    #[test]
    fn expected_shader_output_is_exact() {
        assert_eq!(expected_output(), 32_896.0);
    }

    #[test]
    fn bootstrap_of_constant_is_constant() {
        let estimate = estimate(&[7.0; 20], 1);
        assert_eq!(estimate.median, 7.0);
        assert_eq!(estimate.ci95_lo, 7.0);
        assert_eq!(estimate.ci95_hi, 7.0);
    }

    #[test]
    fn bce_toggle_changes_static_check_count() {
        let baseline = unchecked_uless_count();
        let enabled = compile(true, baseline);
        let disabled = compile(false, baseline);
        assert!(enabled.static_artifact.safe_accesses > 0);
        assert_eq!(disabled.static_artifact.safe_accesses, 0);
        assert!(
            enabled.static_artifact.emitted_checks_by_unchecked_delta
                < disabled.static_artifact.emitted_checks_by_unchecked_delta
        );
    }
}
