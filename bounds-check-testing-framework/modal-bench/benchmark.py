"""
Bounds-Check Elision — Modal.com GPU Benchmark (Tier 3 + Tier 4)

Architecture (from the IICT paper §6 / design doc §3-4):

  Local:  WGSL source  →  naga-cli (patched / unpatched)  →  .spv files
  Cloud:  wgpu-py loads SPIR-V on T4  →  timestamp queries  →  ns measurements

This script runs on Modal's serverless T4 infrastructure. It receives
pre-compiled SPIR-V binaries (compiled locally through naga with
a specific BoundsCheckPolicy) and executes them with isolated device-time
measurement using wgpu-py timestamp queries — the ONLY timing mechanism
that works under both native and browser WebGPU (see design doc §3.2).

Usage (from local machine, after `modal token new`):

    modal run benchmark.py --shaders-dir ./shaders --output results.json
"""

import modal
import json

# ---------------------------------------------------------------------------
# Modal image: wgpu-py with Vulkan support for T4 GPU
# ---------------------------------------------------------------------------
image = (
    modal.Image.debian_slim(python_version="3.12")
    .apt_install(
        "libvulkan1",
        "mesa-vulkan-drivers",
        "libegl1",
        "libgl1",
        "libx11-6",
    )
    .pip_install("wgpu[numpy]>=0.19", "numpy>=1.26")
)

app = modal.App("naga-bce-benchmarks", image=image)

# ---------------------------------------------------------------------------
# Kernel metadata — dispatch dimensions, buffer sizes, workgroup sizes
# Paper §6: reduction, prefix_scan, tiled_matmul
# The three baselines from §4.4 — do NOT add a fourth.
# ---------------------------------------------------------------------------
BASELINES = ["Unchecked", "ReadZeroSkipWrite", "Restrict"]
KERNELS = ["reduction", "prefix_scan", "tiled_matmul"]

KERNEL_META = {
    "reduction": {
        "dispatch": (256, 1, 1),
        "output_bytes": 256 * 4,
        "init_data_bytes": 256 * 4,
    },
    "prefix_scan": {
        "dispatch": (256, 1, 1),
        "output_bytes": 256 * 4,
        "init_data_bytes": 256 * 4,
    },
    "tiled_matmul": {
        "dispatch": (16, 16, 1),
        "output_bytes": 256 * 256 * 4 * 2,  # A + B matrices contiguously
        "init_data_bytes": 256 * 256 * 4 * 2,
    },
}

# Timing parameters matching the Rust harness (design doc §3.3)
DISPATCHES_PER_PASS = 10_000
WARMUP_PASSES = 10
TIMED_PASSES = 1_000

# Per-kernel workgroup multiplier for H100 occupancy.
# tiled_matmul uses 2D dispatch with bounds-checking, so no multiplier.
WORKGROUP_MULTIPLIERS = {
    "reduction": 16,
    "prefix_scan": 16,
    "tiled_matmul": 1,
}


# ---------------------------------------------------------------------------
# The Modal function: runs on a T4 GPU (sync wgpu-py API)
# ---------------------------------------------------------------------------
@app.function(
    gpu="T4",
    timeout=900,
)
def run_kernel_benchmark(
    kernel_name: str,
    baseline: str,
    spirv_bytes: bytes,
    dispatch_x: int,
    dispatch_y: int,
    dispatch_z: int,
    output_bytes: int,
    init_data_bytes: int,
    dispatches_per_pass: int = DISPATCHES_PER_PASS,
    warmup_passes: int = WARMUP_PASSES,
    timed_passes: int = TIMED_PASSES,
):
    """
    Execute a single kernel+baseline combination on T4 and return
    per-dispatch timing statistics.

    wgpu-py uses a SYNCHRONOUS API — adapter/device/buffer operations
    are all blocking calls. The deprecated async variants (await
    request_adapter) have been replaced with their sync equivalents.
    """
    import numpy as np
    import wgpu

    # ---- 1. Initialize wgpu on the T4 ----
    # request_adapter_sync() is the current sync API (request_adapter is deprecated)
    adapter = wgpu.gpu.request_adapter_sync(power_preference="high-performance")

    # Only request timestamp-query — portable, web-and-native (design doc §3.2)
    device = adapter.request_device_sync(
        required_features=["timestamp-query"],
    )
    queue = device.queue
    device_name = adapter.info.get("device", "unknown")

    # ---- 2. Create shader module from SPIR-V ----
    shader_module = device.create_shader_module(code=spirv_bytes)

    # ---- 3. Build pipeline and bind group ----
    # Kernels use @binding(0) = input (read) and @binding(1) = output (read_write)
    bind_group_layout = device.create_bind_group_layout(
        label=f"{kernel_name}_{baseline}_bgl",
        entries=[
            {
                "binding": 0,
                "visibility": wgpu.ShaderStage.COMPUTE,
                "buffer": {"type": wgpu.BufferBindingType.read_only_storage},
            },
            {
                "binding": 1,
                "visibility": wgpu.ShaderStage.COMPUTE,
                "buffer": {"type": wgpu.BufferBindingType.storage},
            },
        ],
    )

    pipeline_layout = device.create_pipeline_layout(
        label=f"{kernel_name}_{baseline}_layout",
        bind_group_layouts=[bind_group_layout],
    )

    compute_pipeline = device.create_compute_pipeline(
        label=f"{kernel_name}_{baseline}_pipeline",
        layout=pipeline_layout,
        compute={"module": shader_module, "entry_point": "main"},
    )

    # ---- 4. Create input and output buffers, initialize with data ----
    input_buffer = device.create_buffer(
        label=f"{kernel_name}_{baseline}_input",
        size=init_data_bytes,
        usage=wgpu.BufferUsage.STORAGE
        | wgpu.BufferUsage.COPY_SRC
        | wgpu.BufferUsage.COPY_DST,
    )
    output_buffer = device.create_buffer(
        label=f"{kernel_name}_{baseline}_output",
        size=output_bytes,
        usage=wgpu.BufferUsage.STORAGE
        | wgpu.BufferUsage.COPY_SRC
        | wgpu.BufferUsage.COPY_DST,
    )

    # Initialize input buffer with non-trivial data via staging buffer
    init_data = np.random.default_rng(42).uniform(
        0.01, 1.0, size=init_data_bytes // 4
    ).astype(np.float32)
    staging_buffer = device.create_buffer(
        label=f"{kernel_name}_{baseline}_staging",
        size=init_data_bytes,
        usage=wgpu.BufferUsage.COPY_SRC | wgpu.BufferUsage.MAP_WRITE,
        mapped_at_creation=True,
    )
    staging_buffer.write_mapped(init_data.tobytes())
    staging_buffer.unmap()

    encoder = device.create_command_encoder()
    encoder.copy_buffer_to_buffer(staging_buffer, 0, input_buffer, 0, init_data_bytes)
    queue.submit([encoder.finish()])

    bind_group = device.create_bind_group(
        label=f"{kernel_name}_{baseline}_bg",
        layout=bind_group_layout,
        entries=[
            {"binding": 0, "resource": {"buffer": input_buffer}},
            {"binding": 1, "resource": {"buffer": output_buffer}},
        ],
    )

    # ---- 5. Timestamp query infrastructure ----
    # wgpu-py 0.31.1 doesn't expose timestamp_period on the queue.
    # In WebGPU spec it's queue.timestampPeriod but wgpu-py may not surface it.
    # Fallback: check adapter limits, else default to 1.0 (common on NVIDIA).
    timestamp_period = 1.0  # default: 1 ns per tick
    try:
        # Some wgpu-py builds expose it via device or adapter limits
        limits = adapter.limits if hasattr(adapter, 'limits') else {}
        if hasattr(device, 'adapter_info'):
            info = device.adapter_info
            # No standard field for timestamp_period in adapter_info
    except Exception:
        pass

    # ---- 6. Warm-up passes (untimed) ----
    for _ in range(warmup_passes):
        encoder = device.create_command_encoder()
        pass_ = encoder.begin_compute_pass()
        pass_.set_pipeline(compute_pipeline)
        pass_.set_bind_group(0, bind_group, [], 0, 999999)
        for _ in range(dispatches_per_pass):
            pass_.dispatch_workgroups(dispatch_x, dispatch_y, dispatch_z)
        pass_.end()
        queue.submit([encoder.finish()])

    # ---- 7. Timed passes with timestamp queries ----
    samples_ns = []

    for _ in range(timed_passes):
        query_set = device.create_query_set(
            type=wgpu.QueryType.timestamp,
            count=2,
        )
        resolve_buffer = device.create_buffer(
            size=16,
            usage=wgpu.BufferUsage.QUERY_RESOLVE | wgpu.BufferUsage.COPY_SRC,
        )
        destination_buffer = device.create_buffer(
            size=16,
            usage=wgpu.BufferUsage.COPY_DST | wgpu.BufferUsage.MAP_READ,
        )

        encoder = device.create_command_encoder()
        pass_ = encoder.begin_compute_pass(
            timestamp_writes={
                "query_set": query_set,
                "beginning_of_pass_write_index": 0,
                "end_of_pass_write_index": 1,
            }
        )
        pass_.set_pipeline(compute_pipeline)
        pass_.set_bind_group(0, bind_group, [], 0, 999999)
        for _ in range(dispatches_per_pass):
            pass_.dispatch_workgroups(dispatch_x, dispatch_y, dispatch_z)
        pass_.end()

        # Resolve + copy (mandatory two-step for map_read)
        encoder.resolve_query_set(query_set, 0, 2, resolve_buffer, 0)
        encoder.copy_buffer_to_buffer(resolve_buffer, 0, destination_buffer, 0, 16)

        queue.submit([encoder.finish()])

        # Read back timestamps — sync map
        destination_buffer.map_sync(wgpu.MapMode.READ)
        data = bytes(destination_buffer.read_mapped())
        ticks = np.frombuffer(data, dtype=np.uint64).tolist()
        destination_buffer.unmap()

        elapsed_ticks = ticks[1] - ticks[0]
        elapsed_ns = elapsed_ticks * timestamp_period
        samples_ns.append(elapsed_ns)

    # ---- 8. Compute statistics ----
    samples_us = np.array(samples_ns) / 1000.0
    samples_us /= dispatches_per_pass

    result = {
        "kernel": kernel_name,
        "baseline": baseline,
        "median_us": float(np.median(samples_us)),
        "p95_us": float(np.percentile(samples_us, 95)),
        "min_us": float(np.min(samples_us)),
        "max_us": float(np.max(samples_us)),
        "std_us": float(np.std(samples_us)),
        "device_name": device_name,
        "timestamp_period_ns": timestamp_period,
    }

    return result


# ---------------------------------------------------------------------------
# Orchestration: runs locally, calls the Modal function sequentially
# ---------------------------------------------------------------------------
@app.local_entrypoint()
def main(
    shaders_dir: str = "./shaders",
    output: str = "results.json",
):
    """
    Local entrypoint that:
    1. Reads all pre-compiled .spv files from shaders_dir
    2. Calls run_kernel_benchmark.remote() sequentially for each pair
    3. Collects results and writes to output JSON
    """
    import os

    print("=" * 60)
    print("  Bounds-Check Elision — Modal.com GPU Benchmark")
    print("  IICT Paper §6 / Design Doc Tier 3+4")
    print("=" * 60)
    print()

    # Scan for available SPIR-V files
    available = {}
    if os.path.isdir(shaders_dir):
        for f in sorted(os.listdir(shaders_dir)):
            if f.endswith(".spv"):
                parts = f.rsplit(".", 1)[0]
                available[parts] = os.path.join(shaders_dir, f)

    if not available:
        print(f"ERROR: No .spv files found in {shaders_dir}/")
        print()
        print("Run compile_shaders.sh first to produce the SPIR-V binaries:")
        print(f"  bash compile_shaders.sh <path-to-wgpu-checkout>")
        print()
        print(f"Expected files in {shaders_dir}/:")
        for kernel in KERNELS:
            for baseline in BASELINES:
                print(f"  {kernel}_{baseline}.spv")
        return

    print(f"Found {len(available)} SPIR-V binaries in {shaders_dir}/")
    print()

    # Build job list
    jobs = []
    for kernel in KERNELS:
        meta = KERNEL_META[kernel]
        dx, dy, dz = meta["dispatch"]
        dx *= WORKGROUP_MULTIPLIERS.get(kernel, 1)

        for baseline in BASELINES:
            key = f"{kernel}_{baseline}"
            if key not in available:
                print(f"  SKIP {key} (no .spv file)")
                continue

            spv_path = available[key]
            spv_bytes = open(spv_path, "rb").read()
            print(
                f"  QUEUED {key} ({len(spv_bytes)} bytes, "
                f"dispatch=({dx},{dy},{dz}), "
                f"buffer={meta['output_bytes']}B)"
            )

            jobs.append({
                "kernel_name": kernel,
                "baseline": baseline,
                "spirv_bytes": spv_bytes,
                "dispatch_x": dx,
                "dispatch_y": dy,
                "dispatch_z": dz,
                "output_bytes": meta["output_bytes"],
                "init_data_bytes": meta["init_data_bytes"],
            })

    if not jobs:
        print("ERROR: No valid SPIR-V files matched the expected naming pattern.")
        return

    print()
    total = len(jobs)
    print(f"Running {total} kernel+baseline combinations on Modal T4...")
    print(f"Each: {DISPATCHES_PER_PASS} dispatches × {WARMUP_PASSES} warmup + {TIMED_PASSES} timed passes")
    print()

    # Submit to Modal — sequential .remote() calls
    results = []
    for i, job in enumerate(jobs):
        key = f"{job['kernel_name']} × {job['baseline']}"
        print(f"  [{i+1}/{total}] {key}...", end=" ", flush=True)
        try:
            result = run_kernel_benchmark.remote(**job)
            results.append(result)
            print(f"median={result['median_us']:.3f} μs  p95={result['p95_us']:.3f} μs")
        except Exception as e:
            print(f"FAILED: {e}")
            results.append({
                "kernel": job["kernel_name"],
                "baseline": job["baseline"],
                "error": str(e),
            })

    # Write full results to JSON
    with open(output, "w") as f:
        json.dump(results, f, indent=2, default=str)

    print()
    print(f"Results written to {output}")
    print()

    # Print summary table
    print("=" * 80)
    print(f"{'kernel':<20} {'baseline':<20} {'median (μs)':>12} {'p95 (μs)':>12} {'device':<16}")
    print("-" * 80)
    for r in results:
        if "error" in r:
            print(f"{'?':<20} {'?':<20} {'ERROR':>12} {'':>12} {r.get('error', '')[:16]}")
        else:
            print(
                f"{r['kernel']:<20} {r['baseline']:<20} "
                f"{r['median_us']:>12.3f} {r['p95_us']:>12.3f} "
                f"{r.get('device_name', '?')[:16]}"
            )
    print("=" * 80)
    print()

    # Compute speedup ratios (unchecked / policy)
    print("Speedup ratios (Unchecked / Policy) — higher = more cost recovered:")
    print("-" * 60)
    for kernel in KERNELS:
        unchecked = next(
            (r for r in results
             if r.get("kernel") == kernel
             and r.get("baseline") == "Unchecked"
             and "error" not in r),
            None,
        )
        for policy_name in ["ReadZeroSkipWrite", "Restrict"]:
            policy = next(
                (r for r in results
                 if r.get("kernel") == kernel
                 and r.get("baseline") == policy_name
                 and "error" not in r),
                None,
            )
            if unchecked and policy and policy["median_us"] > 0:
                ratio = unchecked["median_us"] / policy["median_us"]
                delta = unchecked["median_us"] - policy["median_us"]
                print(f"  {kernel:<20} {policy_name:<20} {ratio:.4f}×  (Δ {delta:+.3f} μs)")
    print()
