use crate::arena::Handle;
use crate::valid::FunctionInfo;
use crate::{Expression, Function, Module, Statement, Block, BinaryOperator, TypeInner};

/// Performs Loop-Invariant Bounds Check Elision (BCE).
/// Populates `safe_indices` in `FunctionInfo` with expressions that are provably safe bounds.
pub fn elide_bounds_checks(
    function: &Function,
    module: &Module,
    info: &mut FunctionInfo,
) {
    // Traverse the function statements
    analyze_block(&function.body, function, module, info);
}

fn analyze_block(
    block: &Block,
    function: &Function,
    module: &Module,
    info: &mut FunctionInfo,
) {
    for stmt in block {
        match *stmt {
            Statement::Loop { ref body, ref continuing, break_if } => {
                // Handle the WGSL `break if` pattern:
                //   loop { body; continuing { i++; break if i >= N; } }
                // break_if = Some(i >= N) means loop breaks when i >= N,
                // so the body runs with i < N -> safe if len >= N.
                if let Some(cond_expr) = break_if {
                    analyze_break_if(cond_expr, function, module, info);
                }
                // Handle the `if i >= N { break; }` pattern inside the body
                analyze_loop_body(body, function, module, info);
                analyze_block(body, function, module, info);
                analyze_block(continuing, function, module, info);
            }
            Statement::Block(ref b) => analyze_block(b, function, module, info),
            Statement::If { ref accept, ref reject, .. } => {
                analyze_block(accept, function, module, info);
                analyze_block(reject, function, module, info);
            }
            Statement::Switch { ref cases, .. } => {
                for case in cases {
                    analyze_block(&case.body, function, module, info);
                }
            }
            _ => {}
        }
    }
}

fn analyze_loop_body(
    body: &Block,
    function: &Function,
    module: &Module,
    info: &mut FunctionInfo,
) {
    // Look for the `if i < N { ... } else { break; }` pattern
    let mut upper_bound = None;
    let mut induction_var = None;

    for stmt in body {
        if let Statement::If { condition, ref reject, .. } = *stmt {
            if reject.iter().any(|s| matches!(*s, Statement::Break)) {
                extract_bound_from_comparison(condition, function, &mut induction_var, &mut upper_bound);
            }
        }
    }

    tag_safe_accesses(induction_var, upper_bound, function, module, info);
}

/// Analyze a `break_if` expression from a `Statement::Loop`.
/// `break_if = i >= N` means the loop breaks when `i >= N`,
/// so the body runs while `i < N`. `arr[i]` is safe if `len >= N`.
fn analyze_break_if(
    break_if: Handle<crate::Expression>,
    function: &Function,
    module: &Module,
    info: &mut FunctionInfo,
) {
    let mut induction_var = None;
    let mut upper_bound = None;

    if let Expression::Binary { op: BinaryOperator::GreaterEqual, left, right } = function.expressions[break_if] {
        if let Expression::Load { pointer } = function.expressions[left] {
            if let Expression::LocalVariable(v) = function.expressions[pointer] {
                induction_var = Some(v);
                if let Expression::Literal(crate::Literal::U32(val)) = function.expressions[right] {
                    upper_bound = Some(val);
                } else if let Expression::Literal(crate::Literal::I32(val)) = function.expressions[right] {
                    upper_bound = Some(val as u32);
                }
            }
        }
    } else if let Expression::Binary { op: BinaryOperator::Greater, left, right } = function.expressions[break_if] {
        // `break if i > N` -> body runs while `i <= N`, so bound is N+1
        if let Expression::Load { pointer } = function.expressions[left] {
            if let Expression::LocalVariable(v) = function.expressions[pointer] {
                induction_var = Some(v);
                if let Expression::Literal(crate::Literal::U32(val)) = function.expressions[right] {
                    upper_bound = Some(val + 1);
                } else if let Expression::Literal(crate::Literal::I32(val)) = function.expressions[right] {
                    upper_bound = Some(val as u32 + 1);
                }
            }
        }
    }

    tag_safe_accesses(induction_var, upper_bound, function, module, info);
}

/// Extract induction variable and upper bound from comparison expressions.
/// Handles `i < N` (bound = N) and `i <= N` (bound = N+1).
fn extract_bound_from_comparison(
    condition: Handle<crate::Expression>,
    function: &Function,
    induction_var: &mut Option<Handle<crate::LocalVariable>>,
    upper_bound: &mut Option<u32>,
) {
    if let Expression::Binary { op: BinaryOperator::Less, left, right } = function.expressions[condition] {
        if let Expression::Load { pointer } = function.expressions[left] {
            if let Expression::LocalVariable(v) = function.expressions[pointer] {
                *induction_var = Some(v);
                if let Expression::Literal(crate::Literal::U32(val)) = function.expressions[right] {
                    *upper_bound = Some(val);
                } else if let Expression::Literal(crate::Literal::I32(val)) = function.expressions[right] {
                    *upper_bound = Some(val as u32);
                }
            }
        }
    } else if let Expression::Binary { op: BinaryOperator::LessEqual, left, right } = function.expressions[condition] {
        if let Expression::Load { pointer } = function.expressions[left] {
            if let Expression::LocalVariable(v) = function.expressions[pointer] {
                *induction_var = Some(v);
                if let Expression::Literal(crate::Literal::U32(val)) = function.expressions[right] {
                    *upper_bound = Some(val + 1);
                } else if let Expression::Literal(crate::Literal::I32(val)) = function.expressions[right] {
                    *upper_bound = Some(val as u32 + 1);
                }
            }
        }
    }
}

/// Mark all `Access` expressions that use the given induction variable
/// as the index, where array length >= bound, as safe.
fn tag_safe_accesses(
    induction_var: Option<Handle<crate::LocalVariable>>,
    upper_bound: Option<u32>,
    function: &Function,
    module: &Module,
    info: &mut FunctionInfo,
) {
    if let (Some(ind_var), Some(max_val)) = (induction_var, upper_bound) {
        for (_handle, expr) in function.expressions.iter() {
            if let Expression::Access { base, index } = *expr {
                if let Expression::Load { pointer } = function.expressions[index] {
                    if let Expression::LocalVariable(v) = function.expressions[pointer] {
                        if v == ind_var {
                            let base_ty = info[base].ty.inner_with(&module.types);
                            if let TypeInner::Pointer { base: ptr_base, .. } = *base_ty {
                                let inner = &module.types[ptr_base].inner;
                                if let TypeInner::Array { size: crate::ArraySize::Constant(sz), .. } = *inner {
                                    if sz.get() >= max_val {
                                        info.safe_indices.insert(index);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
