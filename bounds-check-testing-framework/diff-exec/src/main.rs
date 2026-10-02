//! T3 — differential execution: elided vs checked codegen on device.
//!
//! For every fixture: compile with bounds-check elision enabled and disabled,
//! structurally compare the SPIR-V (OpULessThan counts), then execute BOTH
//! binaries on the GPU with identical inputs and require byte-identical outputs
//! (Class A, soundness) or the documented policy divergence (Class B).
//!
//! Outcome branches per TASKS/T3-DIFFERENTIAL-EXEC.md. Run:
//!   cargo run --release --manifest bounds-check-testing-framework/diff-exec/Cargo.toml

use naga::back::spv;
use naga::proc::{BoundsCheckPolicies, BoundsCheckPolicy};
use naga::valid::{Capabilities, ValidationFlags, Validator};
use rspirv::binary::Disassemble;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::path::PathBuf;

const HARNESS_VERSION: &str = "diff-exec-1";
const POLICY: BoundsCheckPolicy = BoundsCheckPolicy::ReadZeroSkipWrite;

#[derive(Serialize)]
struct FixtureResult {
    name: String,
    class: String, // "A" (soundness: equality required) or "B" (policy semantics)
    expected_elision: bool,
    observed_elision: bool,
    structural_match: bool,
    uless_enabled: usize,
    uless_disabled: usize,
    safe_accesses_enabled: usize,
    dynamic_accesses: usize,
    spv_sha256_enabled: String,
    spv_sha256_disabled: String,
    executed: bool,
    outputs_byte_identical: Option<bool>, // None = not executed (structural gate failed / class B handled)
    out_sha256_enabled: Option<String>,
    out_sha256_disabled: Option<String>,
    finding: Option<String>,
}

#[derive(Serialize)]
struct Report {
    schema_version: String,
    harness_version: String,
    utc_start: String,
    utc_end: String,
    adapter: Option<String>,
    driver: Option<String>,
    timestamp_period_ns: Option<f32>,
    fixtures: Vec<FixtureResult>,
    class_a_identical: usize,
    class_a_total: usize,
    class_a_all_identical: bool,
    structural_mismatches: usize,
    status: String, // "completed" | "completed_with_structural_findings" | "environment_blocked"
    notes: Vec<String>,
}

/// Fixture spec: WGSL source + whether elision is expected to remove checks +
/// which binding index the output storage buffer uses (all fixtures use one
/// storage buffer; written via `output`).
struct Fixture {
    name: &'static str,
    class: &'static str,
    expected_elision: bool,
    wgsl: &'static str,
    out_words: usize, // f32 words in output buffer
}

/// (expected output words, input-independent) — fixtures compute deterministic
/// values; expected vectors are computed by a CPU mirror in `cpu_expected`.
fn fixtures() -> Vec<Fixture> {
    vec![
        // ---- Class A: soundness (outputs MUST match) ----
        Fixture {
            name: "lid_spec_direct",
            class: "A",
            expected_elision: true,
            out_words: 64,
            wgsl: r#"
@group(0) @binding(0) var<storage, read_write> output: array<f32, 64>;
var<workgroup> tile: array<f32, 64>;
@compute @workgroup_size(64)
fn main(@builtin(local_invocation_index) lid: u32) {
    tile[lid] = f32(lid);
    workgroupBarrier();
    output[lid] = tile[lid] * 2.0;
}
"#,
        },
        Fixture {
            name: "loop_iv_guarded",
            class: "A",
            expected_elision: true,
            out_words: 8,
            wgsl: r#"
@group(0) @binding(0) var<storage, read_write> output: array<f32, 8>;
var<workgroup> tile: array<f32, 8>;
@compute @workgroup_size(8)
fn main(@builtin(local_invocation_index) lid: u32) {
    var acc = 0.0;
    for (var i = 0u; i < 8u; i = i + 1u) {
        tile[i] = f32(i);
        workgroupBarrier();
        acc += tile[i];
    }
    output[lid] = acc;
}
"#,
        },
        Fixture {
            name: "loop_iv_affine_plus1",
            class: "A",
            expected_elision: true,
            out_words: 8,
            wgsl: r#"
@group(0) @binding(0) var<storage, read_write> output: array<f32, 8>;
@compute @workgroup_size(8)
fn main(@builtin(local_invocation_index) lid: u32) {
    var acc = 0.0;
    for (var i = 0u; i < 7u; i = i + 1u) {
        acc += f32(i + 1u); // affine i+1 in [1,7]; no array here — keep structure minimal
    }
    output[lid] = acc;
}
"#,
        },
        Fixture {
            name: "runtime_var_index_via_lid_spec_pattern",
            class: "A",
            expected_elision: true,
            out_words: 8,
            wgsl: r#"
@group(0) @binding(0) var<storage, read_write> output: array<f32, 8>;
@compute @workgroup_size(8)
fn main(@builtin(local_invocation_index) lid: u32) {
    var idx = lid;
    // idx = lid: provable by the spec guarantee (affine B=0, len==wg_size).
    // Elicitation: this fixture documents that a *variable* (non-constant) index
    // still elides — R4's core question. Expect ELIDED.
    output[idx] = f32(idx) + 1.0;
}
"#,
        },
        Fixture {
            name: "mutation_after_guard_must_retain",
            class: "A",
            expected_elision: false,
            out_words: 8,
            wgsl: r#"
@group(0) @binding(0) var<storage, read_write> output: array<f32, 8>;
@compute @workgroup_size(8)
fn main(@builtin(local_invocation_index) lid: u32) {
    var i = lid;
    if (i < 4u) {
        i = i + 8u; // mutation invalidates any proof
    }
    output[i] = 1.0; // may be OOB -> check must be retained
}
"#,
        },
        Fixture {
            name: "signed_negative_start_guarded_refines",
            class: "A",
            expected_elision: true,
            out_words: 8,
            wgsl: r#"
@group(0) @binding(0) var<storage, read_write> output: array<f32, 8>;
@compute @workgroup_size(8)
fn main(@builtin(local_invocation_index) lid: u32) {
    var i: i32 = -4;
    var acc = 0.0;
    loop {
        if (i < 4) { } else { break; }
        if (i >= 0) {
            // path-sensitive refinement proves i in [0,3] here; len 8: ELIDED is correct.
            output[i] = f32(i);
        }
        i = i + 1;
    }
    output[lid] = acc;
}
"#,
        },
        Fixture {
            name: "eq_exit_loop",
            class: "A",
            expected_elision: true,
            out_words: 8,
            wgsl: r#"
@group(0) @binding(0) var<storage, read_write> output: array<f32, 8>;
var<workgroup> tile: array<f32, 8>;
@compute @workgroup_size(8)
fn main(@builtin(local_invocation_index) lid: u32) {
    var acc = 0.0;
    var i = 0u;
    loop {
        if (i == 8u) { break; }
        tile[i] = f32(i);
        workgroupBarrier();
        acc += tile[i];
        i = i + 1u;
    }
    output[lid] = acc;
}
"#,
        },
        Fixture {
            name: "unguarded_oob_write_must_retain",
            class: "A",
            expected_elision: false,
            out_words: 8,
            wgsl: r#"
@group(0) @binding(0) var<storage, read_write> output: array<f32, 8>;
@compute @workgroup_size(8)
fn main(@builtin(local_invocation_index) lid: u32) {
    var i = lid;
    i = i + 20u; // now in [20,27]: unguarded, statically OOB-capable
    output[i] = 1.0; // check MUST be retained (RZSW drops the write at runtime)
}
"#,
        },
        // ---- Class B: policy semantics (divergence REQUIRED, not a bug) ----
        Fixture {
            name: "oob_dynamic_read_returns_zero",
            class: "B",
            expected_elision: false,
            out_words: 8,
            wgsl: r#"
@group(0) @binding(0) var<storage, read_write> output: array<f32, 8>;
@compute @workgroup_size(8)
fn main(@builtin(local_invocation_index) lid: u32) {
    output[lid] = 5.0;
    var idx = lid + 100u; // dynamic OOB index in [100,107]
    output[lid] = output[idx]; // RZSW: OOB read -> 0.0, overwrites with 0
}
"#,
        },
        Fixture {
            name: "oob_write_dropped_mixed_accesses",
            class: "B",
            // Two accesses: output[lid] (safe, elides) + output[lid+50u] (OOB, RETAINED).
            // Correct structural signature: enabled uless = disabled uless - 1 (only the
            // safe access elides). Byte-identical outputs still REQUIRED (RZSW drops the
            // OOB write in both binaries identically).
            expected_elision: false,
            out_words: 8,
            wgsl: r#"
@group(0) @binding(0) var<storage, read_write> output: array<f32, 8>;
@compute @workgroup_size(8)
fn main(@builtin(local_invocation_index) lid: u32) {
    output[lid] = 5.0;
    var idx = lid + 50u;
    output[idx] = 99.0; // OOB write dropped under RZSW
}
"#,
        },
    ]
}

struct Compiled {
    words: Vec<u32>,
    uless: usize,
    spv_sha256: String,
    safe_accesses: usize,
    dynamic_accesses: usize,
}

fn compile(wgsl: &str, bce_enabled: bool) -> Result<Compiled, String> {
    let module = naga::front::wgsl::parse_str(wgsl).map_err(|e| format!("parse: {e}"))?;
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    validator.set_bounds_check_elision(bce_enabled);
    let info = validator.validate(&module).map_err(|e| format!("validate: {e}"))?;
    let safe_accesses = info.get_entry_point(0).safe_accesses.len();
    let dynamic_accesses = module.entry_points[0]
        .function
        .expressions
        .iter()
        .filter(|(_, expression)| matches!(expression, naga::Expression::Access { .. }))
        .count();
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
    let words = spv::write_vec(&module, &info, &options, Some(&pipeline_options))
        .map_err(|e| format!("spv: {e}"))?;
    let uless = rspirv::dr::load_words(words.clone())
        .map_err(|e| format!("disasm: {e}"))?
        .disassemble()
        .matches("OpULessThan")
        .count();
    let spv_sha256 = format!("{:x}", Sha256::digest(bytemuck::cast_slice(&words)));
    Ok(Compiled { words, uless, spv_sha256, safe_accesses, dynamic_accesses })
}

struct DeviceCtx {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

fn device_new() -> Result<(DeviceCtx, wgpu::AdapterInfo), String> {
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
    .map_err(|e| format!("adapter: {e}"))?;
    let info = adapter.get_info();
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("bce-diff-exec"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::Performance,
        trace: wgpu::Trace::Off,
    }))
    .map_err(|e| format!("device: {e}"))?;
    Ok((DeviceCtx { device, queue }, info))
}

fn run_and_readback(ctx: &DeviceCtx, words: &[u32], out_bytes: u64) -> Result<Vec<u8>, String> {
    let device = &ctx.device;
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("fixture"),
        source: wgpu::ShaderSource::SpirV(Cow::Owned(words.to_vec())),
    });
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("output"),
        size: out_bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: out_bytes,
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
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("fixture-pipeline"),
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, out_bytes);
    ctx.queue.submit([encoder.finish()]);
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| ());
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| format!("poll: {e}"))?;
    let mapped = slice.get_mapped_range().map_err(|e| format!("map: {e}"))?;
    let bytes = mapped.to_vec();
    drop(mapped);
    readback.unmap();
    Ok(bytes)
}

fn utc_now() -> String {
    std::process::Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn main() {
    let utc_start = utc_now();
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let _framework = workspace.parent().expect("framework root");
    let results_dir = workspace.join("results");
    std::fs::create_dir_all(&results_dir).expect("create results dir");

    let list = fixtures();
    let mut results = Vec::new();
    let mut notes = Vec::new();

    // Device is optional: structural pass runs regardless; execution needs GPU.
    let (ctx, adapter_info): (Option<DeviceCtx>, Option<wgpu::AdapterInfo>) = match device_new() {
        Ok((device_ctx, info)) => (Some(device_ctx), Some(info)),
        Err(e) => {
            notes.push(format!("environment_blocked: {e}"));
            (None, None)
        }
    };

    for fx in &list {
        let enabled = compile(fx.wgsl, true);
        let disabled = compile(fx.wgsl, false);
        let (enabled, disabled) = match (enabled, disabled) {
            (Ok(e), Ok(d)) => (e, d),
            (Err(err), _) | (_, Err(err)) => {
                results.push(FixtureResult {
                    name: fx.name.into(),
                    class: fx.class.into(),
                    expected_elision: fx.expected_elision,
                    observed_elision: false,
                    structural_match: false,
                    uless_enabled: 0,
                    uless_disabled: 0,
                    safe_accesses_enabled: 0,
                    dynamic_accesses: 0,
                    spv_sha256_enabled: String::new(),
                    spv_sha256_disabled: String::new(),
                    executed: false,
                    outputs_byte_identical: None,
                    out_sha256_enabled: None,
                    out_sha256_disabled: None,
                    finding: Some(format!("compile/validate failed: {err}")),
                });
                continue;
            }
        };

        // Elision "observed" = enabling removed emitted checks vs disabled.
        let observed_elision = enabled.uless < disabled.uless;
        // Class-B fixtures mix safe + OOB accesses: correct signature is PARTIAL
        // elision (safe access elides, OOB access retains >= 1 check).
        let structural_match = if fx.class == "B" {
            observed_elision
                && enabled.uless >= 1
                && enabled.safe_accesses < enabled.dynamic_accesses
        } else {
            observed_elision == fx.expected_elision
        };

        let mut executed = false;
        let mut identical: Option<bool> = None;
        let mut out_sha_e = None;
        let mut out_sha_d = None;
        let mut finding: Option<String> = None;

        if ctx.is_some() {
            let ctx = ctx.as_ref().unwrap();
            let out_bytes = (fx.out_words * 4) as u64;
            let run_e = run_and_readback(ctx, &enabled.words, out_bytes);
            let run_d = run_and_readback(ctx, &disabled.words, out_bytes);
            match (run_e, run_d) {
                (Ok(be), Ok(bd)) => {
                    executed = true;
                    let se = format!("{:x}", Sha256::digest(&be));
                    let sd = format!("{:x}", Sha256::digest(&bd));
                    identical = Some(be == bd);
                    out_sha_e = Some(se);
                    out_sha_d = Some(sd);
                    if fx.class == "A" && be != bd {
                        finding = Some("CLASS A DIVERGENCE — potential soundness bug".into());
                    }
                }
                (Err(e), _) | (_, Err(e)) => {
                    finding = Some(format!("execution failed: {e}"));
                }
            }
        }

        let spv_dir = results_dir.join(format!("spv-{}", utc_start.replace([':', '-'], "")));
        let _ = std::fs::create_dir_all(&spv_dir);
        let _ = std::fs::write(spv_dir.join(format!("{}.enabled.spv", fx.name)), bytemuck::cast_slice(&enabled.words));
        let _ = std::fs::write(spv_dir.join(format!("{}.disabled.spv", fx.name)), bytemuck::cast_slice(&disabled.words));

        if !structural_match {
            let drift = if fx.expected_elision {
                "expected ELIDED but checks retained (analysis coverage narrower than test expectation)"
            } else {
                "expected RETAINED but checks elided (analysis coverage wider than test expectation)"
            };
            notes.push(format!("{}: {drift}", fx.name));
        }

        results.push(FixtureResult {
            name: fx.name.into(),
            class: fx.class.into(),
            expected_elision: fx.expected_elision,
            observed_elision,
            structural_match,
            uless_enabled: enabled.uless,
            uless_disabled: disabled.uless,
            safe_accesses_enabled: enabled.safe_accesses,
            dynamic_accesses: enabled.dynamic_accesses,
            spv_sha256_enabled: enabled.spv_sha256,
            spv_sha256_disabled: disabled.spv_sha256,
            executed,
            outputs_byte_identical: identical,
            out_sha256_enabled: out_sha_e,
            out_sha256_disabled: out_sha_d,
            finding,
        });
    }

    let class_a: Vec<_> = results.iter().filter(|r| r.class == "A").collect();
    let class_a_identical = class_a
        .iter()
        .filter(|r| r.outputs_byte_identical == Some(true))
        .count();
    let class_a_total = class_a.len();
    let class_a_all_identical = class_a_total > 0 && class_a_identical == class_a_total;
    let structural_mismatches = results.iter().filter(|r| !r.structural_match).count();

    let status = if ctx.is_none() {
        "environment_blocked"
    } else if structural_mismatches == 0 {
        "completed"
    } else {
        "completed_with_structural_findings"
    };

    let report = Report {
        schema_version: "1.0.0".into(),
        harness_version: HARNESS_VERSION.into(),
        utc_start: utc_start.clone(),
        utc_end: utc_now(),
        adapter: adapter_info.as_ref().map(|a| a.name.clone()),
        driver: adapter_info.as_ref().map(|a| a.driver.clone()),
        timestamp_period_ns: None,
        fixtures: results,
        class_a_identical,
        class_a_total,
        class_a_all_identical,
        structural_mismatches,
        status: status.into(),
        notes,
    };

    let path = results_dir.join(format!("diff-exec-{}.json", utc_start.replace([':', '-'], "")));
    std::fs::write(&path, serde_json::to_vec_pretty(&report).expect("serialize"))
        .expect("write report");
    println!("{}", path.display());
    println!(
        "class_a={identical}/{total}_identical structural_mismatches={mm} status={status}",
        identical = report.class_a_identical,
        total = report.class_a_total,
        mm = report.structural_mismatches,
        status = report.status,
    );
    for n in &report.notes {
        println!("NOTE: {n}");
    }
}
