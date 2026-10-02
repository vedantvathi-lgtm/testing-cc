//! T5 Part B helper — dump per-policy SPIR-V disassembly for a WGSL file.
//!
//! Usage: bce-expectation-dump <fixture.wgsl> <out-enabled.spv> <out-disabled.spv>
//! Writes raw SPIR-V words for elision-enabled and elision-disabled compiles
//! (ReadZeroSkipWrite, matching the diff-exec/paired harness policy). The Python
//! checker disassembles and greps OpULessThan per EXPECT marker line.
//!
//! `--proofs <fixture.wgsl>` instead prints per-access proof results as JSON
//! lines (see `print_proofs`); the checker judges `// EXPECT:` markers from them.
//!
//! Exit codes: 0 = both compiles OK; 2 = parse/validate/codegen failure
//! (message on stderr — the checker treats this as fixture-uncompilable).

use naga::back::spv;
use naga::proc::{BoundsCheckPolicies, BoundsCheckPolicy};
use naga::valid::{Capabilities, ValidationFlags, Validator};
use std::process::ExitCode;

const POLICY: BoundsCheckPolicy = BoundsCheckPolicy::ReadZeroSkipWrite;

fn compile_to_words(wgsl: &str, bce_enabled: bool) -> Result<Vec<u32>, String> {
    let module = naga::front::wgsl::parse_str(wgsl).map_err(|e| format!("parse: {e}"))?;
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    validator.set_bounds_check_elision(bce_enabled);
    let info = validator.validate(&module).map_err(|e| format!("validate: {e}"))?;
    let options = spv::Options {
        bounds_check_policies: BoundsCheckPolicies {
            index: POLICY,
            buffer: POLICY,
            ..Default::default()
        },
        ..Default::default()
    };
    // Entry point: first compute entry (fixtures are single-entry compute).
    let pipeline_options = spv::PipelineOptions {
        shader_stage: naga::ShaderStage::Compute,
        entry_point: module
            .entry_points
            .iter()
            .find(|ep| ep.stage == naga::ShaderStage::Compute)
            .map(|ep| ep.name.clone())
            .ok_or("no compute entry point")?,
    };
    spv::write_vec(&module, &info, &options, Some(&pipeline_options))
        .map_err(|e| format!("spv: {e}"))
}

/// `--proofs <fixture.wgsl>`: one JSON object per runtime `Access` (constant
/// indices excluded, matching the corpus harness): its 1-based source line and
/// whether the elision analysis proved it in bounds.
fn print_proofs(path: &str) -> Result<(), String> {
    let wgsl = std::fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    let module = naga::front::wgsl::parse_str(&wgsl).map_err(|e| format!("parse: {e}"))?;
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    validator.set_bounds_check_elision(true);
    let info = validator.validate(&module).map_err(|e| format!("validate: {e}"))?;
    let mut functions: Vec<(&naga::Function, &naga::valid::FunctionInfo)> = module
        .functions
        .iter()
        .map(|(handle, function)| (function, &info[handle]))
        .collect();
    for (index, entry_point) in module.entry_points.iter().enumerate() {
        functions.push((&entry_point.function, info.get_entry_point(index)));
    }
    for (function, function_info) in functions {
        for (handle, expression) in function.expressions.iter() {
            let naga::Expression::Access { index, .. } = *expression else {
                continue;
            };
            if is_statically_known(&module, function, index) {
                continue;
            }
            let line = function.expressions.get_span(handle).location(&wgsl).line_number;
            let record = serde_json::json!({
                "line": line,
                "safe": function_info.safe_accesses.contains(&handle),
            });
            println!("{record}");
        }
    }
    Ok(())
}

fn is_statically_known(
    module: &naga::Module,
    function: &naga::Function,
    handle: naga::Handle<naga::Expression>,
) -> bool {
    match function.expressions[handle] {
        naga::Expression::Literal(_) | naga::Expression::ZeroValue(_) => true,
        naga::Expression::Constant(constant) => matches!(
            module.global_expressions[module.constants[constant].init],
            naga::Expression::Literal(_) | naga::Expression::ZeroValue(_)
        ),
        _ => false,
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 && args[1] == "--proofs" {
        return match print_proofs(&args[2]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{e}");
                ExitCode::from(2)
            }
        };
    }
    if args.len() != 4 {
        eprintln!("usage: bce-expectation-dump <fixture.wgsl> <out-enabled.spv> <out-disabled.spv>");
        return ExitCode::from(2);
    }
    let wgsl = match std::fs::read_to_string(&args[1]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("read {}: {e}", args[1]);
            return ExitCode::from(2);
        }
    };
    let enabled = match compile_to_words(&wgsl, true) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let disabled = match compile_to_words(&wgsl, false) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    if let Err(e) = std::fs::write(&args[2], bytemuck_bytes(&enabled)) {
        eprintln!("write {}: {e}", args[2]);
        return ExitCode::from(2);
    }
    if let Err(e) = std::fs::write(&args[3], bytemuck_bytes(&disabled)) {
        eprintln!("write {}: {e}", args[3]);
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}

// Minimal little-endian byte cast without pulling bytemuck in.
fn bytemuck_bytes(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_le_bytes()).collect()
}
