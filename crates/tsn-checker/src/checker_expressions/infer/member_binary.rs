use std::collections::HashMap;

use crate::binder::BindResult;
use crate::checker::Checker;
use crate::checker_generics::build_generic_mapping;
use crate::types::{FunctionType, ObjectTypeMember, Type};
use tsn_core::ast::Expr;
use tsn_core::TypeKind;

use super::super::helpers::base_type;
use super::async_members::is_async_member;

pub(super) fn is_atomic_expr(expr: &Expr) -> bool {
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

pub(super) fn infer_member_type(
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

pub(super) fn infer_binary_type(
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
