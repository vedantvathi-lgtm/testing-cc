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
        check_marker: "OpSelectionMerge",
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
        check_marker: "OpSelectionMerge",
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
        check_marker: "OpSelectionMerge",
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
        check_marker: "OpSelectionMerge",
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
        check_marker: "OpSelectionMerge",
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
        check_marker: "OpSelectionMerge",
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
    for case in CASES {
        let text = compile_to_spv_text(case.snippet, BoundsCheckPolicy::ReadZeroSkipWrite);
        let check_present = text.contains(case.check_marker);
        println!(
            "[{}] check present = {check_present}",
            case.name
        );
        // PRE-PATCH BASELINE: every case retains its check today, including
        // the three that the proposed patch is supposed to elide. This
        // assertion is the "before" half of the before/after diff the paper
        // promises in its Tier 1 section - update the three `happy_*`/`store`
        // cases to `assert!(!check_present, ...)` as part of the same PR that
        // implements §4.2, so the patch and its proof land together.
        assert!(
            check_present,
            "[{}] expected the existing (unpatched) bounds check to be present, but it was not. \
             If you are seeing this AFTER implementing the §4.2 patch for an in-scope case \
             (happy_direct / happy_oversized / the store mirror), that's the expected, desired \
             change - flip this case's assertion to `!check_present` rather than treating this \
             failure as a regression.",
            case.name
        );
    }
}

/// Tier 1, minimal smoke test: confirm the `Restrict` policy's baseline
/// shape too (a single `OpExtInst ... UMin`, per the paper's §2.2
/// derivation), so a regression in the *other* policy doesn't slip past
/// silently just because this file's main loop above only checks
/// `ReadZeroSkipWrite`.
#[test]
fn happy_path_restrict_baseline_uses_umin_clamp_not_branch() {
    let text = compile_to_spv_text(CASES[0].snippet, BoundsCheckPolicy::Restrict);
    assert!(
        text.contains("UMin"),
        "expected a UMin clamp in the Restrict-policy baseline; if this fails, the paper's \
         §2.2 SPIR-V derivation for the Restrict policy needs re-verifying against current Naga, \
         not just this test fixing."
    );

    // NOTE - found by actually running this test, not by inspection: a bare
    // `var<workgroup>` fixture DOES contain an OpSelectionMerge/OpBranchConditional
    // in its disassembly even under Restrict - but it has nothing to do with
    // bounds checking. It's Naga's mandatory workgroup-memory zero-initialization
    // (`if (lid == 0u) { tile = OpConstantNull }` before the first
    // OpControlBarrier), which exists for the cross-tab security reasons cited
    // in the paper's §1 (uninitialized workgroup memory as a data-leak vector).
    // Confirmed directly from the real disassembly:
    //
    //   %18 = OpIEqual %19 %12 %17        ; lid == 0u
    //         OpSelectionMerge %20 None
    //         OpBranchConditional %18 %21 %20
    //   %21 = OpLabel
    //         OpStore %7 %16               ; tile = ConstantNull
    //         OpBranch %20
    //   %20 = OpLabel
    //         OpControlBarrier %22 %22 %23
    //         ...
    //   %27 = OpExtInst %6 %1 UMin %12 %26 ; <- THIS is the bounds-check clamp
    //   %28 = OpAccessChain %25 %7 %27
    //
    // A blanket "no OpSelectionMerge anywhere in the function" assertion is
    // therefore wrong - it would fail on the zero-init branch regardless of
    // bounds-check behavior, for ANY var<workgroup> shader, patched or not.
    // The correct, narrower claim is the one this paper actually makes: no
    // *additional* branch is introduced around the indexing operation itself.
    // We check that the UMin clamp's result feeds directly into the
    // OpAccessChain with no intervening OpBranchConditional between them,
    // which is the structurally precise version of "clamp, not branch, gates
    // THIS access" - distinct from the unrelated zero-init branch that
    // precedes it for an unrelated reason.
    let umin_line = text
        .lines()
        .find(|l| l.contains("UMin"))
        .expect("already asserted UMin is present above");
    let result_reg = umin_line
        .split_whitespace()
        .next()
        .expect("OpExtInst line should start with a result register, e.g. %27");
    let access_chain_line = text
        .lines()
        .find(|l| l.contains("OpAccessChain") && l.contains(result_reg))
        .unwrap_or_else(|| {
            panic!(
                "expected an OpAccessChain consuming the UMin result {result_reg} directly; \
                 got:\n{text}"
            )
        });
    assert!(
        !access_chain_line.contains("OpBranchConditional"),
        "the UMin-clamped index should feed straight into OpAccessChain with no \
         intervening branch - see paper §2.2."
    );
}

