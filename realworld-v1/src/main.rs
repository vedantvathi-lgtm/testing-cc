//! realworld-1: the paired-vulkan-4 method (spec 004) applied to real-world shaders and host
//! logic (spec 005). One process = one workload = one independent run. Four slots per round in a
//! rotating order: `no_bce`, `bce`, `bce_dup` (in-round negative control) and `unchecked`.
//! Shaders are compiled with wgpu-hal's production Vulkan options and submitted with
//! `create_shader_module_passthrough`, so the driver receives exactly the recorded words.
//! Correctness oracle: no_bce, bce and bce_dup outputs must be byte-identical, before and after
//! timing (elision may not change any output); the unchecked slot's agreement is reported.

mod blur;
mod compile;
mod vello;
mod workload;

use compile::{compile, sha256, Static, SLOTS};
use serde::Serialize;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use workload::Workload;

const HARNESS_VERSION: &str = "realworld-1";
const BOOTSTRAP: usize = 10_000;
const TARGET_SAMPLE_NS: f64 = 8_000_000.0;

#[derive(Serialize, Clone)]
struct Round {
    round: usize,
    order: Vec<String>,
    ns: [f64; 4],
}

#[derive(Serialize)]
struct Estimate {
    median: f64,
    ci95_lo: f64,
    ci95_hi: f64,
}

fn median(v: &[f64]) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    let n = s.len();
    if n % 2 == 1 { s[n / 2] } else { (s[n / 2 - 1] + s[n / 2]) / 2.0 }
}

fn bootstrap(values: &[f64], seed: u64) -> Estimate {
    let mut x = seed | 1;
    let mut next = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let mut meds: Vec<f64> = (0..BOOTSTRAP).map(|_| median(&(0..values.len()).map(|_| values[next() as usize % values.len()]).collect::<Vec<_>>())).collect();
    meds.sort_by(f64::total_cmp);
    Estimate { median: median(values), ci95_lo: meds[BOOTSTRAP * 25 / 1000], ci95_hi: meds[BOOTSTRAP * 975 / 1000] }
}

fn sh(program: &str, args: &[&str]) -> String {
    Command::new(program).args(args).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default()
}

fn read(path: &str) -> String {
    std::fs::read_to_string(path).map(|s| s.trim().to_string()).unwrap_or_default()
}

pub fn git(path: &Path, args: &[&str]) -> String {
    Command::new("git").args(args).current_dir(path).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default()
}

fn naga_diff_sha(wgpu: &Path) -> String {
    let out = Command::new("git").args(["diff", "HEAD", "--", "naga/src"]).current_dir(wgpu).output().map(|o| o.stdout).unwrap_or_default();
    sha256(&out)
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn workloads(project: &Path) -> Vec<Box<dyn Workload>> {
    let mut v: Vec<Box<dyn Workload>> = vec![
        Box::new(blur::Blur::new("blur", project, 1, false)),
        Box::new(blur::Blur::new("blur_4k", project, 8, false)),
        Box::new(blur::Blur::new("blur_sample", project, 1, true)),
        Box::new(blur::Blur::new("blur_sample_4k", project, 8, true)),
    ];
    let captures = project.join("bounds-check-testing-framework/realworld-v1/captures/vello-a9f11bb-r40");
    for stem in vello::Vello::stems(&captures) {
        v.push(Box::new(vello::Vello::load(&captures, &stem)));
    }
    v
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    queries: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
    period: f32,
}

impl Gpu {
    fn new() -> Result<(Self, wgpu::AdapterInfo), String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor { backends: wgpu::Backends::VULKAN, ..wgpu::InstanceDescriptor::new_without_display_handle() });
        let adapter = match std::env::var("BCE_ADAPTER") {
            Ok(want) => pollster::block_on(instance.enumerate_adapters(wgpu::Backends::VULKAN))
                .into_iter()
                .find(|a| a.get_info().name.to_lowercase().contains(&want.to_lowercase()))
                .ok_or(format!("no adapter matching {want:?}"))?,
            Err(_) => pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions { power_preference: wgpu::PowerPreference::HighPerformance, ..Default::default() }))
                .map_err(|e| format!("adapter: {e}"))?,
        };
        let needed = wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::PASSTHROUGH_SHADERS;
        if !adapter.features().contains(needed) {
            return Err(format!("adapter lacks {:?}", needed - adapter.features()));
        }
        let info = adapter.get_info();
        let limits = wgpu::Limits { max_storage_buffer_binding_size: adapter.limits().max_storage_buffer_binding_size, ..wgpu::Limits::default() };
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("bce-realworld"),
            required_features: needed,
            required_limits: limits,
            experimental_features: unsafe { wgpu::ExperimentalFeatures::enabled() },
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .map_err(|e| format!("device: {e}"))?;
        let queries = device.create_query_set(&wgpu::QuerySetDescriptor { label: Some("ts"), ty: wgpu::QueryType::Timestamp, count: 2 });
        let resolve = device.create_buffer(&wgpu::BufferDescriptor { label: Some("qr"), size: 16, usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false });
        let readback = device.create_buffer(&wgpu::BufferDescriptor { label: Some("qrb"), size: 16, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
        let period = queue.get_timestamp_period();
        Ok((Gpu { device, queue, queries, resolve, readback, period }, info))
    }

    /// GPU ns per repetition of the workload for `slot`.
    fn time(&self, w: &dyn Workload, slot: usize, reps: u32) -> f64 {
        let mut enc = self.device.create_command_encoder(&Default::default());
        w.record(slot, &mut enc, &self.queries, reps);
        enc.resolve_query_set(&self.queries, 0..2, &self.resolve, 0);
        enc.copy_buffer_to_buffer(&self.resolve, 0, &self.readback, 0, 16);
        self.queue.submit([enc.finish()]);
        let bytes = workload::read_buffer(&self.device, &self.readback);
        let t: &[u64] = bytemuck::cast_slice(&bytes);
        let ticks = t[1].checked_sub(t[0]).expect("non-monotonic timestamps");
        assert!(ticks > 0);
        ticks as f64 * f64::from(self.period) / f64::from(reps)
    }
}

#[derive(Serialize)]
struct Correctness {
    phase: String,
    output_bytes: usize,
    output_sha256: [String; 4],
    /// fraction of output elements the oracle requires to be identical (1.0 = byte-identical)
    compared_fraction: f64,
    bce_equals_no_bce: bool,
    bce_dup_equals_no_bce: bool,
    unchecked_equals_no_bce: bool,
    nonzero_fraction_no_bce: f64,
    /// Reference oracle (vello): number of distinct reference outputs, elements (pixels) on
    /// which they disagree among themselves, and per slot whether the output is one of them and
    /// how many elements differ from the nearest one outside those racy elements.
    reference_states: usize,
    reference_racy_elements: usize,
    slot_in_reference_set: [bool; 4],
    slot_mismatch_outside_racy: [usize; 4],
}

fn correctness(phase: &str, outputs: &[Vec<u8>], mask: &Option<(usize, Vec<bool>)>, refs: &[Vec<u8>]) -> Correctness {
    let racy: Vec<bool> = match refs.first() {
        None => Vec::new(),
        Some(first) => (0..first.len() / 4).map(|k| refs.iter().any(|r| r[4 * k..4 * k + 4] != first[4 * k..4 * k + 4])).collect(),
    };
    let outside = |o: &[u8]| -> usize {
        refs.iter()
            .map(|r| if r.len() != o.len() { usize::MAX } else { (0..o.len() / 4).filter(|&k| !racy[k] && o[4 * k..4 * k + 4] != r[4 * k..4 * k + 4]).count() })
            .min()
            .unwrap_or(0)
    };
    let nz = outputs[0].iter().filter(|&&b| b != 0).count() as f64 / outputs[0].len().max(1) as f64;
    let same = |a: &[u8], b: &[u8]| match mask {
        None => a == b,
        Some((size, m)) => a.len() == b.len() && a.chunks(*size).zip(b.chunks(*size)).zip(m).all(|((x, y), &must)| !must || x == y),
    };
    let compared_fraction = mask.as_ref().map_or(1.0, |(_, m)| m.iter().filter(|&&b| b).count() as f64 / m.len() as f64);
    Correctness {
        phase: phase.into(),
        output_bytes: outputs[0].len(),
        output_sha256: std::array::from_fn(|s| sha256(&outputs[s])),
        compared_fraction,
        bce_equals_no_bce: same(&outputs[1], &outputs[0]),
        bce_dup_equals_no_bce: same(&outputs[2], &outputs[0]),
        unchecked_equals_no_bce: same(&outputs[3], &outputs[0]),
        nonzero_fraction_no_bce: nz,
        reference_states: refs.len(),
        reference_racy_elements: racy.iter().filter(|&&b| b).count(),
        slot_in_reference_set: std::array::from_fn(|s| refs.iter().any(|r| r == &outputs[s])),
        slot_mismatch_outside_racy: std::array::from_fn(|s| outside(&outputs[s])),
    }
}

/// Oracle for workloads whose upstream implementation is itself not bit-deterministic (vello).
/// Baseline = executions without elision: the reference images (vello's own renders) and every
/// no_bce replay run (before and after timing). An element is racy if the baseline disagrees on
/// it. New-element count of an output = elements, outside the racy ones, where it differs from
/// the nearest baseline output. Threshold T = max over no_bce runs of the new-element count of
/// that run against the baseline without it (leave-one-out): how much one fresh no-elision run
/// reveals by itself. An elided run (bce, bce_dup) passes if its new-element count <= T.
#[derive(Serialize)]
struct RefOracle {
    runs_per_slot: usize,
    reference_states: usize,
    baseline_distinct: usize,
    racy_elements: usize,
    leave_one_out_threshold: usize,
    runs_equal_to_reference: [usize; 4],
    new_elements_max: [usize; 4],
    runs_over_threshold: [usize; 4],
    ok: bool,
}

fn racy_mask(set: &[&Vec<u8>]) -> Vec<bool> {
    let first = set[0];
    (0..first.len() / 4).map(|k| set.iter().any(|r| r[4 * k..4 * k + 4] != first[4 * k..4 * k + 4])).collect()
}

fn new_elements(o: &[u8], set: &[&Vec<u8>], racy: &[bool]) -> usize {
    set.iter()
        .map(|r| if r.len() != o.len() { usize::MAX } else { (0..o.len() / 4).filter(|&k| !racy[k] && o[4 * k..4 * k + 4] != r[4 * k..4 * k + 4]).count() })
        .min()
        .unwrap()
}

fn dedup<'a>(v: Vec<&'a Vec<u8>>) -> Vec<&'a Vec<u8>> {
    let mut out: Vec<&Vec<u8>> = Vec::new();
    for x in v {
        if !out.iter().any(|y| *y == x) {
            out.push(x);
        }
    }
    out
}

fn ref_oracle(refs: &[Vec<u8>], runs: &[Vec<Vec<u8>>]) -> RefOracle {
    let all: Vec<&Vec<u8>> = refs.iter().chain(runs[0].iter()).collect();
    let racy = racy_mask(&all);
    let baseline = dedup(all);
    let threshold = (0..runs[0].len())
        .map(|i| {
            let rest = dedup(refs.iter().chain(runs[0].iter().enumerate().filter(|(j, _)| *j != i).map(|(_, o)| o)).collect());
            new_elements(&runs[0][i], &rest, &racy_mask(&rest))
        })
        .max()
        .unwrap_or(0);
    let new_max: [usize; 4] = std::array::from_fn(|s| runs[s].iter().map(|o| new_elements(o, &baseline, &racy)).max().unwrap_or(0));
    let over: [usize; 4] = std::array::from_fn(|s| runs[s].iter().filter(|o| new_elements(o, &baseline, &racy) > threshold).count());
    RefOracle {
        runs_per_slot: runs[0].len(),
        reference_states: refs.len(),
        baseline_distinct: baseline.len(),
        racy_elements: racy.iter().filter(|&&b| b).count(),
        leave_one_out_threshold: threshold,
        runs_equal_to_reference: std::array::from_fn(|s| runs[s].iter().filter(|o| refs.iter().any(|r| r == *o)).count()),
        new_elements_max: new_max,
        runs_over_threshold: over,
        ok: over[1] == 0 && over[2] == 0,
    }
}

/// Kills the nvidia-smi sampler on every exit path, including panics (a panicking run used to
/// leave it running; found 2026-09-30 after two Intel timestamp panics).
struct TelemetryGuard(Option<std::process::Child>);

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some(t) = self.0.as_mut() {
            let _ = t.kill();
            let _ = t.wait();
        }
    }
}

fn main() {
    let arg = std::env::args().nth(1).expect("usage: bce-realworld-v1 <workload>|--list");
    let framework = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    let project = framework.parent().unwrap().to_path_buf();
    let mut all = workloads(&project);
    if arg == "--list" {
        for w in &all {
            println!("{}", w.name());
        }
        return;
    }
    let pos = all.iter().position(|w| w.name() == arg).expect("unknown workload");
    let mut w = all.swap_remove(pos);
    let warmup = env_usize("BCE_WARMUP_ROUNDS", 30);
    let rounds = env_usize("BCE_ROUNDS", 120);

    let utc_start = sh("date", &["-u", "+%Y%m%dT%H%M%SZ"]);
    let adapter_tag = std::env::var("BCE_ADAPTER").map(|a| format!("-{}", a.to_lowercase())).unwrap_or_default();
    let run_dir = framework.join("runs-realworld").join(format!("{utc_start}-{}{adapter_tag}", w.name()));
    let art = run_dir.join("artifacts");
    std::fs::create_dir_all(&art).unwrap();

    let runtime_rev = git(&project.join("wgpu"), &["rev-parse", "HEAD"]);
    let runtime_diff = naga_diff_sha(&project.join("wgpu"));
    let build_rev = option_env!("BCE_BUILD_WGPU_REV").unwrap_or("unknown");
    let build_diff = option_env!("BCE_BUILD_NAGA_DIFF_SHA").unwrap_or("unknown");
    if build_rev != runtime_rev || build_diff != runtime_diff {
        std::fs::write(run_dir.join("environment-blocked.txt"), format!("provenance mismatch: built from wgpu {build_rev} (naga/src diff {build_diff}), source now {runtime_rev} (diff {runtime_diff})\n")).unwrap();
        eprintln!("refusing to run: binary was not built from the current wgpu source");
        std::process::exit(4);
    }
    let compute_apps = sh("nvidia-smi", &["--query-compute-apps=pid,process_name", "--format=csv,noheader"]);
    if !compute_apps.is_empty() {
        std::fs::write(run_dir.join("environment-blocked.txt"), format!("other GPU compute apps present:\n{compute_apps}\n")).unwrap();
        eprintln!("refusing to run: other GPU compute apps present");
        std::process::exit(3);
    }

    let specs = w.shaders();
    let mut statics: Vec<Static> = Vec::new();
    let mut words: Vec<Vec<Vec<u32>>> = vec![Vec::new(); 4];
    let mut wg_sizes = Vec::new();
    for spec in &specs {
        std::fs::write(art.join(format!("{}.wgsl", spec.label)), &spec.wgsl).unwrap();
        for slot in 0..4 {
            let (wd, st, wg) = compile(spec, slot);
            std::fs::write(art.join(format!("{}.{}.spv", spec.label, SLOTS[slot])), bytemuck::cast_slice::<u32, u8>(&wd)).unwrap();
            if slot == 0 {
                wg_sizes.push(wg);
            }
            statics.push(st);
            words[slot].push(wd);
        }
        assert_eq!(words[1].last(), words[2].last(), "bce and bce_dup must be identical words");
    }
    let bce_changes_binary = words[0] != words[1];
    let umin: Vec<usize> = (0..4).map(|s| statics.iter().filter(|st| st.slot == SLOTS[s]).map(|st| st.umin_clamps).sum()).collect();
    for f in ["main.rs", "compile.rs", "workload.rs", "blur.rs", "vello.rs"] {
        let src = match f {
            "main.rs" => include_str!("main.rs"),
            "compile.rs" => include_str!("compile.rs"),
            "workload.rs" => include_str!("workload.rs"),
            "blur.rs" => include_str!("blur.rs"),
            _ => include_str!("vello.rs"),
        };
        std::fs::write(art.join(f), src).unwrap();
    }

    let telemetry_file = std::fs::File::create(art.join("telemetry.csv")).unwrap();
    let mut telemetry = TelemetryGuard(Command::new("nvidia-smi")
        .args(["--query-gpu=timestamp,pstate,clocks.gr,clocks.mem,temperature.gpu,power.draw,utilization.gpu", "--format=csv,noheader", "-lms", "200"])
        .stdout(Stdio::from(telemetry_file))
        .spawn()
        .ok());
    let telemetry = &mut telemetry.0;
    let (gpu, adapter) = match Gpu::new() {
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
    let modules: Vec<Vec<wgpu::ShaderModule>> = (0..4)
        .map(|slot| {
            specs
                .iter()
                .enumerate()
                .map(|(i, spec)| unsafe {
                    if std::env::var("BCE_DEBUG_VALIDATED").is_ok() {
                        return gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("debug"), source: wgpu::ShaderSource::Wgsl(Cow::Owned(spec.wgsl.clone())) });
                    }
                    gpu.device.create_shader_module_passthrough(wgpu::ShaderModuleDescriptorPassthrough {
                        label: Some(SLOTS[slot]),
                        entry_points: Cow::Owned(vec![wgpu::PassthroughShaderEntryPoint { name: spec.entry.into(), workgroup_size: wg_sizes[i].into() }]),
                        spirv: Some(Cow::Owned(words[slot][i].clone())),
                        ..Default::default()
                    })
                })
                .collect()
        })
        .collect();
    w.setup(&gpu.device, &gpu.queue, &modules);

    if let Ok(n) = std::env::var("BCE_DETERMINISM") {
        // Diagnostic: n outputs per slot, slots interleaved; every distinct output saved per slot.
        // Images are NOT saved (a 2026-09-30 probe that saved every distinct image filled the disk);
        // varying-pixel sets are accumulated in memory against the first no_bce output instead.
        let n: usize = n.parse().unwrap();
        let mut seen: Vec<std::collections::BTreeMap<String, usize>> = vec![Default::default(); 4];
        let mut varying: Vec<std::collections::BTreeSet<usize>> = vec![Default::default(); 4];
        let mut base: Option<Vec<u8>> = None;
        for _ in 0..n {
            for slot in 0..4 {
                let o = w.output(&gpu.device, &gpu.queue, slot);
                let b = base.get_or_insert_with(|| o.clone());
                for k in 0..o.len() / 4 {
                    if o[4 * k..4 * k + 4] != b[4 * k..4 * k + 4] {
                        varying[slot].insert(k);
                    }
                }
                *seen[slot].entry(sha256(&o)[..8].to_string()).or_default() += 1;
            }
        }
        let v0: std::collections::BTreeSet<usize> = varying[0].union(&varying[3]).copied().collect();
        let v1: std::collections::BTreeSet<usize> = varying[1].union(&varying[2]).copied().collect();
        let modal = |slot: usize| seen[slot].values().copied().max().unwrap_or(0);
        let summary = serde_json::json!({
            "dir": run_dir.display().to_string(), "adapter": adapter.name, "runs_per_slot": n, "base": "first no_bce output",
            "distinct_outputs": {"no_bce": seen[0].len(), "bce": seen[1].len(), "bce_dup": seen[2].len(), "unchecked": seen[3].len()},
            "runs_off_modal_image": {"no_bce": n - modal(0), "bce": n - modal(1), "bce_dup": n - modal(2), "unchecked": n - modal(3)},
            "pixels_varying_no_elision": v0.len(), "pixels_varying_elided": v1.len(),
            "elided_only_pixels": v1.difference(&v0).count(), "no_elision_only_pixels": v0.difference(&v1).count(),
        });
        println!("race-summary {summary}");
        for slot in 0..4 {
            println!("determinism slot={} runs={} outputs={:?}", SLOTS[slot], n, seen[slot]);
        }
        std::fs::write(art.join("determinism.json"), serde_json::to_vec_pretty(&serde_json::json!({"runs_per_slot": n, "adapter": adapter.name, "slots": SLOTS, "counts": seen})).unwrap()).unwrap();
        std::fs::write(run_dir.join("DIAGNOSTIC-ONLY.txt"), "determinism probe; not a measurement\n").unwrap();
        if let Some(t) = telemetry.as_mut() {
            let _ = t.kill();
        }
        return;
    }
    let outputs_before: Vec<Vec<u8>> = (0..4).map(|s| w.output(&gpu.device, &gpu.queue, s)).collect();
    let mask = w.oracle_mask();
    let ref_set = w.reference_outputs(&adapter.name);
    let nondeterministic = ref_set.is_some();
    let refs = ref_set.unwrap_or_default();
    let oracle_runs = env_usize("BCE_ORACLE_RUNS", 20);
    let collect = |w: &dyn Workload| -> Vec<Vec<Vec<u8>>> { (0..4).map(|s| (0..oracle_runs).map(|_| w.output(&gpu.device, &gpu.queue, s)).collect()).collect() };
    let runs_before = if nondeterministic { collect(w.as_ref()) } else { Vec::new() };
    let before = correctness("before", &outputs_before, &mask, &refs);

    let probe = gpu.time(w.as_ref(), 0, 3);
    let reps = (TARGET_SAMPLE_NS / probe).round().clamp(1.0, 4000.0) as u32;

    let mut warm = Vec::new();
    let warm_start = std::time::Instant::now();
    let mut r = 0;
    loop {
        for j in 0..4 {
            let slot = (r + j) % 4;
            let t = gpu.time(w.as_ref(), slot, reps);
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
    let measure_start_unix_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
    let mut data = Vec::with_capacity(rounds);
    for r in 0..rounds {
        let order: Vec<usize> = (0..4).map(|j| (r + j) % 4).collect();
        let mut ns = [0.0; 4];
        for &slot in &order {
            ns[slot] = gpu.time(w.as_ref(), slot, reps);
        }
        data.push(Round { round: r, order: order.iter().map(|&s| SLOTS[s].to_string()).collect(), ns });
    }
    let measure_end_unix_ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
    let own_pid = std::process::id().to_string();
    let compute_apps_after: Vec<String> = sh("nvidia-smi", &["--query-compute-apps=pid,process_name", "--format=csv,noheader"])
        .lines()
        .filter(|line| !line.trim().is_empty() && line.split(',').next().map(str::trim) != Some(own_pid.as_str()))
        .map(str::to_string)
        .collect();
    let quiet_during_run = compute_apps_after.is_empty();
    let outputs_after: Vec<Vec<u8>> = (0..4).map(|s| w.output(&gpu.device, &gpu.queue, s)).collect();
    let after = correctness("after", &outputs_after, &mask, &refs);
    let oracle = nondeterministic.then(|| {
        let mut runs = runs_before;
        for (slot, more) in collect(w.as_ref()).into_iter().enumerate() {
            runs[slot].extend(more);
        }
        ref_oracle(&refs, &runs)
    });
    if let Some(t) = telemetry.as_mut() {
        let _ = t.kill();
    }
    for (phase, outs) in [("before", &outputs_before), ("after", &outputs_after)].into_iter().filter(|_| std::env::var("BCE_SAVE_OUTPUTS").is_ok()) {
        for (slot, out) in outs.iter().enumerate() {
            if slot == 0 || out.len() <= 8 << 20 {
                std::fs::write(art.join(format!("output.{phase}.{}.bin", SLOTS[slot])), out).unwrap();
            }
        }
    }

    let col = |s: usize| data.iter().map(|r| r.ns[s]).collect::<Vec<_>>();
    let ratio = |a: usize, b: usize| data.iter().map(|r| r.ns[a] / r.ns[b]).collect::<Vec<_>>();
    let medians: Vec<f64> = (0..4).map(|s| median(&col(s))).collect();
    let effect_ratio = bootstrap(&ratio(0, 1), 0x5A71);
    let control_ratio = bootstrap(&ratio(2, 1), 0x5A72);
    let residual_ratio = bootstrap(&ratio(1, 3), 0x5A73);
    let all_checks_ratio = bootstrap(&ratio(0, 3), 0x5A74);
    let pos = |r: &Round, s: &str| r.order.iter().position(|o| o == s).unwrap();
    let (mut first, mut second) = (Vec::new(), Vec::new());
    for r in &data {
        let d = r.ns[0] / r.ns[1];
        if pos(r, "no_bce") < pos(r, "bce") { first.push(d) } else { second.push(d) }
    }
    // With an independent reference set (vello), slots may legitimately differ from each other at
    // pixels where the reference itself varies; the oracle is then: every checked slot's output
    // equals a reference image outside those pixels. Otherwise: byte-identical slots.
    let outputs_ok = if let Some(o) = &oracle {
        o.ok && before.nonzero_fraction_no_bce > 0.0
    } else {
        let deterministic = mask.is_some() || before.output_sha256[0] == after.output_sha256[0];
        [&before, &after].iter().all(|c| c.bce_equals_no_bce && c.bce_dup_equals_no_bce && c.nonzero_fraction_no_bce > 0.0) && deterministic
    };
    let control_ok = control_ratio.ci95_lo <= 1.0 && control_ratio.ci95_hi >= 1.0;

    let meta = serde_json::json!({
        "schema": "bce-meta-2",
        "harness_version": HARNESS_VERSION,
        "utc_start": utc_start,
        "utc_end": sh("date", &["-u", "+%Y%m%dT%H%M%SZ"]),
        "sources": {
            "wgpu": {"rev": runtime_rev, "branch": git(&project.join("wgpu"), &["branch", "--show-current"]),
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
        "schema": "bce-result-rw-1",
        "harness_version": HARNESS_VERSION,
        "kernel": w.name(),
        "provenance": w.provenance(),
        "reps_per_sample": reps,
        "warmup_rounds": warmup,
        "rounds": rounds,
        "slots": SLOTS,
        "statics": statics,
        "umin_clamps_per_slot": umin,
        "bce_changes_binary": bce_changes_binary,
        "median_ns_per_rep": {"no_bce": medians[0], "bce": medians[1], "bce_dup": medians[2], "unchecked": medians[3]},
        "ratio_no_bce_over_bce": effect_ratio,
        "ratio_control_bce_dup_over_bce": control_ratio,
        "ratio_residual_bce_over_unchecked": residual_ratio,
        "ratio_all_checks_no_bce_over_unchecked": all_checks_ratio,
        "order_effect_ratio": {"no_bce_first_median": median(&first), "bce_first_median": median(&second), "n_first": first.len(), "n_second": second.len()},
        "warmup_rounds_done": warmup_rounds_done,
        "measure_window_unix_ms": [measure_start_unix_ms, measure_end_unix_ms],
        "correctness": [before, after],
        "reference_oracle": oracle,
        "outputs_ok": outputs_ok,
        "gpu_compute_apps_after": compute_apps_after,
        "quiet_during_run": quiet_during_run,
        "valid": outputs_ok && control_ok && quiet_during_run,
        "rounds_raw": data,
    });
    std::fs::write(run_dir.join("result.json"), serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    println!(
        "{} valid={} outputs_ok={} changes_binary={} umin={:?} bce={:.0}ns no_bce/bce={:.4} [{:.4},{:.4}] control={:.4} bce/unchecked={:.4} -> {}",
        w.name(), outputs_ok && control_ok && quiet_during_run, outputs_ok, bce_changes_binary, umin, medians[1], effect_ratio.median, effect_ratio.ci95_lo, effect_ratio.ci95_hi,
        control_ratio.median, residual_ratio.median, run_dir.display()
    );
}
