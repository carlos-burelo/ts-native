mod async_members;

use std::collections::HashMap;

use crate::binder::BindResult;
use crate::checker::{Checker, ExprInfo};
use crate::checker_generics::{build_call_mapping, build_generic_mapping};
use crate::types::{FunctionParam, FunctionType, ObjectTypeMember, Type};
use rustc_hash::FxHashMap;
use tsn_core::ast::{Expr, Stmt};
use tsn_core::{Diagnostic, TypeKind};

use self::async_members::is_async_member;
use super::helpers::base_type;

impl Checker {
    pub(crate) fn infer_type(&mut self, expr: &Expr, bind: &BindResult) -> Type {
        let expr_id = expr as *const Expr as usize;
        let key = (expr_id, self.current_scope, self.infer_env_rev);
        if let Some(cached) = self.infer_cache.get(&key) {
            return cached.clone();
        }

        if is_atomic_expr(expr) {
            let start = expr.range().start.offset;
            if let Some(info) = self.expr_types.get(&start) {
                return info.ty.clone();
            }
        }

        if let Expr::NonNull { expression, .. } = expr {
            return self.infer_type(expression, bind).non_nullified();
        }

        let ty = self.infer_type_impl(expr, bind);
        let is_opt_call = matches!(
            expr,
            Expr::Call {
                callee,
                optional: false,
                ..
            } if matches!(callee.as_ref(), Expr::Member { optional: true, .. })
        );

        let resolved = match expr {
            Expr::Member { optional: true, .. } => Type::make_nullable(ty),
            Expr::Call { optional: true, .. } => Type::make_nullable(ty),
            _ if is_opt_call => Type::make_nullable(ty),
            _ => ty,
        };
        self.infer_cache.insert(key, resolved.clone());
        resolved
    }

    fn infer_type_impl(&mut self, expr: &Expr, bind: &BindResult) -> Type {
        match expr {
            Expr::Identifier { name, .. } => {
                let scope = bind.scopes.get(self.current_scope);
                let opt_id = scope.resolve(name, &bind.scopes);
                if let Some(id) = opt_id {
                    if let Some(narrowed_stack) = self.narrowed_types.get(&id) {
                        if let Some(narrowed_ty) = narrowed_stack.last() {
                            return narrowed_ty.clone();
                        }
                    }

                    // Prefer checker-resolved type (has generic substitution) over binder's raw type
                    if let Some(inferred) = self.var_types.get(&id).cloned() {
                        return inferred;
                    }

                    let ty_opt = &bind.arena.get(id).ty;
                    if let Some(ty) = ty_opt {
                        if !ty.is_dynamic() {
                            return ty.clone();
                        }
                    }
                }

                Type::Dynamic
            }

            Expr::This { .. } => {
                if let Some(cn) = &self.current_class {
                    return Type::named(cn.clone());
                }
                Type::Dynamic
            }

            Expr::New { callee, .. } => {
                if let Expr::Identifier { name, .. } = callee.as_ref() {
                    return Type::named(name.clone());
                }
                crate::binder::infer_expr_type(expr, Some(bind))
            }

            Expr::Call {
                callee,
                type_args,
                args,
                ..
            } => {
                let callee_ty = self.infer_type(callee, bind).non_nullified();
                if let TypeKind::Fn(ft) = &callee_ty.0 {
                    let mapping = build_call_mapping(callee, type_args, args, ft, self, bind);
                    let ret = if mapping.is_empty() {
                        *ft.return_type.clone()
                    } else {
                        ft.return_type.substitute(&mapping)
                    };

                    let ret = if matches!(ret.0, TypeKind::This) {
                        if let Expr::Member { object, .. } = callee.as_ref() {
                            let receiver_ty = self.infer_type(object, bind);
                            if !receiver_ty.is_dynamic() {
                                receiver_ty
                            } else {
                                ret
                            }
                        } else {
                            ret
                        }
                    } else {
                        ret
                    };

                    let is_async_callee = if let Expr::Identifier { name, .. } = callee.as_ref() {
                        let scope = bind.scopes.get(self.current_scope);
                        scope
                            .resolve(name, &bind.scopes)
                            .map(|id| bind.arena.get(id).is_async)
                            .unwrap_or(false)
                    } else if self
                        .extension_calls
                        .contains_key(&expr.range().start.offset)
                    {
                        self.extension_calls
                            .get(&expr.range().start.offset)
                            .and_then(|mangled| {
                                let scope = bind.scopes.get(bind.global_scope);
                                scope.resolve(mangled, &bind.scopes)
                            })
                            .map(|id| bind.arena.get(id).is_async)
                            .unwrap_or(false)
                    } else {
                        false
                    };

                    if is_async_callee
                        && !matches!(&ret.0, TypeKind::Generic(n, _, _) if n == tsn_core::well_known::FUTURE)
                        && !ret.is_dynamic()
                        && ret != Type::Void
                    {
                        Type::generic(tsn_core::well_known::FUTURE.to_owned(), vec![ret])
                    } else {
                        ret
                    }
                } else {
                    Type::Dynamic
                }
            }

            Expr::Conditional {
                consequent,
                alternate,
                ..
            } => {
                let t_ty = self.infer_type(consequent, bind);
                let f_ty = self.infer_type(alternate, bind);
                if t_ty == f_ty {
                    t_ty
                } else if t_ty.is_dynamic() {
                    f_ty
                } else if f_ty.is_dynamic() {
                    t_ty
                } else {
                    Type::union(vec![t_ty, f_ty])
                }
            }

            Expr::Member {
                object,
                property,
                computed: false,
                ..
            } => infer_member_type(self, expr, object, property, bind),

            Expr::Member {
                object,
                property,
                computed: true,
                ..
            } => {
                let obj_ty = self.infer_type(object, bind);
                let idx_ty = self.infer_type(property, bind);
                match &obj_ty.0 {
                    tsn_core::TypeKind::Array(elem) => *elem.clone(),
                    tsn_core::TypeKind::Object(members) => {
                        for member in members {
                            if let ObjectTypeMember::Index {
                                key_ty, value_ty, ..
                            } = member
                            {
                                if self.types_compatible_cached(key_ty, &idx_ty, None) {
                                    return Type::make_nullable((**value_ty).clone());
                                }
                            }
                        }
                        Type::Dynamic
                    }
                    _ => crate::binder::infer_expr_type(expr, Some(bind)),
                }
            }

            Expr::Arrow {
                params,
                return_type,
                body,
                ..
            } => {
                if return_type.is_some() {
                    return crate::binder::infer_expr_type(expr, Some(bind));
                }

                let ret_ty = match body.as_ref() {
                    tsn_core::ast::ArrowBody::Expr(e) => self
                        .expr_types
                        .get(&e.range().start.offset)
                        .map(|info| info.ty.clone())
                        .unwrap_or(Type::Dynamic),
                    tsn_core::ast::ArrowBody::Block(block) => {
                        let return_tys = collect_checked_return_types(block, &self.expr_types);
                        match return_tys.len() {
                            0 => Type::Void,
                            1 => return_tys.into_iter().next().unwrap(),
                            _ => Type::union(return_tys),
                        }
                    }
                };
                let ps = params
                    .iter()
                    .map(|p| {
                        let name = crate::binder::pattern_lead_name(&p.pattern).to_owned();
                        let mut ty = p
                            .type_ann
                            .as_ref()
                            .or_else(|| match &p.pattern {
                                tsn_core::ast::Pattern::Identifier { type_ann, .. } => {
                                    type_ann.as_ref()
                                }
                                _ => None,
                            })
                            .map(|m| self.resolve_type_node_cached(m, bind))
                            .unwrap_or_else(|| {
                                if self.warn_implicit_dynamic && !name.is_empty() && name != "_" {
                                    self.diagnostics.push(Diagnostic::hint(
                                        format!("el parámetro '{name}' no tiene anotación de tipo — se asumió 'dynamic'"),
                                        p.pattern.range().clone(),
                                    ));
                                }
                                Type::Dynamic
                            });
                        if p.is_rest && !matches!(ty.0, tsn_core::TypeKind::Array(_)) {
                            ty = Type::array(ty);
                        }
                        FunctionParam {
                            name: Some(name),
                            ty,
                            optional: p.is_optional,
                            is_rest: p.is_rest,
                        }
                    })
                    .collect();
                Type::fn_(FunctionType {
                    params: ps,
                    return_type: Box::new(ret_ty),
                    is_arrow: true,
                    type_params: vec![],
                })
            }

            Expr::Satisfies { expression, .. } => self.infer_type(expression, bind),

            Expr::Await { argument, .. } => {
                let inner = self.infer_type(argument, bind);
                match &inner.0 {
                    TypeKind::Generic(name, args, _origin)
                        if name == tsn_core::well_known::FUTURE && args.len() == 1 =>
                    {
                        args[0].clone()
                    }
                    _ => inner,
                }
            }

            Expr::Binary {
                op, left, right, ..
            } => infer_binary_type(self, op, left, right, bind),

            Expr::Unary { op, operand, .. } => {
                use tsn_core::ast::operators::UnaryOp;
                let inner = self.infer_type(operand, bind);
                match op {
                    UnaryOp::Not => Type::Bool,
                    UnaryOp::Minus | UnaryOp::Plus => inner,
                    _ => Type::Dynamic,
                }
            }

            Expr::Logical {
                op, left, right, ..
            } => {
                use tsn_core::ast::operators::LogicalOp;
                match op {
                    LogicalOp::Nullish => {
                        let r = self.infer_type(right, bind);
                        if r.is_dynamic() {
                            self.infer_type(left, bind)
                        } else {
                            r
                        }
                    }
                    LogicalOp::And | LogicalOp::Or => {
                        let l = self.infer_type(left, bind);
                        let r = self.infer_type(right, bind);
                        if l == r {
                            l
                        } else if l.is_dynamic() {
                            r
                        } else {
                            l
                        }
                    }
                }
            }

            _ => crate::binder::infer_expr_type(expr, Some(bind)),
        }
    }
}

/// Walk a block statement and collect the inferred types of all `return` expressions,
/// using the already-checked `expr_types` map. Does NOT descend into nested function bodies.
fn collect_checked_return_types(stmt: &Stmt, expr_types: &FxHashMap<u32, ExprInfo>) -> Vec<Type> {
    let mut out = Vec::new();
    collect_returns(stmt, expr_types, &mut out);
    out
}

fn collect_returns(stmt: &Stmt, expr_types: &FxHashMap<u32, ExprInfo>, out: &mut Vec<Type>) {
    match stmt {
        Stmt::Block { stmts, .. } => {
            for s in stmts {
                collect_returns(s, expr_types, out);
            }
        }
        Stmt::Return {
            argument: Some(e), ..
        } => {
            let offset = e.range().start.offset;
            if let Some(info) = expr_types.get(&offset) {
                if !info.ty.is_dynamic() {
                    out.push(info.ty.clone());
                }
            }
        }
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            collect_returns(consequent, expr_types, out);
            if let Some(alt) = alternate {
                collect_returns(alt, expr_types, out);
            }
        }
        Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
            collect_returns(body, expr_types, out);
        }
        Stmt::For { body, .. } | Stmt::ForIn { body, .. } | Stmt::ForOf { body, .. } => {
            collect_returns(body, expr_types, out);
        }
        Stmt::Try {
            block,
            catch,
            finally,
            ..
        } => {
            collect_returns(block, expr_types, out);
            if let Some(c) = catch {
                collect_returns(c.body.as_ref(), expr_types, out);
            }
            if let Some(f) = finally {
                collect_returns(f, expr_types, out);
            }
        }
        Stmt::Labeled { body, .. } => collect_returns(body, expr_types, out),
        Stmt::Switch { cases, .. } => {
            for case in cases {
                for s in &case.body {
                    collect_returns(s, expr_types, out);
                }
            }
        }
        // Nested Decl::Function / Expr::Arrow have their own return context — skip
        _ => {}
    }
}

fn is_atomic_expr(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Identifier { .. }
            | Expr::This { .. }
            | Expr::Super { .. }
            | Expr::IntLiteral { .. }
            | Expr::FloatLiteral { .. }
            | Expr::BigIntLiteral { .. }
            | Expr::DecimalLiteral { .. }
            | Expr::StrLiteral { .. }
            | Expr::CharLiteral { .. }
            | Expr::BoolLiteral { .. }
            | Expr::RegexLiteral { .. }
            | Expr::NullLiteral { .. }
    )
}

fn infer_member_type(
    checker: &mut Checker,
    expr: &Expr,
    object: &Expr,
    property: &Expr,
    bind: &BindResult,
) -> Type {
    let obj_ty_raw = checker.infer_type(object, bind);
    let obj_ty = obj_ty_raw.non_nullified();
    let obj_ty = if matches!(obj_ty.0, tsn_core::TypeKind::Never) {
        obj_ty_raw
    } else {
        obj_ty
    };

    let Expr::Identifier {
        name: prop_name, ..
    } = property
    else {
        return crate::binder::infer_expr_type(expr, Some(bind));
    };

    let wrap_async_method_type = |ty: Type| {
        if !is_async_member(&obj_ty, prop_name, bind) {
            return ty;
        }
        match &ty.0 {
            TypeKind::Fn(ft)
                if !matches!(&ft.return_type.0, TypeKind::Generic(name, _, _) if name == tsn_core::well_known::FUTURE)
                    && *ft.return_type != Type::Void
                    && !ft.return_type.is_dynamic() =>
            {
                let mut wrapped = ft.clone();
                wrapped.return_type = Box::new(Type::generic(
                    tsn_core::well_known::FUTURE.to_owned(),
                    vec![(*ft.return_type).clone()],
                ));
                Type::fn_(wrapped)
            }
            _ => ty,
        }
    };

    if let Some(m) = checker.find_member(&obj_ty, prop_name, bind) {
        return match m {
            ObjectTypeMember::Property { ty, .. } => ty.clone(),
            ObjectTypeMember::Method {
                params,
                return_type,
                is_arrow,
                ..
            } => wrap_async_method_type(Type::fn_(FunctionType {
                params: params.clone(),
                return_type: return_type.clone(),
                is_arrow,
                type_params: vec![],
            })),
            _ => Type::Dynamic,
        };
    }

    match &obj_ty.0 {
        TypeKind::Named(class_name, _origin) | TypeKind::Generic(class_name, _, _origin) => {
            let mapping = if let TypeKind::Generic(_, type_args, _orig) = &obj_ty.0 {
                build_generic_mapping(class_name, type_args, checker, bind)
            } else {
                HashMap::new()
            };

            let m_ty = checker.find_member_type(&obj_ty, prop_name, bind);
            if !m_ty.is_dynamic() {
                if mapping.is_empty() {
                    return wrap_async_method_type(m_ty);
                }
                return wrap_async_method_type(m_ty.substitute(&mapping));
            }
        }

        TypeKind::Array(elem) => {
            if prop_name == "length" {
                return Type::Int;
            }
            // flat() unwraps one level of nesting: int[][] → flat() → int[]
            // Standard T-substitution would give (int[])[] = int[][] (wrong).
            // Use the inner element type of T so T[] resolves correctly.
            let effective_elem = if prop_name == "flat" {
                match &elem.0 {
                    tsn_core::TypeKind::Array(inner) => *inner.clone(),
                    _ => *elem.clone(),
                }
            } else {
                *elem.clone()
            };
            let mapping = build_generic_mapping(
                tsn_core::well_known::ARRAY,
                &[effective_elem],
                checker,
                bind,
            );
            let m_ty = checker.find_member_type(&obj_ty, prop_name, bind);
            if !m_ty.is_dynamic() {
                return wrap_async_method_type(m_ty.substitute(&mapping));
            }
        }
        _ => {
            let m_ty = checker.find_member_type(&obj_ty, prop_name, bind);
            if !m_ty.is_dynamic() {
                return wrap_async_method_type(m_ty);
            }
        }
    }

    crate::binder::infer_expr_type(expr, Some(bind))
}

fn infer_binary_type(
    checker: &mut Checker,
    op: &tsn_core::ast::operators::BinaryOp,
    left: &Expr,
    right: &Expr,
    bind: &BindResult,
) -> Type {
    use tsn_core::ast::operators::BinaryOp;

    match op {
        BinaryOp::Eq
        | BinaryOp::NotEq
        | BinaryOp::Lt
        | BinaryOp::Gt
        | BinaryOp::LtEq
        | BinaryOp::GtEq
        | BinaryOp::Instanceof
        | BinaryOp::In => Type::Bool,
        _ => {
            let l = base_type(&checker.infer_type(left, bind)).clone();
            let r = base_type(&checker.infer_type(right, bind)).clone();
            match op {
                BinaryOp::Add => match (&l.0, &r.0) {
                    (TypeKind::Str, _) | (_, TypeKind::Str) => Type::Str,
                    (TypeKind::Decimal, _) | (_, TypeKind::Decimal) => Type::Decimal,
                    (TypeKind::Float, _) | (_, TypeKind::Float) => Type::Float,
                    (TypeKind::Int, TypeKind::Int) => Type::Int,
                    _ => Type::Dynamic,
                },
                BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod | BinaryOp::Pow => {
                    match (&l.0, &r.0) {
                        (TypeKind::Decimal, _) | (_, TypeKind::Decimal) => Type::Decimal,
                        (TypeKind::Float, _) | (_, TypeKind::Float) => Type::Float,
                        (TypeKind::Int, TypeKind::Int) => Type::Int,
                        _ => Type::Dynamic,
                    }
                }
                _ => Type::Dynamic,
            }
        }
    }
}
