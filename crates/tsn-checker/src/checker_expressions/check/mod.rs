mod calls;
mod contextual;
mod exhaustiveness;
mod members;

use super::helpers::{base_type, op_str};
use crate::binder::BindResult;
use crate::checker::Checker;
use crate::intrinsic::resolve_intrinsic;
use crate::types::Type;
use tsn_core::ast::operators::BinaryOp;
use tsn_core::ast::Expr;
use tsn_core::{Diagnostic, TypeKind};

impl Checker {
    pub(crate) fn check_expr(&mut self, expr: &Expr, bind: &BindResult) {
        self.check_expr_no_record(expr, bind);
        let start = expr.range().start.offset;
        let end = expr.range().end.offset.saturating_sub(1);
        let ty = self.infer_type(expr, bind);

        let mut symbol_id = None;
        if let Expr::Identifier { name, .. } = expr {
            let scope = bind.scopes.get(self.current_scope);
            symbol_id = scope.resolve(name, &bind.scopes);
        }

        self.expr_types.insert(
            start,
            crate::checker::ExprInfo {
                ty: ty.clone(),
                symbol_id,
            },
        );
        self.expr_types
            .insert(end, crate::checker::ExprInfo { ty, symbol_id });
    }

    fn check_expr_no_record(&mut self, expr: &Expr, bind: &BindResult) {
        match expr {
            Expr::Arrow {
                params,
                return_type,
                body,
                range,
                ..
            } => {
                let saved_expected = self.expected_return_type.take();
                // Explicit annotation wins; fall back to expected fn return type from context.
                self.expected_return_type = return_type
                    .as_ref()
                    .map(|rt| self.resolve_type_node_cached(rt, bind))
                    .or_else(|| self.expected_return_from_fn_type());

                let saved_scope = self.current_scope;
                if let Some(fn_scope) = self.next_child_scope(bind) {
                    self.current_scope = fn_scope;
                }

                // Inject expected param types for untyped params when context provides a Fn type.
                if let Some(expected_fn) = self.expected_fn_type() {
                    self.apply_contextual_arrow_params(params, &expected_fn, bind);
                }

                match body.as_ref() {
                    tsn_core::ast::ArrowBody::Block(stmt) => self.check_stmt(stmt, bind),
                    tsn_core::ast::ArrowBody::Expr(e) => {
                        self.check_expr(e, bind);
                        let actual = self.infer_type(e, bind);
                        if let Some(expected) = self.expected_return_type.clone() {
                            if !expected.is_dynamic()
                                && !actual.is_dynamic()
                                && !self.types_compatible_cached(&expected, &actual, Some(bind))
                            {
                                self.diagnostics.push(Diagnostic::error(
                                    format!(
                                        "type mismatch: arrow function is declared to return '{}', but returns '{}'",
                                        expected, actual
                                    ),
                                    range.clone(),
                                ));
                            }
                        }
                    }
                }

                self.current_scope = saved_scope;
                self.expected_return_type = saved_expected;
            }
            Expr::Function {
                return_type, body, ..
            } => {
                let saved_expected = self.expected_return_type.take();
                self.expected_return_type = return_type
                    .as_ref()
                    .map(|rt| self.resolve_type_node_cached(rt, bind));

                let saved_scope = self.current_scope;
                if let Some(fn_scope) = self.next_child_scope(bind) {
                    self.current_scope = fn_scope;
                }

                self.check_stmt(body, bind);

                self.current_scope = saved_scope;
                self.expected_return_type = saved_expected;
            }
            Expr::As { expression, .. } => self.check_expr(expression, bind),
            Expr::Satisfies {
                expression,
                type_ann,
                range,
            } => {
                self.check_expr(expression, bind);
                let declared_ty = self.resolve_type_node_cached(type_ann, bind);
                let inferred_ty = self.infer_type(expression, bind);
                if !declared_ty.is_dynamic()
                    && !inferred_ty.is_dynamic()
                    && !self.types_compatible_cached(&declared_ty, &inferred_ty, Some(bind))
                {
                    self.diagnostics.push(Diagnostic::error(
                        format!(
                            "expression does not satisfy '{}': got '{}'",
                            declared_ty, inferred_ty
                        ),
                        range.clone(),
                    ));
                }
            }
            Expr::Await {
                argument, range, ..
            } => {
                self.check_expr(argument, bind);
                let arg_ty = self.infer_type(argument, bind);
                if !arg_ty.is_dynamic()
                    && !matches!(&arg_ty.0, TypeKind::Generic(n, _, _) if n == tsn_core::well_known::FUTURE)
                    && !matches!(&arg_ty.0, TypeKind::Void)
                {
                    self.diagnostics.push(Diagnostic::warning(
                        format!(
                            "'await' applied to non-Future type '{}' has no effect",
                            arg_ty
                        ),
                        range.clone(),
                    ));
                }
            }
            Expr::Yield { argument, .. } => {
                if let Some(arg) = argument {
                    self.check_expr(arg, bind);
                }
            }
            Expr::Unary { operand, .. } => self.check_expr(operand, bind),
            Expr::Binary {
                left,
                right,
                op,
                range,
            } => {
                self.check_expr(left, bind);
                self.check_expr(right, bind);
                let l_ty = self.infer_type(left, bind);
                let r_ty = self.infer_type(right, bind);

                let l_base = base_type(&l_ty);
                let r_base = base_type(&r_ty);
                if !l_base.is_dynamic() && !r_base.is_dynamic() {
                    let is_decimal = |t: &Type| matches!(&t.0, tsn_core::TypeKind::Named(n, _origin) if n == tsn_core::well_known::DECIMAL);
                    let valid = match op {
                        BinaryOp::Add => {
                            ((l_base == &Type::Int || l_base == &Type::Float || is_decimal(l_base))
                                && (r_base == &Type::Int
                                    || r_base == &Type::Float
                                    || is_decimal(r_base)))
                                || l_base == &Type::Str
                                || r_base == &Type::Str
                        }
                        BinaryOp::Sub
                        | BinaryOp::Mul
                        | BinaryOp::Div
                        | BinaryOp::Mod
                        | BinaryOp::Pow => {
                            (l_base == &Type::Int || l_base == &Type::Float || is_decimal(l_base))
                                && (r_base == &Type::Int
                                    || r_base == &Type::Float
                                    || is_decimal(r_base))
                        }
                        BinaryOp::BitAnd
                        | BinaryOp::BitOr
                        | BinaryOp::BitXor
                        | BinaryOp::Shl
                        | BinaryOp::Shr
                        | BinaryOp::UShr => l_base == &Type::Int && r_base == &Type::Int,
                        BinaryOp::Lt | BinaryOp::Gt | BinaryOp::LtEq | BinaryOp::GtEq => {
                            ((l_base == &Type::Int || l_base == &Type::Float || is_decimal(l_base))
                                && (r_base == &Type::Int
                                    || r_base == &Type::Float
                                    || is_decimal(r_base)))
                                || (l_base == &Type::Str && r_base == &Type::Str)
                        }
                        _ => true,
                    };
                    if !valid {
                        self.diagnostics.push(Diagnostic::error(
                            format!(
                                "invalid binary operation '{}' between '{}' and '{}'",
                                op_str(op),
                                l_ty,
                                r_ty
                            ),
                            range.clone(),
                        ));
                    }
                }
            }
            Expr::Logical { left, right, .. } => {
                self.check_expr(left, bind);
                self.check_expr(right, bind);
            }
            Expr::Assign {
                target,
                value,
                range,
                ..
            } => {
                let prev = self.is_assignment_target;
                self.is_assignment_target = true;
                self.check_expr(target, bind);
                self.is_assignment_target = prev;
                self.check_expr(value, bind);

                self.check_extension_assignment(target, bind);

                if let Expr::Identifier { name, .. } = target.as_ref() {
                    let scope = bind.scopes.get(self.current_scope);
                    if let Some(id) = scope.resolve(name, &bind.scopes) {
                        let sym = bind.arena.get(id);
                        if sym.kind == crate::symbol::SymbolKind::Const {
                            self.diagnostics.push(Diagnostic::error(
                                format!("cannot reassign to constant '{}'", name),
                                range.clone(),
                            ));
                        }
                    }
                }

                let target_ty = self.infer_type(target, bind);
                let value_ty = self.infer_type(value, bind);
                if !target_ty.is_dynamic()
                    && !value_ty.is_dynamic()
                    && !self.types_compatible_cached(&target_ty, &value_ty, Some(bind))
                {
                    self.diagnostics.push(Diagnostic::error(
                        format!(
                            "type mismatch: cannot assign '{}' to '{}'",
                            value_ty, target_ty
                        ),
                        range.clone(),
                    ));
                }
            }
            Expr::Call {
                callee,
                args,
                type_args,
                range,
                ..
            } => self.check_call_expr(callee, args, type_args, range, bind),
            Expr::New {
                callee,
                args,
                range,
                ..
            } => {
                if let Expr::Identifier { name: cls_name, .. } = callee.as_ref() {
                    if self.abstract_classes.contains(cls_name) {
                        self.diagnostics.push(Diagnostic::error(
                            format!("cannot instantiate abstract class '{}'", cls_name),
                            range.clone(),
                        ));
                    }
                }
                self.check_expr(callee, bind);
                for arg in args {
                    match arg {
                        tsn_core::ast::Arg::Positional(e) => self.check_expr(e, bind),
                        tsn_core::ast::Arg::Named { value, .. } => self.check_expr(value, bind),
                        tsn_core::ast::Arg::Spread(e) => self.check_expr(e, bind),
                    }
                }
            }

            Expr::Conditional {
                test,
                consequent,
                alternate,
                ..
            } => {
                self.check_expr(test, bind);
                self.check_expr(consequent, bind);
                self.check_expr(alternate, bind);
            }

            Expr::Member {
                object,
                property,
                computed,
                optional,
                range,
                ..
            } => self.check_member_expr(expr, object, property, *computed, *optional, range, bind),

            Expr::Paren { expression, .. } => self.check_expr(expression, bind),
            Expr::NonNull { expression, .. } => self.check_expr(expression, bind),

            Expr::Array { elements, .. } => self.check_array_with_context(elements, bind),

            Expr::Object { properties, .. } => self.check_object_with_context(properties, bind),

            Expr::Template { parts, .. } => {
                for p in parts {
                    if let tsn_core::ast::TemplatePart::Interpolation(e) = p {
                        self.check_expr(e, bind);
                    }
                }
            }

            Expr::Sequence { expressions, .. } => {
                for e in expressions {
                    self.check_expr(e, bind);
                }
            }

            Expr::ClassExpr { declaration, .. } => {
                let _ = declaration;
            }

            Expr::Match {
                subject,
                cases,
                range,
                ..
            } => {
                self.check_expr(subject, bind);
                for case in cases {
                    let saved_scope = self.current_scope;
                    if let Some(arm_scope) = self.next_child_scope(bind) {
                        self.current_scope = arm_scope;
                    }

                    if let Some(g) = &case.guard {
                        self.check_expr(g, bind);
                    }
                    match &case.body {
                        tsn_core::ast::MatchBody::Expr(e) => self.check_expr(e, bind),
                        tsn_core::ast::MatchBody::Block(stmt) => self.check_stmt(stmt, bind),
                    }
                    self.current_scope = saved_scope;
                }
                let subject_ty = self.infer_type(subject, bind);
                self.check_match_exhaustiveness(&subject_ty, cases, range, bind);
            }

            Expr::Update { operand, .. } => self.check_expr(operand, bind),
            Expr::Spread { argument, .. } => self.check_expr(argument, bind),

            Expr::Pipeline { left, right, .. } => {
                self.check_expr(left, bind);
                let saved_pipeline = self.in_pipeline_rhs;
                self.in_pipeline_rhs = true;
                self.check_expr(right, bind);
                self.in_pipeline_rhs = saved_pipeline;
            }

            Expr::Range { start, end, .. } => {
                self.check_expr(start, bind);
                self.check_expr(end, bind);
            }

            Expr::TaggedTemplate { tag, template, .. } => {
                self.check_expr(tag, bind);
                self.check_expr(template, bind);
            }

            Expr::Identifier { name, range, .. } => {
                if name == "_" {
                    if !self.is_assignment_target && !self.in_pipeline_rhs {
                        self.diagnostics.push(Diagnostic::error(
                            "cannot use '_' as a value; '_' is the discard placeholder".to_string(),
                            range.clone(),
                        ));
                    }
                    return;
                }

                if resolve_intrinsic(name).is_some() {
                    return;
                }

                let scope = bind.scopes.get(self.current_scope);
                if scope.resolve(name, &bind.scopes).is_none() && !self.is_assignment_target {
                    self.diagnostics.push(Diagnostic::error(
                        format!("undefined variable: {}", name),
                        range.clone(),
                    ));
                }
            }

            Expr::IntLiteral { .. }
            | Expr::FloatLiteral { .. }
            | Expr::BigIntLiteral { .. }
            | Expr::DecimalLiteral { .. }
            | Expr::StrLiteral { .. }
            | Expr::CharLiteral { .. }
            | Expr::BoolLiteral { .. }
            | Expr::RegexLiteral { .. }
            | Expr::NullLiteral { .. }
            | Expr::Super { .. }
            | Expr::This { .. } => {}
        }
    }
}
