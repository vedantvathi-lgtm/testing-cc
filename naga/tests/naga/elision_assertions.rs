//! Tier 1 + Tier 2 explicit assertion tests for builtin-seeded bounds-check
//! elision (IICT paper §4 / §4.3, design doc Tiers 1-2).
//!
//! This module is deliberately independent of the `naga_test` snapshot
//! harness used elsewhere in this directory. The snapshot system (see
//! `snapshots.rs`) is the right tool for *regression* detection once a case
//! is known-good (CI catches drift via `git diff --exit-code naga/tests/out`),
//! but it doesn't itself assert anything about *which* opcodes appear - a
//! human has to eyeball the generated `.spvasm` once and trust it forever
//! after. The whole point of bounds-check elision is a *soundness* claim, so
//! this file states the claim as an explicit, readable assertion per case,
//! using the same `rspirv` disassembler the snapshot system already depends
//! on (`naga/Cargo.toml` `[dev-dependencies]` - no new dependency needed).
//!
//! HOW TO WIRE THIS IN: add `mod elision_assertions;` to `tests/naga/main.rs`.
//!
//! STATUS: every case here currently exercises the *unpatched* Naga
//! (`access_needs_check` only elides compile-time-constant indices, per
//! `naga/src/proc/index.rs:322`). Cases 1, 2, and 7 are written so that, once
//! the patch in the paper's §4.2 lands, flipping their expected assertion
//! from "check retained" to "check elided" is the only change this file
//! needs - that's intentional, so a `git diff` on this file at patch-merge
//! time *is* the human-readable summary of what changed.

use naga::back::spv;
use naga::proc::{BoundsCheckPolicies, BoundsCheckPolicy};
use naga::valid::{Capabilities, ValidationFlags, Validator};
use rspirv::binary::Disassemble;

/// Compile `source` to SPIR-V text under the given bounds-check policy and
/// return the disassembly, exactly as `write_output_spv_inner` in
/// `snapshots.rs` does it, minus the golden-file write.
fn compile_to_spv_text(source: &str, policy: BoundsCheckPolicy) -> String {
    let module = naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|e| panic!("WGSL parse failed: {e}"));

    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .unwrap_or_else(|e| panic!("validation failed: {e}"));

    let options = spv::Options {
        bounds_check_policies: BoundsCheckPolicies {
            index: policy,
            buffer: policy,
            ..Default::default()
        },
        ..Default::default()
    };

    let words = spv::write_vec(&module, &info, &options, None)
        .unwrap_or_else(|e| panic!("SPIR-V codegen failed: {e}"));

    rspirv::dr::load_words(words)
        .expect("naga produced invalid SPIR-V")
        .disassemble()
}

/// One row of the design doc's soundness table. `name` only exists so a
/// failing assertion prints something a human can immediately map back to
/// the paper's §4.3 case list, instead of a bare boolean.
struct Case {
    name: &'static str,
    snippet: &'static str,
    /// Substring that must be ABSENT if the check is correctly elided for
    /// this case, i.e. presence of this substring means "check still here".
    /// Uses `OpULessThan` for ReadZeroSkipWrite (zero-init uses OpIEqual, not OpULessThan)
    /// and `UMin` for Restrict.
    check_marker: &'static str,
}

const CASES: &[Case] = &[
    Case {
        name: "happy_direct (exact-size buffer, lid in main())",
        snippet: r#"
            var<workgroup> tile: array<f32, 64>;
            @compute @workgroup_size(64)
            fn main(@builtin(local_invocation_index) lid: u32) {
                let v = tile[lid];
                tile[0] = v;
            }
        "#,
        check_marker: "OpULessThan",
    },
    Case {
        name: "happy_oversized (lid into a larger-than-needed buffer)",
        snippet: r#"
            var<workgroup> tile: array<f32, 128>;
            @compute @workgroup_size(64)
            fn main(@builtin(local_invocation_index) lid: u32) {
                let v = tile[lid];
                tile[0] = v;
            }
        "#,
        check_marker: "OpULessThan",
    },
    Case {
        name: "stencil_overflow (lid + 1u; upper bound 64 is NOT < 64)",
        snippet: r#"
            var<workgroup> tile: array<f32, 64>;
            @compute @workgroup_size(64)
            fn main(@builtin(local_invocation_index) lid: u32) {
                let v = tile[lid + 1u];
                tile[0] = v;
            }
        "#,
        check_marker: "OpULessThan",
    },
    Case {
        name: "wraparound (lid - 1u; THE critical soundness case)",
        snippet: r#"
            var<workgroup> tile: array<f32, 64>;
            @compute @workgroup_size(64)
            fn main(@builtin(local_invocation_index) lid: u32) {
                let v = tile[lid - 1u];
                tile[0] = v;
            }
        "#,
        check_marker: "OpULessThan",
    },
    Case {
        name: "multiplicative (lid * 2u; explicitly out of scope for v1)",
        snippet: r#"
            var<workgroup> tile: array<f32, 128>;
            @compute @workgroup_size(64)
            fn main(@builtin(local_invocation_index) lid: u32) {
                let v = tile[lid * 2u];
                tile[0] = v;
            }
        "#,
        check_marker: "OpULessThan",
    },
    Case {
        name: "helper_forwarded (lid passed through a helper fn; v1 scope limit)",
        snippet: r#"
            var<workgroup> tile: array<f32, 64>;
            fn touch(i: u32, v: f32) {
                tile[i] = v;
            }
            @compute @workgroup_size(64)
            fn main(@builtin(local_invocation_index) lid: u32) {
                touch(lid, 1.0);
            }
        "#,
        check_marker: "OpULessThan",
    },
];

/// Tier 2: every case above, compiled under `ReadZeroSkipWrite` (the
/// `wgpu`-native default - the policy under which the cost of a retained
/// check is largest, per the paper's §2.2 derivation, so this is the most
/// sensitive setting to test soundness under). For each case we record
/// today's baseline truth, and `CASES` above carries the future-tense
/// comment explaining which lines move once the patch lands.
#[test]
fn elision_soundness_table_read_zero_skip_write() {
    for (i, case) in CASES.iter().enumerate() {
        let text = compile_to_spv_text(case.snippet, BoundsCheckPolicy::ReadZeroSkipWrite);
        let check_present = text.contains(case.check_marker);
        println!(
            "[{}] check present = {check_present}",
            case.name
        );
        // POST-PATCH ASSERTIONS:
        // Cases 0 (happy_direct) and 1 (happy_oversized) should have checks ELIDED
        // because lid ∈ [0, 64) and array size >= workgroup size.
        // All other cases should still have checks present.
        if i == 0 || i == 1 {
            assert!(
                !check_present,
                "[{}] bounds check should be ELIDED (lid-based index, array >= wg_size) but OpULessThan was found in the disassembly.",
                case.name
            );
        } else {
            assert!(
                check_present,
                "[{}] bounds check should be PRESENT but was not found.",
                case.name
            );
        }
    }
}

/// Tier 1: Verify that `Restrict` policy produces NO `UMin` clamp for the
/// `happy_direct` case, confirming that builtin-seeded bounds-check elision
/// successfully strips the clamp when `lid` ∈ [0, workgroup_size) and the
/// array length >= workgroup size.
///
/// NOTE: The disassembly still contains `OpSelectionMerge`/`OpBranchConditional`
/// from Naga's mandatory workgroup-memory zero-initialization, which is
/// unrelated to bounds checking - see the detailed analysis below.
#[test]
fn happy_direct_restrict_elides_umin() {
    let text = compile_to_spv_text(CASES[0].snippet, BoundsCheckPolicy::Restrict);

    // The bounds-check clamp should be ELIDED because lid ∈ [0, 64) and
    // tile has exactly 64 elements -> every access is provably safe.
    assert!(
        !text.contains("UMin"),
        "UMin should NOT be present under Restrict policy when bounds-check is elided"
    );

    // Verify the zero-init branch is still present (unrelated to bounds checks)
    assert!(
        text.contains("OpIEqual") && text.contains("OpSelectionMerge"),
        "Expected workgroup zero-initialization (OpIEqual + OpSelectionMerge) to still be present"
    );

    // Confirm that the access chain directly uses lid, not a clamped value
    assert!(
        text.contains("OpAccessChain"),
        "Expected an OpAccessChain for the tile[lid] access"
    );
}

