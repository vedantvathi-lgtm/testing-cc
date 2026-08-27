use naga::back::{msl, spv};
use naga::proc::{BoundsCheckPolicies, BoundsCheckPolicy};
use naga::valid::{Capabilities, ValidationFlags, Validator};
use naga::Expression;
use rspirv::binary::Disassemble;

fn validate(wgsl: &str) -> (naga::Module, naga::valid::ModuleInfo) {
    let module = naga::front::wgsl::parse_str(wgsl).expect("WGSL parse");
    let info = Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .expect("validation");
    (module, info)
}

fn access_proofs(wgsl: &str) -> Vec<bool> {
    let (module, info) = validate(wgsl);
    let function = &module.entry_points[0].function;
    let function_info = info.get_entry_point(0);
    function
        .expressions
        .iter()
        .filter_map(|(handle, expression)| {
            matches!(expression, Expression::Access { .. })
                .then_some(function_info.safe_accesses.contains(&handle))
        })
        .collect()
}

fn compile_msl(wgsl: &str) -> String {
    let (module, info) = validate(wgsl);
    let mut options = msl::Options::default();
    options.bounds_check_policies = BoundsCheckPolicies {
        index: BoundsCheckPolicy::Restrict,
        buffer: BoundsCheckPolicy::Restrict,
        ..Default::default()
    };
    msl::write_string(
        &module,
        &info,
        &options,
        &msl::PipelineOptions::default(),
    )
    .expect("MSL codegen")
    .0
}

fn compile_spv(wgsl: &str) -> String {
    let (module, info) = validate(wgsl);
    let options = spv::Options {
        bounds_check_policies: BoundsCheckPolicies {
            index: BoundsCheckPolicy::ReadZeroSkipWrite,
            buffer: BoundsCheckPolicy::ReadZeroSkipWrite,
            ..Default::default()
        },
        ..Default::default()
    };
    let words = spv::write_vec(&module, &info, &options, None).expect("SPIR-V codegen");
    rspirv::dr::load_words(words)
        .expect("valid SPIR-V")
        .disassemble()
}

fn verify(name: &str, wgsl: &str, expected_safe: bool) {
    assert_eq!(access_proofs(wgsl), [expected_safe], "{name}");
    let msl = compile_msl(wgsl);
    let spv = compile_spv(wgsl);
    let msl_restricts = msl.contains("metal::min(unsigned(");
    let spv_checks = spv.contains("OpULessThan");
    assert_eq!(msl_restricts, !expected_safe, "{name}: MSL");
    assert_eq!(spv_checks, !expected_safe, "{name}: SPIR-V");
    println!("{name}: safe={expected_safe}");
}

fn main() {
    verify(
        "undersized flat builtin",
        r#"
            var<workgroup> tile: array<f32, 4>;
            @compute @workgroup_size(8)
            fn main(@builtin(local_invocation_index) lid: u32) {
                tile[lid] = 1.0;
            }
        "#,
        false,
    );
    verify(
        "oversized flat builtin",
        r#"
            var<workgroup> tile: array<f32, 16>;
            @compute @workgroup_size(8)
            fn main(@builtin(local_invocation_index) lid: u32) {
                tile[lid] = 1.0;
            }
        "#,
        true,
    );
    verify(
        "undersized component builtin",
        r#"
            var<workgroup> tile: array<f32, 4>;
            @compute @workgroup_size(8, 1, 1)
            fn main(@builtin(local_invocation_id) id: vec3<u32>) {
                tile[id.x] = 1.0;
            }
        "#,
        false,
    );
    verify(
        "oversized component builtin",
        r#"
            var<workgroup> tile: array<f32, 4>;
            @compute @workgroup_size(2, 1, 1)
            fn main(@builtin(local_invocation_id) id: vec3<u32>) {
                tile[id.x] = 1.0;
            }
        "#,
        true,
    );
}
