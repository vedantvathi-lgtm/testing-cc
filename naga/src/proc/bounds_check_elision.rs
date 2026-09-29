use alloc::vec::Vec;

use crate::arena::Handle;
use crate::valid::FunctionInfo;
use crate::{
    BinaryOperator, Block, Expression, Function, Literal, LocalVariable, Module, ScalarKind,
    Statement, TypeInner,
};

type Ranges = crate::FastHashMap<Handle<LocalVariable>, IntRange>;
type Values = crate::FastHashMap<Handle<Expression>, IntRange>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IntRange {
    U32 { min: u32, max: u32 },
    I32 { min: i32, max: i32 },
}

impl IntRange {
    const fn full(kind: ScalarKind) -> Option<Self> {
        match kind {
            ScalarKind::Uint => Some(Self::U32 {
                min: u32::MIN,
                max: u32::MAX,
            }),
            ScalarKind::Sint => Some(Self::I32 {
                min: i32::MIN,
                max: i32::MAX,
            }),
            _ => None,
        }
    }

    fn exact(self) -> Option<i64> {
        match self {
            Self::U32 { min, max } if min == max => Some(i64::from(min)),
            Self::I32 { min, max } if min == max => Some(i64::from(min)),
            _ => None,
        }
    }

    fn add(self, rhs: Self) -> Option<Self> {
        match (self, rhs) {
            (Self::U32 { min: a, max: b }, Self::U32 { min: c, max: d }) => Some(Self::U32 {
                min: a.checked_add(c)?,
                max: b.checked_add(d)?,
            }),
            (Self::I32 { min: a, max: b }, Self::I32 { min: c, max: d }) => Some(Self::I32 {
                min: a.checked_add(c)?,
                max: b.checked_add(d)?,
            }),
            _ => None,
        }
    }

    fn sub(self, rhs: Self) -> Option<Self> {
        match (self, rhs) {
            (Self::U32 { min: a, max: b }, Self::U32 { min: c, max: d }) => Some(Self::U32 {
                min: a.checked_sub(d)?,
                max: b.checked_sub(c)?,
            }),
            (Self::I32 { min: a, max: b }, Self::I32 { min: c, max: d }) => Some(Self::I32 {
                min: a.checked_sub(d)?,
                max: b.checked_sub(c)?,
            }),
            _ => None,
        }
    }

    fn mul(self, rhs: Self) -> Option<Self> {
        match (self, rhs) {
            (Self::U32 { min: a, max: b }, Self::U32 { min: c, max: d }) => {
                let values = [
                    a.checked_mul(c)?,
                    a.checked_mul(d)?,
                    b.checked_mul(c)?,
                    b.checked_mul(d)?,
                ];
                Some(Self::U32 {
                    min: *values.iter().min()?,
                    max: *values.iter().max()?,
                })
            }
            (Self::I32 { min: a, max: b }, Self::I32 { min: c, max: d }) => {
                let values = [
                    a.checked_mul(c)?,
                    a.checked_mul(d)?,
                    b.checked_mul(c)?,
                    b.checked_mul(d)?,
                ];
                Some(Self::I32 {
                    min: *values.iter().min()?,
                    max: *values.iter().max()?,
                })
            }
            _ => None,
        }
    }

    fn join(self, rhs: Self) -> Option<Self> {
        match (self, rhs) {
            (Self::U32 { min: a, max: b }, Self::U32 { min: c, max: d }) => Some(Self::U32 {
                min: a.min(c),
                max: b.max(d),
            }),
            (Self::I32 { min: a, max: b }, Self::I32 { min: c, max: d }) => Some(Self::I32 {
                min: a.min(c),
                max: b.max(d),
            }),
            _ => None,
        }
    }

    fn min_i64(self) -> i64 {
        match self {
            Self::U32 { min, .. } => i64::from(min),
            Self::I32 { min, .. } => i64::from(min),
        }
    }

    fn max_i64(self) -> i64 {
        match self {
            Self::U32 { max, .. } => i64::from(max),
            Self::I32 { max, .. } => i64::from(max),
        }
    }

    fn fits(self, length: u32) -> bool {
        match self {
            Self::U32 { max, .. } => max < length,
            Self::I32 { min, max } => min >= 0 && i64::from(max) < i64::from(length),
        }
    }

    fn lower(self, value: i64) -> Option<Self> {
        match self {
            Self::U32 { min, max } => {
                let value = u32::try_from(value).ok()?;
                let min = min.max(value);
                (min <= max).then_some(Self::U32 { min, max })
            }
            Self::I32 { min, max } => {
                let value = i32::try_from(value).ok()?;
                let min = min.max(value);
                (min <= max).then_some(Self::I32 { min, max })
            }
        }
    }

    fn upper(self, value: i64) -> Option<Self> {
        match self {
            Self::U32 { min, max } => {
                let value = u32::try_from(value).ok()?;
                let max = max.min(value);
                (min <= max).then_some(Self::U32 { min, max })
            }
            Self::I32 { min, max } => {
                let value = i32::try_from(value).ok()?;
                let max = max.min(value);
                (min <= max).then_some(Self::I32 { min, max })
            }
        }
    }
}

/// What a branch condition refines.
#[derive(Clone, Copy)]
enum Target {
    Local(Handle<LocalVariable>),
    Value(Handle<Expression>),
}

#[derive(Clone)]
struct Flow {
    ranges: Ranges,
    values: Values,
    falls_through: bool,
}

struct Analysis<'a> {
    function: &'a Function,
    module: &'a Module,
    info: &'a mut FunctionInfo,
    workgroup_size: Option<[u32; 3]>,
}

pub fn elide_bounds_checks(function: &Function, module: &Module, info: &mut FunctionInfo) {
    let workgroup_size = module.entry_points.iter().find_map(|entry_point| {
        core::ptr::eq(&entry_point.function, function).then_some(entry_point.workgroup_size)
    });
    let mut analysis = Analysis {
        function,
        module,
        info,
        workgroup_size,
    };
    let initial = analysis.default_ranges();
    analysis.analyze_block(&function.body, initial, Values::default());
}

impl Analysis<'_> {
    fn default_ranges(&self) -> Ranges {
        let mut ranges: Ranges = self
            .function
            .local_variables
            .iter()
            .filter_map(|(handle, _)| self.local_full_range(handle).map(|range| (handle, range)))
            .collect();
        for (handle, variable) in self.function.local_variables.iter() {
            if let Some(init) = variable.init {
                if let Some(range) = self.expression_range(init, &Values::default()) {
                    ranges.insert(handle, range);
                }
            }
        }
        ranges
    }

    #[allow(clippy::pattern_type_mismatch)]
    fn analyze_block(&mut self, block: &Block, mut ranges: Ranges, mut values: Values) -> Flow {
        for statement in block {
            match statement {
                Statement::Emit(expressions) => {
                    for handle in expressions.clone() {
                        if let Some(range) = self.emitted_expression_range(handle, &ranges, &values)
                        {
                            values.insert(handle, range);
                        } else {
                            values.remove(&handle);
                        }
                        self.prove_access(handle, &values);
                    }
                }
                Statement::Block(nested) => {
                    let flow = self.analyze_block(nested, ranges, values);
                    if !flow.falls_through {
                        return flow;
                    }
                    ranges = flow.ranges;
                    values = flow.values;
                }
                Statement::If {
                    condition,
                    accept,
                    reject,
                } => {
                    let accept_flow = self
                        .refine(*condition, true, ranges.clone(), values.clone())
                        .map(|(facts, known)| self.analyze_block(accept, facts, known));
                    let reject_flow = self
                        .refine(*condition, false, ranges.clone(), values.clone())
                        .map(|(facts, known)| self.analyze_block(reject, facts, known));
                    match (accept_flow, reject_flow) {
                        (Some(left), Some(right)) if left.falls_through && right.falls_through => {
                            ranges = join_ranges(&left.ranges, &right.ranges);
                            values = join_values(&left.values, &right.values);
                        }
                        (Some(left), _) if left.falls_through => {
                            ranges = left.ranges;
                            values = left.values;
                        }
                        (_, Some(right)) if right.falls_through => {
                            ranges = right.ranges;
                            values = right.values;
                        }
                        _ => {
                            return Flow {
                                ranges,
                                values,
                                falls_through: false,
                            };
                        }
                    }
                }
                Statement::Switch { cases, .. } => {
                    let mut mutated = crate::FastHashSet::default();
                    for case in cases {
                        collect_mutated(&case.body, self.function, &mut mutated);
                    }
                    let mut case_ranges = ranges.clone();
                    for &local in &mutated {
                        self.reset_local(local, &mut case_ranges);
                    }
                    for case in cases {
                        self.analyze_block(&case.body, case_ranges.clone(), values.clone());
                    }
                    for local in mutated {
                        self.reset_local(local, &mut ranges);
                    }
                }
                Statement::Loop {
                    body,
                    continuing,
                    break_if,
                } => {
                    self.analyze_loop(body, continuing, *break_if, &ranges, &values);
                    for local in mutated_locals(body, continuing, self.function) {
                        self.reset_local(local, &mut ranges);
                    }
                }
                Statement::Store { pointer, value } => {
                    if let Expression::LocalVariable(local) = self.function.expressions[*pointer] {
                        if let Some(range) = self.expression_range(*value, &values) {
                            ranges.insert(local, range);
                        } else {
                            self.reset_local(local, &mut ranges);
                        }
                    } else if let Some(local) = root_local(*pointer, self.function) {
                        self.reset_local(local, &mut ranges);
                    }
                }
                Statement::Atomic { pointer, .. } => {
                    if let Some(local) = root_local(*pointer, self.function) {
                        self.reset_local(local, &mut ranges);
                    }
                }
                Statement::Call { .. } => self.reset_all_locals(&mut ranges),
                Statement::Break
                | Statement::Continue
                | Statement::Return { .. }
                | Statement::Kill => {
                    return Flow {
                        ranges,
                        values,
                        falls_through: false,
                    };
                }
                _ => {}
            }
        }
        Flow {
            ranges,
            values,
            falls_through: true,
        }
    }

    fn analyze_loop(
        &mut self,
        body: &Block,
        continuing: &Block,
        break_if: Option<Handle<Expression>>,
        incoming: &Ranges,
        values: &Values,
    ) {
        let mutated = mutated_locals(body, continuing, self.function);
        let mut entry = incoming.clone();
        for &local in &mutated {
            self.reset_local(local, &mut entry);
        }
        for local in mutated {
            if let Some(range) = self.loop_range(local, body, continuing, break_if, incoming) {
                entry.insert(local, range);
            }
        }
        let body_flow = self.analyze_block(body, entry.clone(), values.clone());
        self.analyze_block(
            continuing,
            if body_flow.falls_through {
                body_flow.ranges
            } else {
                entry
            },
            if body_flow.falls_through {
                body_flow.values
            } else {
                values.clone()
            },
        );
    }

    fn loop_range(
        &self,
        local: Handle<LocalVariable>,
        body: &Block,
        continuing: &Block,
        break_if: Option<Handle<Expression>>,
        incoming: &Ranges,
    ) -> Option<IntRange> {
        let guard_position =
            loop_guard(body, local, self.function).map(|(_, _, position)| position);
        let start = incoming
            .get(&local)
            .copied()
            .and_then(IntRange::exact)
            .or_else(|| {
                guard_position.and_then(|position| {
                    body.iter().take(position).find_map(|statement| {
                        initializer_store(local, statement, self.function, self.module)
                    })
                })
            })?;
        // `i = i >> c` / `i = i / c` only move a non-negative `i` towards zero, so
        // every value the loop sees lies in [0, start] whatever the exit test.
        if shrinking_update(local, body, continuing, self.function, self.module) {
            return make_range(self.local_full_range(local)?, 0, start).filter(|_| start >= 0);
        }
        let (step, template, update_location) =
            single_update(local, body, continuing, self.function, self.module)?;
        let (condition, breaks_when_true, terminal_reaches_header) =
            if let Some(condition) = break_if {
                (condition, true, false)
            } else {
                let (condition, breaks_when_true, guard_position) =
                    loop_guard(body, local, self.function)?;
                let terminal_reaches_header = match update_location {
                    UpdateLocation::Body(position) => position >= guard_position,
                    UpdateLocation::Continuing => true,
                };
                (condition, breaks_when_true, terminal_reaches_header)
            };
        let (op, bound) = comparison(condition, local, self.function, self.module)?;
        let exit_op = if breaks_when_true {
            op
        } else {
            negate_comparison(op)?
        };
        let (active, exit) = recurrence_ranges(start, step, exit_op, bound, template)?;
        if terminal_reaches_header {
            active.join(exit)
        } else {
            Some(active)
        }
    }

    fn emitted_expression_range(
        &self,
        handle: Handle<Expression>,
        ranges: &Ranges,
        values: &Values,
    ) -> Option<IntRange> {
        if let Expression::Load { pointer } = self.function.expressions[handle] {
            return match self.function.expressions[pointer] {
                Expression::LocalVariable(local) => ranges.get(&local).copied(),
                _ => None,
            };
        }
        self.expression_range(handle, values)
    }

    fn expression_range(&self, handle: Handle<Expression>, values: &Values) -> Option<IntRange> {
        if let Some(range) = values.get(&handle) {
            return Some(*range);
        }
        match self.function.expressions[handle] {
            Expression::Literal(literal) => literal_range(literal),
            Expression::Constant(constant) => {
                global_range(self.module.constants[constant].init, self.module)
            }
            Expression::FunctionArgument(index) => self.argument_range(index),
            Expression::AccessIndex { base, index } => self.builtin_component_range(base, index),
            Expression::Load { .. } => None,
            Expression::Binary { op, left, right } => match op {
                BinaryOperator::Add => self
                    .expression_range(left, values)?
                    .add(self.expression_range(right, values)?),
                BinaryOperator::Subtract => self
                    .expression_range(left, values)?
                    .sub(self.expression_range(right, values)?),
                BinaryOperator::Multiply => self
                    .expression_range(left, values)?
                    .mul(self.expression_range(right, values)?),
                BinaryOperator::ShiftLeft => shift_left(
                    self.expression_range(left, values)?,
                    self.expression_range(right, values)?,
                ),
                // These bound their result even when the left operand is unknown.
                BinaryOperator::Divide => divide(
                    self.range_or_full(left, values)?,
                    self.expression_range(right, values)?,
                ),
                BinaryOperator::Modulo => modulo(
                    self.range_or_full(left, values)?,
                    self.expression_range(right, values)?,
                ),
                BinaryOperator::ShiftRight => shift_right(
                    self.range_or_full(left, values)?,
                    self.expression_range(right, values)?,
                ),
                BinaryOperator::And => bit_and(
                    self.range_or_full(left, values)?,
                    self.range_or_full(right, values)?,
                ),
                _ => None,
            },
            Expression::Math {
                fun,
                arg,
                arg1,
                arg2,
                ..
            } => {
                let first = self.range_or_full(arg, values)?;
                match fun {
                    crate::MathFunction::Min => {
                        min_range(first, self.range_or_full(arg1?, values)?)
                    }
                    crate::MathFunction::Max => {
                        max_range(first, self.range_or_full(arg1?, values)?)
                    }
                    crate::MathFunction::Clamp => min_range(
                        max_range(first, self.range_or_full(arg1?, values)?)?,
                        self.range_or_full(arg2?, values)?,
                    ),
                    _ => None,
                }
            }
            Expression::Select { accept, reject, .. } => self
                .expression_range(accept, values)?
                .join(self.expression_range(reject, values)?),
            Expression::As {
                expr,
                kind,
                convert: None | Some(4),
            } => convert_range(self.expression_range(expr, values)?, kind),
            _ => None,
        }
    }

    /// The known range of `handle`, or the full range of its 32-bit integer type.
    fn range_or_full(&self, handle: Handle<Expression>, values: &Values) -> Option<IntRange> {
        self.expression_range(handle, values).or_else(|| {
            match *self.info[handle].ty.inner_with(&self.module.types) {
                TypeInner::Scalar(scalar) if scalar.width == 4 => IntRange::full(scalar.kind),
                _ => None,
            }
        })
    }

    fn prove_access(&mut self, handle: Handle<Expression>, values: &Values) {
        let Expression::Access { base, index } = self.function.expressions[handle] else {
            return;
        };
        let Some(range) = self.expression_range(index, values) else {
            return;
        };
        let inner = self.info[base].ty.inner_with(&self.module.types);
        let Ok(super::IndexableLength::Known(length)) =
            inner.indexable_length_resolved(self.module)
        else {
            return;
        };
        if range.fits(length) {
            self.info.safe_accesses.insert(handle);
        }
    }

    /// Facts that hold on the edge where `condition` evaluates to `truth`.
    ///
    /// `None` means the edge is infeasible: the refined range of some local is
    /// empty. It never means "cannot refine" — a condition the analysis does not
    /// model returns the facts unchanged. Treating unknown as infeasible would
    /// discard the effects of a branch that can actually run.
    fn refine(
        &self,
        condition: Handle<Expression>,
        truth: bool,
        mut ranges: Ranges,
        mut values: Values,
    ) -> Option<(Ranges, Values)> {
        let Expression::Binary { op, left, right } = self.function.expressions[condition] else {
            if let Expression::Unary {
                op: crate::UnaryOperator::LogicalNot,
                expr,
            } = self.function.expressions[condition]
            {
                return self.refine(expr, !truth, ranges, values);
            }
            return Some((ranges, values));
        };
        match (op, truth) {
            (BinaryOperator::LogicalAnd, true) | (BinaryOperator::LogicalOr, false) => {
                let (ranges, values) = self.refine(left, truth, ranges, values)?;
                return self.refine(right, truth, ranges, values);
            }
            (BinaryOperator::LogicalAnd, false) | (BinaryOperator::LogicalOr, true) => {
                return Some((ranges, values));
            }
            _ => {}
        }
        // A condition we cannot model teaches us nothing: keep the facts as they
        // are. Only an empty refined range makes this edge infeasible.
        let Some((target, refined)) =
            self.comparison_fact(op, left, right, truth, &ranges, &values)
        else {
            return Some((ranges, values));
        };
        let refined = refined?;
        match target {
            Target::Local(local) => {
                ranges.insert(local, refined);
            }
            // The expression's value was computed before the branch and cannot
            // change inside it, so its refined range holds for the whole edge.
            Target::Value(handle) => {
                values.insert(handle, refined);
            }
        }
        Some((ranges, values))
    }

    /// What `left op right` constrains when it evaluates to `truth`: the target
    /// (a local variable or an already-computed integer value) and its refined
    /// range, where `Some(None)` means the refined range is empty. Returns
    /// `None` if the condition constrains nothing we track.
    fn comparison_fact(
        &self,
        op: BinaryOperator,
        left: Handle<Expression>,
        right: Handle<Expression>,
        truth: bool,
        ranges: &Ranges,
        values: &Values,
    ) -> Option<(Target, Option<IntRange>)> {
        // Also rejects non-comparison operators (e.g. boolean `&`) on both edges.
        let negated = negate_comparison(op)?;
        let op = if truth { op } else { negated };
        let (subject, op, other) = match (
            self.target(left, ranges, values),
            self.target(right, ranges, values),
        ) {
            (Some(target), _) => (target, op, right),
            (None, Some(target)) => (target, swap_comparison(op)?, left),
            (None, None) => return None,
        };
        let (target, current) = subject;
        let bound = self.expression_range(other, values)?;
        let refined = match (op, bound.exact()) {
            (_, Some(exact)) => refine_range(current, op, exact),
            // A bound known only as a range: `x < e` with `e <= hi` gives `x < hi`, etc.
            (BinaryOperator::Less | BinaryOperator::LessEqual, None) => {
                refine_range(current, op, bound.max_i64())
            }
            (BinaryOperator::Greater | BinaryOperator::GreaterEqual, None) => {
                refine_range(current, op, bound.min_i64())
            }
            (BinaryOperator::Equal, None) => current
                .lower(bound.min_i64())
                .and_then(|range| range.upper(bound.max_i64())),
            _ => Some(current),
        };
        Some((target, refined))
    }

    /// A comparison operand we can refine, with its current range: a loaded
    /// 32-bit integer local, or an integer value whose range we already know.
    fn target(
        &self,
        handle: Handle<Expression>,
        ranges: &Ranges,
        values: &Values,
    ) -> Option<(Target, IntRange)> {
        if let Some(local) = loaded_local(handle, self.function) {
            let current = ranges
                .get(&local)
                .copied()
                .or_else(|| self.local_full_range(local))?;
            return Some((Target::Local(local), current));
        }
        let current = self.expression_range(handle, values)?;
        Some((Target::Value(handle), current))
    }

    #[allow(clippy::pattern_type_mismatch)]
    fn argument_range(&self, index: u32) -> Option<IntRange> {
        match self.function.arguments[index as usize].binding.as_ref()? {
            crate::Binding::BuiltIn(crate::BuiltIn::LocalInvocationIndex) => {
                let size = self.workgroup_size?;
                let total = size[0].checked_mul(size[1])?.checked_mul(size[2])?;
                (total > 0).then_some(IntRange::U32 {
                    min: 0,
                    max: total - 1,
                })
            }
            _ => None,
        }
    }

    fn builtin_component_range(&self, base: Handle<Expression>, index: u32) -> Option<IntRange> {
        let Expression::FunctionArgument(argument) = self.function.expressions[base] else {
            return None;
        };
        if !matches!(
            self.function.arguments[argument as usize].binding,
            Some(crate::Binding::BuiltIn(crate::BuiltIn::LocalInvocationId))
        ) {
            return None;
        }
        let size = *self.workgroup_size?.get(index as usize)?;
        (size > 0).then_some(IntRange::U32 {
            min: 0,
            max: size - 1,
        })
    }

    fn local_full_range(&self, local: Handle<LocalVariable>) -> Option<IntRange> {
        match self.module.types[self.function.local_variables[local].ty].inner {
            TypeInner::Scalar(scalar) if scalar.width == 4 => IntRange::full(scalar.kind),
            _ => None,
        }
    }

    fn reset_local(&self, local: Handle<LocalVariable>, ranges: &mut Ranges) {
        if let Some(range) = self.local_full_range(local) {
            ranges.insert(local, range);
        } else {
            ranges.remove(&local);
        }
    }

    fn reset_all_locals(&self, ranges: &mut Ranges) {
        for (local, _) in self.function.local_variables.iter() {
            self.reset_local(local, ranges);
        }
    }
}

const fn literal_range(literal: Literal) -> Option<IntRange> {
    match literal {
        Literal::U32(value) => Some(IntRange::U32 {
            min: value,
            max: value,
        }),
        Literal::I32(value) => Some(IntRange::I32 {
            min: value,
            max: value,
        }),
        _ => None,
    }
}

fn global_range(handle: Handle<Expression>, module: &Module) -> Option<IntRange> {
    match module.global_expressions[handle] {
        Expression::Literal(literal) => literal_range(literal),
        Expression::Constant(constant) => global_range(module.constants[constant].init, module),
        Expression::Binary { op, left, right } => {
            let left = global_range(left, module)?;
            let right = global_range(right, module)?;
            match op {
                BinaryOperator::Add => left.add(right),
                BinaryOperator::Subtract => left.sub(right),
                BinaryOperator::Multiply => left.mul(right),
                _ => None,
            }
        }
        _ => None,
    }
}

fn loaded_local(handle: Handle<Expression>, function: &Function) -> Option<Handle<LocalVariable>> {
    let Expression::Load { pointer } = function.expressions[handle] else {
        return None;
    };
    let Expression::LocalVariable(local) = function.expressions[pointer] else {
        return None;
    };
    Some(local)
}

fn root_local(handle: Handle<Expression>, function: &Function) -> Option<Handle<LocalVariable>> {
    match function.expressions[handle] {
        Expression::LocalVariable(local) => Some(local),
        Expression::Access { base, .. } | Expression::AccessIndex { base, .. } => {
            root_local(base, function)
        }
        _ => None,
    }
}

fn refine_range(range: IntRange, op: BinaryOperator, bound: i64) -> Option<IntRange> {
    match op {
        BinaryOperator::Less => range.upper(bound.checked_sub(1)?),
        BinaryOperator::LessEqual => range.upper(bound),
        BinaryOperator::Greater => range.lower(bound.checked_add(1)?),
        BinaryOperator::GreaterEqual => range.lower(bound),
        BinaryOperator::Equal => range.lower(bound)?.upper(bound),
        BinaryOperator::NotEqual => match range.exact() {
            Some(value) if value == bound => None,
            _ if bound == range.max_i64() => range.upper(bound - 1),
            _ if bound == range.min_i64() => range.lower(bound + 1),
            _ => Some(range),
        },
        _ => None,
    }
}

const fn negate_comparison(op: BinaryOperator) -> Option<BinaryOperator> {
    Some(match op {
        BinaryOperator::Less => BinaryOperator::GreaterEqual,
        BinaryOperator::LessEqual => BinaryOperator::Greater,
        BinaryOperator::Greater => BinaryOperator::LessEqual,
        BinaryOperator::GreaterEqual => BinaryOperator::Less,
        BinaryOperator::Equal => BinaryOperator::NotEqual,
        BinaryOperator::NotEqual => BinaryOperator::Equal,
        _ => return None,
    })
}

const fn swap_comparison(op: BinaryOperator) -> Option<BinaryOperator> {
    Some(match op {
        BinaryOperator::Less => BinaryOperator::Greater,
        BinaryOperator::LessEqual => BinaryOperator::GreaterEqual,
        BinaryOperator::Greater => BinaryOperator::Less,
        BinaryOperator::GreaterEqual => BinaryOperator::LessEqual,
        BinaryOperator::Equal | BinaryOperator::NotEqual => op,
        _ => return None,
    })
}

fn join_ranges(left: &Ranges, right: &Ranges) -> Ranges {
    left.iter()
        .filter_map(|(&local, &range)| {
            right
                .get(&local)
                .and_then(|&other| range.join(other))
                .map(|joined| (local, joined))
        })
        .collect()
}

fn join_values(left: &Values, right: &Values) -> Values {
    left.iter()
        .filter_map(|(&expression, &range)| {
            right
                .get(&expression)
                .and_then(|&other| range.join(other))
                .map(|joined| (expression, joined))
        })
        .collect()
}

fn mutated_locals(
    body: &Block,
    continuing: &Block,
    function: &Function,
) -> crate::FastHashSet<Handle<LocalVariable>> {
    let mut result = crate::FastHashSet::default();
    collect_mutated(body, function, &mut result);
    collect_mutated(continuing, function, &mut result);
    result
}

#[allow(clippy::pattern_type_mismatch)]
fn collect_mutated(
    block: &Block,
    function: &Function,
    result: &mut crate::FastHashSet<Handle<LocalVariable>>,
) {
    for statement in block {
        match statement {
            Statement::Store { pointer, .. } | Statement::Atomic { pointer, .. } => {
                if let Some(local) = root_local(*pointer, function) {
                    result.insert(local);
                }
            }
            Statement::Block(block) => collect_mutated(block, function, result),
            Statement::If { accept, reject, .. } => {
                collect_mutated(accept, function, result);
                collect_mutated(reject, function, result);
            }
            Statement::Switch { cases, .. } => {
                for case in cases {
                    collect_mutated(&case.body, function, result);
                }
            }
            Statement::Loop {
                body, continuing, ..
            } => {
                collect_mutated(body, function, result);
                collect_mutated(continuing, function, result);
            }
            Statement::Call { .. } => {
                for (local, _) in function.local_variables.iter() {
                    result.insert(local);
                }
            }
            _ => {}
        }
    }
}

#[derive(Clone, Copy)]
enum UpdateLocation {
    Body(usize),
    Continuing,
}

fn single_update(
    local: Handle<LocalVariable>,
    body: &Block,
    continuing: &Block,
    function: &Function,
    module: &Module,
) -> Option<(i64, IntRange, UpdateLocation)> {
    let mut updates = Vec::new();
    for (position, statement) in body.iter().enumerate() {
        if let Some((target, step, template)) = direct_update(statement, function, module) {
            if target == local {
                updates.push((step, template, UpdateLocation::Body(position)));
            }
        }
    }
    for statement in continuing {
        if let Some((target, step, template)) = direct_update(statement, function, module) {
            if target == local {
                updates.push((step, template, UpdateLocation::Continuing));
            }
        }
    }
    if updates.len() != 1 {
        return None;
    }
    let mutations = local_mutation_count(body, local, function)
        .checked_add(local_mutation_count(continuing, local, function))?;
    let initializers = body
        .iter()
        .filter(|statement| initializer_store(local, statement, function, module).is_some())
        .count();
    (mutations == 1 || (mutations == 2 && initializers == 1)).then(|| updates[0])
}

/// Whether the only mutation of `local` in the loop is `local = local >> c`
/// (`c >= 1`) or `local = local / c` (`c >= 2`) with a constant `c`.
fn shrinking_update(
    local: Handle<LocalVariable>,
    body: &Block,
    continuing: &Block,
    function: &Function,
    module: &Module,
) -> bool {
    let is_shrink = |statement: &Statement| {
        let Statement::Store { pointer, value } = *statement else {
            return false;
        };
        if function.expressions[pointer] != Expression::LocalVariable(local) {
            return false;
        }
        let Expression::Binary { op, left, right } = function.expressions[value] else {
            return false;
        };
        if loaded_local(left, function) != Some(local) {
            return false;
        }
        match (op, exact_constant(right, function, module)) {
            (BinaryOperator::ShiftRight, Some(c)) => (1..32).contains(&c),
            (BinaryOperator::Divide, Some(c)) => c >= 2,
            _ => false,
        }
    };
    let shrinks = body
        .iter()
        .chain(continuing.iter())
        .filter(|s| is_shrink(s))
        .count();
    let mutations = local_mutation_count(body, local, function)
        .saturating_add(local_mutation_count(continuing, local, function));
    shrinks == 1 && mutations == 1
}

fn initializer_store(
    local: Handle<LocalVariable>,
    statement: &Statement,
    function: &Function,
    module: &Module,
) -> Option<i64> {
    let Statement::Store { pointer, value } = *statement else {
        return None;
    };
    if function.expressions[pointer] != Expression::LocalVariable(local) {
        return None;
    }
    exact_constant(value, function, module)
}

#[allow(clippy::pattern_type_mismatch)]
fn local_mutation_count(block: &Block, local: Handle<LocalVariable>, function: &Function) -> usize {
    block
        .iter()
        .map(|statement| match statement {
            Statement::Store { pointer, .. } | Statement::Atomic { pointer, .. } => {
                usize::from(root_local(*pointer, function) == Some(local))
            }
            Statement::Block(block) => local_mutation_count(block, local, function),
            Statement::If { accept, reject, .. } => local_mutation_count(accept, local, function)
                .saturating_add(local_mutation_count(reject, local, function)),
            Statement::Switch { cases, .. } => cases.iter().fold(0usize, |count, case| {
                count.saturating_add(local_mutation_count(&case.body, local, function))
            }),
            Statement::Loop {
                body, continuing, ..
            } => local_mutation_count(body, local, function)
                .saturating_add(local_mutation_count(continuing, local, function)),
            Statement::Call { .. } => 1,
            _ => 0,
        })
        .fold(0usize, usize::saturating_add)
}

fn direct_update(
    statement: &Statement,
    function: &Function,
    module: &Module,
) -> Option<(Handle<LocalVariable>, i64, IntRange)> {
    let Statement::Store { pointer, value } = *statement else {
        return None;
    };
    let Expression::LocalVariable(local) = function.expressions[pointer] else {
        return None;
    };
    let Expression::Binary { op, left, right } = function.expressions[value] else {
        return None;
    };
    if loaded_local(left, function) != Some(local) {
        return None;
    }
    let constant = match function.expressions[right] {
        Expression::Literal(literal) => literal_range(literal)?,
        Expression::Constant(constant) => global_range(module.constants[constant].init, module)?,
        _ => return None,
    };
    let magnitude = constant.exact()?;
    if magnitude <= 0 {
        return None;
    }
    let step = match op {
        BinaryOperator::Add => magnitude,
        BinaryOperator::Subtract => magnitude.checked_neg()?,
        _ => return None,
    };
    Some((local, step, constant))
}

fn loop_guard(
    body: &Block,
    local: Handle<LocalVariable>,
    function: &Function,
) -> Option<(Handle<Expression>, bool, usize)> {
    body.iter().enumerate().find_map(|(position, statement)| {
        #[allow(clippy::pattern_type_mismatch)]
        let Statement::If {
            condition,
            accept,
            reject,
        } = statement
        else {
            return None;
        };
        if !condition_mentions(*condition, local, function) {
            return None;
        }
        if matches!(accept.last(), Some(Statement::Break)) {
            Some((*condition, true, position))
        } else if matches!(reject.last(), Some(Statement::Break)) {
            Some((*condition, false, position))
        } else {
            None
        }
    })
}

fn condition_mentions(
    condition: Handle<Expression>,
    local: Handle<LocalVariable>,
    function: &Function,
) -> bool {
    match function.expressions[condition] {
        Expression::Binary { left, right, .. } => {
            loaded_local(left, function) == Some(local)
                || loaded_local(right, function) == Some(local)
        }
        _ => false,
    }
}

fn comparison(
    condition: Handle<Expression>,
    local: Handle<LocalVariable>,
    function: &Function,
    module: &Module,
) -> Option<(BinaryOperator, i64)> {
    let Expression::Binary { op, left, right } = function.expressions[condition] else {
        return None;
    };
    if loaded_local(left, function) == Some(local) {
        Some((op, exact_constant(right, function, module)?))
    } else if loaded_local(right, function) == Some(local) {
        Some((
            swap_comparison(op)?,
            exact_constant(left, function, module)?,
        ))
    } else {
        None
    }
}

fn exact_constant(handle: Handle<Expression>, function: &Function, module: &Module) -> Option<i64> {
    match function.expressions[handle] {
        Expression::Literal(literal) => literal_range(literal)?.exact(),
        Expression::Constant(constant) => {
            global_range(module.constants[constant].init, module)?.exact()
        }
        _ => None,
    }
}

fn recurrence_ranges(
    start: i64,
    step: i64,
    exit_op: BinaryOperator,
    bound: i64,
    template: IntRange,
) -> Option<(IntRange, IntRange)> {
    let (min, max, exit) = if step > 0 {
        let distance = bound.checked_sub(start)?;
        if distance <= 0 {
            return None;
        }
        let iterations = match exit_op {
            BinaryOperator::GreaterEqual => ceil_div(distance, step)?,
            BinaryOperator::Greater => ceil_div(distance.checked_add(1)?, step)?,
            BinaryOperator::Equal if distance % step == 0 => distance / step,
            _ => return None,
        };
        let max = start.checked_add(iterations.checked_sub(1)?.checked_mul(step)?)?;
        let exit = start.checked_add(iterations.checked_mul(step)?)?;
        (start, max, exit)
    } else if step < 0 {
        let magnitude = step.checked_neg()?;
        let distance = start.checked_sub(bound)?;
        if distance <= 0 {
            return None;
        }
        let iterations = match exit_op {
            BinaryOperator::LessEqual => ceil_div(distance, magnitude)?,
            BinaryOperator::Less => ceil_div(distance.checked_add(1)?, magnitude)?,
            BinaryOperator::Equal if distance % magnitude == 0 => distance / magnitude,
            _ => return None,
        };
        let min = start.checked_sub(iterations.checked_sub(1)?.checked_mul(magnitude)?)?;
        let exit = start.checked_sub(iterations.checked_mul(magnitude)?)?;
        (min, start, exit)
    } else {
        return None;
    };
    Some((
        make_range(template, min, max)?,
        make_range(template, exit, exit)?,
    ))
}

const fn divide(left: IntRange, right: IntRange) -> Option<IntRange> {
    use IntRange::{I32, U32};
    match (left, right) {
        (U32 { min: a, max: b }, U32 { min: c, max: d }) if c > 0 => Some(U32 {
            min: a / d,
            max: b / c,
        }),
        (I32 { min: a, max: b }, I32 { min: c, max: d }) if a >= 0 && c > 0 => Some(I32 {
            min: a / d,
            max: b / c,
        }),
        _ => None,
    }
}

/// `left % k` for a constant `k > 0`. Signed operands must be non-negative,
/// since WGSL `%` keeps the sign of the dividend.
fn modulo(left: IntRange, right: IntRange) -> Option<IntRange> {
    use IntRange::{I32, U32};
    let k = right.exact()?;
    match left {
        U32 { min, max } => {
            let k = u32::try_from(k).ok().filter(|&k| k > 0)?;
            Some(if min / k == max / k {
                U32 {
                    min: min % k,
                    max: max % k,
                }
            } else {
                U32 { min: 0, max: k - 1 }
            })
        }
        I32 { min, max } if min >= 0 => {
            let k = i32::try_from(k).ok().filter(|&k| k > 0)?;
            Some(if min / k == max / k {
                I32 {
                    min: min % k,
                    max: max % k,
                }
            } else {
                I32 { min: 0, max: k - 1 }
            })
        }
        I32 { .. } => None,
    }
}

/// Shift amounts are taken modulo 32 by WGSL, so only `[0, 31]` is modelled.
const fn shift_amounts(right: IntRange) -> Option<(u32, u32)> {
    match right {
        IntRange::U32 { min, max } if max < 32 => Some((min, max)),
        _ => None,
    }
}

fn shift_right(left: IntRange, right: IntRange) -> Option<IntRange> {
    use IntRange::{I32, U32};
    let (c, d) = shift_amounts(right)?;
    match left {
        U32 { min, max } => Some(U32 {
            min: min >> d,
            max: max >> c,
        }),
        I32 { min, max } if min >= 0 => Some(I32 {
            min: min >> d,
            max: max >> c,
        }),
        I32 { .. } => None,
    }
}

fn shift_left(left: IntRange, right: IntRange) -> Option<IntRange> {
    use IntRange::{I32, U32};
    let (c, d) = shift_amounts(right)?;
    match left {
        U32 { min, max } => {
            let high = u32::try_from(u64::from(max) << d).ok()?;
            Some(U32 {
                min: min << c,
                max: high,
            })
        }
        I32 { min, max } if min >= 0 => {
            let high = i32::try_from(i64::from(max) << d).ok()?;
            Some(I32 {
                min: min << c,
                max: high,
            })
        }
        I32 { .. } => None,
    }
}

/// `left & right` is at most the smaller non-negative operand's maximum.
fn bit_and(left: IntRange, right: IntRange) -> Option<IntRange> {
    use IntRange::{I32, U32};
    match (left, right) {
        (U32 { max: a, .. }, U32 { max: b, .. }) => Some(U32 {
            min: 0,
            max: a.min(b),
        }),
        (I32 { min: a, max: b }, I32 { min: c, max: d }) => match (a >= 0, c >= 0) {
            (true, true) => Some(I32 {
                min: 0,
                max: b.min(d),
            }),
            (true, false) => Some(I32 { min: 0, max: b }),
            (false, true) => Some(I32 { min: 0, max: d }),
            (false, false) => None,
        },
        _ => None,
    }
}

fn min_range(left: IntRange, right: IntRange) -> Option<IntRange> {
    use IntRange::{I32, U32};
    match (left, right) {
        (U32 { min: a, max: b }, U32 { min: c, max: d }) => Some(U32 {
            min: a.min(c),
            max: b.min(d),
        }),
        (I32 { min: a, max: b }, I32 { min: c, max: d }) => Some(I32 {
            min: a.min(c),
            max: b.min(d),
        }),
        _ => None,
    }
}

fn max_range(left: IntRange, right: IntRange) -> Option<IntRange> {
    use IntRange::{I32, U32};
    match (left, right) {
        (U32 { min: a, max: b }, U32 { min: c, max: d }) => Some(U32 {
            min: a.max(c),
            max: b.max(d),
        }),
        (I32 { min: a, max: b }, I32 { min: c, max: d }) => Some(I32 {
            min: a.max(c),
            max: b.max(d),
        }),
        _ => None,
    }
}

/// Integer conversion (or bitcast) between `u32` and `i32`: value-preserving
/// only when the whole range is representable in the target type.
fn convert_range(range: IntRange, kind: ScalarKind) -> Option<IntRange> {
    use IntRange::{I32, U32};
    match (range, kind) {
        (U32 { .. }, ScalarKind::Uint) | (I32 { .. }, ScalarKind::Sint) => Some(range),
        (U32 { min, max }, ScalarKind::Sint) => Some(I32 {
            min: i32::try_from(min).ok()?,
            max: i32::try_from(max).ok()?,
        }),
        (I32 { min, max }, ScalarKind::Uint) => Some(U32 {
            min: u32::try_from(min).ok()?,
            max: u32::try_from(max).ok()?,
        }),
        _ => None,
    }
}

fn ceil_div(numerator: i64, denominator: i64) -> Option<i64> {
    numerator
        .checked_add(denominator.checked_sub(1)?)?
        .checked_div(denominator)
}

fn make_range(template: IntRange, min: i64, max: i64) -> Option<IntRange> {
    match template {
        IntRange::U32 { .. } => Some(IntRange::U32 {
            min: u32::try_from(min).ok()?,
            max: u32::try_from(max).ok()?,
        }),
        IntRange::I32 { .. } => Some(IntRange::I32 {
            min: i32::try_from(min).ok()?,
            max: i32::try_from(max).ok()?,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsigned_interval_arithmetic_matches_enumeration() {
        for a in 0..=6u32 {
            for b in a..=6 {
                for c in 0..=6u32 {
                    for d in c..=6 {
                        let left = IntRange::U32 { min: a, max: b };
                        let right = IntRange::U32 { min: c, max: d };
                        let sums: Vec<_> =
                            (a..=b).flat_map(|x| (c..=d).map(move |y| x + y)).collect();
                        assert_eq!(
                            left.add(right),
                            Some(IntRange::U32 {
                                min: *sums.iter().min().unwrap(),
                                max: *sums.iter().max().unwrap(),
                            })
                        );
                        let products: Vec<_> =
                            (a..=b).flat_map(|x| (c..=d).map(move |y| x * y)).collect();
                        assert_eq!(
                            left.mul(right),
                            Some(IntRange::U32 {
                                min: *products.iter().min().unwrap(),
                                max: *products.iter().max().unwrap(),
                            })
                        );
                        let differences: Vec<_> = (a..=b)
                            .flat_map(|x| (c..=d).filter_map(move |y| x.checked_sub(y)))
                            .collect();
                        if a >= d {
                            assert_eq!(
                                left.sub(right),
                                Some(IntRange::U32 {
                                    min: *differences.iter().min().unwrap(),
                                    max: *differences.iter().max().unwrap(),
                                })
                            );
                        } else {
                            assert_eq!(left.sub(right), None);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn signed_interval_arithmetic_matches_enumeration() {
        for a in -4..=4i32 {
            for b in a..=4 {
                for c in -4..=4i32 {
                    for d in c..=4 {
                        let left = IntRange::I32 { min: a, max: b };
                        let right = IntRange::I32 { min: c, max: d };
                        let products: Vec<_> =
                            (a..=b).flat_map(|x| (c..=d).map(move |y| x * y)).collect();
                        assert_eq!(
                            left.mul(right),
                            Some(IntRange::I32 {
                                min: *products.iter().min().unwrap(),
                                max: *products.iter().max().unwrap(),
                            })
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn interval_overflow_and_underflow_are_rejected() {
        assert_eq!(
            IntRange::U32 {
                min: u32::MAX,
                max: u32::MAX,
            }
            .add(IntRange::U32 { min: 1, max: 1 }),
            None
        );
        assert_eq!(
            IntRange::U32 { min: 0, max: 1 }.sub(IntRange::U32 { min: 1, max: 1 }),
            None
        );
        assert_eq!(
            IntRange::I32 {
                min: i32::MAX,
                max: i32::MAX,
            }
            .mul(IntRange::I32 { min: 2, max: 2 }),
            None
        );
    }

    #[test]
    fn recurrence_requires_reachable_exit_and_no_wrap() {
        let template = IntRange::U32 { min: 2, max: 2 };
        assert_eq!(
            recurrence_ranges(0, 2, BinaryOperator::Equal, 8, template),
            Some((
                IntRange::U32 { min: 0, max: 6 },
                IntRange::U32 { min: 8, max: 8 }
            ))
        );
        assert_eq!(
            recurrence_ranges(0, 2, BinaryOperator::Equal, 7, template),
            None
        );
        assert_eq!(
            recurrence_ranges(
                i64::from(u32::MAX) - 1,
                2,
                BinaryOperator::Equal,
                i64::from(u32::MAX) + 1,
                template,
            ),
            None
        );
    }

    // ===== T5 Part A — property sweeps (TASKS/T5-FUZZ-EXPECTATION-HARNESS.md) =====
    // Boundary-centered interval universe: values near 0, 1, len-1-ish, MAX-1, MAX
    // (u32) and MIN+1, -1, 0, 1, MAX-1, MAX (i32). Coverage of each sweep is the
    // full cartesian product of intervals over this universe — exhaustive, not
    // random; the domain is small enough that this is strictly stronger than
    // sampled fuzzing for these invariants.

    const U_UNIVERSE: [u32; 8] = [0, 1, 2, 7, 100, u32::MAX - 1, u32::MAX, u32::MAX];
    const I_UNIVERSE: [i32; 8] = [i32::MIN, i32::MIN + 1, -1, 0, 1, 7, i32::MAX - 1, i32::MAX];

    fn u_intervals() -> Vec<IntRange> {
        let mut v = Vec::new();
        for &a in &U_UNIVERSE {
            for &b in &U_UNIVERSE {
                if a <= b {
                    v.push(IntRange::U32 { min: a, max: b });
                }
            }
        }
        v
    }

    fn i_intervals() -> Vec<IntRange> {
        let mut v = Vec::new();
        for &a in &I_UNIVERSE {
            for &b in &I_UNIVERSE {
                if a <= b {
                    v.push(IntRange::I32 { min: a, max: b });
                }
            }
        }
        v
    }

    /// P1 (u32): add/sub/mul, when Some, exactly equal the concrete enumeration;
    /// when any endpoint op would overflow, the result is None. `sub` may be None
    /// even without underflow (min > max would invert) — accepted as conservative.
    #[test]
    fn property_u32_arithmetic_sound_and_tight() {
        for left in u_intervals() {
            for right in u_intervals() {
                let (a, b) = (left, right);
                // add
                match a.add(b) {
                    Some(r) => {
                        // endpoints must be exactly the corner-derived bounds
                        // (add is monotone: lo = amin+bmin, hi = amax+bmax)
                        let lo = u64::from(a_min(a)) + u64::from(a_min(b));
                        let hi = u64::from(a_max(a)) + u64::from(a_max(b));
                        assert!(hi <= u64::from(u32::MAX), "Some(add) must not overflow");
                        assert_eq!(u64::from(a_min_of(r)), lo);
                        assert_eq!(u64::from(a_max_of(r)), hi);
                    }
                    None => {
                        // must be justified: at least one endpoint pair overflows
                        let any_overflow =
                            u64::from(a_max(a)) + u64::from(a_max(b)) > u64::from(u32::MAX);
                        assert!(any_overflow, "None(add) must be overflow-justified");
                    }
                }
                // mul (endpoints only: 4-corner rule is exact for monotone +- mul on
                // nonneg domains, and u32 is nonneg)
                match a.mul(b) {
                    Some(r) => {
                        // nonneg domain: mul is monotone in each arg, so the
                        // 4-corner rule gives exact bounds
                        let corners = [
                            u64::from(a_min(a)) * u64::from(a_min(b)),
                            u64::from(a_min(a)) * u64::from(a_max(b)),
                            u64::from(a_max(a)) * u64::from(a_min(b)),
                            u64::from(a_max(a)) * u64::from(a_max(b)),
                        ];
                        let lo = *corners.iter().min().unwrap();
                        let hi = *corners.iter().max().unwrap();
                        assert!(hi <= u64::from(u32::MAX), "Some(mul) must not overflow");
                        assert_eq!(u64::from(a_min_of(r)), lo);
                        assert_eq!(u64::from(a_max_of(r)), hi);
                    }
                    None => {
                        let any_overflow =
                            u64::from(a_max(a)) * u64::from(a_max(b)) > u64::from(u32::MAX);
                        assert!(any_overflow, "None(mul) must be overflow-justified");
                    }
                }
                // sub: containment soundness via corner bounds (sub is
                // monotone-decreasing in rhs: lo = amin-bmax, hi = amax-bmin),
                // plus spot enumeration only on small spans
                if let Some(r) = a.sub(b) {
                    let lo = u64::from(a_min(a)) - u64::from(a_max(b));
                    let hi = u64::from(a_max(a)) - u64::from(a_min(b));
                    assert_eq!(u64::from(a_min_of(r)), lo);
                    assert_eq!(u64::from(a_max_of(r)), hi);
                    // enumerate only when BOTH spans are small (a zero-span
                    // operand makes the product guard useless: span*0 == 0)
                    let span_a = u64::from(a_max(a)) - u64::from(a_min(a));
                    let span_b = u64::from(a_max(b)) - u64::from(a_min(b));
                    if span_a <= 64 && span_b <= 64 {
                        for x in a_min(a)..=a_max(a) {
                            for y in a_min(b)..=a_max(b) {
                                if let Some(d) = x.checked_sub(y) {
                                    assert!(d >= a_min_of(r) && d <= a_max_of(r));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// P1/P2 (i32): same for the signed domain; join/sub properties.
    #[test]
    fn property_i32_arithmetic_sound_and_tight() {
        for left in i_intervals() {
            for right in i_intervals() {
                let (a, b) = (left, right);
                match a.mul(b) {
                    Some(r) => {
                        // 4-corner rule is exact for i32 mul when no overflow
                        let corners = [
                            i64::from(i_min(a)) * i64::from(i_min(b)),
                            i64::from(i_min(a)) * i64::from(i_max(b)),
                            i64::from(i_max(a)) * i64::from(i_min(b)),
                            i64::from(i_max(a)) * i64::from(i_max(b)),
                        ];
                        let lo = *corners.iter().min().unwrap();
                        let hi = *corners.iter().max().unwrap();
                        assert!(
                            hi <= i64::from(i32::MAX) && lo >= i64::from(i32::MIN),
                            "Some(mul) must not overflow"
                        );
                        assert_eq!(i64::from(i_min_of(r)), lo);
                        assert_eq!(i64::from(i_max_of(r)), hi);
                    }
                    None => {
                        let corners = [
                            i64::from(i_min(a)) * i64::from(i_min(b)),
                            i64::from(i_min(a)) * i64::from(i_max(b)),
                            i64::from(i_max(a)) * i64::from(i_min(b)),
                            i64::from(i_max(a)) * i64::from(i_max(b)),
                        ];
                        let any_overflow = corners
                            .iter()
                            .any(|&c| c > i64::from(i32::MAX) || c < i64::from(i32::MIN));
                        assert!(any_overflow, "None(mul) must be overflow-justified");
                    }
                }
                // join: same-kind join never shrinks
                if let Some(j) = a.join(b) {
                    assert!(i_min_of(j) <= i_min(a) && i_max_of(j) >= i_max(a));
                    assert!(i_min_of(j) <= i_min(b) && i_max_of(j) >= i_max(b));
                }
            }
        }
    }

    /// P2: join never shrinks (u32) — including that join with self is self.
    #[test]
    fn property_join_never_shrinks_u32() {
        for a in u_intervals() {
            for b in u_intervals() {
                let j = a.join(b).expect("same-kind u32 join is always Some");
                assert!(a_min_of(j) <= a_min_of(a) && a_max_of(j) >= a_max_of(a));
                assert!(a_min_of(j) <= a_min_of(b) && a_max_of(j) >= a_max_of(b));
            }
        }
    }

    /// P3: refinement never widens and never inverts; cross-kind refine rejected.
    #[test]
    fn property_refinement_never_widens() {
        for r in u_intervals().into_iter().chain(i_intervals()) {
            for &bound in &[0i64, 1, 7, 100] {
                for op in [
                    BinaryOperator::Less,
                    BinaryOperator::LessEqual,
                    BinaryOperator::Greater,
                    BinaryOperator::GreaterEqual,
                    BinaryOperator::Equal,
                ] {
                    if let Some(refined) = refine_range(r, op, bound) {
                        match (r, refined) {
                            (
                                IntRange::U32 { min, max },
                                IntRange::U32 {
                                    min: rmin,
                                    max: rmax,
                                },
                            ) => {
                                assert!(rmin >= min && rmax <= max, "refine widened u32");
                                assert!(rmin <= rmax, "refine inverted u32");
                            }
                            (
                                IntRange::I32 { min, max },
                                IntRange::I32 {
                                    min: rmin,
                                    max: rmax,
                                },
                            ) => {
                                assert!(rmin >= min && rmax <= max, "refine widened i32");
                                assert!(rmin <= rmax, "refine inverted i32");
                            }
                            _ => panic!("refine_range changed interval kind"),
                        }
                    }
                }
            }
        }
    }

    /// P4: fits() soundness — the elision predicate itself. For every interval
    /// where fits(len) is true, EVERY concrete value must be a valid index.
    #[test]
    fn property_fits_soundness() {
        let lens = [1u32, 2, 8, 100];
        for r in u_intervals() {
            for &len in &lens {
                if r.fits(len) {
                    for x in a_min(r)..=a_max(r) {
                        assert!(x < len, "fits() admitted OOB index {x} for len {len}");
                    }
                }
            }
        }
        for r in i_intervals() {
            for &len in &lens {
                if r.fits(len) {
                    for x in i_min(r)..=i_max(r) {
                        assert!(
                            x >= 0 && (x as i64) < i64::from(len),
                            "fits() admitted OOB index {x} for len {len}"
                        );
                    }
                }
            }
        }
    }

    // helpers to read endpoints without pattern-matching at every site
    fn a_min(r: IntRange) -> u32 {
        match r {
            IntRange::U32 { min, .. } => min,
            _ => unreachable!("u32 helper used on i32"),
        }
    }
    fn a_max(r: IntRange) -> u32 {
        match r {
            IntRange::U32 { max, .. } => max,
            _ => unreachable!("u32 helper used on i32"),
        }
    }
    fn a_min_of(r: IntRange) -> u32 {
        a_min(r)
    }
    fn a_max_of(r: IntRange) -> u32 {
        a_max(r)
    }
    fn i_min(r: IntRange) -> i32 {
        match r {
            IntRange::I32 { min, .. } => min,
            _ => unreachable!("i32 helper used on u32"),
        }
    }
    fn i_max(r: IntRange) -> i32 {
        match r {
            IntRange::I32 { max, .. } => max,
            _ => unreachable!("i32 helper used on u32"),
        }
    }
    fn i_min_of(r: IntRange) -> i32 {
        i_min(r)
    }
    fn i_max_of(r: IntRange) -> i32 {
        i_max(r)
    }
}
