//! Does the elision pass mark `t[i]` safe when a GLSL `&&` branch assigns i = 100?
use naga::front::glsl::{Frontend, Options};
use naga::valid::{Capabilities, ValidationFlags, Validator};
use naga::{Expression, ShaderStage};
fn run(name: &str, src: &str) {
    let module = Frontend::default().parse(&Options::from(ShaderStage::Compute), src).expect("glsl parse");
    let has_logical_and = module.functions.iter().any(|(_, f)| f.expressions.iter().any(|(_, e)| matches!(e, Expression::Binary { op: naga::BinaryOperator::LogicalAnd, .. })));
    for elide in [false, true] {
        let mut v = Validator::new(ValidationFlags::all(), Capabilities::all());
        v.set_bounds_check_elision(elide);
        let info = v.validate(&module).expect("validate");
        let mut proofs: Vec<(String, bool)> = Vec::new();
        for (fh, f) in module.functions.iter() {
        let fi = &info[fh];
        proofs.extend(f.expressions.iter().filter_map(|(h, e)| match e {
            Expression::Access { base, .. } => {
                let b = match f.expressions[*base] { Expression::GlobalVariable(g) => module.global_variables[g].name.clone().unwrap_or_default(), _ => "?".into() };
                Some((b, fi.safe_accesses.contains(&h)))
            }
            _ => None,
        }));
        }
        println!("{name:26} ir_has_LogicalAnd={has_logical_and} elision={elide:5} access_proofs={proofs:?}");
    }
}
fn main() {
    let pre = "#version 450\nlayout(local_size_x = 1) in;\nlayout(std430, set = 0, binding = 0) buffer B { int o[4]; };\nshared float t[8];\nvoid main() {\n  int a = o[0]; int b = o[1];\n  uint i = 0u;\n";
    run("and_assign_100", &format!("{pre}  if (a < b && b > 0) {{ i = 100u; }}\n  t[i] = 1.0;\n}}\n"));
    run("plain_lt_assign_100", &format!("{pre}  if (a < b) {{ i = 100u; }}\n  t[i] = 1.0;\n}}\n"));
    run("const_cond_assign_100", &format!("{pre}  if (b > 0) {{ i = 100u; }}\n  t[i] = 1.0;\n}}\n"));
    run("control_no_if", &format!("{pre}  t[i] = 1.0;\n}}\n"));
    let bug = format!("{pre}  if (a < b && b > 0) {{ i = 100u; }}\n  t[i] = 1.0;\n}}\n");
    println!("SPIR-V OpULessThan for and_assign_100: elision_off={} elision_on={}", spv_checks(&bug, false), spv_checks(&bug, true));
    println!("--- repro GLSL ---\n{bug}");
}

#[allow(dead_code)]
pub fn spv_checks(src: &str, elide: bool) -> usize {
    use naga::back::spv;
    use naga::proc::{BoundsCheckPolicies, BoundsCheckPolicy};
    let module = Frontend::default().parse(&Options::from(ShaderStage::Compute), src).unwrap();
    let mut v = Validator::new(ValidationFlags::all(), Capabilities::all());
    v.set_bounds_check_elision(elide);
    let info = v.validate(&module).unwrap();
    let opts = spv::Options { bounds_check_policies: BoundsCheckPolicies { index: BoundsCheckPolicy::ReadZeroSkipWrite, buffer: BoundsCheckPolicy::ReadZeroSkipWrite, ..Default::default() }, ..Default::default() };
    let words = spv::write_vec(&module, &info, &opts, None).unwrap();
    // OpULessThan = 176 (spirv-0.4.0+sdk-1.4.341.0 autogen_spirv.rs:3772)
    let mut i = 5; let mut n = 0;
    while i < words.len() { let op = words[i] & 0xFFFF; let wc = (words[i] >> 16) as usize; if op == 176 { n += 1; } if wc == 0 { break; } i += wc; }
    n
}
