//! Bounds Check Elision — Stress Tests & Limit Testing
//!
//! These tests deliberately push the elision to its breaking points:
//! - Boundary: array_len == wg_size (exact fit), > wg_size (oversized)
//! - Affine builtin expressions with checked overflow and underflow
//! - 3D workgroups: local_invocation_id.x, (y,z combinations)
//! - Storage buffer accesses (dynamic-length arrays)
//! - Loop induction variables (if/break, break_if)
//! - Multiple lid-based accesses in one shader
//! - SPIR-V instruction count verification

use naga::back::spv;
use naga::proc::{BoundsCheckPolicies, BoundsCheckPolicy};
use naga::valid::{Capabilities, ValidationFlags, Validator};
use rspirv::binary::Disassemble;

fn access_proofs(text: &str) -> Vec<bool> {
    let module = naga::front::wgsl::parse_str(text).expect("WGSL parse");
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .expect("validation");
    let function = &module.entry_points[0].function;
    let function_info = info.get_entry_point(0);

    function
        .expressions
        .iter()
        .filter_map(|(handle, expression)| match *expression {
            naga::Expression::Access { .. } => Some(function_info.safe_accesses.contains(&handle)),
            _ => None,
        })
        .collect()
}

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

fn compile_with_elision(
    text: &str,
    policy: BoundsCheckPolicy,
    bounds_check_elision: bool,
) -> String {
    let module = naga::front::wgsl::parse_str(text).expect("WGSL parse");
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    validator.set_bounds_check_elision(bounds_check_elision);
    let info = validator.validate(&module).expect("validation");
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

fn compile_msl(text: &str) -> String {
    let module = naga::front::wgsl::parse_str(text).expect("WGSL parse");
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .expect("validation");
    let options = naga::back::msl::Options {
        bounds_check_policies: BoundsCheckPolicies {
            index: BoundsCheckPolicy::Restrict,
            buffer: BoundsCheckPolicy::Restrict,
            ..Default::default()
        },
        ..Default::default()
    };
    naga::back::msl::write_string(
        &module,
        &info,
        &options,
        &naga::back::msl::PipelineOptions::default(),
    )
    .expect("MSL codegen")
    .0
}

fn compile_hlsl(text: &str) -> String {
    let module = naga::front::wgsl::parse_str(text).expect("WGSL parse");
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .expect("validation");
    let options = naga::back::hlsl::Options::default();
    let pipeline_options = naga::back::hlsl::PipelineOptions::default();
    let mut output = String::new();
    naga::back::hlsl::Writer::new(&mut output, &options, &pipeline_options)
        .write(&module, &info, None)
        .expect("HLSL codegen");
    output
}

/// Count occurrences of a substring in SPIR-V disassembly.
fn count(text: &str, marker: &str) -> usize {
    text.matches(marker).count()
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

/// lid * 2: when lid = 255, lid*2 = 510, which fits in array<512>.
#[test]
fn lid_times_two_safe_affine_elided() {
    let source = r#"
        var<workgroup> tile: array<f32, 512>;
        @compute @workgroup_size(256)
        fn main(@builtin(local_invocation_index) lid: u32) {
            tile[lid * 2u] = 1.0;
        }
        "#;
    assert_eq!(access_proofs(source), [true]);
    let text = compile(source, BoundsCheckPolicy::Restrict);
    assert!(!text.contains("UMin"), "lid*2: UMin should be elided");
}

#[test]
fn lid_plus_one_safe_affine_elided() {
    let source = r#"
        var<workgroup> tile: array<f32, 65>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            tile[lid + 1u] = 1.0;
        }
    "#;
    assert_eq!(access_proofs(source), [true]);
}

#[test]
fn two_lid_plus_one_exact_fit_elided() {
    let source = r#"
        var<workgroup> tile: array<f32, 128>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            tile[2u * lid + 1u] = 1.0;
        }
    "#;
    assert_eq!(access_proofs(source), [true]);
}

#[test]
fn affine_multiplication_overflow_retains_check() {
    let source = r#"
        var<workgroup> tile: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            tile[lid * 0xffffffffu] = 1.0;
        }
    "#;
    assert_eq!(access_proofs(source), [false]);
}

#[test]
fn same_index_source_is_proved_per_access() {
    let source = r#"
        var<workgroup> wide: array<f32, 64>;
        var<workgroup> narrow: array<f32, 32>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            wide[lid] = 1.0;
            narrow[lid] = 1.0;
        }
    "#;
    assert_eq!(access_proofs(source), [true, false]);
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
/// expression shape. The interval analysis recognizes the component range.
#[test]
fn local_invocation_id_x_elided() {
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
        !text.contains("UMin"),
        "local_invocation_id.x should be elided"
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
    assert!(
        text.contains("UMin") || text.contains("ULessThan") || text.contains("OpSelectionMerge"),
        "storage buffer: bounds check must be retained for runtime-sized arrays"
    );
}

// =========================================================================
//  LOOP INDUCTION VARIABLES
// =========================================================================

/// Simple for loop: `for (var i = 0u; i < 64u; i++)` accessing `arr[i]`.
/// Elision SHOULD apply via bounds_check_elision.rs (if/break pattern).
#[test]
fn loop_induction_variable_if_break() {
    let source = r#"
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
        "#;
    assert_eq!(access_proofs(source), [true]);
    let text = compile(source, BoundsCheckPolicy::Restrict);
    assert!(
        !text.contains("UMin"),
        "loop if/break: UMin should be elided (i < 64, arr has 64)"
    );
}

#[test]
fn loop_guard_does_not_prove_access_before_guard() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 0u;
            loop {
                arr[i] = 1.0;
                if i < 64u { } else { break; }
                i = i + 1u;
            }
        }
    "#;
    assert_eq!(access_proofs(source), [false]);
}

#[test]
fn loop_guard_does_not_prove_access_after_loop() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 0u;
            loop {
                if i < 64u { } else { break; }
                i = i + 1u;
            }
            arr[i] = 1.0;
        }
    "#;
    assert_eq!(access_proofs(source), [false]);
}

#[test]
fn branch_fact_applies_only_to_dominated_arm() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(1)
        fn main(@builtin(local_invocation_index) lid: u32) {
            var i = lid;
            if i < 64u {
                arr[i] = 1.0;
            } else {
                arr[i] = 2.0;
            }
        }
    "#;
    assert_eq!(access_proofs(source), [true, false]);
}

#[test]
fn mutation_after_guard_kills_range_fact() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(1)
        fn main(@builtin(local_invocation_index) lid: u32) {
            var i = lid;
            if i < 64u {
                i = 100u;
                arr[i] = 1.0;
            }
        }
    "#;
    assert_eq!(access_proofs(source), [false]);
}

#[test]
fn captured_load_keeps_value_at_capture_point() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 100u;
            let captured = i;
            i = 0u;
            arr[captured] = 1.0;
        }
    "#;
    assert_eq!(access_proofs(source), [false]);
    assert_eq!(
        count(
            &compile(source, BoundsCheckPolicy::ReadZeroSkipWrite),
            "OpULessThan"
        ),
        1
    );
}

#[test]
fn captured_safe_load_is_not_invalidated_by_later_mutation() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 7u;
            let captured = i;
            i = 100u;
            arr[captured] = 1.0;
        }
    "#;
    assert_eq!(access_proofs(source), [true]);
}

#[test]
fn captured_affine_expression_keeps_value_at_capture_point() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 99u;
            let captured = i + 1u;
            i = 0u;
            arr[captured] = 1.0;
        }
    "#;
    assert_eq!(access_proofs(source), [false]);
}

#[test]
fn early_return_guard_proves_fallthrough_access() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(128)
        fn main(@builtin(local_invocation_index) lid: u32) {
            var i = lid;
            if i >= 64u {
                return;
            }
            arr[i] = 1.0;
        }
    "#;
    assert_eq!(access_proofs(source), [true]);
}

#[test]
fn unsafe_branch_is_preserved_at_join() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            var i = 0u;
            if lid < 32u {
                i = lid;
            } else {
                i = 100u;
            }
            arr[i] = 1.0;
        }
    "#;
    assert_eq!(access_proofs(source), [false]);
}

#[test]
fn safe_branches_join_to_safe_interval() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            var i = 0u;
            if lid < 32u {
                i = lid;
            } else {
                i = 63u;
            }
            arr[i] = 1.0;
        }
    "#;
    assert_eq!(access_proofs(source), [true]);
}

#[test]
fn signed_negative_lower_bound_retains_check() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(1)
        fn main() {
            var i = -1i;
            if i < 64i {
                arr[i] = 1.0;
            }
        }
    "#;
    assert_eq!(access_proofs(source), [false]);
}

#[test]
fn signed_non_negative_guard_elides_check() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 17i;
            if i >= 0i {
                arr[i] = 1.0;
            }
        }
    "#;
    assert_eq!(access_proofs(source), [true]);
}

#[test]
fn short_circuit_compound_guard_remains_checked() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(2)
        fn main(@builtin(local_invocation_index) lid: u32) {
            var i = -1i;
            if lid == 0u {
                i = 17i;
            }
            if i >= 0i && i < 64i {
                arr[i] = 1.0;
            }
        }
    "#;
    assert_eq!(access_proofs(source), [false]);
}

#[test]
fn disjunctive_signed_guard_remains_checked() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(2)
        fn main(@builtin(local_invocation_index) lid: u32) {
            var i = -1i;
            if lid == 0u {
                i = 17i;
            }
            if i < 0i || i < 64i {
                arr[i] = 1.0;
            }
        }
    "#;
    assert_eq!(access_proofs(source), [false]);
}

#[test]
fn negated_guard_refines_fallthrough() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 17i;
            if !(i < 64i) {
                return;
            }
            arr[i] = 1.0;
        }
    "#;
    assert_eq!(access_proofs(source), [true]);
}

#[test]
fn call_does_not_restore_local_initializer() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        fn noop() {}
        @compute @workgroup_size(1)
        fn main() {
            var i = 0u;
            i = 100u;
            noop();
            arr[i] = 1.0;
        }
    "#;
    assert_eq!(access_proofs(source), [false]);
}

#[test]
fn switch_break_path_is_included_after_switch() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(1)
        fn main(@builtin(local_invocation_index) lid: u32) {
            var i = 0u;
            switch lid {
                case 0u: {
                    i = 100u;
                    break;
                }
                default: { i = 0u; }
            }
            arr[i] = 1.0;
        }
    "#;
    assert_eq!(access_proofs(source), [false]);
}

#[test]
fn nested_second_update_rejects_recurrence() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(1)
        fn main(@builtin(local_invocation_index) lid: u32) {
            var i = 0u;
            loop {
                arr[i] = 1.0;
                if lid == 0u {
                    i = i + 100u;
                }
                continuing {
                    i = i + 1u;
                    break if i >= 64u;
                }
            }
        }
    "#;
    assert_eq!(access_proofs(source), [false]);
}

#[test]
fn two_direct_updates_reject_recurrence() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 0u;
            loop {
                arr[i] = 1.0;
                i = i + 1u;
                continuing {
                    i = i + 1u;
                    break if i >= 64u;
                }
            }
        }
    "#;
    assert_eq!(access_proofs(source), [false]);
}

#[test]
fn maximum_inclusive_bound_does_not_overflow_analysis() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 0xffffffffu;
            loop {
                if i <= 0xffffffffu { } else { break; }
                arr[i] = 1.0;
                break;
            }
        }
    "#;
    assert_eq!(access_proofs(source), [false]);
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
    assert!(
        !text.contains("UMin"),
        "break_if: UMin should be elided (i < 128, arr has 128)"
    );
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
    assert!(
        text.contains("UMin"),
        "loop undersized: UMin must be retained (arr has 32, loop to 64)"
    );
}

/// WGSL `while` loop: `while i < 64u { arr[i] = f32(i); i++; }`
/// The parser lowers this to `loop { if i < 64 { } else { break; } body; }`
/// which matches our `if/break` analysis.
#[test]
fn while_loop_induction_variable() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            var i = 0u;
            while i < 64u {
                arr[i] = f32(i);
                i = i + 1u;
            }
        }
        "#;
    assert_eq!(access_proofs(source), [true]);
    let text = compile(source, BoundsCheckPolicy::Restrict);
    assert!(
        !text.contains("UMin"),
        "while loop: UMin should be elided (i < 64, arr has 64)"
    );
}

/// WGSL `for` loop: `for (var i = 0u; i < 64u; i++) { arr[i] = f32(i); }`
/// The parser lowers this to a loop with if/break and a continuing block.
#[test]
fn for_loop_induction_variable() {
    let source = r#"
        var<workgroup> arr: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            for (var i = 0u; i < 64u; i = i + 1u) {
                arr[i] = f32(i);
            }
        }
        "#;
    assert_eq!(access_proofs(source), [true]);
    let text = compile(source, BoundsCheckPolicy::Restrict);
    assert!(
        !text.contains("UMin"),
        "for loop: UMin should be elided (i < 64, arr has 64)"
    );
}

#[test]
fn local_array_for_loop_induction_variable() {
    let source = r#"
        @compute @workgroup_size(1)
        fn main() {
            var tmp: array<i32, 2> = array(1, 2);
            for (var i = 0; i < 2; i++) {
                tmp[i] = 1;
            }
        }
    "#;
    assert_eq!(access_proofs(source), [true]);
}

#[test]
fn continuing_update_does_not_prove_access_before_late_guard() {
    let source = r#"
        @compute @workgroup_size(1)
        fn main() {
            var tmp: array<i32, 2> = array(1, 2);
            var i = 0;
            loop {
                tmp[i] = 1;
                if i < 2 { } else { break; }
                continuing {
                    i = i + 1;
                }
            }
        }
    "#;
    assert_eq!(access_proofs(source), [false]);
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
    assert!(
        !text.contains("UMin"),
        "nested loops 2D: UMin should be elided (i < 4, j < 4, matrix is 4x4)"
    );
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
    assert!(
        !text.contains("UMin"),
        "non-zero init: UMin should be elided (bound=64, arr has 64)"
    );
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
    assert!(
        text.contains("UMin"),
        "non-zero init undersized: UMin must be retained (arr has 48, bound=64)"
    );
}

/// Decrementing loop: i goes from 63 down to 0.
#[test]
fn decrement_loop_elided() {
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
    assert!(!text.contains("UMin"), "decrement loop should be elided");
}

/// Decrementing loop with `break if i == 0u`.
#[test]
fn decrement_break_if_elided() {
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
    assert!(
        !text.contains("UMin"),
        "decrement break_if should be elided"
    );
}

#[test]
fn reachable_step_two_equality_exit_elided() {
    let source = r#"
        var<workgroup> arr: array<f32, 8>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 0u;
            loop {
                arr[i] = 1.0;
                continuing {
                    i = i + 2u;
                    break if i == 8u;
                }
            }
        }
    "#;
    assert_eq!(access_proofs(source), [true]);
}

#[test]
fn unreachable_step_two_equality_exit_retains_check() {
    let source = r#"
        var<workgroup> arr: array<f32, 8>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 0u;
            loop {
                arr[i] = 1.0;
                continuing {
                    i = i + 2u;
                    break if i == 7u;
                }
            }
        }
    "#;
    assert_eq!(access_proofs(source), [false]);
}

#[test]
fn positive_recurrence_overflow_retains_check() {
    let source = r#"
        var<workgroup> arr: array<f32, 2>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 0xfffffffeu;
            loop {
                arr[i - 0xfffffffeu] = 1.0;
                continuing {
                    i = i + 2u;
                    break if i == 0u;
                }
            }
        }
    "#;
    assert_eq!(access_proofs(source), [false]);
}

#[test]
fn negative_recurrence_underflow_retains_check() {
    let source = r#"
        var<workgroup> arr: array<f32, 2>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 1u;
            loop {
                arr[i] = 1.0;
                continuing {
                    i = i - 2u;
                    break if i == 0xffffffffu;
                }
            }
        }
    "#;
    assert_eq!(access_proofs(source), [false]);
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
    assert_eq!(
        c_umin, u_umin,
        "dual lid: UMin count should match between checked ({c_umin}) and unchecked ({u_umin})"
    );
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
    assert_eq!(
        uless_count, 0,
        "OpULessThan should be 0 for elided tile[lid] (got {uless_count})"
    );
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
    assert_eq!(
        uless_count, 1,
        "OpULessThan should be 1 for tile[lid+1] (got {uless_count})"
    );
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
    assert_eq!(
        uless_count, 1,
        "OpULessThan should be 1 for tile[lid-1] (got {uless_count})"
    );
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
    let rzw = count(
        &compile(wgsl, BoundsCheckPolicy::ReadZeroSkipWrite),
        "OpAccessChain",
    );
    let rest = count(&compile(wgsl, BoundsCheckPolicy::Restrict), "OpAccessChain");
    let unch = count(
        &compile(wgsl, BoundsCheckPolicy::Unchecked),
        "OpAccessChain",
    );
    // All should have the same number (just 1 access chain, no extra from bounds)
    assert_eq!(rzw, unch, "ReadZeroSkipWrite ({rzw}) vs Unchecked ({unch})");
    assert_eq!(rest, unch, "Restrict ({rest}) vs Unchecked ({unch})");
}

#[test]
fn validator_toggle_isolates_elision_effect() {
    let source = r#"
        var<workgroup> tile: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            let value = tile[lid];
            tile[0] = value;
        }
    "#;
    let enabled = compile_with_elision(source, BoundsCheckPolicy::ReadZeroSkipWrite, true);
    let disabled = compile_with_elision(source, BoundsCheckPolicy::ReadZeroSkipWrite, false);
    assert_eq!(count(&enabled, "OpULessThan"), 0);
    assert_eq!(count(&disabled, "OpULessThan"), 1);
}

#[test]
fn msl_uses_access_specific_proof() {
    let safe = r#"
        var<workgroup> tile: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            tile[lid] = 1.0;
        }
    "#;
    let unsafe_source = r#"
        var<workgroup> tile: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            tile[lid + 1u] = 1.0;
        }
    "#;
    assert!(!compile_msl(safe).contains("metal::min(unsigned("));
    assert!(compile_msl(unsafe_source).contains("metal::min(unsigned("));
}

#[test]
fn hlsl_uses_access_specific_proof() {
    let safe = r#"
        var<workgroup> tile: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            tile[lid] = 1.0;
        }
    "#;
    let unsafe_source = r#"
        var<workgroup> tile: array<f32, 64>;
        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            tile[lid + 1u] = 1.0;
        }
    "#;
    assert!(!compile_hlsl(safe).contains("min(uint("));
    assert!(compile_hlsl(unsafe_source).contains("min(uint("));
}

// ---------------------------------------------------------------------------
// Branch refinement: an unmodellable condition must mean "nothing learned",
// never "branch infeasible". WGSL lowers `&&`/`||` to control flow, but the
// GLSL and SPIR-V front ends produce `LogicalAnd`/`LogicalOr` expressions, so
// those inputs are needed to reach the conjunction/disjunction refinement.
// ---------------------------------------------------------------------------

/// Proof results for every `Access` in every function of `module`.
fn module_access_proofs(module: &naga::Module) -> Vec<bool> {
    module_access_proofs_where(module, |_| true)
}

/// Proof results for `Access` expressions whose base is a `var<workgroup>`
/// global. The SPIR-V front end turns constant buffer indexing into `Access`
/// too, so tests on SPIR-V input select the workgroup array explicitly.
fn workgroup_access_proofs(module: &naga::Module) -> Vec<bool> {
    module_access_proofs_where(module, |base| {
        base.is_some_and(|global| {
            module.global_variables[global].space == naga::AddressSpace::WorkGroup
        })
    })
}

fn module_access_proofs_where(
    module: &naga::Module,
    keep: impl Fn(Option<naga::Handle<naga::GlobalVariable>>) -> bool,
) -> Vec<bool> {
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(module)
        .expect("validation");
    let mut proofs = Vec::new();
    for (handle, function) in module.functions.iter() {
        let function_info = &info[handle];
        proofs.extend(function.expressions.iter().filter_map(
            |(expr, expression)| match *expression {
                naga::Expression::Access { base, .. } if keep(base_global(function, base)) => {
                    Some(function_info.safe_accesses.contains(&expr))
                }
                _ => None,
            },
        ));
    }
    for (index, entry_point) in module.entry_points.iter().enumerate() {
        let function_info = info.get_entry_point(index);
        let function = &entry_point.function;
        proofs.extend(function.expressions.iter().filter_map(
            |(expr, expression)| match *expression {
                naga::Expression::Access { base, .. } if keep(base_global(function, base)) => {
                    Some(function_info.safe_accesses.contains(&expr))
                }
                _ => None,
            },
        ));
    }
    proofs
}

fn base_global(
    function: &naga::Function,
    base: naga::Handle<naga::Expression>,
) -> Option<naga::Handle<naga::GlobalVariable>> {
    match function.expressions[base] {
        naga::Expression::GlobalVariable(global) => Some(global),
        naga::Expression::Access { base, .. } | naga::Expression::AccessIndex { base, .. } => {
            base_global(function, base)
        }
        _ => None,
    }
}

fn parse_glsl_compute(text: &str) -> naga::Module {
    naga::front::glsl::Frontend::default()
        .parse(
            &naga::front::glsl::Options::from(naga::ShaderStage::Compute),
            text,
        )
        .expect("GLSL parse")
}

fn glsl_access_proofs(text: &str) -> Vec<bool> {
    module_access_proofs(&parse_glsl_compute(text))
}

/// Compile GLSL to SPIR-V with naga, then analyse the module produced by the
/// SPIR-V front end, so the conjunction arrives through SPIR-V input.
fn spv_roundtrip_access_proofs(text: &str) -> Vec<bool> {
    let module = parse_glsl_compute(text);
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .expect("validation");
    let words = spv::write_vec(&module, &info, &spv::Options::default(), None).expect("SPIR-V out");
    let bytes: Vec<u8> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
    let module = naga::front::spv::parse_u8_slice(&bytes, &naga::front::spv::Options::default())
        .expect("SPIR-V parse");
    workgroup_access_proofs(&module)
}

const GLSL_PRELUDE: &str = "#version 450
layout(local_size_x = 1) in;
layout(std430, set = 0, binding = 0) buffer B { int o[4]; };
shared float t[8];
void main() {
    int a = o[0];
    int b = o[1];
";

#[test]
fn glsl_and_branch_assignment_retains_check() {
    let source = format!(
        "{GLSL_PRELUDE}    uint i = 0u;
    if (a < b && b > 0) {{ i = 100u; }}
    t[i] = 1.0;
}}
"
    );
    assert_eq!(glsl_access_proofs(&source), [false]);
}

#[test]
fn glsl_or_else_assignment_retains_check() {
    let source = format!(
        "{GLSL_PRELUDE}    uint i = 0u;
    if (a < b || b > 0) {{ }} else {{ i = 100u; }}
    t[i] = 1.0;
}}
"
    );
    assert_eq!(glsl_access_proofs(&source), [false]);
}

#[test]
fn spv_input_and_branch_assignment_retains_check() {
    let source = format!(
        "{GLSL_PRELUDE}    uint i = 0u;
    if (a < b && b > 0) {{ i = 100u; }}
    t[i] = 1.0;
}}
"
    );
    assert_eq!(spv_roundtrip_access_proofs(&source), [false]);
}

#[test]
fn glsl_and_branch_safe_assignment_elides() {
    let source = format!(
        "{GLSL_PRELUDE}    uint i = 0u;
    if (a < b && b > 0) {{ i = 3u; }}
    t[i] = 1.0;
}}
"
    );
    assert_eq!(glsl_access_proofs(&source), [true]);
}

#[test]
fn glsl_and_modellable_part_still_refines() {
    let source = format!(
        "{GLSL_PRELUDE}    uint i = uint(o[2]);
    if (i < 4u && a < b) {{ t[i] = 1.0; }}
}}
"
    );
    assert_eq!(glsl_access_proofs(&source), [true]);
}

#[test]
fn two_variable_comparison_does_not_stop_analysis() {
    let source = r#"
        @group(0) @binding(0) var<uniform> n: vec2<i32>;
        var<workgroup> sh: array<u32, 256>;
        @compute @workgroup_size(256)
        fn main(@builtin(local_invocation_id) local_id: vec3<u32>) {
            var a = n.x;
            var b = n.y;
            if a < b {}
            sh[local_id.x] = 1u;
        }
    "#;
    assert_eq!(access_proofs(source), [true]);
}

#[test]
fn two_variable_comparison_if_else_does_not_stop_analysis() {
    let source = r#"
        @group(0) @binding(0) var<uniform> n: vec2<i32>;
        var<workgroup> sh: array<u32, 256>;
        @compute @workgroup_size(256)
        fn main(@builtin(local_invocation_id) local_id: vec3<u32>) {
            var a = n.x;
            var b = n.y;
            var c = 0;
            if a < b {
                c = a;
            } else {
                c = b;
            }
            sh[local_id.x] = 1u;
        }
    "#;
    assert_eq!(access_proofs(source), [true]);
}

#[test]
fn infeasible_branch_still_recognised() {
    let source = r#"
        var<workgroup> t: array<f32, 8>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 5u;
            if i < 3u {
                i = 100u;
            }
            t[i] = 1.0;
        }
    "#;
    assert_eq!(access_proofs(source), [true]);
}

#[test]
fn wgsl_non_short_circuit_and_else_assignment_retains_check() {
    // `&` on booleans is not lowered to control flow, so the condition reaches
    // refinement as `Binary And`, which is not a comparison.
    let source = r#"
        @group(0) @binding(0) var<uniform> n: vec2<i32>;
        var<workgroup> t: array<f32, 8>;
        @compute @workgroup_size(1)
        fn main() {
            let a = n.x;
            let b = n.y;
            var i = 0u;
            if (a < b) & (b > 0) {
            } else {
                i = 100u;
            }
            t[i] = 1.0;
        }
    "#;
    assert_eq!(access_proofs(source), [false]);
}

#[test]
fn wgsl_bool_equality_else_assignment_retains_check() {
    // Comparing a `bool` local: the local is not a 32-bit integer, so there is
    // nothing to refine — which must not make either branch infeasible.
    let source = r#"
        @group(0) @binding(0) var<uniform> n: vec2<i32>;
        var<workgroup> t: array<f32, 8>;
        @compute @workgroup_size(1)
        fn main() {
            var flag = n.x > 0;
            var i = 0u;
            if flag == true {
            } else {
                i = 100u;
            }
            t[i] = 1.0;
        }
    "#;
    assert_eq!(access_proofs(source), [false]);
}

// ---------------------------------------------------------------------------
// Feature 005: rules targeted at the residual checks measured by paired-vulkan-4
// (runs-v4/summary-{nvidia,intel}.json). Each rule has a must-elide test and a
// must-retain neighbour that differs by one constant.
// ---------------------------------------------------------------------------

/// Proof of the single dynamic access `t[<index>]` in a compute shader with
/// `t: array<u32, len>`, workgroup size `wg`, a runtime `x: u32`, runtime
/// `xi: i32`, and `prelude` statements before the access.
fn single_proof(len: u32, wg: u32, prelude: &str, index: &str) -> bool {
    let source = format!(
        "@group(0) @binding(0) var<storage, read> xs: array<u32, 4>;
        @group(0) @binding(1) var<storage, read> xis: array<i32, 4>;
        var<workgroup> t: array<u32, {len}>;
        @compute @workgroup_size({wg})
        fn main(@builtin(local_invocation_index) lid: u32) {{
            let x = xs[1];
            let xi = xis[1];
            {prelude}
            t[{index}] = lid;
        }}"
    );
    let proofs = access_proofs(&source);
    assert_eq!(
        proofs.len(),
        1,
        "expected exactly one dynamic access: {proofs:?}"
    );
    proofs[0]
}

#[test]
fn modulo_by_constant_bounds_index() {
    assert!(single_proof(8, 64, "", "lid % 8u"));
    assert!(!single_proof(8, 64, "", "lid % 9u"));
    assert!(single_proof(8, 1, "", "x % 8u"));
}

#[test]
fn signed_modulo_of_unknown_value_is_not_non_negative() {
    // xi % 8 is in [-7, 7] for i32; converting to u32 can wrap to a huge index.
    assert!(!single_proof(8, 1, "", "u32(xi % 8)"));
}

#[test]
fn shift_right_and_divide_by_constant_bound_index() {
    assert!(single_proof(8, 64, "", "lid >> 3u"));
    assert!(!single_proof(8, 64, "", "lid >> 2u"));
    assert!(single_proof(8, 64, "", "lid / 8u"));
    assert!(!single_proof(8, 64, "", "lid / 4u"));
}

#[test]
fn and_mask_bounds_index() {
    assert!(single_proof(8, 1, "", "x & 7u"));
    assert!(!single_proof(8, 1, "", "x & 8u"));
}

#[test]
fn min_max_clamp_bound_index() {
    assert!(single_proof(8, 1, "", "min(x, 7u)"));
    assert!(!single_proof(8, 1, "", "min(x, 8u)"));
    assert!(single_proof(8, 1, "", "clamp(x, 0u, 7u)"));
    assert!(!single_proof(8, 1, "", "clamp(x, 0u, 8u)"));
    assert!(!single_proof(8, 1, "", "max(x, 3u)"));
}

#[test]
fn select_joins_both_arms() {
    assert!(single_proof(8, 1, "", "select(1u, 7u, x > 3u)"));
    assert!(!single_proof(8, 1, "", "select(1u, 8u, x > 3u)"));
}

#[test]
fn int_conversion_keeps_proven_non_negative_range() {
    let source = "var<workgroup> t: array<u32, 4>;
        @compute @workgroup_size(1)
        fn main() {
            for (var c = 0; c < 4; c = c + 1) {
                t[u32(c)] = 1u;
            }
        }";
    assert_eq!(access_proofs(source), [true]);
    // an unknown i32 may be negative and wrap
    assert!(!single_proof(8, 1, "", "u32(xi)"));
}

#[test]
fn guard_on_builtin_refines_builtin() {
    assert!(single_proof(8, 64, "if lid >= 8u { return; }", "lid"));
    assert!(!single_proof(8, 64, "if lid >= 9u { return; }", "lid"));
}

#[test]
fn guard_with_range_valued_bound_refines() {
    // n is in [0, 8] (min), so i < n implies i <= 7.
    let elide = "var<workgroup> t: array<u32, 8>;
        @group(0) @binding(0) var<storage, read> xs: array<u32, 4>;
        @compute @workgroup_size(1)
        fn main() {
            let n = min(xs[1], 8u);
            for (var i = 0u; i < n; i = i + 1u) {
                t[i] = 1u;
            }
        }";
    assert_eq!(access_proofs(elide), [true]);
    let retain = elide.replace("min(xs[1], 8u)", "min(xs[1], 9u)");
    assert_eq!(access_proofs(&retain), [false]);
}

#[test]
fn shift_right_recurrence_bounds_loop_variable() {
    let elide = "var<workgroup> t: array<u32, 128>;
        @compute @workgroup_size(1)
        fn main() {
            for (var s = 128u; s > 0u; s = s >> 1u) {
                t[s - 1u] = 1u;
            }
        }";
    assert_eq!(access_proofs(elide), [true]);
    let retain = elide.replace("array<u32, 128>", "array<u32, 127>");
    assert_eq!(access_proofs(&retain), [false]);
}

#[test]
fn shift_recurrence_with_second_mutation_retains_check() {
    let source = "var<workgroup> t: array<u32, 128>;
        @group(0) @binding(0) var<storage, read> xs: array<u32, 4>;
        @compute @workgroup_size(1)
        fn main() {
            for (var s = 128u; s > 0u; s = s >> 1u) {
                if xs[0] == 1u { s = 1000u; }
                t[s - 1u] = 1u;
            }
        }";
    assert_eq!(access_proofs(source), [false]);
}

#[test]
fn tree_reduction_partner_access_elides() {
    // The reduce_tree benchmark kernel: s[lid + stride] under `lid < stride`.
    let elide = "var<workgroup> s: array<u32, 256>;
        @compute @workgroup_size(256)
        fn main(@builtin(local_invocation_index) lid: u32) {
            for (var stride = 128u; stride > 0u; stride = stride >> 1u) {
                if (lid < stride) {
                    s[lid] = s[lid] + s[lid + stride];
                }
                workgroupBarrier();
            }
        }";
    assert_eq!(access_proofs(elide), [true, true, true]);
    // stride starting at 256 lets lid + stride reach 511
    let retain = elide.replace("var stride = 128u", "var stride = 256u");
    assert_eq!(access_proofs(&retain), [true, true, false]);
}

#[test]
fn top_guard_equality_exit_elides() {
    let elide = "var<workgroup> t: array<u32, 8>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 0u;
            loop {
                if (i == 8u) { break; }
                t[i] = 1u;
                i = i + 1u;
            }
        }";
    assert_eq!(access_proofs(elide), [true]);
    let retain = elide.replace("i == 8u", "i == 9u");
    assert_eq!(access_proofs(&retain), [false]);
}

// ---------------------------------------------------------------------------
// A loop guard only bounds the loop variable if every iteration passes it: a
// `continue` before the guard skips it while the `continuing` update still runs.
// ---------------------------------------------------------------------------

const CONTINUE_BEFORE_GUARD: &str = "@group(0) @binding(0) var<storage, read> xs: array<u32, 4>;
    var<workgroup> t: array<u32, 9>;
    @compute @workgroup_size(1)
    fn main() {
        var i = 0u;
        loop {
            t[i] = 1u;
            if (xs[0] == 1u) { continue; }
            if (i >= 8u) { break; }
            continuing { i = i + 1u; }
        }
    }";

#[test]
fn continue_before_loop_guard_retains_check() {
    // With xs[0] == 1 the guard never runs and i passes 8: t[9] is out of bounds.
    assert_eq!(access_proofs(CONTINUE_BEFORE_GUARD), [false]);
}

#[test]
fn loop_guard_without_continue_still_bounds_variable() {
    let source = CONTINUE_BEFORE_GUARD.replace("if (xs[0] == 1u) { continue; }\n", "");
    assert_eq!(access_proofs(&source), [true]);
}

#[test]
fn continue_after_loop_guard_still_bounds_variable() {
    let source = CONTINUE_BEFORE_GUARD
        .replace("            if (xs[0] == 1u) { continue; }\n", "")
        .replace(
            "if (i >= 8u) { break; }",
            "if (i >= 8u) { break; }\n            if (xs[0] == 1u) { continue; }",
        );
    assert_eq!(access_proofs(&source), [true]);
}

// ---------------------------------------------------------------------------
// Feature 006: relational facts (a >= b gives a - b >= 0) and geometric loop
// variables (i *= c, i <<= c). Target: the Hillis-Steele scan residual measured
// by paired-vulkan-4 (+3.65% Intel, +0.71% NVIDIA).
// ---------------------------------------------------------------------------

const PREFIX_SCAN: &str = "var<workgroup> s: array<u32, 256>;
    @compute @workgroup_size(256)
    fn main(@builtin(local_invocation_index) lid: u32) {
        for (var off = 1u; off < 256u; off = off * 2u) {
            var t = 0u;
            if (lid >= off) {
                t = s[lid - off];
            }
            workgroupBarrier();
            s[lid] = s[lid] + t;
            workgroupBarrier();
        }
    }";

#[test]
fn guarded_difference_is_non_negative() {
    // s[lid - off], s[lid] (store), s[lid] (load)
    assert_eq!(access_proofs(PREFIX_SCAN), [true, true, true]);
}

#[test]
fn guarded_difference_minus_one_retains_check() {
    let source = PREFIX_SCAN.replace("s[lid - off]", "s[lid - off - 1u]");
    assert_eq!(access_proofs(&source), [false, true, true]);
}

#[test]
fn relation_is_dropped_when_operand_is_mutated() {
    let source = PREFIX_SCAN.replace(
        "t = s[lid - off];",
        "off = off + 1u;\n                t = s[lid - off];",
    );
    assert!(!access_proofs(&source)[0]);
}

#[test]
fn relation_does_not_survive_the_branch() {
    let source = PREFIX_SCAN.replace(
        "if (lid >= off) {\n                t = s[lid - off];\n            }",
        "if (lid >= off) {\n                t = 1u;\n            }\n            t = t + s[lid - off];",
    );
    assert!(!access_proofs(&source)[0]);
}

#[test]
fn geometric_loop_variable_is_bounded_by_its_exit() {
    let elide = "var<workgroup> t: array<u32, 255>;
        @compute @workgroup_size(1)
        fn main() {
            for (var off = 1u; off < 256u; off = off * 2u) {
                t[off - 1u] = 1u;
            }
        }";
    assert_eq!(access_proofs(elide), [true]);
    let shift = elide.replace("off = off * 2u", "off = off << 1u");
    assert_eq!(access_proofs(&shift), [true]);
    // off reaches 128, so t[off] needs 129 elements
    let retain = elide
        .replace("array<u32, 255>", "array<u32, 128>")
        .replace("t[off - 1u]", "t[off]");
    assert_eq!(access_proofs(&retain), [false]);
}

// ---------------------------------------------------------------------------
// Soundness regressions found by the 2026-10-01 audit (S1 stale loads, S2
// `continue` edges into `continuing`, S3 in-body re-initialisation, and
// override-sized workgroups). Every `retain` shader performs an out-of-bounds
// access on some execution; before the fix its check was removed.
// ---------------------------------------------------------------------------

#[test]
fn stale_load_snapshot_does_not_refine_the_current_value() {
    let retain = "@group(0) @binding(0) var<storage, read> xs: array<u32, 4>;
        var<workgroup> t: array<u32, 4>;
        @compute @workgroup_size(1)
        fn main() {
            var i = xs[0];
            let small = i < 4u;
            i = xs[1];
            if (small) { t[i] = 1u; }
        }";
    assert_eq!(access_proofs(retain), [false]);
    // Without the intervening store the guard does bound `i`.
    let elide = retain.replace("            i = xs[1];\n", "");
    assert_eq!(access_proofs(&elide), [true]);
}

#[test]
fn loop_guard_on_a_pre_update_snapshot_retains_check() {
    // `done` is computed before the update, so the loop still runs with i == 9.
    let retain = "var<workgroup> t9: array<u32, 9>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 0u;
            loop { t9[i] = 1u; let done = i >= 9u; i = i + 1u; if (done) { break; } }
        }";
    assert_eq!(access_proofs(retain), [false]);
    let continuing = "var<workgroup> t9: array<u32, 9>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 0u;
            loop {
                t9[i] = 1u;
                continuing { let d = i >= 9u; i = i + 1u; break if d; }
            }
        }";
    assert_eq!(access_proofs(continuing), [false]);
    // Comparing the updated value is fine: i < 9 at every access.
    let elide = "var<workgroup> t9: array<u32, 9>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 0u;
            loop { t9[i] = 1u; i = i + 1u; if (i >= 9u) { break; } }
        }";
    assert_eq!(access_proofs(elide), [true]);
}

#[test]
fn glsl_postfix_increment_guard_retains_check() {
    // `i++ < 7u` compares the old value; the access uses the new one (up to 7).
    let retain = "#version 450
layout(local_size_x = 1) in;
layout(std430, set = 0, binding = 0) buffer X { uint xs[]; };
shared uint t[7];
void main() { uint i = xs[0]; if (i++ < 7u) { t[i] = 1u; } }
";
    assert!(glsl_access_proofs(retain).iter().all(|&proved| !proved));
    let elide = retain.replace("if (i++ < 7u) { t[i] = 1u; }", "if (i < 7u) { t[i] = 1u; }");
    assert!(glsl_access_proofs(&elide).contains(&true));
}

#[test]
fn continue_edge_reaches_continuing_block() {
    // With xs[0] == 1 the body continues before decrementing, so `continuing`
    // sees i == 8.
    let retain = "@group(0) @binding(0) var<storage, read> xs: array<u32, 4>;
        var<workgroup> t: array<u32, 8>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 8u;
            loop {
                if (i == 0u) { break; }
                if (xs[0] == 1u) { continue; }
                i = i - 1u;
                continuing { t[i] = 1u; }
            }
        }";
    assert_eq!(access_proofs(retain), [false]);
    let skip_guard = "var<workgroup> t: array<u32, 4>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 0u;
            loop {
                if (i >= 4u) { continue; }
                continuing { t[i] = 1u; i = i + 1u; break if i >= 10u; }
            }
        }";
    assert_eq!(access_proofs(skip_guard), [false]);
    // Both paths into `continuing` keep i in [0, 2].
    let elide = "@group(0) @binding(0) var<storage, read> xs: array<u32, 4>;
        var<workgroup> t: array<u32, 4>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 0u;
            loop {
                if (i >= 3u) { break; }
                if (xs[0] == 1u) { continue; }
                continuing { t[i] = 1u; i = i + 1u; }
            }
        }";
    assert_eq!(access_proofs(elide), [true]);
}

#[test]
fn in_body_reinitialisation_is_not_a_loop_start() {
    // The body resets i to 100, so the second iteration indexes t[99].
    let retain = "var<workgroup> t: array<u32, 9>;
        @compute @workgroup_size(1)
        fn main() {
            var i = 8u;
            loop { t[i] = 1u; i = 100u; if (i == 0u) { break; } i = i - 1u; }
        }";
    assert_eq!(access_proofs(retain), [false]);
}

#[test]
fn override_sized_workgroup_is_proved_only_after_overrides_resolve() {
    let source = "override WG: u32 = 64;
        var<workgroup> a: array<u32, 1>;
        var<workgroup> b: array<u32, 64>;
        @compute @workgroup_size(WG)
        fn main(@builtin(local_invocation_id) l: vec3u) { a[l.x] = 1u; b[l.x] = 1u; }";
    let module = naga::front::wgsl::parse_str(source).expect("WGSL parse");
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .expect("validation");
    // Before overrides are resolved the size is a placeholder: nothing is proved.
    assert!(info.get_entry_point(0).safe_accesses.is_empty());
    let constants = naga::back::PipelineConstants::from_iter([("WG".to_string(), 64.0)]);
    let (resolved, resolved_info) =
        naga::back::pipeline_constants::process_overrides(&module, &info, None, &constants)
            .expect("overrides");
    let function = &resolved.entry_points[0].function;
    let proofs: Vec<bool> = function
        .expressions
        .iter()
        .filter_map(|(handle, expression)| match *expression {
            naga::Expression::Access { .. } => Some(
                resolved_info
                    .get_entry_point(0)
                    .safe_accesses
                    .contains(&handle),
            ),
            _ => None,
        })
        .collect();
    // l.x < 64: fits b (64 elements), not a (1 element).
    assert_eq!(proofs, [false, true]);
}

// =========================================================================
//  RUNTIME-SIZED ARRAYS (spec 006): `i < arrayLength(&a)` proves `a[i]`
// =========================================================================

fn runtime_array_umins(body: &str) -> usize {
    let text = compile(
        &format!(
            r#"
            struct S {{ n: u32, data: array<u32> }}
            @group(0) @binding(0) var<storage, read_write> a: array<u32>;
            @group(0) @binding(1) var<storage, read_write> b: array<u32>;
            @group(0) @binding(2) var<storage, read_write> s: S;
            @compute @workgroup_size(64)
            fn main(@builtin(global_invocation_id) g: vec3<u32>) {{
                {body}
            }}
            "#
        ),
        BoundsCheckPolicy::Restrict,
    );
    count(&text, "UMin")
}

#[test]
fn runtime_array_guarded_by_its_length_elided() {
    let body = "let i = g.x; if (i < arrayLength(&a)) { a[i] = 1u; }";
    assert_eq!(runtime_array_umins(body), 0);
}

#[test]
fn runtime_array_early_return_guard_elided() {
    let body = "let i = g.x; if (i >= arrayLength(&a)) { return; } a[i] = 1u;";
    assert_eq!(runtime_array_umins(body), 0);
}

#[test]
fn runtime_array_loop_to_length_elided() {
    let body = "for (var i = 0u; i < arrayLength(&a); i++) { a[i] = i; }";
    assert_eq!(runtime_array_umins(body), 0);
}

#[test]
fn runtime_array_struct_member_elided() {
    let body = "let i = g.x; if (i < arrayLength(&s.data)) { s.data[i] = 1u; }";
    assert_eq!(runtime_array_umins(body), 0);
}

#[test]
fn runtime_array_guard_on_other_buffer_retained() {
    let body = "let i = g.x; if (i < arrayLength(&a)) { b[i] = 1u; }";
    assert_eq!(runtime_array_umins(body), 1);
}

#[test]
fn runtime_array_non_strict_guard_retained() {
    let body = "let i = g.x; if (i <= arrayLength(&a)) { a[i] = 1u; }";
    assert_eq!(runtime_array_umins(body), 1);
}

#[test]
fn runtime_array_index_changed_after_guard_retained() {
    let body = "var i = g.x; if (i < arrayLength(&a)) { i = i + 1u; a[i] = 1u; }";
    assert_eq!(runtime_array_umins(body), 1);
}

#[test]
fn runtime_array_reject_edge_retained() {
    let body = "let i = g.x; if (i < arrayLength(&a)) { } else { a[i] = 1u; }";
    assert_eq!(runtime_array_umins(body), 1);
}

#[test]
fn runtime_array_non_dominating_guard_retained() {
    let body = "let i = g.x; if (g.y == 0u) { if (i < arrayLength(&a)) { } } a[i] = 1u;";
    assert_eq!(runtime_array_umins(body), 1);
}

#[test]
fn runtime_array_signed_index_without_lower_bound_retained() {
    let body = "let i = i32(g.x) - 5; if (u32(i) < arrayLength(&a)) { a[i] = 1u; }";
    assert_eq!(runtime_array_umins(body), 1);
}

#[test]
fn runtime_array_length_bound_by_let_elided() {
    let body = "let n = arrayLength(&a); let i = g.x; if (i < n) { a[i] = 1u; }";
    assert_eq!(runtime_array_umins(body), 0);
}

#[test]
fn runtime_array_loop_index_bumped_after_guard_retained() {
    let body = "for (var i = 0u; i < arrayLength(&a); i++) { i = i + 1u; a[i] = i; }";
    assert_eq!(runtime_array_umins(body), 1);
}

// =========================================================================
//  CALLS: a call only writes the caller's locals it receives pointers to
// =========================================================================

fn local_array_umins(prelude: &str, body: &str) -> usize {
    let text = compile(
        &format!(
            r#"
            @group(0) @binding(0) var<storage, read_write> o: array<f32>;
            {prelude}
            @compute @workgroup_size(1)
            fn main() {{
                var area: array<f32, 4>;
                {body}
                o[0] = area[0];
            }}
            "#
        ),
        BoundsCheckPolicy::Restrict,
    );
    count(&text, "UMin")
}

#[test]
fn call_without_pointer_arguments_keeps_loop_fact() {
    let prelude = "fn shade(x: f32) -> f32 { return x * 2.0; }";
    let body = "for (var i = 0u; i < 4u; i += 1u) { let s = shade(f32(i)); area[i] = s; }";
    assert_eq!(local_array_umins(prelude, body), 1);
}

#[test]
fn call_with_pointer_to_index_retained() {
    let prelude = "fn bump(p: ptr<function, u32>) { *p = *p + 5u; }";
    let body = "for (var i = 0u; i < 4u; i += 1u) { bump(&i); area[i] = 1.0; }";
    assert_eq!(local_array_umins(prelude, body), 2);
}

#[test]
fn call_with_pointer_to_struct_holding_index_retained() {
    let prelude = "struct C { i: u32 } fn bump(p: ptr<function, C>) { (*p).i = 9u; }";
    let body = "var c = C(0u); if (c.i < 4u) { bump(&c); area[c.i] = 1.0; }";
    assert_eq!(local_array_umins(prelude, body), 2);
}

// =========================================================================
//  REPEATED EXPRESSIONS: a guard on `e` covers a re-computed copy of `e`
// =========================================================================

fn scan_umins(body: &str) -> usize {
    let text = compile(
        &format!(
            r#"
            @group(0) @binding(0) var<storage, read_write> o: array<u32>;
            var<workgroup> sh: array<u32, 256>;
            @compute @workgroup_size(256)
            fn main(@builtin(local_invocation_id) local_id: vec3<u32>) {{
                {body}
            }}
            "#
        ),
        BoundsCheckPolicy::Restrict,
    );
    count(&text, "UMin")
}

#[test]
fn repeated_sum_guard_covers_upsweep_access() {
    let body = "for (var i = 0u; i < 8u; i += 1u) {
        if local_id.x + (1u << i) < 256u { sh[local_id.x] = sh[local_id.x + (1u << i)]; }
        workgroupBarrier();
    }";
    assert_eq!(scan_umins(body), 0);
}

#[test]
fn repeated_relation_guard_covers_downsweep_access() {
    let body = "for (var i = 0u; i < 8u; i += 1u) {
        if local_id.x >= 1u << i { sh[local_id.x] = sh[local_id.x - (1u << i)]; }
        workgroupBarrier();
    }";
    assert_eq!(scan_umins(body), 0);
}

#[test]
fn repeated_builtin_guard_covers_minus_one() {
    let body = "if local_id.x > 0u { o[0] = sh[local_id.x - 1u]; }";
    assert_eq!(scan_umins(body), 1);
}

#[test]
fn repeated_expression_after_store_retained() {
    let body = "var k = o[0];
        if local_id.x + k < 256u { k = k + 300u; sh[local_id.x + k] = 1u; }";
    assert_eq!(scan_umins(body), 2);
}

#[test]
fn similar_but_different_expression_retained() {
    let body = "for (var i = 0u; i < 8u; i += 1u) {
        if local_id.x + (1u << i) < 256u { sh[local_id.x] = sh[local_id.x + (2u << i)]; }
    }";
    assert_eq!(scan_umins(body), 1);
}

#[test]
fn repeated_expression_across_diverging_join_retained() {
    let body = "var k = o[0];
        if local_id.x + k < 256u { o[1] = 0u; }
        if o[2] == 0u { k = 7u; }
        sh[local_id.x + k] = 1u;";
    assert_eq!(scan_umins(body), 4);
}

#[test]
fn repeated_expression_after_loop_carried_update_retained() {
    let body = "var k = 0u;
        loop {
            if local_id.x + k < 256u { o[1] = 0u; }
            k = k + 200u;
            if k > 1000u { break; }
            sh[local_id.x + k] = 1u;
        }";
    assert_eq!(scan_umins(body), 2);
}
