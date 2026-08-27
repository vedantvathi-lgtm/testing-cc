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
    let mut options = naga::back::msl::Options::default();
    options.bounds_check_policies = BoundsCheckPolicies {
        index: BoundsCheckPolicy::Restrict,
        buffer: BoundsCheckPolicy::Restrict,
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
