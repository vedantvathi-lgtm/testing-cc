//! Per-access census of runtime (non-constant) `Access` expressions in WGSL files.
//! Same runtime-access definition as corpus_hit_rate v5 (`is_statically_known_index`).
//! Emits one JSON line per runtime access: file, fn, stage, line, source text,
//! base kind, address space, index expression tree, and whether elision proved it safe.
use naga::valid::{Capabilities, FunctionInfo, ValidationFlags, Validator};
use naga::{Expression, Function, Handle, Module, ShaderStage, TypeInner};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::{env, fs, path::Path};

fn known(module: &Module, fun: &Function, h: Handle<Expression>) -> bool {
    match fun.expressions[h] {
        Expression::Literal(_) | Expression::ZeroValue(_) => true,
        Expression::Constant(c) => matches!(
            module.global_expressions[module.constants[c].init],
            Expression::Literal(_) | Expression::ZeroValue(_)
        ),
        _ => false,
    }
}

fn tree(module: &Module, fun: &Function, args_builtin: &[String], h: Handle<Expression>, depth: u32) -> String {
    if depth > 6 { return "…".into(); }
    let t = |x| tree(module, fun, args_builtin, x, depth + 1);
    match &fun.expressions[h] {
        Expression::Literal(l) => format!("lit({l:?})"),
        Expression::Constant(c) => format!("const({})", module.constants[*c].name.clone().unwrap_or_default()),
        Expression::Override(o) => format!("override({})", module.overrides[*o].name.clone().unwrap_or_default()),
        Expression::ZeroValue(_) => "zero".into(),
        Expression::FunctionArgument(i) => format!("arg[{}]", args_builtin.get(*i as usize).cloned().unwrap_or_else(|| format!("#{i}"))),
        Expression::LocalVariable(v) => format!("local({})", fun.local_variables[*v].name.clone().unwrap_or_default()),
        Expression::GlobalVariable(g) => {
            let gv = &module.global_variables[*g];
            format!("global({}:{:?})", gv.name.clone().unwrap_or_default(), gv.space)
        }
        Expression::Load { pointer } => format!("load({})", t(*pointer)),
        Expression::AccessIndex { base, index } => format!("{}.{index}", t(*base)),
        Expression::Access { base, index } => format!("{}[{}]", t(*base), t(*index)),
        Expression::Binary { op, left, right } => format!("({} {:?} {})", t(*left), op, t(*right)),
        Expression::Unary { op, expr } => format!("{op:?}({})", t(*expr)),
        Expression::As { expr, kind, convert } => format!("as<{kind:?}{}>({})", convert.map(|w| w.to_string()).unwrap_or_default(), t(*expr)),
        Expression::Math { fun: f, arg, arg1, arg2, .. } => {
            let mut s = format!("{f:?}({}", t(*arg));
            for a in [arg1, arg2].into_iter().flatten() { s += &format!(", {}", t(*a)); }
            s + ")"
        }
        Expression::Select { condition, accept, reject } => format!("select({}, {}, {})", t(*condition), t(*accept), t(*reject)),
        Expression::ArrayLength(e) => format!("arrayLength({})", t(*e)),
        Expression::CallResult(f) => format!("call({})", module.functions[*f].name.clone().unwrap_or_default()),
        Expression::Splat { value, .. } => format!("splat({})", t(*value)),
        Expression::Swizzle { vector, .. } => format!("swizzle({})", t(*vector)),
        Expression::AtomicResult { .. } => "atomic_result".into(),
        Expression::Relational { fun: f, argument } => format!("{f:?}({})", t(*argument)),
        Expression::Derivative { .. } => "derivative".into(),
        Expression::ImageQuery { .. } => "image_query".into(),
        Expression::ImageLoad { .. } | Expression::ImageSample { .. } => "image_op".into(),
        Expression::Compose { .. } => "compose".into(),
        other => format!("{:?}", std::mem::discriminant(other)).replace("Discriminant", "expr"),
    }
}

fn base_kind(module: &Module, info: &FunctionInfo, base: Handle<Expression>) -> (String, String) {
    let inner = info[base].ty.inner_with(&module.types);
    let (pointee, space) = match *inner {
        TypeInner::Pointer { base, space } => (&module.types[base].inner, format!("{space:?}")),
        TypeInner::ValuePointer { .. } => return ("vector/matrix(ptr)".into(), "ptr".into()),
        ref v => (v, "value".into()),
    };
    let k = match *pointee {
        TypeInner::Array { size: naga::ArraySize::Dynamic, .. } => "runtime_sized_array".into(),
        TypeInner::Array { size: naga::ArraySize::Constant(n), .. } => format!("fixed_array[{n}]"),
        TypeInner::Array { size: naga::ArraySize::Pending(_), .. } => "override_sized_array".into(),
        TypeInner::BindingArray { .. } => "binding_array".into(),
        TypeInner::Vector { size, .. } => format!("vector{}", size as u8),
        TypeInner::Matrix { .. } => "matrix".into(),
        _ => "other".into(),
    };
    (k, space)
}

fn census(path: &Path, corpus: &str, seen: &mut HashSet<String>) {
    let Ok(src) = fs::read_to_string(path) else { return };
    let norm: String = src.split_whitespace().collect::<Vec<_>>().join(" ");
    let hash = format!("{:x}", Sha256::digest(norm.as_bytes()));
    if !seen.insert(hash) { return; }
    let module = match naga::front::wgsl::parse_str(&src) {
        Ok(m) => m,
        Err(e) => { eprintln!("PARSE_FAIL\t{corpus}\t{}\t{}", path.display(), e.emit_to_string(&src).lines().next().unwrap_or("")); return; }
    };
    let info = match Validator::new(ValidationFlags::all(), Capabilities::all()).validate(&module) {
        Ok(i) => i,
        Err(e) => { eprintln!("VALIDATE_FAIL\t{corpus}\t{}\t{}", path.display(), e.emit_to_string(&src).lines().next().unwrap_or("")); return; }
    };
    eprintln!("OK\t{corpus}\t{}", path.display());
    let lines: Vec<&str> = src.lines().collect();
    let mut emit = |fun: &Function, fi: &FunctionInfo, fname: String, stage: String, args_builtin: Vec<String>| {
        for (h, e) in fun.expressions.iter() {
            if let Expression::Access { base, index } = *e {
                if known(&module, fun, index) { continue; }
                let span = fun.expressions.get_span(h);
                let line = span.location(&src).line_number;
                let text = lines.get(line.saturating_sub(1) as usize).map(|s| s.trim()).unwrap_or("");
                let (kind, space) = base_kind(&module, fi, base);
                let rec = serde_json::json!({
                    "corpus": corpus, "file": path.display().to_string(), "fn": fname, "stage": stage,
                    "line": line, "src": text, "base_kind": kind, "space": space,
                    "base": tree(&module, fun, &args_builtin, base, 0),
                    "index": tree(&module, fun, &args_builtin, index, 0),
                    "safe": fi.safe_accesses.contains(&h),
                });
                println!("{rec}");
            }
        }
    };
    for (h, f) in module.functions.iter() {
        emit(f, &info[h], f.name.clone().unwrap_or_default(), "helper".into(), f.arguments.iter().map(|a| a.name.clone().unwrap_or_default()).collect());
    }
    for (i, ep) in module.entry_points.iter().enumerate() {
        let names = ep.function.arguments.iter().map(|a| match &a.binding {
            Some(naga::Binding::BuiltIn(b)) => format!("{b:?}"),
            _ => a.name.clone().unwrap_or_default(),
        }).collect();
        let stage = match ep.stage { ShaderStage::Compute => "compute", ShaderStage::Vertex => "vertex", ShaderStage::Fragment => "fragment", _ => "other" };
        emit(&ep.function, info.get_entry_point(i), ep.name.clone(), stage.into(), names);
    }
}

fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    if let Ok(rd) = fs::read_dir(dir) {
        let mut v: Vec<_> = rd.flatten().map(|e| e.path()).collect();
        v.sort();
        for p in v {
            if p.is_dir() { walk(&p, out) } else if p.extension().is_some_and(|x| x == "wgsl") { out.push(p) }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let corpus = &args[1];
    let mut seen = HashSet::new();
    for root in &args[2..] {
        let mut files = Vec::new();
        let p = Path::new(root);
        if p.is_file() { files.push(p.to_path_buf()) } else { walk(p, &mut files) }
        for f in files { census(&f, corpus, &mut seen); }
    }
}
