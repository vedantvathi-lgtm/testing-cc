//! Epoch-0 teaching tool (sections 0.2 / 0.3). Not part of the project.
//!
//!   bce-epoch-tools ir   <file.wgsl|file.comp>   IR of every entry point: expression arena with
//!                                                handles, statement tree, safe_accesses
//!   bce-epoch-tools spv  <file.wgsl> <outdir>    SPIR-V for {Restrict, ReadZeroSkipWrite, Unchecked}
//!                                                x {elision off, on}; index = buffer = policy
//!   bce-epoch-tools msl  <file.wgsl> [off]       MSL (Restrict), elision on unless `off`
//!   bce-epoch-tools hlsl <file.wgsl> [off]       HLSL (restrict_indexing), elision on unless `off`

use naga::back::spv;
use naga::proc::{BoundsCheckPolicies, BoundsCheckPolicy};
use naga::valid::{Capabilities, ValidationFlags, Validator};

fn parse(path: &str) -> naga::Module {
    let text = std::fs::read_to_string(path).expect("read");
    if path.ends_with(".wgsl") {
        naga::front::wgsl::parse_str(&text).expect("WGSL parse")
    } else {
        let options = naga::front::glsl::Options::from(naga::ShaderStage::Compute);
        naga::front::glsl::Frontend::default()
            .parse(&options, &text)
            .expect("GLSL parse")
    }
}

fn validate(module: &naga::Module, elision: bool) -> naga::valid::ModuleInfo {
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    validator.set_bounds_check_elision(elision);
    validator.validate(module).expect("validate")
}

fn dump_block(block: &naga::Block, depth: usize) {
    let pad = "    ".repeat(depth);
    for statement in block.iter() {
        match *statement {
            naga::Statement::Emit(ref range) => {
                let handles: Vec<usize> = range.clone().map(|h| h.index()).collect();
                println!("{pad}Emit({:?})", handles);
            }
            naga::Statement::If { condition, ref accept, ref reject } => {
                println!("{pad}If {{ condition: [{}] }}", condition.index());
                println!("{pad}  accept:");
                dump_block(accept, depth + 1);
                println!("{pad}  reject:");
                dump_block(reject, depth + 1);
            }
            naga::Statement::Loop { ref body, ref continuing, break_if } => {
                println!("{pad}Loop {{ break_if: {:?} }}", break_if.map(|h| h.index()));
                println!("{pad}  body:");
                dump_block(body, depth + 1);
                println!("{pad}  continuing:");
                dump_block(continuing, depth + 1);
            }
            naga::Statement::Block(ref inner) => {
                println!("{pad}Block:");
                dump_block(inner, depth + 1);
            }
            naga::Statement::Store { pointer, value } => {
                println!("{pad}Store {{ pointer: [{}], value: [{}] }}", pointer.index(), value.index());
            }
            ref other => println!("{pad}{:?}", other),
        }
    }
}

fn ir(path: &str) {
    let module = parse(path);
    let info = validate(&module, true);
    let mut all: Vec<(String, &naga::Function, &naga::valid::FunctionInfo)> = module
        .functions
        .iter()
        .map(|(h, f)| (format!("function [{}] {:?}", h.index(), f.name), f, &info[h]))
        .collect();
    for (index, entry_point) in module.entry_points.iter().enumerate() {
        all.push((format!("entry point `{}`", entry_point.name), &entry_point.function, info.get_entry_point(index)));
    }
    for (title, function, fun_info) in all {
        println!("== {title}");
        for (handle, local) in function.local_variables.iter() {
            println!("local [{}] {:?} init={:?}", handle.index(), local.name, local.init.map(|h| h.index()));
        }
        println!("-- expressions (arena order; [n] = Handle<Expression> index)");
        for (handle, expression) in function.expressions.iter() {
            let mark = if fun_info.safe_accesses.contains(&handle) { "   <= in safe_accesses" } else { "" };
            println!("[{:>2}] {:?}{}", handle.index(), expression, mark);
        }
        println!("-- body (statement tree)");
        dump_block(&function.body, 0);
        let mut safe: Vec<usize> = fun_info.safe_accesses.iter().map(|h| h.index()).collect();
        safe.sort();
        println!("-- safe_accesses = {:?}", safe);
    }
}

fn spv_words(module: &naga::Module, elision: bool, policy: BoundsCheckPolicy) -> Vec<u32> {
    let info = validate(module, elision);
    let options = spv::Options {
        lang_version: (1, 6),
        bounds_check_policies: BoundsCheckPolicies { index: policy, buffer: policy, ..Default::default() },
        // Keep the listing short: no loop-bounding counter (wgpu turns this ON; see 0.1).
        force_loop_bounding: false,
        ..Default::default()
    };
    let entry = &module.entry_points[0];
    let pipeline = spv::PipelineOptions { shader_stage: entry.stage, entry_point: entry.name.clone() };
    spv::write_vec(module, &info, &options, Some(&pipeline)).expect("spv")
}

fn spv(path: &str, out: &str) {
    let module = parse(path);
    std::fs::create_dir_all(out).unwrap();
    for (name, policy) in [
        ("restrict", BoundsCheckPolicy::Restrict),
        ("rzsw", BoundsCheckPolicy::ReadZeroSkipWrite),
        ("unchecked", BoundsCheckPolicy::Unchecked),
    ] {
        for (tag, elision) in [("off", false), ("on", true)] {
            let words = spv_words(&module, elision, policy);
            let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
            let file = format!("{out}/{name}-elision-{tag}.spv");
            std::fs::write(&file, bytes).unwrap();
            println!("{file}: {} words", words.len());
        }
    }
}

fn msl(path: &str, elision: bool) {
    let module = parse(path);
    let info = validate(&module, elision);
    let options = naga::back::msl::Options {
        bounds_check_policies: BoundsCheckPolicies {
            index: BoundsCheckPolicy::Restrict,
            buffer: BoundsCheckPolicy::Restrict,
            ..Default::default()
        },
        fake_missing_bindings: true,
        ..Default::default()
    };
    let text = naga::back::msl::write_string(&module, &info, &options, &Default::default()).expect("msl").0;
    println!("{text}");
}

fn hlsl(path: &str, elision: bool) {
    let module = parse(path);
    let info = validate(&module, elision);
    let options = naga::back::hlsl::Options { fake_missing_bindings: true, ..Default::default() };
    let pipeline = naga::back::hlsl::PipelineOptions::default();
    let mut text = String::new();
    naga::back::hlsl::Writer::new(&mut text, &options, &pipeline).write(&module, &info, None).expect("hlsl");
    println!("{text}");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("ir") => ir(&args[2]),
        Some("spv") => spv(&args[2], &args[3]),
        Some("msl") => msl(&args[2], args.get(3).map(String::as_str) != Some("off")),
        Some("hlsl") => hlsl(&args[2], args.get(3).map(String::as_str) != Some("off")),
        _ => eprintln!("usage: see the doc comment at the top of src/main.rs"),
    }
}
