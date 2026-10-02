use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::Command;

use naga::valid::{Capabilities, FunctionInfo, ValidationFlags, Validator};
use naga::{Expression, Function, Handle, ShaderStage, Statement};
use rspirv::binary::Disassemble;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SCHEMA_VERSION: &str = "5.0.0";
const HARNESS_VERSION: &str = "corpus-hit-rate-5";

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
    shape_local_load: usize,
    shape_literal: usize,
    shape_unsupported: usize,
    safe_builtin: usize,
    safe_affine: usize,
    safe_local_load: usize,
    safe_literal: usize,
    safe_unsupported: usize,
    spv_uless_enabled: usize,
    spv_uless_disabled: usize,
    spv_uless_delta: isize,
    codegen_fail: usize,
}

#[derive(Deserialize, Serialize)]
struct FileRecord {
    record_type: &'static str,
    schema_version: &'static str,
    file: PathBuf,
    content_hash: String,
    codegen_errors: Vec<String>,
    #[serde(flatten)]
    stats: FileStats,
}

#[derive(Serialize)]
struct DuplicateRecord<'a> {
    record_type: &'static str,
    schema_version: &'static str,
    file: &'a Path,
    content_hash: &'a str,
    duplicate_of: &'a Path,
}

#[derive(Serialize)]
struct FailureRecord<'a> {
    record_type: &'static str,
    schema_version: &'static str,
    file: &'a Path,
    failure_kind: &'a str,
    message: &'a str,
}

#[derive(Serialize)]
struct SourceState {
    path: String,
    revision: String,
    branch: String,
    status: String,
    diff_sha256: String,
    untracked_sha256: String,
}

#[derive(Serialize)]
struct RunRecord {
    record_type: &'static str,
    schema_version: &'static str,
    harness_version: &'static str,
    corpus_name: String,
    arguments: Vec<String>,
    harness_source_sha256: String,
    rustc_version: String,
    cargo_version: String,
    sources: Vec<SourceState>,
}

#[derive(Clone, Copy)]
enum Shape {
    Builtin,
    Affine,
    LocalLoad,
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
            Expression::LocalVariable(_) => Shape::LocalLoad,
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
                        && matches!(rhs, Shape::Builtin | Shape::Affine | Shape::LocalLoad))
                        || (matches!(rhs, Shape::Literal)
                            && matches!(
                                lhs,
                                Shape::Builtin | Shape::Affine | Shape::LocalLoad
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
                    Shape::LocalLoad => stats.shape_local_load += 1,
                    Shape::Literal => stats.shape_literal += 1,
                    Shape::Unsupported => stats.shape_unsupported += 1,
                }
                if info.safe_accesses.contains(&handle) {
                    stats.elided += 1;
                    stats.elided_in_compute += usize::from(is_compute);
                    match shape(fun, index) {
                        Shape::Builtin => stats.safe_builtin += 1,
                        Shape::Affine => stats.safe_affine += 1,
                        Shape::LocalLoad => stats.safe_local_load += 1,
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
    total.shape_local_load += part.shape_local_load;
    total.shape_literal += part.shape_literal;
    total.shape_unsupported += part.shape_unsupported;
    total.safe_builtin += part.safe_builtin;
    total.safe_affine += part.safe_affine;
    total.safe_local_load += part.safe_local_load;
    total.safe_literal += part.safe_literal;
    total.safe_unsupported += part.safe_unsupported;
    total.spv_uless_enabled += part.spv_uless_enabled;
    total.spv_uless_disabled += part.spv_uless_disabled;
    total.spv_uless_delta = total.spv_uless_delta.saturating_add(part.spv_uless_delta);
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

fn command_output(program: &str, args: &[&str], cwd: Option<&Path>) -> String {
    let mut command = Command::new(program);
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_default()
}

fn source_state(path: &Path) -> SourceState {
    let revision = command_output("git", &["rev-parse", "HEAD"], Some(path));
    let branch = command_output("git", &["branch", "--show-current"], Some(path));
    let status = command_output("git", &["status", "--porcelain=v1"], Some(path));
    let diff = Command::new("git")
        .args(["diff", "--binary", "HEAD"])
        .current_dir(path)
        .output()
        .map(|output| output.stdout)
        .unwrap_or_default();
    let untracked = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard", "-z"])
        .current_dir(path)
        .output()
        .map(|output| output.stdout)
        .unwrap_or_default();
    let mut untracked_hasher = Sha256::new();
    for relative in untracked
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
    {
        untracked_hasher.update(relative);
        if let Ok(bytes) = fs::read(path.join(String::from_utf8_lossy(relative).as_ref())) {
            untracked_hasher.update(bytes);
        }
    }
    SourceState {
        path: path.display().to_string(),
        revision,
        branch,
        status,
        diff_sha256: format!("{:x}", Sha256::digest(diff)),
        untracked_sha256: format!("{:x}", untracked_hasher.finalize()),
    }
}

fn git_root(path: &Path) -> Option<PathBuf> {
    let cwd = if path.is_dir() { path } else { path.parent()? };
    let root = command_output("git", &["rev-parse", "--show-toplevel"], Some(cwd));
    (!root.is_empty()).then(|| PathBuf::from(root))
}

fn collect_calls(block: &naga::Block, calls: &mut HashSet<Handle<Function>>) {
    for statement in block {
        match statement {
            Statement::Block(block) => collect_calls(block, calls),
            Statement::If { accept, reject, .. } => {
                collect_calls(accept, calls);
                collect_calls(reject, calls);
            }
            Statement::Switch { cases, .. } => {
                for case in cases {
                    collect_calls(&case.body, calls);
                }
            }
            Statement::Loop {
                body, continuing, ..
            } => {
                collect_calls(body, calls);
                collect_calls(continuing, calls);
            }
            Statement::Call { function, .. } => {
                calls.insert(*function);
            }
            _ => {}
        }
    }
}

fn compute_reachable_functions(module: &naga::Module) -> HashSet<Handle<Function>> {
    let mut reachable = HashSet::new();
    let mut pending = Vec::new();
    for entry_point in &module.entry_points {
        if entry_point.stage == ShaderStage::Compute {
            let mut direct = HashSet::new();
            collect_calls(&entry_point.function.body, &mut direct);
            pending.extend(direct);
        }
    }
    while let Some(function) = pending.pop() {
        if !reachable.insert(function) {
            continue;
        }
        let mut direct = HashSet::new();
        collect_calls(&module.functions[function].body, &mut direct);
        pending.extend(direct);
    }
    reachable
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
    let compute_reachable = compute_reachable_functions(&module);
    for (handle, fun) in module.functions.iter() {
        let part = classify(
            &module,
            &enabled_info[handle],
            fun,
            compute_reachable.contains(&handle),
        );
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
            total.spv_uless_delta = *disabled as isize - *enabled as isize;
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
        record_type: "file",
        schema_version: SCHEMA_VERSION,
        file: path.to_path_buf(),
        content_hash: normalized_hash(&source),
        codegen_errors,
        stats: total.clone(),
    })
    .map_err(|error| format!("json error: {error}"))?;
    serde_json::from_str::<serde_json::Value>(&json)
        .map_err(|error| format!("json round-trip error: {error}"))?;
    Ok((total, json))
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
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_dir = manifest_dir.parent().expect("project directory");
    let mut source_roots = vec![manifest_dir.clone(), project_dir.join("wgpu")];
    for argument in &args[2..] {
        if let Some(root) = git_root(Path::new(argument)) {
            source_roots.push(root);
        }
    }
    source_roots.sort();
    source_roots.dedup();
    println!(
        "{}",
        serde_json::to_string(&RunRecord {
            record_type: "run",
            schema_version: SCHEMA_VERSION,
            harness_version: HARNESS_VERSION,
            corpus_name: name.clone(),
            arguments: args[2..].to_vec(),
            harness_source_sha256: format!("{:x}", Sha256::digest(include_bytes!("main.rs"))),
            rustc_version: command_output("rustc", &["--version", "--verbose"], None),
            cargo_version: command_output("cargo", &["--version", "--verbose"], None),
            sources: source_roots.iter().map(|root| source_state(root)).collect(),
        })
        .expect("run record must serialize")
    );
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
                println!(
                    "{}",
                    serde_json::to_string(&FailureRecord {
                        record_type: "failure",
                        schema_version: SCHEMA_VERSION,
                        file,
                        failure_kind: "read",
                        message: &error.to_string(),
                    })
                    .expect("failure record must serialize")
                );
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
                    record_type: "duplicate",
                    schema_version: SCHEMA_VERSION,
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
                let failure_kind = if reason.starts_with("parse") {
                    parse_fail += 1;
                    "parse"
                } else if reason.starts_with("validate") || reason.starts_with("disabled validate")
                {
                    validate_fail += 1;
                    "validate"
                } else {
                    read_fail += 1;
                    "other"
                };
                println!(
                    "{}",
                    serde_json::to_string(&FailureRecord {
                        record_type: "failure",
                        schema_version: SCHEMA_VERSION,
                        file,
                        failure_kind,
                        message: &reason,
                    })
                    .expect("failure record must serialize")
                );
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
    eprintln!("SUMMARY\t{name}\tschema={SCHEMA_VERSION}\tfiles={}\tunique={}\tduplicates={duplicates}\tok={ok}\tparse_fail={parse_fail}\tvalidate_fail={validate_fail}\tread_fail={read_fail}\tfunctions={}\tcompute_eps={}\taccess_expressions={}\truntime_accesses={}\tconstant_accesses={}\truntime_in_compute={}\telided={} ({all_rate:.2}%)\telided_in_compute={} ({compute_rate:.2}%)\tshape_builtin={}\tshape_affine={}\tshape_local_load={}\tshape_literal={}\tshape_unsupported={}\tsafe_builtin={}\tsafe_affine={}\tsafe_local_load={}\tsafe_literal={}\tsafe_unsupported={}\tspv_uless_enabled={}\tspv_uless_disabled={}\tspv_uless_delta={}\tcodegen_fail={}", files.len(), files.len().saturating_sub(duplicates), aggregate.functions, aggregate.compute_eps, aggregate.access_expressions, aggregate.runtime_accesses, aggregate.constant_accesses, aggregate.runtime_in_compute, aggregate.elided, aggregate.elided_in_compute, aggregate.shape_builtin, aggregate.shape_affine, aggregate.shape_local_load, aggregate.shape_literal, aggregate.shape_unsupported, aggregate.safe_builtin, aggregate.safe_affine, aggregate.safe_local_load, aggregate.safe_literal, aggregate.safe_unsupported, aggregate.spv_uless_enabled, aggregate.spv_uless_disabled, aggregate.spv_uless_delta, aggregate.codegen_fail);
}
