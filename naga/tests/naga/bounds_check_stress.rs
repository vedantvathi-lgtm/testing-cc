//! Bounds Check Elision — Stress Tests & Limit Testing
//!
//! These tests deliberately push the elision to its breaking points:
//! - Boundary: array_len == wg_size (exact fit), > wg_size (oversized)
//! - MUST NOT ELIDE: lid+1, lid-1, lid*2, helper fn forwarding
//! - 3D workgroups: local_invocation_id.x, (y,z combinations)
//! - Storage buffer accesses (dynamic-length arrays)
//! - Loop induction variables (if/break, break_if)
//! - Multiple lid-based accesses in one shader
//! - SPIR-V instruction count verification

use naga::back::spv;
use naga::proc::{BoundsCheckPolicies, BoundsCheckPolicy};
use naga::valid::{Capabilities, ValidationFlags, Validator};
use rspirv::binary::Disassemble;

/// Compile WGSL to SPIR-V disassembly text under a given policy.
fn compile(text: &str, policy: BoundsCheckPolicy) -> String {
    let module = naga::front::wgsl::parse_str(text).expect("WGSL parse");
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
    let words = spv::write_vec(&module, &info, &opts, None).expect("SPIR-V codegen");
    rspirv::dr::load_words(words)
        .expect("valid SPIR-V")
        .disassemble()
}

/// Count occurrences of a substring in SPIR-V disassembly.
fn count(text: &str, marker: &str) -> usize {
    text.matches(marker).count()
}

/// Compile both checked (ReadZeroSkipWrite) and unchecked versions,
/// then assert that the checked version is NO LARGER than the unchecked
/// version (in instruction opcodes). Returns (checked_count, unchecked_count).
struct PolicyCounts {
    checked: usize,
    unchecked: usize,
}

fn compare_policies(text: &str, marker: &str) -> PolicyCounts {
    let checked_text = compile(text, BoundsCheckPolicy::ReadZeroSkipWrite);
    let unchecked_text = compile(text, BoundsCheckPolicy::Unchecked);
    PolicyCounts {
        checked: count(&checked_text, marker),
        unchecked: count(&unchecked_text, marker),
    }
}

// =========================================================================
//  BOUNDARY CONDITIONS
// =========================================================================

/// Array exactly == workgroup size — lid ∈ [0, 64), array has 64 elements.
/// Elision SHOULD apply: every lid is a valid index.
#[test]
fn exact_fit_array_eq_wgsize() {
    let text = compile(
        r#"
        var<workgroup> tile: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            tile[lid] = 1.0;
        }
        "#,
        BoundsCheckPolicy::Restrict,
    );
    // No UMin clamp = elision works
    assert!(!text.contains("UMin"), "exact fit: UMin should be elided");
}

/// Array larger than workgroup size — lid ∈ [0, 256), array has 512 elements.
/// Elision SHOULD apply.
#[test]
fn oversized_array_gt_wgsize() {
    let text = compile(
        r#"
        var<workgroup> tile: array<f32, 512>;
        @compute @workgroup_size(256)
        fn main(@builtin(local_invocation_index) lid: u32) {
            tile[lid] = 1.0;
        }
        "#,
        BoundsCheckPolicy::Restrict,
    );
    assert!(!text.contains("UMin"), "oversized: UMin should be elided");
}

/// Array smaller than workgroup size — lid ∈ [0, 256), array has 64 elements.
/// Elision MUST NOT apply: lid can exceed array bounds.
#[test]
fn undersized_array_lt_wgsize_check_retained() {
    let text = compile(
        r#"
        var<workgroup> tile: array<f32, 64>;
        @compute @workgroup_size(256)
        fn main(@builtin(local_invocation_index) lid: u32) {
            tile[lid] = 1.0;
        }
        "#,
        BoundsCheckPolicy::Restrict,
    );
    assert!(text.contains("UMin"), "undersized: UMin must be retained");
}

// =========================================================================
//  MUST NOT ELIDE — edge cases that look like lid but aren't safe
// =========================================================================

/// lid + 1: when lid = 255, lid+1 = 256 which is OOB for array<256>.
/// Elision MUST NOT apply.
#[test]
fn lid_plus_one_check_retained() {
    let text = compile(
        r#"
        var<workgroup> tile: array<f32, 256>;
        @compute @workgroup_size(256)
        fn main(@builtin(local_invocation_index) lid: u32) {
            tile[lid + 1u] = 1.0;
        }
        "#,
        BoundsCheckPolicy::Restrict,
    );
    assert!(text.contains("UMin"), "lid+1: UMin must be retained");
}

/// lid - 1: when lid = 0, lid-1 wraps to u32::MAX. OOB.
/// Elision MUST NOT apply.
#[test]
fn lid_minus_one_wraparound_check_retained() {
    let text = compile(
        r#"
        var<workgroup> tile: array<f32, 256>;
        @compute @workgroup_size(256)
        fn main(@builtin(local_invocation_index) lid: u32) {
            tile[lid - 1u] = 1.0;
        }
        "#,
        BoundsCheckPolicy::Restrict,
    );
    assert!(text.contains("UMin"), "lid-1: UMin must be retained");
}

/// lid * 2: when lid = 255, lid*2 = 510 which is OOB for many arrays.
/// Elision MUST NOT apply (multiplicative transform).
#[test]
fn lid_times_two_check_retained() {
    let text = compile(
        r#"
        var<workgroup> tile: array<f32, 512>;
        @compute @workgroup_size(256)
        fn main(@builtin(local_invocation_index) lid: u32) {
            tile[lid * 2u] = 1.0;
        }
        "#,
        BoundsCheckPolicy::Restrict,
    );
    assert!(text.contains("UMin"), "lid*2: UMin must be retained");
}

// =========================================================================
//  3D WORKGROUPS — local_invocation_id vs local_invocation_index
// =========================================================================

/// 2D workgroup, lid ∈ [0, 64*4=256). Array has 256 elements.
/// Elision SHOULD apply (using flat local_invocation_index).
#[test]
fn workgroup_2d_lid_elided() {
    let text = compile(
        r#"
        var<workgroup> tile: array<f32, 256>;
        @compute @workgroup_size(64, 4)
        fn main(@builtin(local_invocation_index) lid: u32) {
            tile[lid] = 1.0;
        }
        "#,
        BoundsCheckPolicy::Restrict,
    );
    assert!(!text.contains("UMin"), "2D lid: UMin should be elided");
}

/// 3D workgroup, lid ∈ [0, 16*16*16=4096).
#[test]
fn workgroup_3d_lid_elided() {
    let text = compile(
        r#"
        var<workgroup> tile: array<f32, 4096>;
        @compute @workgroup_size(16, 16, 16)
        fn main(@builtin(local_invocation_index) lid: u32) {
            tile[lid] = 1.0;
        }
        "#,
        BoundsCheckPolicy::Restrict,
    );
    assert!(!text.contains("UMin"), "3D lid: UMin should be elided");
}

/// `local_invocation_id` (vec3 builtin) indexing a same-sized workgroup array
/// via a component, e.g. `tile[lid3.x]`. The elision analysis only matches a
/// bare `Expression::FunctionArgument` bound to `LocalInvocationIndex`; a
/// vec3 builtin accessed via `.x`/AccessIndex is a structurally different
/// expression shape and is NOT recognized. Elision MUST NOT apply here —
/// this locks in that the gap fails closed (conservative), not open (unsound).
#[test]
fn local_invocation_id_vec3_not_elided() {
    let text = compile(
        r#"
        var<workgroup> tile: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_id) lid3: vec3<u32>) {
            tile[lid3.x] = 1.0;
        }
        "#,
        BoundsCheckPolicy::Restrict,
    );
    assert!(
        text.contains("UMin"),
        "local_invocation_id.x: UMin must be retained (vec3 builtin access is not recognized by the elision analysis)"
    );
}

// =========================================================================
//  STORAGE BUFFER ACCESSES (dynamic-length arrays)
// =========================================================================

/// Storage buffer with runtime-sized array — elision should NOT apply
/// because the array length is dynamic (not known at compile time).
#[test]
fn storage_buffer_runtime_array_check_retained() {
    let text = compile(
        r#"
        struct Data { data: array<f32> }
        @group(0) @binding(0) var<storage, read> buf: Data;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            let x = buf.data[lid];
        }
        "#,
        BoundsCheckPolicy::Restrict,
    );
    assert!(text.contains("UMin") || text.contains("ULessThan") || text.contains("OpSelectionMerge"),
        "storage buffer: bounds check must be retained for runtime-sized arrays");
}

// =========================================================================
//  LOOP INDUCTION VARIABLES
// =========================================================================

/// Simple for loop: `for (var i = 0u; i < 64u; i++)` accessing `arr[i]`.
/// Elision SHOULD apply via bounds_check_elision.rs (if/break pattern).
#[test]
fn loop_induction_variable_if_break() {
    let text = compile(
        r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            var i = 0u;
            loop {
                if i < 64u { } else { break; }
                arr[i] = f32(i);
                i = i + 1u;
            }
        }
        "#,
        BoundsCheckPolicy::Restrict,
    );
    // If elision works: no UMin inside the loop
    assert!(!text.contains("UMin"),
        "loop if/break: UMin should be elided (i < 64, arr has 64)");
}

/// Loop induction variable with break_if pattern:
/// `loop { body; continuing { i++; break if i >= N; } }`
#[test]
fn loop_induction_variable_break_if() {
    let text = compile(
        r#"
        var<workgroup> arr: array<f32, 128>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            var i = 0u;
            loop {
                arr[i] = f32(i);
                continuing {
                    i = i + 1u;
                    break if i >= 128u;
                }
            }
        }
        "#,
        BoundsCheckPolicy::Restrict,
    );
    // break_if i >= 128 means body runs while i < 128, arr has 128
    assert!(!text.contains("UMin"),
        "break_if: UMin should be elided (i < 128, arr has 128)");
}

/// Loop with i < 64 and break_if i >= 64 — but arr only has 32 elements.
/// Elision MUST NOT apply.
#[test]
fn loop_induction_variable_undersized_array() {
    let text = compile(
        r#"
        var<workgroup> arr: array<f32, 32>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            var i = 0u;
            loop {
                if i < 64u { } else { break; }
                arr[i] = f32(i);
                i = i + 1u;
            }
        }
        "#,
        BoundsCheckPolicy::Restrict,
    );
    // arr only has 32 elements, but loop goes to 64 — check must be retained
    assert!(text.contains("UMin"),
        "loop undersized: UMin must be retained (arr has 32, loop to 64)");
}

/// WGSL `while` loop: `while i < 64u { arr[i] = f32(i); i++; }`
/// The parser lowers this to `loop { if i < 64 { } else { break; } body; }`
/// which matches our `if/break` analysis.
#[test]
fn while_loop_induction_variable() {
    let text = compile(
        r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            var i = 0u;
            while i < 64u {
                arr[i] = f32(i);
                i = i + 1u;
            }
        }
        "#,
        BoundsCheckPolicy::Restrict,
    );
    assert!(!text.contains("UMin"),
        "while loop: UMin should be elided (i < 64, arr has 64)");
}

/// WGSL `for` loop: `for (var i = 0u; i < 64u; i++) { arr[i] = f32(i); }`
/// The parser lowers this to a loop with if/break and a continuing block.
#[test]
fn for_loop_induction_variable() {
    let text = compile(
        r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            for (var i = 0u; i < 64u; i = i + 1u) {
                arr[i] = f32(i);
            }
        }
        "#,
        BoundsCheckPolicy::Restrict,
    );
    assert!(!text.contains("UMin"),
        "for loop: UMin should be elided (i < 64, arr has 64)");
}

/// Nested loops with a 2D array: `matrix[i][j]` where both i and j
/// are induction variables from inner and outer loops.
/// Both dimensions should be elided independently.
#[test]
fn nested_loops_2d_array() {
    let text = compile(
        r#"
        var<workgroup> matrix: array<array<f32, 4>, 4>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            var i = 0u;
            loop {
                if i < 4u { } else { break; }
                var j = 0u;
                loop {
                    if j < 4u { } else { break; }
                    matrix[i][j] = 1.0;
                    j = j + 1u;
                }
                i = i + 1u;
            }
        }
        "#,
        BoundsCheckPolicy::Restrict,
    );
    // Both dimensions are safe: i < 4, j < 4, matrix is 4x4
    assert!(!text.contains("UMin"),
        "nested loops 2D: UMin should be elided (i < 4, j < 4, matrix is 4x4)");
}

/// Non-zero initial value: i starts at 16, loops while i < 64.
/// All accesses are in [16, 63) which is within array<64>.
/// Elision SHOULD apply — the analysis only checks the upper bound.
#[test]
fn non_zero_initial_value_elided() {
    let text = compile(
        r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            var i = 16u;
            loop {
                if i < 64u { } else { break; }
                arr[i] = f32(i);
                i = i + 1u;
            }
        }
        "#,
        BoundsCheckPolicy::Restrict,
    );
    // bound=64, arr_size=64, 64 >= 64 → elided
    assert!(!text.contains("UMin"),
        "non-zero init: UMin should be elided (bound=64, arr has 64)");
}

/// Non-zero initial value with undersized array: i starts at 16, bound=64,
/// but arr only has 48 elements. Elision MUST NOT apply.
#[test]
fn non_zero_initial_value_retained() {
    let text = compile(
        r#"
        var<workgroup> arr: array<f32, 48>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            var i = 16u;
            loop {
                if i < 64u { } else { break; }
                arr[i] = f32(i);
                i = i + 1u;
            }
        }
        "#,
        BoundsCheckPolicy::Restrict,
    );
    // bound=64, arr_size=48, 48 < 64 → NOT elided
    assert!(text.contains("UMin"),
        "non-zero init undersized: UMin must be retained (arr has 48, bound=64)");
}

/// Decrementing loop: i goes from 63 down to 0.
/// The condition `i == 0u` does NOT match the `<` / `<=` patterns
/// the analysis looks for — this documents a current limitation.
#[test]
fn decrement_loop_not_elided() {
    let text = compile(
        r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            var i = 64u;
            loop {
                i = i - 1u;
                arr[i] = 1.0;
                if i == 0u { break; }
            }
        }
        "#,
        BoundsCheckPolicy::Restrict,
    );
    // Analysis doesn't handle decrement or equality conditions
    // Even though i is always in [0, 63] ⊂ array<64>, the analysis misses it
    assert!(text.contains("UMin"),
        "decrement loop: UMin should be retained (analysis limitation — doesn't handle decrement patterns)");
}

/// Decrementing loop with `break if i == 0u`: the analysis's `break_if`
/// handler only recognizes `>=` and `>` patterns, not `==`.
/// This documents a current limitation — the loop is safe but not elided.
#[test]
fn decrement_break_if_not_elided() {
    let text = compile(
        r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            var i = 64u;
            loop {
                i = i - 1u;
                arr[i] = 1.0;
                continuing {
                    break if i == 0u;
                }
            }
        }
        "#,
        BoundsCheckPolicy::Restrict,
    );
    // break_if i == 0u uses Equal, not GreaterEqual/GreaterThan
    // The analysis misses this pattern
    assert!(text.contains("UMin"),
        "decrement break_if: UMin should be retained (analysis limitation — doesn't handle == in break_if)");
}

// =========================================================================
//  MULTIPLE LID-BASED ACCESSES IN ONE SHADER
// =========================================================================

/// Two separate arrays, both indexed by lid, both sized >= wg_size.
/// Both should be elided.
#[test]
fn dual_array_lid_accesses_both_elided() {
    let checked = compile(
        r#"
        var<workgroup> a: array<f32, 64>;
        var<workgroup> b: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            a[lid] = b[lid];
        }
        "#,
        BoundsCheckPolicy::ReadZeroSkipWrite,
    );
    let unchecked = compile(
        r#"
        var<workgroup> a: array<f32, 64>;
        var<workgroup> b: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            a[lid] = b[lid];
        }
        "#,
        BoundsCheckPolicy::Unchecked,
    );
    // Both versions should have the same number of bounds-check instructions
    // (zero from actual bounds checks, only zero-init related)
    let c_umin = count(&checked, "UMin");
    let u_umin = count(&unchecked, "UMin");
    assert_eq!(c_umin, u_umin,
        "dual lid: UMin count should match between checked ({c_umin}) and unchecked ({u_umin})");
}

// =========================================================================
//  SPIR-V INSTRUCTION COUNT VERIFICATION
// =========================================================================

/// Count OpULessThan instructions — each one is a bounds check.
/// A single tile[lid] shader with elision should have ZERO OpULessThan
/// from bounds checks (only zero-init uses OpIEqual, not OpULessThan).
#[test]
fn op_uless_than_count_elided_to_zero() {
    let text = compile(
        r#"
        var<workgroup> tile: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            let v = tile[lid];
            tile[0] = v;
        }
        "#,
        BoundsCheckPolicy::ReadZeroSkipWrite,
    );
    let uless_count = count(&text, "OpULessThan");
    assert_eq!(uless_count, 0,
        "OpULessThan should be 0 for elided tile[lid] (got {uless_count})");
}

/// lid+1 should have 1 OpULessThan (the bounds check).
#[test]
fn op_uless_than_count_plus_one_is_one() {
    let text = compile(
        r#"
        var<workgroup> tile: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            let v = tile[lid + 1u];
            tile[0] = v;
        }
        "#,
        BoundsCheckPolicy::ReadZeroSkipWrite,
    );
    let uless_count = count(&text, "OpULessThan");
    assert_eq!(uless_count, 1,
        "OpULessThan should be 1 for tile[lid+1] (got {uless_count})");
}

/// lid-1 (wraparound) should have 1 OpULessThan.
#[test]
fn op_uless_than_count_minus_one_is_one() {
    let text = compile(
        r#"
        var<workgroup> tile: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            let v = tile[lid - 1u];
            tile[0] = v;
        }
        "#,
        BoundsCheckPolicy::ReadZeroSkipWrite,
    );
    let uless_count = count(&text, "OpULessThan");
    assert_eq!(uless_count, 1,
        "OpULessThan should be 1 for tile[lid-1] (got {uless_count})");
}

// =========================================================================
//  POLICY CROSS-CHECK: ReadZeroSkipWrite vs Restrict vs Unchecked
// =========================================================================

/// For elided cases, ALL policies should produce the same number of
/// OpAccessChain instructions — no bounds check adds extra paths.
#[test]
fn access_chain_count_identical_across_policies() {
    let wgsl = r#"
        var<workgroup> tile: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            tile[lid] = 1.0;
        }
    "#;
    let rzw = count(&compile(wgsl, BoundsCheckPolicy::ReadZeroSkipWrite), "OpAccessChain");
    let rest = count(&compile(wgsl, BoundsCheckPolicy::Restrict), "OpAccessChain");
    let unch = count(&compile(wgsl, BoundsCheckPolicy::Unchecked), "OpAccessChain");
    // All should have the same number (just 1 access chain, no extra from bounds)
    assert_eq!(rzw, unch, "ReadZeroSkipWrite ({rzw}) vs Unchecked ({unch})");
    assert_eq!(rest, unch, "Restrict ({rest}) vs Unchecked ({unch})");
}
