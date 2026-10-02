#!/bin/bash
# compile_shaders.sh — Compile WGSL kernels through naga with all three
# BoundsCheckPolicy baselines, producing SPIR-V binaries for Modal upload.
#
# Usage:
#   bash compile_shaders.sh <path-to-wgpu-checkout>
#
# This script compiles each of the three paper kernels (§6) through the
# LOCAL naga-cli from your wgpu checkout, under each of the three policies:
#   Unchecked, ReadZeroSkipWrite, Restrict
#
# Output: ./shaders/{kernel}_{baseline}.spv  (9 files total)
#
# The design doc §4.1 is explicit: no fourth baseline. Do not add one.

set -euo pipefail

WGPU_DIR="${1:-.}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SHADER_DIR="${SCRIPT_DIR}/shaders"
NAGA_CLI=""

# ---- Locate naga CLI binary ----
# The binary is named "naga" (not "naga-cli") in naga v0.14.0+
NAGA_CLI=""
for candidate in "${WGPU_DIR}/target/release/naga" "${WGPU_DIR}/target/debug/naga" naga; do
    if command -v "${candidate}" &>/dev/null || [ -x "${candidate}" ]; then
        NAGA_CLI="${candidate}"
        break
    fi
done

if [ -z "${NAGA_CLI}" ]; then
    echo "ERROR: naga binary not found."
    echo ""
    echo "Build it from your naga/wgpu checkout:"
    echo "  cd ${WGPU_DIR}"
    echo "  cargo build --release -p naga-cli"
    echo ""
    echo "Or install from crates.io:"
    echo "  cargo install naga-cli"
    exit 1
fi

echo "Using naga: ${NAGA_CLI}"
${NAGA_CLI} --version 2>/dev/null || echo "(version check failed, continuing anyway)"

# naga CLI uses separate flags: --index-bounds-check-policy and
# --buffer-bounds-check-policy (each takes Restrict, ReadZeroSkipWrite,
# or Unchecked). These are verified naga v0.14.0 flags.

# ---- Create output directory ----
mkdir -p "${SHADER_DIR}"

# ---- Kernel WGSL sources ----
# Each kernel uses SEPARATE input (@binding(0)) and output (@binding(1))
# buffers to avoid the input-overwrite correctness bug. Both reduction and
# prefix_scan need a separate input buffer to read from, and write results
# to the output buffer. Tiled matmul packs A and B into the input buffer
# and writes C to the output buffer.

# Kernel 1: Workgroup-tiled reduction
# - Pure workgroup memory access, low arithmetic intensity
# - Bounds-check cost should be MOST visible here
KERNEL_REDUCTION='
@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
var<workgroup> tile: array<f32, 256>;

@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32,
        @builtin(workgroup_id) wid: vec3<u32>) {
    tile[lid] = input[wid.x * 256u + lid];
    workgroupBarrier();

    // Tree reduction in workgroup memory
    for (var stride: u32 = 128u; stride > 0u; stride >>= 1u) {
        if (lid < stride) {
            tile[lid] = tile[lid] + tile[lid + stride];
        }
        workgroupBarrier();
    }

    if (lid == 0u) {
        output[wid.x] = tile[0];
    }
}
'

# Kernel 2: Hillis-Steele prefix scan
# - Both workgroup memory access and sequential dependency chain
# - Bounds-check cost should be visible (ILP chain is short)
KERNEL_PREFIX_SCAN='
@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
var<workgroup> temp: array<f32, 256>;

@compute @workgroup_size(256)
fn main(@builtin(local_invocation_index) lid: u32,
        @builtin(workgroup_id) wid: vec3<u32>) {
    let base = wid.x * 256u;
    temp[lid] = input[base + lid];
    workgroupBarrier();

    // Hillis-Steele inclusive scan
    var offset = 1u;
    for (var stride: u32 = 1u; stride < 256u; stride <<= 1u) {
        var val: f32 = 0.0;
        if (lid >= stride) {
            val = temp[lid - stride];
        }
        workgroupBarrier();
        temp[lid] = temp[lid] + val;
        workgroupBarrier();
    }

    output[base + lid] = temp[lid];
}
'

# Kernel 3: Tiled matrix multiply
# - High arithmetic intensity, enough ILP to potentially HIDE bounds-check cost
# - Paper §2.2 predicts this kernel may show NO measurable policy difference
# - A (M×K) and B (K×N) are in input buffer; C (M×N) goes to output buffer
KERNEL_TILED_MATMUL='
@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
var<workgroup> tile_a: array<f32, 256>;
var<workgroup> tile_b: array<f32, 256>;

const TILE: u32 = 16u;
const M: u32 = 256u;
const K: u32 = 256u;
const N: u32 = 256u;

@compute @workgroup_size(16, 16)
fn main(@builtin(local_invocation_id) lid: vec3<u32>,
        @builtin(workgroup_id) wid: vec3<u32>) {
    let row = wid.y * TILE + lid.y;
    let col = wid.x * TILE + lid.x;
    var sum: f32 = 0.0;

    for (var t: u32 = 0u; t < K / TILE; t++) {
        // Cooperatively load tiles from input (read-only)
        let a_col = t * TILE + lid.x;
        if (row < M && a_col < K) {
            tile_a[lid.y * TILE + lid.x] = input[row * K + a_col];
        } else {
            tile_a[lid.y * TILE + lid.x] = 0.0;
        }

        let b_row = t * TILE + lid.y;
        if (b_row < K && col < N) {
            tile_b[lid.y * TILE + lid.x] = input[M * K + b_row * N + col];
        } else {
            tile_b[lid.y * TILE + lid.x] = 0.0;
        }
        workgroupBarrier();

        // Compute partial dot product
        for (var i: u32 = 0u; i < TILE; i++) {
            sum += tile_a[lid.y * TILE + i] * tile_b[i * TILE + lid.x];
        }
        workgroupBarrier();
    }

    if (row < M && col < N) {
        output[row * N + col] = sum;
    }
}
'

# ---- Compile each kernel × baseline ----
declare -A KERNELS=(
    ["reduction"]="${KERNEL_REDUCTION}"
    ["prefix_scan"]="${KERNEL_PREFIX_SCAN}"
    ["tiled_matmul"]="${KERNEL_TILED_MATMUL}"
)

# Build policy flags using naga's separate --index-bounds-check-policy
# and --buffer-bounds-check-policy flags (naga v0.14.0 verified)
build_policy_flags() {
    local baseline="$1"
    case "${baseline}" in
        Unchecked)
            echo ""
            ;;
        ReadZeroSkipWrite)
            echo '--index-bounds-check-policy ReadZeroSkipWrite --buffer-bounds-check-policy ReadZeroSkipWrite'
            ;;
        Restrict)
            echo '--index-bounds-check-policy Restrict --buffer-bounds-check-policy Restrict'
            ;;
    esac
}

BASELINES=("Unchecked" "ReadZeroSkipWrite" "Restrict")
echo ""
echo "Compiling ${#KERNELS[@]} kernels × ${#BASELINES[@]} baselines = $(( ${#KERNELS[@]} * ${#BASELINES[@]} )) SPIR-V files..."
echo ""

ERRORS=0

for kernel_name in "${!KERNELS[@]}"; do
    kernel_src="${KERNELS[$kernel_name]}"
    tmp_wgsl=$(mktemp /tmp/naga_bce_XXXXXX.wgsl)
    echo "${kernel_src}" > "${tmp_wgsl}"

    for baseline in "${BASELINES[@]}"; do
        policy_flags=$(build_policy_flags "${baseline}")
        out_spv="${SHADER_DIR}/${kernel_name}_${baseline}.spv"

        echo -n "  ${kernel_name} × ${baseline}... "

        # shellcheck disable=SC2086
        if ${NAGA_CLI} ${policy_flags} "${tmp_wgsl}" "${out_spv}" 2>/dev/null; then
            size=$(wc -c < "${out_spv}")
            echo "OK (${size} bytes)"
        else
            echo "FAILED"
            ERRORS=$((ERRORS + 1))
        fi
    done

    rm -f "${tmp_wgsl}"
done

echo ""

if [ ${ERRORS} -gt 0 ]; then
    echo "ERRORS: ${ERRORS}/${#KERNELS[@]}/${#BASELINES[@]} compilations failed."
    echo "Check naga-cli version and flag format. Run 'naga-cli --help' for details."
    exit 1
fi

echo "Done. SPIR-V files in: ${SHADER_DIR}/"
ls -la "${SHADER_DIR}"/*.spv 2>/dev/null || echo "(no files produced)"
echo ""
echo "Next step: run the Modal benchmark:"
echo "  cd modal-bench && modal run benchmark.py --shaders-dir ${SHADER_DIR}"
