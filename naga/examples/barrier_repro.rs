use naga::back::spv;
use naga::proc::{BoundsCheckPolicies, BoundsCheckPolicy};
use naga::valid::{Capabilities, ValidationFlags, Validator};
use rspirv::binary::Disassemble;

fn compile(text: &str) -> String {
    let module = naga::front::wgsl::parse_str(text).expect("WGSL parse");
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .expect("validation");
    let opts = spv::Options {
        bounds_check_policies: BoundsCheckPolicies {
            index: BoundsCheckPolicy::ReadZeroSkipWrite,
            buffer: BoundsCheckPolicy::ReadZeroSkipWrite,
            ..Default::default()
        },
        ..Default::default()
    };
    let words = spv::write_vec(&module, &info, &opts, None).expect("SPIR-V codegen");
    rspirv::dr::load_words(words).expect("valid SPIR-V").disassemble()
}

fn main() {
    // WGUL is the LAST statement in the loop body (nothing follows it in that
    // static block); the increment lives in `continuing` (a SEPARATE Block),
    // and the hazard write `data[lid]=...` sits at the TOP of the body, so it
    // only becomes "the next thing that happens" via the loop back-edge.
    // A same-block-only forward scan cannot see it.
    let cross_iter = r#"
        var<workgroup> data: array<f32, 64>;
        var<workgroup> flag: u32;

        @compute @workgroup_size(64)
        fn main(@builtin(local_invocation_index) lid: u32) {
            var i = 0u;
            loop {
                if i < 4u { } else { break; }
                data[lid] = f32(i);
                let v = workgroupUniformLoad(&flag);
                continuing {
                    i = i + 1u;
                }
            }
        }
    "#;
    println!("=== WGUL is last stmt in body; hazard write at top of body (via back-edge only) ===");
    let b = compile(cross_iter);
    let pre_post_only = b.matches("OpControlBarrier").count();
    println!("Total OpControlBarrier count: {pre_post_only} (expect 3 if barrier kept: 1 zero-init + pre + post; 2 if post ELIDED)");
    println!("{}", b);
}
