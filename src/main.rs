use std::any::Any;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use naga::valid::{Capabilities, FunctionInfo, ValidationFlags, Validator};
use naga::{Expression, Function, Handle, ShaderStage};
use rspirv::binary::Disassemble;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Default, Deserialize, Serialize)]
struct FileStats {
    functions: usize,
    compute_eps: usize,
    access_expressions: usize,
    runtime_accesses: usize,
    constant_accesses: usize,
    static_accesses: usize,
    elided: usize,
    elided_in_compute: usize,
    runtime_in_compute: usize,
    shape_builtin: usize,
    shape_affine: usize,
    shape_loop_local: usize,
    shape_literal: usize,
    shape_unsupported: usize,
    safe_builtin: usize,
    safe_affine: usize,
    safe_loop_local: usize,
    safe_literal: usize,
    safe_unsupported: usize,
    spv_uless_enabled: usize,
    spv_uless_disabled: usize,
    spv_uless_delta: usize,
    codegen_fail: usize,
}

#[derive(Deserialize, Serialize)]
struct FileRecord {
    file: PathBuf,
    content_hash: String,
    codegen_errors: Vec<String>,
    #[serde(flatten)]
    stats: FileStats,
}

#[derive(Serialize)]
struct DuplicateRecord<'a> {
    file: &'a Path,
    content_hash: &'a str,
    duplicate_of: &'a Path,
}

#[derive(Clone, Copy)]
enum Shape {
    Builtin,
    Affine,
    LoopLocal,
    Literal,
    Unsupported,
}

fn shape(fun: &Function, handle: Handle<Expression>) -> Shape {
    match fun.expressions[handle] {
        Expression::Literal(_) => Shape::Literal,
        Expression::FunctionArgument(index) => match fun.arguments[index as usize].binding {
            Some(naga::Binding::BuiltIn(naga::BuiltIn::LocalInvocationIndex)) => Shape::Builtin,
            _ => Shape::Unsupported,
        },
        Expression::AccessIndex { base, .. } => match fun.expressions[base] {
            Expression::FunctionArgument(index)
                if matches!(
                    fun.arguments[index as usize].binding,
                    Some(naga::Binding::BuiltIn(naga::BuiltIn::LocalInvocationId))
                ) =>
            {
                Shape::Builtin
            }
            _ => Shape::Unsupported,
        },
        Expression::Load { pointer } => match fun.expressions[pointer] {
            Expression::LocalVariable(_) => Shape::LoopLocal,
            _ => Shape::Unsupported,
        },
        Expression::As { expr, .. } => shape(fun, expr),
        Expression::Binary { op, left, right } => {
            use naga::BinaryOperator;
            if !matches!(
                op,
                BinaryOperator::Add | BinaryOperator::Subtract | BinaryOperator::Multiply
            ) {
                return Shape::Unsupported;
            }
            let lhs = shape(fun, left);
            let rhs = shape(fun, right);
            match op {
                BinaryOperator::Add | BinaryOperator::Subtract
                    if !matches!(lhs, Shape::Unsupported)
                        && !matches!(rhs, Shape::Unsupported)
                        && !matches!((lhs, rhs), (Shape::Literal, Shape::Literal)) =>
                {
                    Shape::Affine
                }
                BinaryOperator::Multiply
                    if (matches!(lhs, Shape::Literal)
                        && matches!(rhs, Shape::Builtin | Shape::Affine | Shape::LoopLocal))
                        || (matches!(rhs, Shape::Literal)
                            && matches!(
                                lhs,
                                Shape::Builtin | Shape::Affine | Shape::LoopLocal
                            )) =>
                {
                    Shape::Affine
                }
                _ => Shape::Unsupported,
            }
        }
        _ => Shape::Unsupported,
    }
}

fn is_statically_known_index(
    module: &naga::Module,
    fun: &Function,
    handle: Handle<Expression>,
) -> bool {
    match fun.expressions[handle] {
        Expression::Literal(_) | Expression::ZeroValue(_) => true,
        Expression::Constant(constant) => matches!(
            module.global_expressions[module.constants[constant].init],
            Expression::Literal(_) | Expression::ZeroValue(_)
        ),
        _ => false,
    }
}

fn classify(
    module: &naga::Module,
    info: &FunctionInfo,
    fun: &Function,
    is_compute: bool,
) -> FileStats {
    let mut stats = FileStats::default();
    stats.compute_eps = usize::from(is_compute);
    for (handle, expr) in fun.expressions.iter() {
        match *expr {
            Expression::Access { index, .. } => {
                stats.access_expressions += 1;
                if is_statically_known_index(module, fun, index) {
                    stats.constant_accesses += 1;
                    continue;
                }
                stats.runtime_accesses += 1;
                stats.runtime_in_compute += usize::from(is_compute);
                match shape(fun, index) {
                    Shape::Builtin => stats.shape_builtin += 1,
                    Shape::Affine => stats.shape_affine += 1,
                    Shape::LoopLocal => stats.shape_loop_local += 1,
                    Shape::Literal => stats.shape_literal += 1,
                    Shape::Unsupported => stats.shape_unsupported += 1,
                }
                if info.safe_accesses.contains(&handle) {
                    stats.elided += 1;
                    stats.elided_in_compute += usize::from(is_compute);
                    match shape(fun, index) {
                        Shape::Builtin => stats.safe_builtin += 1,
                        Shape::Affine => stats.safe_affine += 1,
                        Shape::LoopLocal => stats.safe_loop_local += 1,
                        Shape::Literal => stats.safe_literal += 1,
                        Shape::Unsupported => stats.safe_unsupported += 1,
                    }
                }
            }
            Expression::AccessIndex { .. } => stats.static_accesses += 1,
            _ => {}
        }
    }
    stats
}

fn merge(total: &mut FileStats, part: &FileStats) {
    total.functions += part.functions;
    total.compute_eps += part.compute_eps;
    total.access_expressions += part.access_expressions;
    total.runtime_accesses += part.runtime_accesses;
    total.constant_accesses += part.constant_accesses;
    total.static_accesses += part.static_accesses;
    total.elided += part.elided;
    total.elided_in_compute += part.elided_in_compute;
    total.runtime_in_compute += part.runtime_in_compute;
    total.shape_builtin += part.shape_builtin;
    total.shape_affine += part.shape_affine;
    total.shape_loop_local += part.shape_loop_local;
    total.shape_literal += part.shape_literal;
    total.shape_unsupported += part.shape_unsupported;
    total.safe_builtin += part.safe_builtin;
    total.safe_affine += part.safe_affine;
    total.safe_loop_local += part.safe_loop_local;
    total.safe_literal += part.safe_literal;
    total.safe_unsupported += part.safe_unsupported;
    total.spv_uless_enabled += part.spv_uless_enabled;
    total.spv_uless_disabled += part.spv_uless_disabled;
    total.spv_uless_delta += part.spv_uless_delta;
    total.codegen_fail += part.codegen_fail;
}

fn spv_uless_count(module: &naga::Module, info: &naga::valid::ModuleInfo) -> Result<usize, String> {
    let options = naga::back::spv::Options {
        bounds_check_policies: naga::proc::BoundsCheckPolicies {
            index: naga::proc::BoundsCheckPolicy::ReadZeroSkipWrite,
            buffer: naga::proc::BoundsCheckPolicy::ReadZeroSkipWrite,
            ..Default::default()
        },
        ..Default::default()
    };
    let words = naga::back::spv::write_vec(module, info, &options, None)
        .map_err(|error| error.to_string())?;
    let text = rspirv::dr::load_words(words)
        .map_err(|error| error.to_string())?
        .disassemble();
    Ok(text.matches("OpULessThan").count())
}

fn panic_message(payload: Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_string()
    }
}

fn safe_spv_uless_count(
    module: &naga::Module,
    info: &naga::valid::ModuleInfo,
) -> Result<usize, String> {
    catch_unwind(AssertUnwindSafe(|| spv_uless_count(module, info)))
        .map_err(|payload| format!("SPIR-V codegen panicked: {}", panic_message(payload)))?
}

fn normalized_hash(source: &str) -> String {
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    format!("{:x}", Sha256::digest(normalized.as_bytes()))
}

fn process(path: &Path) -> Result<(FileStats, String), String> {
    let source = fs::read_to_string(path).map_err(|e| format!("read error: {e}"))?;
    let module = naga::front::wgsl::parse_str(&source).map_err(|e| {
        format!(
            "parse error: {}",
            e.emit_to_string(&source).lines().next().unwrap_or("")
        )
    })?;
    let mut enabled_validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    let enabled_info = enabled_validator.validate(&module).map_err(|e| {
        format!(
            "validate error: {}",
            e.emit_to_string(&source).lines().next().unwrap_or("")
        )
    })?;
    let mut disabled_validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    disabled_validator.set_bounds_check_elision(false);
    let disabled_info = disabled_validator.validate(&module).map_err(|e| {
        format!(
            "disabled validate error: {}",
            e.emit_to_string(&source).lines().next().unwrap_or("")
        )
    })?;
    let mut total = FileStats::default();
    for (handle, fun) in module.functions.iter() {
        let part = classify(&module, &enabled_info[handle], fun, false);
        merge(&mut total, &part);
    }
    for (i, ep) in module.entry_points.iter().enumerate() {
        let part = classify(
            &module,
            enabled_info.get_entry_point(i),
            &ep.function,
            ep.stage == ShaderStage::Compute,
        );
        merge(&mut total, &part);
    }
    total.functions = module.functions.len() + module.entry_points.len();
    let enabled_codegen = safe_spv_uless_count(&module, &enabled_info);
    let disabled_codegen = safe_spv_uless_count(&module, &disabled_info);
    let mut codegen_errors = Vec::new();
    match (&enabled_codegen, &disabled_codegen) {
        (Ok(enabled), Ok(disabled)) => {
            total.spv_uless_enabled = *enabled;
            total.spv_uless_disabled = *disabled;
            total.spv_uless_delta = disabled.saturating_sub(*enabled);
        }
        _ => {
            if let Err(error) = enabled_codegen {
                codegen_errors.push(format!("BCE enabled: {error}"));
            }
            if let Err(error) = disabled_codegen {
                codegen_errors.push(format!("BCE disabled: {error}"));
            }
            total.codegen_fail = 1;
        }
    }
    let json = serde_json::to_string(&FileRecord {
        file: path.to_path_buf(),
        content_hash: normalized_hash(&source),
        codegen_errors,
        stats: total,
    })
    .map_err(|error| format!("json error: {error}"))?;
    let record: FileRecord =
        serde_json::from_str(&json).map_err(|error| format!("json round-trip error: {error}"))?;
    Ok((record.stats, json))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialized_record_round_trips_with_runtime_and_codegen_fields() {
        let path = env::temp_dir().join(format!("corpus-hit-rate-{}.wgsl", std::process::id()));
        fs::write(
            &path,
            "var<workgroup> a: array<f32, 1>; @compute @workgroup_size(1) fn main(@builtin(local_invocation_index) i: u32) { a[i] = 1.0; }",
        )
        .unwrap();
        let (_, line) = process(&path).unwrap();
        let record: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(record["access_expressions"], 1);
        assert_eq!(record["runtime_accesses"], 1);
        assert_eq!(record["constant_accesses"], 0);
        assert!(record.get("spv_uless_enabled").is_some());
        assert!(record.get("spv_uless_disabled").is_some());
        assert!(record.get("spv_uless_delta").is_some());
        assert!(record.get("codegen_errors").is_some());
        assert!(record.get("codegen_fail").is_some());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn constant_access_is_not_a_runtime_candidate() {
        let source = "@compute @workgroup_size(1) fn main() { var a: array<i32, 2>; a[1] = 7; }";
        let module = naga::front::wgsl::parse_str(source).unwrap();
        let info = Validator::new(ValidationFlags::all(), Capabilities::all())
            .validate(&module)
            .unwrap();
        let stats = classify(
            &module,
            info.get_entry_point(0),
            &module.entry_points[0].function,
            true,
        );
        assert_eq!(stats.access_expressions, 0);
        assert_eq!(stats.static_accesses, 1);
        assert_eq!(stats.runtime_accesses, 0);
    }

    #[test]
    fn dedup_hash_preserves_token_boundaries_and_comments() {
        assert_ne!(normalized_hash("a b"), normalized_hash("ab"));
        assert_ne!(normalized_hash("a//x\nb"), normalized_hash("a//y\nb"));
        assert_eq!(normalized_hash("a\r\nb"), normalized_hash("a\nb"));
    }
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "wgsl") {
                out.push(path);
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: corpus_hit_rate <corpus-name> <dir-or-file>...");
        std::process::exit(2);
    }
    let name = &args[1];
    let mut files = Vec::new();
    for arg in &args[2..] {
        let path = Path::new(arg);
        if path.is_dir() {
            walk(path, &mut files);
        } else {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    let mut seen = HashMap::<String, PathBuf>::new();
    let mut ok = 0;
    let mut parse_fail = 0;
    let mut validate_fail = 0;
    let mut read_fail = 0;
    let mut duplicates = 0;
    let mut aggregate = FileStats::default();
    for file in &files {
        let source = match fs::read_to_string(file) {
            Ok(value) => value,
            Err(error) => {
                read_fail += 1;
                eprintln!("SKIP\t{}\tread error: {error}", file.display());
                continue;
            }
        };
        let hash = normalized_hash(&source);
        if let Some(original) = seen.insert(hash.clone(), file.clone()) {
            duplicates += 1;
            println!(
                "{}",
                serde_json::to_string(&DuplicateRecord {
                    file,
                    content_hash: &hash,
                    duplicate_of: &original,
                })
                .expect("paths and hashes must serialize")
            );
            continue;
        }
        match process(file) {
            Ok((stats, line)) => {
                println!("{line}");
                ok += 1;
                merge(&mut aggregate, &stats);
            }
            Err(reason) => {
                if reason.starts_with("parse") {
                    parse_fail += 1;
                } else if reason.starts_with("validate") {
                    validate_fail += 1;
                } else {
                    read_fail += 1;
                }
                eprintln!("SKIP\t{}\t{reason}", file.display());
            }
        }
    }
    let all_rate = if aggregate.runtime_accesses == 0 {
        0.0
    } else {
        100.0 * aggregate.elided as f64 / aggregate.runtime_accesses as f64
    };
    let compute_rate = if aggregate.runtime_in_compute == 0 {
        0.0
    } else {
        100.0 * aggregate.elided_in_compute as f64 / aggregate.runtime_in_compute as f64
    };
    eprintln!("SUMMARY\t{name}\tfiles={}\tunique={}\tduplicates={duplicates}\tok={ok}\tparse_fail={parse_fail}\tvalidate_fail={validate_fail}\tread_fail={read_fail}\tfunctions={}\tcompute_eps={}\taccess_expressions={}\truntime_accesses={}\tconstant_accesses={}\truntime_in_compute={}\telided={} ({all_rate:.2}%)\telided_in_compute={} ({compute_rate:.2}%)\tshape_builtin={}\tshape_affine={}\tshape_loop_local={}\tshape_literal={}\tshape_unsupported={}\tsafe_builtin={}\tsafe_affine={}\tsafe_loop_local={}\tsafe_literal={}\tsafe_unsupported={}\tspv_uless_enabled={}\tspv_uless_disabled={}\tspv_uless_delta={}\tcodegen_fail={}", files.len(), files.len().saturating_sub(duplicates), aggregate.functions, aggregate.compute_eps, aggregate.access_expressions, aggregate.runtime_accesses, aggregate.constant_accesses, aggregate.runtime_in_compute, aggregate.elided, aggregate.elided_in_compute, aggregate.shape_builtin, aggregate.shape_affine, aggregate.shape_loop_local, aggregate.shape_literal, aggregate.shape_unsupported, aggregate.safe_builtin, aggregate.safe_affine, aggregate.safe_loop_local, aggregate.safe_literal, aggregate.safe_unsupported, aggregate.spv_uless_enabled, aggregate.spv_uless_disabled, aggregate.spv_uless_delta, aggregate.codegen_fail);
}
