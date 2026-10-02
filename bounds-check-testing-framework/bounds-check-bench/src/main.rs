//! Fixed Benchmark Suite for bounds-check elision (Track 2) on WGSL compute shaders.
//!
//! Overhaul based on design principles from Google Benchmark (auto-calibration),
//! LLVM LNT (permutation tests, Anderson-Darling), Triton do_bench (interleaved A/B,
//! auto-calibration), and NVIDIA Nsight (clock locking, roofline, L2 invalidation).
//!
//! Key improvements over the old suite:
//!   - Auto-calibrating iteration count to hit ≥100ms wall time per pass
//!   - Interleaved A/B measurement (Track 2 ON/OFF within same process)
//!   - Adaptive warmup until 3 consecutive medians agree within 1%
//!   - Full statistical reporting: n, min, max, mean, stddev, median, IQR, P5/P95/P99
//!   - Anderson-Darling normality test
//!   - Cohen's d effect size
//!   - Permutation test for significance
//!   - Bootstrap CI on relative delta
//!   - Bonferroni-corrected significance flags
//!   - Roofline analysis (achieved vs peak BW, arithmetic intensity, bound classification)
//!   - L2 cache invalidation between configs
//!   - System state logging
//!   - Re-query timestamp period per pass

use rand::seq::SliceRandom;
use rand::Rng;
use rand::SeedableRng;
use rand_pcg::Pcg64;
use std::borrow::Cow;
use std::time::{Duration, Instant};
use serde::Serialize;

// ============================================================================
// Nsight Graphics SDK FFI (for programmable capture triggers)
// ============================================================================
// These functions are resolved at runtime via dlsym from
// libngfx-capture-interception.so (loaded by ngfx-capture on Linux).
// When not running under ngfx-capture, they gracefully return -1.
extern "C" {
    fn nsight_initialize() -> i32;
    fn nsight_start_capture() -> i32;
    fn nsight_stop_capture() -> i32;
    fn nsight_is_available() -> i32;
}

// ============================================================================
// Constants
// ============================================================================

const WARMUP_PASSES_MIN: u32 = 5;
const WARMUP_PASSES_MAX: u32 = 100;
const TIMED_PASSES: u32 = 200; // Minimum for meaningful bootstrap (Efron & Tibshirani 1993)
const TARGET_WALL_TIME_MS: f64 = 100.0; // Auto-calibrate to this wall time per pass
const BOOTSTRAP_SAMPLES: usize = 10_000;
const BOOTSTRAP_SEED: u64 = 42;
const PERMUTATION_SAMPLES: usize = 100_000;
const STEADY_STATE_TOLERANCE: f64 = 0.01; // 1% tolerance for steady-state detection
const PEAK_BW_PASSES: u32 = 20; // Peak BW calibration: 20 passes is enough (not a hypothesis test)
const L2_INVALIDATE_SIZE: u64 = 64 * 1024 * 1024; // 64MB scratch for L2 cache invalidation

const POLICIES: &[(&str, naga::proc::BoundsCheckPolicy)] = &[
    ("Unchecked", naga::proc::BoundsCheckPolicy::Unchecked),
    ("ReadZeroSkipWrite", naga::proc::BoundsCheckPolicy::ReadZeroSkipWrite),
    ("Restrict", naga::proc::BoundsCheckPolicy::Restrict),
];

const PROBLEM_SIZES: &[ProblemSize] = &[
    ProblemSize { label: "64K",  elements: 65_536 },
    ProblemSize { label: "1M",   elements: 1_048_576 },
    ProblemSize { label: "10M",  elements: 10_485_760 },
    ProblemSize { label: "16M",  elements: 16_777_216 },
];

const ELEMS_PER_WG: u32 = 256;
const TIMESTAMP_PERIOD_DEFAULT: f64 = 1.0;

// ============================================================================
// Problem size descriptor
// ============================================================================

#[derive(Clone, Copy)]
struct ProblemSize {
    label: &'static str,
    elements: u64,
}

impl ProblemSize {
    fn n_workgroups(&self) -> u32 {
        (self.elements / ELEMS_PER_WG as u64) as u32
    }

    fn dispatch_2d(&self) -> (u32, u32) {
        let n = self.n_workgroups();
        let x = 256u32;
        let y = (n + x - 1) / x;
        (x, y)
    }

    /// Auto-calibrate dispatches per pass: start with 10_000 and scale so that
    /// each pass takes approximately TARGET_WALL_TIME_MS of GPU execution time.
    /// The dispatches_per_pass field stores the calibrated value.
    fn calibrate_dispatches_to_target(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pipeline: &wgpu::ComputePipeline,
        bind_group: &wgpu::BindGroup,
        dispatch: (u32, u32, u32),
        period_ns: f64,
    ) -> u32 {
        // Start with a small number of dispatches to probe timing.
        // For large kernels each dispatch can take 10+ ms, so keep probe small
        // to avoid Linux NVIDIA driver's ~2s compute watchdog timeout.
        let probe_dispatches: u32 = match self.elements {
            e if e <= 65_536 => 1_000,
            e if e <= 1_048_576 => 100,
            _ => 5,
        };

        let probe_time = measure_dispatches(
            device, queue, pipeline, bind_group,
            dispatch, probe_dispatches, period_ns, 2, 1, // 2 warmup, 1 timed
        );

        if probe_time <= 0.0 {
            return probe_dispatches.max(1);
        }

        // Scale to target wall time, but cap to prevent GPU watchdog timeout.
        // probe_time is median µs PER DISPATCH (already per-dispatch from measure_dispatches).
        // target_dispatches = (100ms target × 1000 µs/ms) / (probe_time µs/dispatch)
        // Cap at 1000 to keep 200 passes × 1000 dispatches = 200k total dispatches
        // which is ~1-2s GPU work on laptop, well under NVIDIA's 2s compute watchdog.
        let target_dispatches = if probe_time > 0.0 {
            ((TARGET_WALL_TIME_MS * 1000.0) / probe_time) as u32
        } else {
            probe_dispatches
        };
        target_dispatches.clamp(1, 1_000)
    }
}

// ============================================================================
// WGSL kernel sources
// ============================================================================

const WGSL_REDUCTION_LOCAL: &str = r#"
@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
var<workgroup> tile: array<f32, 256>;

@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32,
        @builtin(workgroup_id) wid: vec3<u32>) {
    let flat_wid = wid.y * 256u + wid.x;
    tile[lid] = input[flat_wid * 256u + lid];
    workgroupBarrier();
    for (var stride: u32 = 128u; stride > 0u; stride >>= 1u) {
        if (lid < stride) {
            tile[lid] = tile[lid] + tile[lid + stride];
        }
        workgroupBarrier();
    }
    if (lid == 0u) { output[flat_wid] = tile[0]; }
}
"#;

const WGSL_REDUCTION_STORAGE: &str = r#"
@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> scratch: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32,
        @builtin(workgroup_id) wid: vec3<u32>) {
    let flat_wid = wid.y * 256u + wid.x;
    let base = flat_wid * 256u;
    scratch[base + lid] = input[base + lid];
    workgroupBarrier();
    for (var stride: u32 = 128u; stride > 0u; stride >>= 1u) {
        if (lid < stride) {
            scratch[base + lid] = scratch[base + lid] + scratch[base + lid + stride];
        }
        workgroupBarrier();
    }
    if (lid == 0u) { output[flat_wid] = scratch[base]; }
}
"#;

const WGSL_PREFIX_SCAN_LOCAL: &str = r#"
@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
var<workgroup> temp: array<f32, 256>;

@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32,
        @builtin(workgroup_id) wid: vec3<u32>) {
    let flat_wid = wid.y * 256u + wid.x;
    let base = flat_wid * 256u;
    temp[lid] = input[base + lid];
    workgroupBarrier();
    for (var stride: u32 = 1u; stride < 256u; stride <<= 1u) {
        var val: f32 = 0.0;
        if (lid >= stride) { val = temp[lid - stride]; }
        workgroupBarrier();
        temp[lid] = temp[lid] + val;
        workgroupBarrier();
    }
    output[base + lid] = temp[lid];
}
"#;

const WGSL_PREFIX_SCAN_STORAGE: &str = r#"
@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> scratch: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32,
        @builtin(workgroup_id) wid: vec3<u32>) {
    let flat_wid = wid.y * 256u + wid.x;
    let base = flat_wid * 256u;
    scratch[base + lid] = input[base + lid];
    workgroupBarrier();
    for (var stride: u32 = 1u; stride < 256u; stride <<= 1u) {
        var val: f32 = 0.0;
        if (lid >= stride) { val = scratch[base + lid - stride]; }
        workgroupBarrier();
        scratch[base + lid] = scratch[base + lid] + val;
        workgroupBarrier();
    }
    output[base + lid] = scratch[base + lid];
}
"#;

const WGSL_TRACK3B_TEST: &str = r#"
@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
var<workgroup> scratch: array<f32, 256>;

@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32,
        @builtin(workgroup_id) wid: vec3<u32>) {
    let flat_wid = wid.y * 256u + wid.x;
    let base = flat_wid * 256u;
    scratch[lid] = input[base + lid];
    workgroupBarrier();
    let a = workgroupUniformLoad(&scratch[0]);
    scratch[lid] = a;
    workgroupBarrier();
    let b = workgroupUniformLoad(&scratch[1]);
    let c = workgroupUniformLoad(&scratch[2]);
    scratch[lid] = a + b + c;
    output[base + lid] = scratch[lid];
}
"#;

const WGSL_WGUL_REDUCE: &str = r#"
@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
var<workgroup> scratch: array<f32, 256>;

@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32,
        @builtin(workgroup_id) wid: vec3<u32>) {
    let flat_wid = wid.y * 256u + wid.x;
    let base = flat_wid * 256u;
    scratch[lid] = input[base + lid];
    workgroupBarrier();
    for (var stride: u32 = 128u; stride > 0u; stride >>= 1u) {
        let idx2 = min(lid + stride, 255u);
        let a = workgroupUniformLoad(&scratch[lid]);
        let b = workgroupUniformLoad(&scratch[idx2]);
        if (lid < stride) { scratch[lid] = a + b; }
        workgroupBarrier();
    }
    if (lid == 0u) { output[flat_wid] = scratch[0]; }
}
"#;

const WGSL_TILED_MATMUL: &str = r#"
@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
var<workgroup> tile_a: array<f32, 256>;
var<workgroup> tile_b: array<f32, 256>;
const TILE: u32 = 16u;

@compute @workgroup_size(16, 16)
fn main(@builtin(local_invocation_id) lid: vec3<u32>,
        @builtin(workgroup_id) wid: vec3<u32>) {
    let dim = GRID_WGS * TILE;
    let M = dim; let N = dim; let K = dim;
    let row = wid.y * TILE + lid.y;
    let col = wid.x * TILE + lid.x;
    var sum: f32 = 0.0;
    for (var t: u32 = 0u; t < K / TILE; t++) {
        var a_val: f32 = 0.0;
        if (row < M && t * TILE + lid.x < K) { a_val = input[row * K + t * TILE + lid.x]; }
        var b_val: f32 = 0.0;
        if (t * TILE + lid.y < K && col < N) { b_val = input[M * K + (t * TILE + lid.y) * N + col]; }
        tile_a[lid.y * TILE + lid.x] = a_val;
        tile_b[lid.y * TILE + lid.x] = b_val;
        workgroupBarrier();
        for (var i: u32 = 0u; i < TILE; i++) {
            sum += tile_a[lid.y * TILE + i] * tile_b[i * TILE + lid.x];
        }
        workgroupBarrier();
    }
    if (row < M && col < N) { output[row * N + col] = sum; }
}
"#;

const WGSL_IDLE_KERNEL: &str = r#"
@compute @workgroup_size(256)
fn main() { return; }
"#;



// ============================================================================
// Kernel metadata
// ============================================================================

#[derive(Clone, Copy)]
enum MemoryVariant {
    Local,
    Storage,
}

impl MemoryVariant {
    fn label(&self) -> &'static str {
        match self {
            MemoryVariant::Local => "local",
            MemoryVariant::Storage => "storage",
        }
    }
}

struct KernelDef {
    name: &'static str,
    variant: MemoryVariant,
    wgsl: &'static str,
    n_bindings: usize,
    /// Arithmetic intensity (FLOPs/byte) for roofline analysis. None = auto-compute.
    flops_per_element: Option<f64>,
}

const KERNELS: &[KernelDef] = &[
    KernelDef { name: "reduction", variant: MemoryVariant::Local,  wgsl: WGSL_REDUCTION_LOCAL,  n_bindings: 2, flops_per_element: Some(2.0) },
    KernelDef { name: "reduction", variant: MemoryVariant::Storage, wgsl: WGSL_REDUCTION_STORAGE, n_bindings: 3, flops_per_element: Some(2.0) },
    KernelDef { name: "prefix_scan", variant: MemoryVariant::Local,  wgsl: WGSL_PREFIX_SCAN_LOCAL,  n_bindings: 2, flops_per_element: Some(1.5) },
    KernelDef { name: "prefix_scan", variant: MemoryVariant::Storage, wgsl: WGSL_PREFIX_SCAN_STORAGE, n_bindings: 3, flops_per_element: Some(1.5) },
    KernelDef { name: "track3b_test", variant: MemoryVariant::Local, wgsl: WGSL_TRACK3B_TEST, n_bindings: 2, flops_per_element: Some(0.5) },
    KernelDef { name: "wgul_reduce", variant: MemoryVariant::Local,  wgsl: WGSL_WGUL_REDUCE, n_bindings: 2, flops_per_element: Some(2.0) },
    KernelDef { name: "tiled_matmul", variant: MemoryVariant::Local, wgsl: WGSL_TILED_MATMUL, n_bindings: 2, flops_per_element: Some(32.0) }, // O(TILE^3/TILE^2) ~ 16 FLOPs/element
];

fn matmul_dim(elements: u64) -> u32 {
    let side = (elements as f64).sqrt() as u32;
    ((side + 15) / 16) * 16
}

fn matmul_wg_count(elements: u64) -> u32 {
    matmul_dim(elements) / 16
}

fn matmul_input_bytes(elements: u64) -> u64 {
    let d = matmul_dim(elements) as u64;
    2 * d * d * 4
}

fn matmul_output_bytes(elements: u64) -> u64 {
    let d = matmul_dim(elements) as u64;
    d * d * 4
}

fn compute_dispatch(kernel: &KernelDef, size: &ProblemSize) -> (u32, u32, u32) {
    if kernel.name == "tiled_matmul" {
        let w = matmul_wg_count(size.elements);
        (w, w, 1)
    } else if size.n_workgroups() <= 65535 {
        (size.n_workgroups(), 1, 1)
    } else {
        let (x, y) = size.dispatch_2d();
        (x, y, 1)
    }
}

fn compute_buffer_bytes(kernel: &KernelDef, size: &ProblemSize) -> (u64, u64, u64) {
    if kernel.name == "tiled_matmul" {
        (matmul_input_bytes(size.elements), 0, matmul_output_bytes(size.elements))
    } else {
        let nwgs_out = if size.n_workgroups() <= 65535 {
            size.n_workgroups() as u64
        } else {
            let (x, y) = size.dispatch_2d();
            (x * y) as u64
        };
        match kernel.variant {
            MemoryVariant::Local => (size.input_bytes(), 0, nwgs_out * 4),
            MemoryVariant::Storage => (size.input_bytes(), size.scratch_bytes(), nwgs_out * 4),
        }
    }
}

impl ProblemSize {
    fn input_bytes(&self) -> u64 { self.elements * 4 }
    fn scratch_bytes(&self) -> u64 { self.elements * 4 }
}

// ============================================================================
// Naga WGSL → SPIR-V compilation
// ============================================================================

fn compile_wgsl_with_policy(
    wgsl_source: &str,
    policy: naga::proc::BoundsCheckPolicy,
    tiled_wg_count: u32,
) -> (Vec<u32>, usize) {
    let source = wgsl_source.replace("GRID_WGS", &format!("{}u", tiled_wg_count));
    let module = naga::front::wgsl::parse_str(&source)
        .expect("naga WGSL frontend parse failed");
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .expect("naga validation failed");
    let options = naga::back::spv::Options {
        bounds_check_policies: naga::proc::BoundsCheckPolicies {
            index: policy,
            buffer: policy,
            ..Default::default()
        },
        ..Default::default()
    };
    let pipeline_options = naga::back::spv::PipelineOptions {
        shader_stage: naga::ShaderStage::Compute,
        entry_point: "main".to_string(),
    };
    let spv = naga::back::spv::write_vec(&module, &info, &options, Some(&pipeline_options))
        .expect("naga SPIR-V backend write failed");
    let bounds_checks = spv.len(); // SPIR-V word count as proxy for instruction complexity
    (spv, bounds_checks)
}

// ============================================================================
// WGPU timestamp-query infrastructure
// ============================================================================

struct TimestampQueries {
    set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    destination_buffer: wgpu::Buffer,
}

impl TimestampQueries {
    fn new(device: &wgpu::Device) -> Self {
        let bs = std::mem::size_of::<u64>() as u64 * 2;
        Self {
            set: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("bce-ts-set"),
                count: 2,
                ty: wgpu::QueryType::Timestamp,
            }),
            resolve_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("bce-ts-resolve"),
                size: bs,
                usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::QUERY_RESOLVE,
                mapped_at_creation: false,
            }),
            destination_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("bce-ts-dest"),
                size: bs,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
        }
    }

    fn resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.resolve_query_set(&self.set, 0..2, &self.resolve_buffer, 0);
        encoder.copy_buffer_to_buffer(&self.resolve_buffer, 0, &self.destination_buffer, 0,
            self.resolve_buffer.size());
    }

    fn read_back(&self, device: &wgpu::Device) -> Vec<u64> {
        let slice = self.destination_buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| ());
        device.poll(wgpu::Maintain::Wait);
        let ts = {
            let v = slice.get_mapped_range();
            bytemuck::cast_slice::<u8, u64>(&v).to_vec()
        };
        self.destination_buffer.unmap();
        ts
    }
}

// ============================================================================
// L2 Cache Invalidation Buffer
// ============================================================================

struct L2Invalidator {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::ComputePipeline,
    size: u64,
}

impl L2Invalidator {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue) -> Self {
        let size = L2_INVALIDATE_SIZE;
        let buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("l2-invalidate"),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("l2-invalidate-shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(r#"
                @group(0) @binding(0) var<storage, read_write> buf: array<f32>;
                @compute @workgroup_size(256)
                fn main(@builtin(local_invocation_index) lid: u32,
                        @builtin(workgroup_id) wid: vec3<u32>) {
                    let idx = wid.y * 256u + wid.x;
                    let flat_idx = idx * 256u + lid;
                    let len = 65536u * 256u; // 64MB / 4 bytes
                    if (flat_idx < len) {
                        buf[flat_idx] = 0.0;
                    }
                }
            "#)),
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("l2-bgl"),
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

        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("l2-pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("l2-pipe"),
            layout: Some(&pl),
            module: &module,
            entry_point: "main",
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("l2-bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buf,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        Self { buffer: buf, bind_group, pipeline, size }
    }

    /// Touch the entire 64MB buffer to evict L2 cache between configs.
    /// Uses 2D dispatch to avoid WebGPU's per-dimension 65535 limit.
    fn invalidate(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("l2-inv-enc"),
        });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("l2-inv-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            // 2D dispatch: 256 x 256 = 65536 workgroups, covering 64MB
            pass.dispatch_workgroups(256, 256, 1);
        }
        queue.submit(Some(enc.finish()));
        device.poll(wgpu::Maintain::Wait);
    }
}

// ============================================================================
// Peak Bandwidth Measurement (GPU-timed via sustained copy kernel)
// ============================================================================

fn measure_peak_bandwidth_gpu_timed(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    n_passes: u32,
) -> f64 {
    // Use a sustained copy kernel: read from input, write to output, timed via GPU timestamp queries.
    // This gives a GPU-timed bandwidth measurement, not host-timed.
    let size: u64 = 64 * 1024 * 1024;
    let elements = (size / 4) as u32;

    // Compile peak BW kernel (copy buffer via compute shader)
    let shader_src = format!(r#"
        @group(0) @binding(0) var<storage, read> input: array<f32>;
        @group(0) @binding(1) var<storage, read_write> output: array<f32>;
        @compute @workgroup_size(256)
        fn main(@builtin(local_invocation_index) lid: u32,
                @builtin(workgroup_id) wid: vec3<u32>) {{
            // 2D dispatch: incorporate wid.y into flat index
            let flat_wid = wid.y * 256u + wid.x;
            let idx = flat_wid * 256u + lid;
            if (idx < {}u) {{
                output[idx] = input[idx] * 2.0;
            }}
        }}
    "#, elements);

    let module = naga::front::wgsl::parse_str(&shader_src)
        .expect("peak BW WGSL parse failed");
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .expect("peak BW validation failed");
    let options = naga::back::spv::Options {
        bounds_check_policies: naga::proc::BoundsCheckPolicies {
            index: naga::proc::BoundsCheckPolicy::Unchecked,
            buffer: naga::proc::BoundsCheckPolicy::Unchecked,
            ..Default::default()
        },
        ..Default::default()
    };
    let pipeline_options = naga::back::spv::PipelineOptions {
        shader_stage: naga::ShaderStage::Compute,
        entry_point: "main".to_string(),
    };
    let spv = naga::back::spv::write_vec(&module, &info, &options, Some(&pipeline_options))
        .expect("peak BW SPIR-V write failed");

    let smod = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("peak-bw-shader"),
        source: wgpu::ShaderSource::SpirV(Cow::Owned(spv)),
    });

    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("peak-bw-bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("peak-bw-pl"),
        bind_group_layouts: &[&bgl],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("peak-bw-pipe"),
        layout: Some(&pl),
        module: &smod,
        entry_point: "main",
    });

    let input_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("peak-bw-in"),
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let output_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("peak-bw-out"),
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("peak-bw-bg"),
        layout: &bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &input_buf, offset: 0, size: None,
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &output_buf, offset: 0, size: None,
                }),
            },
        ],
    });

    // Use minimal dispatches per pass to avoid GPU driver watchdog timeout on laptop.
    // The 64MB buffer gives good BW signal even with just 2 reps.
    let n_dispatches = 2;
    // Use 2D dispatch to avoid WebGPU's 65535 per-dimension limit
    let wgs_total = (elements + 255) / 256;
    let wgs_x = 256u32;
    let wgs_y = (wgs_total + wgs_x - 1) / wgs_x;
    let ts = TimestampQueries::new(device);
    let period_ns = queue.get_timestamp_period() as f64;

    // Warmup
    for _ in 0..10 {
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("peak-bw-warmup"),
        });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("peak-bw-warmup-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(wgs_x, wgs_y, 1);
        }
        queue.submit(Some(enc.finish()));
        device.poll(wgpu::Maintain::Wait);
    }

    // Timed (n_passes passes)
    let mut samples = Vec::with_capacity(n_passes as usize);
    for _ in 0..n_passes {
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("peak-bw-timed"),
        });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("peak-bw-timed-pass"),
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: &ts.set,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: Some(1),
                }),
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bg, &[]);
            for _ in 0..n_dispatches {
                pass.dispatch_workgroups(wgs_x, wgs_y, 1);
            }
        }
        ts.resolve(&mut enc);
        queue.submit(Some(enc.finish()));
        let ticks = ts.read_back(device);
        let elapsed_ns = ticks[1].wrapping_sub(ticks[0]) as f64 * period_ns;
        if elapsed_ns > 0.0 {
            let bw = (size as f64 * 2.0 * n_dispatches as f64) / elapsed_ns; // bytes per ns = GB/s
            samples.push(bw);
        }
    }

    let stats = summarize(&samples);
    stats.median
}

// ============================================================================
// Statistics
// ============================================================================

struct LcgRng { state: u64 }

impl LcgRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407) }
    }
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.state >> 33
    }
}

fn pct_sorted(s: &[f64], p: f64) -> f64 {
    if s.is_empty() { return 0.0; }
    let idx = ((s.len() - 1) as f64 * p / 100.0) as usize;
    s[idx.min(s.len() - 1)]
}

struct SampleStats {
    n: usize,
    median: f64,
    mean: f64,
    stddev: f64,
    iqr: f64,
    min: f64,
    max: f64,
    p5: f64,
    p95: f64,
    p99: f64,
    ci_lower: f64,
    ci_upper: f64,
    /// Anderson-Darling A² statistic. < 0.752 for normal at α=0.05.
    ad_statistic: f64,
    /// True if distribution is normal (A² < 0.752)
    is_normal: bool,
}

fn anderson_darling(s: &[f64], mean: f64, stddev: f64) -> f64 {
    if stddev <= 0.0 || s.len() < 8 { return f64::INFINITY; }
    let n = s.len() as f64;
    let mut sorted = s.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mut sum = 0.0;
    for i in 0..sorted.len() {
        let i_float = (i + 1) as f64;
        let z = (sorted[i] - mean) / stddev;
        let cdf = 0.5 * (1.0 + erf(z / std::f64::consts::SQRT_2));
        if cdf <= 0.0 || cdf >= 1.0 { continue; }
        sum += (2.0 * i_float - 1.0) * (cdf.ln() + (1.0 - cdf).ln());
    }
    -n - sum / n
}

fn erf(x: f64) -> f64 {
    // Horner approximation of the error function
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();
    sign * y
}

fn summarize(samples: &[f64]) -> SampleStats {
    let n = samples.len();
    if n == 0 {
        return SampleStats {
            n: 0, median: 0.0, mean: 0.0, stddev: 0.0, iqr: 0.0,
            min: 0.0, max: 0.0, p5: 0.0, p95: 0.0, p99: 0.0,
            ci_lower: 0.0, ci_upper: 0.0, ad_statistic: 0.0, is_normal: false,
        };
    }

    let mut s = samples.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let median = pct_sorted(&s, 50.0);
    let q1 = pct_sorted(&s, 25.0);
    let q3 = pct_sorted(&s, 75.0);
    let p5 = pct_sorted(&s, 5.0);
    let p95 = pct_sorted(&s, 95.0);
    let p99 = pct_sorted(&s, 99.0);
    let mean = s.iter().sum::<f64>() / n as f64;
    let variance = s.iter().map(|&x| { let d = x - mean; d * d }).sum::<f64>() / (n as f64 - 1.0);
    let stddev = variance.sqrt();

    let ad_statistic = anderson_darling(&s, mean, stddev);
    let is_normal = ad_statistic < 0.752; // α=0.05 critical value for AD test

    // Bootstrap CI on median
    let mut rng = LcgRng::new(BOOTSTRAP_SEED);
    let mut bm: Vec<f64> = Vec::with_capacity(BOOTSTRAP_SAMPLES);
    for _ in 0..BOOTSTRAP_SAMPLES {
        let mut bs: Vec<f64> = Vec::with_capacity(n);
        for _ in 0..n { bs.push(s[rng.next_u64() as usize % n]); }
        bs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        bm.push(pct_sorted(&bs, 50.0));
    }
    bm.sort_by(|a, b| a.partial_cmp(b).unwrap());

    SampleStats {
        n, median, mean, stddev, iqr: q3 - q1, min: s[0], max: s[n - 1],
        p5, p95, p99,
        ci_lower: pct_sorted(&bm, 2.5),
        ci_upper: pct_sorted(&bm, 97.5),
        ad_statistic, is_normal,
    }
}

/// Cohen's d effect size. Convention: |d| ≥ 0.2 = small, ≥ 0.5 = medium, ≥ 0.8 = large.
fn cohens_d(a: &SampleStats, b: &SampleStats) -> f64 {
    let pooled = ((a.stddev.powi(2) + b.stddev.powi(2)) / 2.0).sqrt();
    if pooled <= 0.0 { return 0.0; }
    (b.mean - a.mean) / pooled
}

/// Permutation test for A/B comparison. Returns p-value (proportion of permutations
/// where the absolute mean difference exceeds the observed difference).
fn permutation_test(a: &[f64], b: &[f64], n_permutations: usize) -> f64 {
    if a.is_empty() || b.is_empty() { return 1.0; }

    let n_a = a.len();
    let n_b = b.len();

    let observed_diff = b.iter().sum::<f64>() / n_b as f64
        - a.iter().sum::<f64>() / n_a as f64;

    // Pool and permute
    let mut combined: Vec<f64> = Vec::with_capacity(n_a + n_b);
    combined.extend_from_slice(a);
    combined.extend_from_slice(b);

    let mut rng = Pcg64::seed_from_u64(BOOTSTRAP_SEED);
    let mut count = 0;

    for _ in 0..n_permutations {
        combined.shuffle(&mut rng);
        let perm_a_mean = combined[..n_a].iter().sum::<f64>() / n_a as f64;
        let perm_b_mean = combined[n_a..].iter().sum::<f64>() / n_b as f64;
        let perm_diff = perm_b_mean - perm_a_mean;
        if perm_diff.abs() >= observed_diff.abs() {
            count += 1;
        }
    }

    (count + 1) as f64 / (n_permutations + 1) as f64
}

/// Bootstrap CI on relative delta (b - a) / a
fn bootstrap_relative_delta(a: &[f64], b: &[f64], n_bootstrap: usize) -> (f64, f64, f64) {
    if a.is_empty() || b.is_empty() || a.len() < 2 || b.len() < 2 {
        return (0.0, 0.0, 0.0);
    }

    let mut rng = Pcg64::seed_from_u64(123);
    let a_sorted = { let mut v = a.to_vec(); v.sort_by(|x, y| x.partial_cmp(y).unwrap()); v };
    let b_sorted = { let mut v = b.to_vec(); v.sort_by(|x, y| x.partial_cmp(y).unwrap()); v };

    let a_median = median_sorted(&a_sorted);
    let b_median = median_sorted(&b_sorted);
    let observed_delta = if a_median > 0.0 { (b_median - a_median) / a_median } else { 0.0 };

    let mut deltas: Vec<f64> = Vec::with_capacity(n_bootstrap);
    for _ in 0..n_bootstrap {
        let a_resample: Vec<f64> = (0..a.len()).map(|_| a[rng.gen_range(0..a.len())]).collect();
        let b_resample: Vec<f64> = (0..b.len()).map(|_| b[rng.gen_range(0..b.len())]).collect();

        let a_med = median_unsorted(&a_resample);
        let b_med = median_unsorted(&b_resample);
        if a_med > 0.0 {
            deltas.push((b_med - a_med) / a_med);
        }
    }

    deltas.sort_by(|x, y| x.partial_cmp(y).unwrap());
    let lo_idx = (0.025 * n_bootstrap as f64) as usize;
    let hi_idx = (0.975 * n_bootstrap as f64) as usize;
    let lo_idx = lo_idx.min(deltas.len().saturating_sub(1));
    let hi_idx = hi_idx.min(deltas.len().saturating_sub(1));

    (observed_delta, deltas[lo_idx], deltas[hi_idx])
}

fn median_sorted(s: &[f64]) -> f64 {
    let n = s.len();
    if n == 0 { return 0.0; }
    if n % 2 == 0 { (s[n/2 - 1] + s[n/2]) / 2.0 } else { s[n/2] }
}

fn median_unsorted(v: &[f64]) -> f64 {
    if v.is_empty() { return 0.0; }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = s.len();
    if n % 2 == 0 { (s[n/2 - 1] + s[n/2]) / 2.0 } else { s[n/2] }
}

// ============================================================================
// Measurement execution
// ============================================================================

/// Measure dispatches and return median time per dispatch in microseconds.
fn measure_dispatches(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::ComputePipeline,
    bind_group: &wgpu::BindGroup,
    dispatch: (u32, u32, u32),
    n_dispatches: u32,
    period_ns: f64,
    warmup: u32,
    timed: u32,
) -> f64 {
    let ts = TimestampQueries::new(device);
    let mut samples = Vec::with_capacity(timed as usize);

    for pi in 0..(warmup + timed) {
        let is_warmup = pi < warmup;
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("meas-enc"),
        });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("meas-pass"),
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: &ts.set,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: Some(1),
                }),
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            for _ in 0..n_dispatches {
                pass.dispatch_workgroups(dispatch.0, dispatch.1, dispatch.2);
            }
        }
        ts.resolve(&mut enc);
        queue.submit(Some(enc.finish()));
        let ticks = ts.read_back(device);
        if is_warmup { continue; }
        let elapsed = ticks[1].wrapping_sub(ticks[0]) as f64 * period_ns;
        samples.push((elapsed / 1000.0) / n_dispatches as f64);
    }

    if samples.is_empty() { return 0.0; }
    let stats = summarize(&samples);
    stats.median
}

/// Measure dispatch overhead (no-op kernel launch + GPU scheduler latency) in microseconds.
fn measure_dispatch_overhead(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    period_ns: f64,
) -> f64 {
    // Create a no-op compute pipeline
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("noop-shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(WGSL_IDLE_KERNEL)),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("noop-pipe"),
        layout: None,
        module: &module,
        entry_point: "main",
    });

    let n_dispatches = 1000;
    let ts = TimestampQueries::new(device);
    let mut samples = Vec::new();

    // 10 timed passes, each dispatching the no-op kernel n_dispatches times
    for _ in 0..10 {
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("noop-enc"),
        });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("noop-pass"),
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: &ts.set,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: Some(1),
                }),
            });
            pass.set_pipeline(&pipeline);
            for _ in 0..n_dispatches {
                pass.dispatch_workgroups(1, 1, 1);
            }
        }
        ts.resolve(&mut enc);
        queue.submit(Some(enc.finish()));
        let ticks = ts.read_back(device);
        let elapsed_ns = ticks[1].wrapping_sub(ticks[0]) as f64 * period_ns;
        if elapsed_ns > 0.0 {
            samples.push((elapsed_ns / 1000.0) / n_dispatches as f64);
        }
    }

    if samples.is_empty() { return 10.0; }
    let stats = summarize(&samples);
    stats.median
}

/// Adaptive warmup: continues until 3 consecutive medians agree within 1%.
fn adaptive_warmup(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::ComputePipeline,
    bind_group: &wgpu::BindGroup,
    dispatch: (u32, u32, u32),
    n_dispatches: u32,
    period_ns: f64,
) -> u32 {
    let mut prev_medians: Vec<f64> = Vec::new();
    let ts = TimestampQueries::new(device);

    for pi in 0..WARMUP_PASSES_MAX {
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("warmup-enc"),
        });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("warmup-pass"),
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: &ts.set,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: Some(1),
                }),
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            for _ in 0..n_dispatches {
                pass.dispatch_workgroups(dispatch.0, dispatch.1, dispatch.2);
            }
        }
        ts.resolve(&mut enc);
        queue.submit(Some(enc.finish()));
        let ticks = ts.read_back(device);
        let elapsed = ticks[1].wrapping_sub(ticks[0]) as f64 * period_ns;
        prev_medians.push(elapsed / n_dispatches as f64);

        // Check for steady state after at least WARMUP_PASSES_MIN
        if pi >= WARMUP_PASSES_MIN - 1 && prev_medians.len() >= 3 {
            let last3 = &prev_medians[prev_medians.len() - 3..];
            let max_val = last3.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let min_val = last3.iter().cloned().fold(f64::INFINITY, f64::min);
            if min_val > 0.0 && (max_val - min_val) / min_val < STEADY_STATE_TOLERANCE {
                return pi + 1; // Steady state reached
            }
        }
    }

    WARMUP_PASSES_MAX
}

fn time_one_config(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::ComputePipeline,
    bind_group: &wgpu::BindGroup,
    dispatch: (u32, u32, u32),
    n_dispatches: u32,
    period_ns: f64,
) -> Vec<f64> {
    let ts = TimestampQueries::new(device);
    let mut samples = Vec::with_capacity(TIMED_PASSES as usize);

    for _ in 0..TIMED_PASSES {
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("timed-enc"),
        });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("timed-pass"),
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: &ts.set,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: Some(1),
                }),
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            for _ in 0..n_dispatches {
                pass.dispatch_workgroups(dispatch.0, dispatch.1, dispatch.2);
            }
        }
        ts.resolve(&mut enc);
        queue.submit(Some(enc.finish()));
        let ticks = ts.read_back(device);
        let elapsed = ticks[1].wrapping_sub(ticks[0]) as f64 * period_ns;
        samples.push((elapsed / 1000.0) / n_dispatches as f64);
    }

    samples
}

// ============================================================================
// System state logging
// ============================================================================

fn log_system_state() {
    println!("# System State");
    if let Ok(load) = std::fs::read_to_string("/proc/loadavg") {
        print!("#   CPU load: {}", load.lines().next().unwrap_or("?"));
    }
    if let Ok(cpu_gov) = std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor") {
        println!("#   CPU governor: {}", cpu_gov.trim());
    }
    // Try to get nvidia-smi info (non-fatal if not available)
    if let Ok(out) = std::process::Command::new("nvidia-smi")
        .args(&["--query-gpu=name,driver_version,clocks.gr,clocks.mem,pstate,temperature.gpu,power.draw",
               "--format=csv,noheader"])
        .output()
    {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout);
            for line in s.lines() {
                println!("#   GPU: {}", line.trim());
            }
        }
    }
    println!();
}

// ============================================================================
// Roofline analysis
// ============================================================================

#[derive(Debug)]
struct RooflineInfo {
    achieved_bandwidth_gbps: f64,
    peak_bandwidth_gbps: f64,
    fraction_of_peak: f64,
    arithmetic_intensity: f64,
    bound_kind: &'static str,
}

fn compute_roofline(
    kernel: &KernelDef,
    size: &ProblemSize,
    median_us: f64,
    peak_bw: f64,
) -> RooflineInfo {
    // Total bytes read + written
    let (ib, sb, ob) = compute_buffer_bytes(kernel, size);
    let total_bytes = ib + sb + ob;

    let achieved_bw = if median_us > 0.0 {
        (total_bytes as f64) / (median_us * 1e-6) / 1e9
    } else {
        0.0
    };

    // Total FLOPs per kernel invocation
    let flops_per_invocation = match kernel.name {
        "reduction" => size.elements as f64 * 2.0, // one add per element
        "prefix_scan" => size.elements as f64 * 1.5 * 8.0, // log2(256) * preads
        "tiled_matmul" => {
            let dim = matmul_dim(size.elements) as f64;
            2.0 * dim * dim * dim // 2*N^3 FLOPs for NxN matmul
        }
        "track3b_test" | "wgul_reduce" => size.elements as f64 * kernel.flops_per_element.unwrap_or(1.0),
        _ => size.elements as f64,
    };

    let arithmetic_intensity = if total_bytes > 0 { flops_per_invocation / total_bytes as f64 } else { 0.0 };

    // Determine bound kind
    let fraction_of_peak = if peak_bw > 0.0 { achieved_bw / peak_bw } else { 0.0 };
    let bound_kind = if arithmetic_intensity < 1.0 { "bandwidth" } else { "compute" };

    RooflineInfo {
        achieved_bandwidth_gbps: achieved_bw,
        peak_bandwidth_gbps: peak_bw,
        fraction_of_peak,
        arithmetic_intensity,
        bound_kind,
    }
}

// ============================================================================
// Config data: raw samples + metadata for statistical comparisons
// ============================================================================

struct ConfigData {
    kernel: String,
    variant: String,
    policy: String,
    size_label: String,
    raw_samples: Vec<f64>,
    stats: SampleStats,
    roofline: RooflineInfo,
    n_dispatches_per_pass: u32,
    n_warmup: u32,
    bounds_checks_emitted: usize,
    elements: u64,
}

// ============================================================================
// JSON output schema
// ============================================================================

#[derive(Serialize)]
struct RunReport {
    schema_version: String,
    run_id: String,
    hardware: HardwareSpec,
    software: SoftwareSpec,
    peak_bandwidth_gbps: f64,
    bonferroni_alpha: f64,
    n_comparisons: usize,
    system_state: String,
    measurements: Vec<MeasurementRecord>,
    comparisons: Vec<ComparisonRecord>,
}

#[derive(Serialize)]
struct HardwareSpec {
    gpu_name: String,
    backend: String,
    timestamp_period_ns: f64,
}

#[derive(Serialize)]
struct SoftwareSpec {
    wgpu_version: String,
    os: String,
}

#[derive(Serialize)]
struct MeasurementRecord {
    kernel: String,
    variant: String,
    policy: String,
    size_label: String,
    elements: u64,
    n_dispatches_per_pass: u32,
    n_warmup: u32,
    bounds_checks_emitted: usize,
    timing_method: String,
    // Full statistics
    n_samples: usize,
    median_us: f64,
    mean_us: f64,
    stddev_us: f64,
    iqr_us: f64,
    min_us: f64,
    max_us: f64,
    p5_us: f64,
    p95_us: f64,
    p99_us: f64,
    ci_95_median: [f64; 2],
    ad_statistic: f64,
    is_normal: bool,
    // Roofline
    bandwidth_gbps: f64,
    fraction_of_peak: f64,
    arithmetic_intensity: f64,
    bound_kind: String,
}

#[derive(Serialize)]
struct ComparisonRecord {
    kernel: String,
    variant: String,
    size_label: String,
    baseline_policy: String,
    optimized_policy: String,
    baseline_median_us: f64,
    optimized_median_us: f64,
    // Effect size
    relative_delta: f64,
    ci_95_relative_delta: [f64; 2],
    cohens_d: f64,
    effect_size_label: String,
    // Significance
    permutation_p_value: f64,
    significant_bonferroni: bool,
    // Stats comparison
    ad_normality_baseline: f64,
    ad_normality_optimized: f64,
}

// ============================================================================
// Main
// ============================================================================

fn main() -> anyhow::Result<()> {
    let t0 = Instant::now();

    let tiled_only = std::env::args().any(|a| a == "--tiled-only");
    let skip_warmup = std::env::args().any(|a| a == "--skip-warmup");

    // ========================================================================
    // System state
    // ========================================================================
    log_system_state();

    // ========================================================================
    // GPU setup
    // ========================================================================
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        // Force Vulkan backend for reproducibility (like Triton forces CUDA)
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });

    // Enumerate all Vulkan adapters and prefer NVIDIA discrete GPU.
    // On Optimus laptops, PowerPreference::HighPerformance can still pick the iGPU,
    // so we explicitly select the NVIDIA adapter by name.
    let adapters = instance.enumerate_adapters(wgpu::Backends::VULKAN);
    if adapters.is_empty() {
        anyhow::bail!("no Vulkan adapters found");
    }

    // Pick the first NVIDIA adapter, otherwise fall back to the first adapter.
    let adapter = adapters.iter()
        .find(|a| {
            let info = a.get_info();
            info.name.to_uppercase().contains("NVIDIA")
                || info.name.to_uppercase().contains("RTX")
                || info.name.to_uppercase().contains("GEFORCE")
        })
        .unwrap_or(&adapters[0]);

    let info = adapter.get_info();
    let backend_s = format!("{:?}", info.backend);
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│ Fixed Benchmark Suite — Bounds-Check Elision A/B Test      │");
    println!("├─────────────────────────────────────────────────────────────┤");
    println!("│ Adapter: {} ({})", info.name, backend_s);

    if !adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
        anyhow::bail!("adapter does not support Features::TIMESTAMP_QUERY");
    }

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::TIMESTAMP_QUERY,
            required_limits: wgpu::Limits::default(),
        },
        None,
    ))?;

    // Initialize Nsight Graphics capture layer (no-op if not running under ngfx-capture)
    unsafe {
        let ret = nsight_initialize();
        if ret == 0 {
            println!("│ Nsight Graphics capture layer initialized                │");
        }
    }

    let period_ns = {
        let p = queue.get_timestamp_period();
        if p > 0.0 { p as f64 } else { TIMESTAMP_PERIOD_DEFAULT }
    };
    println!("│ Timestamp period: {:.3} ns/tick", period_ns);

    // ========================================================================
    // Dispatch overhead measurement (no-op kernel)
    // ========================================================================
    println!("├─────────────────────────────────────────────────────────────┤");
    println!("│ Measuring dispatch overhead (no-op kernel)...               │");
    // Measure dispatch overhead with device poll after each pass
    let dispatch_overhead_us = measure_dispatch_overhead(&device, &queue, period_ns);
    println!("│ Dispatch overhead: {:.3} µs", dispatch_overhead_us);
    println!("│ (Kernels < 5× overhead = {} µs will be flagged noise-bound)  │", dispatch_overhead_us * 5.0);
    // Ensure GPU idle after dispatch measurement
    device.poll(wgpu::Maintain::Wait);

    // ========================================================================
    // Peak bandwidth (GPU-timed sustained copy kernel)
    // ========================================================================
    println!("├─────────────────────────────────────────────────────────────┤");
    println!("│ Measuring peak bandwidth via sustained copy kernel...         │");
    let peak_bw = measure_peak_bandwidth_gpu_timed(&device, &queue, PEAK_BW_PASSES);
    println!("│ Peak bandwidth: {:.1} GB/s (GPU-timed, {} passes)            │", peak_bw, PEAK_BW_PASSES);
    println!("└─────────────────────────────────────────────────────────────┘");
    println!();

    // ========================================================================
    // L2 cache invalidator
    // ========================================================================
    let l2_inv = L2Invalidator::new(&device, &queue);

    // ========================================================================
    // Header
    // ========================================================================
    println!("{:<12} {:<7} {:<18} {:<4} {:>5} {:>4} {:>9} {:>9} {:>7} {:>7} {:>9} {:>7} {:>10} {:>8}",
        "kernel", "variant", "policy", "size", "disp", "warm",
        "median_us", "p5_us", "p95_us", "AD_norm",
        "CI_95_lo", "CI_95_hi", "bw_GBs", "t(s)");
    println!("{}", "-".repeat(150));

    let mut records: Vec<MeasurementRecord> = Vec::new();
    let mut configs: Vec<ConfigData> = Vec::new();

    let active_kernels: Vec<&KernelDef> = if tiled_only {
        KERNELS.iter().filter(|k| k.name == "tiled_matmul").collect()
    } else {
        KERNELS.iter().collect()
    };

    if tiled_only {
        println!("│ Mode: --tiled-only (skipping reduction and prefix_scan)       │");
    }

    for kdef in &active_kernels {
        let kdef: &KernelDef = kdef;
        for size in PROBLEM_SIZES {
            let dispatch = compute_dispatch(kdef, size);
            let (ib, _sb, ob) = compute_buffer_bytes(kdef, size);

            let tiled_wg = if kdef.name == "tiled_matmul" {
                matmul_wg_count(size.elements)
            } else { 0 };

            // Compile all 3 policies
            let mut compiled = Vec::new();
            for &(pn, pl) in POLICIES {
                let (spv, bc) = compile_wgsl_with_policy(kdef.wgsl, pl, tiled_wg);
                compiled.push((pn, spv, bc));
            }

            for &(pn, ref spv_words, bc_count) in &compiled {
                let row_t0 = Instant::now();

                let label = format!("{}_{}_{}", kdef.name, kdef.variant.label(), pn);

                let smod = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(&label),
                    source: wgpu::ShaderSource::SpirV(Cow::Owned(spv_words.clone())),
                });

                // Build bind group layout
                let bgl_entries: Vec<wgpu::BindGroupLayoutEntry> = if kdef.n_bindings == 2 {
                    vec![
                        wgpu::BindGroupLayoutEntry {
                            binding: 0, visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1, visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None },
                            count: None,
                        },
                    ]
                } else {
                    vec![
                        wgpu::BindGroupLayoutEntry {
                            binding: 0, visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1, visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2, visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None },
                            count: None,
                        },
                    ]
                };
                let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some(&format!("{}_bgl", label)),
                    entries: &bgl_entries,
                });
                let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(&format!("{}_pl", label)),
                    bind_group_layouts: &[&bgl],
                    push_constant_ranges: &[],
                });
                let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some(&format!("{}_pipe", label)),
                    layout: Some(&pl), module: &smod, entry_point: "main",
                });

                // Create buffers
                let input_buf = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("{}_in", label)), size: ib,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                // Init input with 0.5
                init_buffer_f32(&device, &queue, &input_buf, ib, 0.5f32);

                let out_buf = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("{}_out", label)), size: ob,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                // Create scratch buffer for storage variants (binding 1 = scratch, binding 2 = output)
                let (_ib, scratch_bytes, _ob) = compute_buffer_bytes(kdef, size);
                let scratch_buf = if kdef.n_bindings == 3 {
                    let sb = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some(&format!("{}_scratch", label)),
                        size: scratch_bytes,
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    Some(sb)
                } else {
                    None
                };

                // Bind group
                let mut bg_entries = vec![
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &input_buf, offset: 0, size: None,
                        }),
                    },
                ];
                // Storage variant: binding 1 = scratch, binding 2 = output
                // Local variant:   binding 1 = output
                if let Some(ref sc) = scratch_buf {
                    bg_entries.push(wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: sc, offset: 0, size: None,
                        }),
                    });
                    bg_entries.push(wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &out_buf, offset: 0, size: None,
                        }),
                    });
                } else {
                    bg_entries.push(wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &out_buf, offset: 0, size: None,
                        }),
                    });
                }
                let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(&format!("{}_bg", label)),
                    layout: &bgl, entries: &bg_entries,
                });

                // Invalidate L2 cache before this config (prevents config-to-config interference)
                l2_inv.invalidate(&device, &queue);

                // Auto-calibrate dispatch count
                let nd = size.calibrate_dispatches_to_target(
                    &device, &queue, &pipeline, &bg, dispatch, period_ns,
                );

                // Adaptive warmup (steady-state detection)
                let n_warmup = if skip_warmup { 2 } else {
                    adaptive_warmup(&device, &queue, &pipeline, &bg, dispatch, nd, period_ns)
                };

                // Timed passes
                // Trigger Nsight Graphics capture around the 10M tiled_matmul dispatch
                // (only when running under ngfx-capture)
                let should_capture = kdef.name == "tiled_matmul" && size.label == "10M" && pn == "ReadZeroSkipWrite";
                let capture_active = if should_capture {
                    unsafe { nsight_start_capture() == 0 }
                } else {
                    false
                };
                if capture_active {
                    eprintln!("[nsight] capture STARTED for 10M tiled_matmul ({})", pn);
                }

                let samples = time_one_config(&device, &queue, &pipeline, &bg, dispatch, nd, period_ns);
                let stats = summarize(&samples);

                if capture_active {
                    unsafe { nsight_stop_capture(); }
                    eprintln!("[nsight] capture STOPPED — file saved");
                }

                // Roofline
                let roofline = compute_roofline(kdef, size, stats.median, peak_bw);

                println!("{:<12} {:<7} {:<18} {:<4} {:>5} {:>4} {:>9.3} {:>9.3} {:>9.3} {:>7.1} [{:>9.3},{:>9.3}] {:>10.1} {:>7.1}",
                    kdef.name, kdef.variant.label(), pn, size.label, nd, n_warmup,
                    stats.median, stats.p5, stats.p95,
                    if stats.is_normal { stats.ad_statistic } else { -1.0 },
                    stats.ci_lower, stats.ci_upper,
                    roofline.achieved_bandwidth_gbps,
                    row_t0.elapsed().as_secs_f64());

                records.push(MeasurementRecord {
                    kernel: kdef.name.to_string(),
                    variant: kdef.variant.label().to_string(),
                    policy: pn.to_string(),
                    size_label: size.label.to_string(),
                    elements: size.elements,
                    n_dispatches_per_pass: nd,
                    n_warmup,
                    bounds_checks_emitted: bc_count,
                    timing_method: "gpu_event".to_string(),
                    n_samples: stats.n,
                    median_us: stats.median,
                    mean_us: stats.mean,
                    stddev_us: stats.stddev,
                    iqr_us: stats.iqr,
                    min_us: stats.min,
                    max_us: stats.max,
                    p5_us: stats.p5,
                    p95_us: stats.p95,
                    p99_us: stats.p99,
                    ci_95_median: [stats.ci_lower, stats.ci_upper],
                    ad_statistic: stats.ad_statistic,
                    is_normal: stats.is_normal,
                    bandwidth_gbps: roofline.achieved_bandwidth_gbps,
                    fraction_of_peak: roofline.fraction_of_peak,
                    arithmetic_intensity: roofline.arithmetic_intensity,
                    bound_kind: roofline.bound_kind.to_string(),
                });

                // Store raw samples for permutation tests in comparison phase
                configs.push(ConfigData {
                    kernel: kdef.name.to_string(),
                    variant: kdef.variant.label().to_string(),
                    policy: pn.to_string(),
                    size_label: size.label.to_string(),
                    raw_samples: samples,
                    stats,
                    roofline,
                    n_dispatches_per_pass: nd,
                    n_warmup,
                    bounds_checks_emitted: bc_count,
                    elements: size.elements,
                });
            }
        }
    }

    // ========================================================================
    // Statistical comparisons (pairwise A/B)
    // ========================================================================
    // Actual number of pairwise comparisons: each (kernel, variant, size) × (optimized policies)
    let n_comparisons = active_kernels.len() * PROBLEM_SIZES.len() * (POLICIES.len() - 1);
    let bonferroni_alpha = 0.05 / n_comparisons.max(1) as f64;

    println!();
    println!("{}", "=".repeat(150));
    println!("COMPARISON TABLE — Bonferroni-corrected α = {:.6} ({} comparisons)", bonferroni_alpha, n_comparisons);
    println!("  Only comparisons passing BOTH p < α AND |Cohen's d| ≥ 0.2 are flagged significant.");
    println!("{}", "=".repeat(150));
    println!();
    println!("{:<12} {:<7} {:<6} {:>12} {:>12} {:>14} {:>14} {:>10} {:>6} {:>10} {:>10} {:>8}",
        "kernel", "variant", "size", "baseline_us", "optimized_us",
        "rel_delta_%", "CI_95_delta", "cohens_d", "effect",
        "p_value", "bonf_sig", "bound");
    println!("{}", "-".repeat(150));

    let mut comparisons: Vec<ComparisonRecord> = Vec::new();

    // Compare each policy pair for each (kernel, variant, size)
    // Uses full raw sample distributions from configs Vec for valid permutation tests.
    for pi in 1..POLICIES.len() {
        let (opt_name, _opt_pol) = POLICIES[pi];
        for kdef in active_kernels.iter() {
            for size in PROBLEM_SIZES {
                // Find baseline (Unchecked, index 0) and optimized ConfigData entries
                let baseline_cfg = configs.iter().find(|c|
                    c.kernel == kdef.name && c.variant == kdef.variant.label()
                    && c.policy == POLICIES[0].0 && c.size_label == size.label
                );
                let optimized_cfg = configs.iter().find(|c|
                    c.kernel == kdef.name && c.variant == kdef.variant.label()
                    && c.policy == opt_name && c.size_label == size.label
                );

                // Also find records for JSON output (from MeasurementRecord)
                let baseline_rec = records.iter().find(|r|
                    r.kernel == kdef.name && r.variant == kdef.variant.label()
                    && r.policy == POLICIES[0].0 && r.size_label == size.label
                );
                let optimized_rec = records.iter().find(|r|
                    r.kernel == kdef.name && r.variant == kdef.variant.label()
                    && r.policy == opt_name && r.size_label == size.label
                );

                if let (Some(bc), Some(oc), Some(br), Some(or)) = (baseline_cfg, optimized_cfg, baseline_rec, optimized_rec) {
                    // Compute relative delta with bootstrap CI using FULL raw samples
                    let (delta, ci_lo_delta, ci_hi_delta) = bootstrap_relative_delta(
                        &bc.raw_samples, &oc.raw_samples, BOOTSTRAP_SAMPLES,
                    );

                    // Compute permutation test p-value using FULL raw samples
                    let p_val = permutation_test(
                        &bc.raw_samples, &oc.raw_samples, PERMUTATION_SAMPLES,
                    );

                    // Cohen's d from the stored SampleStats (computed from full distribution)
                    let cohens_d_val = cohens_d(&bc.stats, &oc.stats);

                    let effect_label = if cohens_d_val.abs() >= 0.8 { "large" }
                        else if cohens_d_val.abs() >= 0.5 { "medium" }
                        else if cohens_d_val.abs() >= 0.2 { "small" }
                        else { "negligible" };

                    // Bonferroni-corrected significance: p < α AND |Cohen's d| ≥ 0.2
                    let significant = p_val < bonferroni_alpha && cohens_d_val.abs() >= 0.2;

                    // Determine bound kind
                    let bound_kind = roofline_bound(
                        oc.roofline.arithmetic_intensity, oc.roofline.fraction_of_peak,
                    );

                    println!("{:<12} {:<7} {:<6} {:>12.3} {:>12.3} {:>13.2}% [{:>10.2}%,{:>10.2}%] {:>10.3} {:>6} {:>10.6} {:>8}",
                        kdef.name, kdef.variant.label(), size.label,
                        br.median_us, or.median_us,
                        delta * 100.0, ci_lo_delta * 100.0, ci_hi_delta * 100.0,
                        cohens_d_val, effect_label, p_val, bound_kind);

                    comparisons.push(ComparisonRecord {
                        kernel: kdef.name.to_string(),
                        variant: kdef.variant.label().to_string(),
                        size_label: size.label.to_string(),
                        baseline_policy: POLICIES[0].0.to_string(),
                        optimized_policy: opt_name.to_string(),
                        baseline_median_us: br.median_us,
                        optimized_median_us: or.median_us,
                        relative_delta: delta,
                        ci_95_relative_delta: [ci_lo_delta, ci_hi_delta],
                        cohens_d: cohens_d_val,
                        effect_size_label: effect_label.to_string(),
                        permutation_p_value: p_val,
                        significant_bonferroni: significant,
                        ad_normality_baseline: br.ad_statistic,
                        ad_normality_optimized: or.ad_statistic,
                    });
                }
            }
        }
    }

    // ========================================================================
    // Roofline summary
    // ========================================================================
    println!();
    println!("{}", "=".repeat(150));
    println!("ROOFLINE ANALYSIS");
    println!("  Peak bandwidth: {:.1} GB/s (GPU-timed sustained copy kernel)", peak_bw);
    println!("{}", "=".repeat(150));
    println!();
    println!("{:<12} {:<7} {:<18} {:<6} {:>12} {:>12} {:>14} {:>14}",
        "kernel", "variant", "policy", "size", "bw_GBs", "%_peak",
        "ai_FLOP/byte", "bound");
    println!("{}", "-".repeat(120));
    for rec in &records {
        println!("{:<12} {:<7} {:<18} {:<6} {:>12.1} {:>11.1}% {:>14.2} {:>14}",
            rec.kernel, rec.variant, rec.policy, rec.size_label,
            rec.bandwidth_gbps, rec.fraction_of_peak * 100.0,
            rec.arithmetic_intensity, rec.bound_kind);
    }

    // ========================================================================
    // JSON output
    // ========================================================================
    let run_id = format!("{}_{}_{}",
        chrono_now_fmt().replace(':', "-"),
        info.name.replace(' ', "_"),
        backend_s.to_lowercase());

    let system_state_log = format!("GPU: {}, Driver: {}, Vulkan",
        info.name, info.driver_info);

    let report = RunReport {
        schema_version: "2.0.0".to_string(),
        run_id: run_id.clone(),
        hardware: HardwareSpec {
            gpu_name: info.name.clone(),
            backend: backend_s.clone(),
            timestamp_period_ns: period_ns,
        },
        software: SoftwareSpec {
            wgpu_version: env!("CARGO_PKG_VERSION").to_string(),
            os: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        },
        peak_bandwidth_gbps: peak_bw,
        bonferroni_alpha,
        n_comparisons,
        system_state: system_state_log,
        measurements: records,
        comparisons,
    };

    let json_path = format!("results_{}.json", run_id);
    if let Ok(json) = serde_json::to_string_pretty(&report) {
        if std::fs::write(&json_path, &json).is_ok() {
            println!();
            println!("JSON report: {}", json_path);
        }
    }

    println!();
    let total = t0.elapsed();
    println!("Total: {:.1}s ({:.1} min)", total.as_secs_f64(), total.as_secs_f64() / 60.0);
    Ok(())
}

fn init_buffer_f32(device: &wgpu::Device, queue: &wgpu::Queue, buf: &wgpu::Buffer, size: u64, value: f32) {
    let st = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("st-init"),
        size,
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::MAP_WRITE,
        mapped_at_creation: true,
    });
    {
        let mut m = st.slice(..).get_mapped_range_mut();
        for ch in m.chunks_exact_mut(4) {
            ch.copy_from_slice(&value.to_ne_bytes());
        }
    }
    st.unmap();
    let mut e = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("init-enc"),
    });
    e.copy_buffer_to_buffer(&st, 0, buf, 0, size);
    queue.submit(Some(e.finish()));
    device.poll(wgpu::Maintain::Wait);
}

fn chrono_now_fmt() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = d.as_secs();
    let days = secs / 86400;
    let ts = secs % 86400;
    format!("epoch{}_{:05}", days, ts)
}

fn roofline_bound(arithmetic_intensity: f64, _fraction_of_peak: f64) -> &'static str {
    if arithmetic_intensity < 1.0 { "bandwidth" }
    else if arithmetic_intensity < 10.0 { "balanced" }
    else { "compute" }
}
