use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use naga::valid::{Capabilities, FunctionInfo, ValidationFlags, Validator};
use naga::{Expression, Function, ShaderStage};

#[derive(Default)]
struct FileStats {
    functions: usize,
    compute_eps: usize,
    dynamic_accesses: usize,
    static_accesses: usize,
    elided: usize,
    elided_in_compute: usize,
    dyn_in_compute: usize,
    builtin_lid: usize,
    loop_iv: usize,
    other: usize,
}

fn classify(info: &FunctionInfo, fun: &Function, is_compute: bool) -> FileStats {
    let mut fs = FileStats::default();
    if is_compute {
        fs.compute_eps += 1;
    }
    for (_, expr) in fun.expressions.iter() {
        match *expr {
            Expression::Access { index, .. } => {
                if is_compute {
                    fs.dyn_in_compute += 1;
                }
                fs.dynamic_accesses += 1;
                if info.safe_indices.contains(&index) {
                    fs.elided += 1;
                    if is_compute {
                        fs.elided_in_compute += 1;
                    }
                }
            }
            Expression::AccessIndex { .. } => fs.static_accesses += 1,
            _ => {}
        }
    }
    for handle in info.safe_indices.iter() {
        match fun.expressions[*handle] {
            Expression::FunctionArgument(_) => fs.builtin_lid += 1,
            Expression::Load { pointer } => {
                if matches!(fun.expressions[pointer], Expression::LocalVariable(_)) {
                    fs.loop_iv += 1;
                } else {
                    fs.other += 1;
                }
            }
            _ => fs.other += 1,
        }
    }
    fs
}

fn merge(total: &mut FileStats, fs: &FileStats) {
    total.functions += fs.functions;
    total.compute_eps += fs.compute_eps;
    total.dynamic_accesses += fs.dynamic_accesses;
    total.static_accesses += fs.static_accesses;
    total.elided += fs.elided;
    total.elided_in_compute += fs.elided_in_compute;
    total.dyn_in_compute += fs.dyn_in_compute;
    total.builtin_lid += fs.builtin_lid;
    total.loop_iv += fs.loop_iv;
    total.other += fs.other;
}

fn process(path: &Path) -> Result<(FileStats, String), String> {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return Err(format!("read error: {e}")),
    };
    let module = match naga::front::wgsl::parse_str(&source) {
        Ok(m) => m,
        Err(e) => {
            return Err(format!(
                "parse error: {}",
                e.emit_to_string(&source).lines().next().unwrap_or("")
            ))
        }
    };
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    let info = match validator.validate(&module) {
        Ok(i) => i,
        Err(e) => {
            return Err(format!(
                "validate error: {}",
                e.emit_to_string(&source).lines().next().unwrap_or("")
            ))
        }
    };

    let mut total = FileStats::default();
    for (h, fun) in module.functions.iter() {
        let fi = &info[h];
        let fs = classify(fi, fun, false);
        merge(&mut total, &fs);
    }
    for (i, ep) in module.entry_points.iter().enumerate() {
        let fi: &FunctionInfo = info.get_entry_point(i);
        let is_compute = ep.stage == ShaderStage::Compute;
        let fs = classify(fi, &ep.function, is_compute);
        merge(&mut total, &fs);
    }
    total.functions = module.functions.len() + module.entry_points.len();

    let json = format!(
        r#"{{"file":"{}","functions":{},"compute_eps":{},"dynamic":{},"dyn_in_compute":{},"static_idx":{},"elided":{},"elid_in_compute":{},"builtin_lid":{},"loop_iv":{},"other":{}}}"#,
        path.display(),
        total.functions,
        total.compute_eps,
        total.dynamic_accesses,
        total.dyn_in_compute,
        total.static_accesses,
        total.elided,
        total.elided_in_compute,
        total.builtin_lid,
        total.loop_iv,
        total.other
    );
    Ok((total, json))
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "wgsl") {
                out.push(p);
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
    let name = args[1].clone();
    let mut files = Vec::new();
    for a in &args[2..] {
        let p = Path::new(a);
        if p.is_dir() {
            walk(p, &mut files);
        } else {
            files.push(p.to_path_buf());
        }
    }
    files.sort();

    let mut ok = 0usize;
    let mut parse_fail = 0usize;
    let mut validate_fail = 0usize;
    let mut read_fail = 0usize;
    let mut agg = FileStats::default();

    for f in &files {
        match process(f) {
            Ok((fs, line)) => {
                println!("{line}");
                ok += 1;
                merge(&mut agg, &fs);
            }
            Err(reason) => {
                if reason.starts_with("parse") {
                    parse_fail += 1;
                } else if reason.starts_with("validate") {
                    validate_fail += 1;
                } else {
                    read_fail += 1;
                }
                eprintln!("SKIP\t{}\t{reason}", f.display());
            }
        }
    }

    let pct_all = if agg.dynamic_accesses > 0 {
        100.0 * agg.elided as f64 / agg.dynamic_accesses as f64
    } else {
        0.0
    };
    let pct_compute = if agg.dyn_in_compute > 0 {
        100.0 * agg.elided_in_compute as f64 / agg.dyn_in_compute as f64
    } else {
        0.0
    };
    eprintln!(
        "SUMMARY\t{name}\tfiles={}\tok={ok}\tparse_fail={parse_fail}\tvalidate_fail={validate_fail}\tread_fail={read_fail}\tfunctions={}\tcompute_eps={}\tdynamic_accesses={}\tdyn_in_compute={}\telided={} ({pct_all:.2}%)\telided_in_compute={} ({pct_compute:.2}%)\tbuiltin_lid={}\tloop_iv={}",
        files.len(),
        agg.functions,
        agg.compute_eps,
        agg.dynamic_accesses,
        agg.dyn_in_compute,
        agg.elided,
        agg.elided_in_compute,
        agg.builtin_lid,
        agg.loop_iv
    );
}
