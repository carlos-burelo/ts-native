use tsn_core::ast::operators::{BinaryOp, LogicalOp, UnaryOp};
use tsn_core::ast::{ArrayEl, Expr, ObjectProp, Param, Pattern, PropKey};

use super::type_resolution::resolve_type_node;
use crate::types::{ObjectTypeMember, Type};
use tsn_core::TypeKind;

pub fn infer_expr_type(expr: &Expr, ctx: Option<&dyn crate::types::TypeContext>) -> Type {
    match expr {
        Expr::IntLiteral { .. } => Type::Int,
        Expr::FloatLiteral { .. } => Type::Float,
        Expr::DecimalLiteral { .. } => Type::named(tsn_core::well_known::DECIMAL.into()),
        Expr::StrLiteral { value, .. } => Type::literal_str(value.clone()),
        Expr::CharLiteral { .. } => Type::Char,
        Expr::BoolLiteral { .. } => Type::Bool,
        Expr::NullLiteral { .. } => Type::Null,
        Expr::Template { .. } => Type::Str,
        Expr::Paren { expression, .. } => infer_expr_type(expression, ctx),
        Expr::As { type_ann, .. } => resolve_type_node(type_ann, ctx),
        Expr::Satisfies { expression, .. } => infer_expr_type(expression, ctx),
        Expr::Await { argument, .. } => {
            let inner = infer_expr_type(argument, ctx);

            match &inner.0 {
                TypeKind::Generic(name, args, _)
                    if name == tsn_core::well_known::FUTURE && args.len() == 1 =>
                {
                    args[0].clone()
                }
                _ => inner,
            }
        }
        Expr::NonNull { expression, .. } => infer_expr_type(expression, ctx),
        Expr::Logical {
            op, left, right, ..
        } => match op {
            LogicalOp::Nullish => {
                let rhs = infer_expr_type(right, ctx);
                if !rhs.is_dynamic() {
                    return rhs;
                }
                infer_expr_type(left, ctx)
            }
            LogicalOp::And | LogicalOp::Or => {
                let l = infer_expr_type(left, ctx);
                let r = infer_expr_type(right, ctx);
                if l.0 == r.0 {
                    l
                } else {
                    Type::Dynamic
                }
            }
        },
        Expr::Member {
            object,
            property,
            computed,
            ..
        } => {
            let obj_ty = infer_expr_type(object, ctx);
            if *computed {
                return match &obj_ty.0 {
                    TypeKind::Array(inner) => (**inner).clone(),
                    _ => Type::Dynamic,
                };
            }
            let prop_name = match property.as_ref() {
                Expr::Identifier { name, .. } => name,
                _ => return Type::Dynamic,
            };

            if let Some(ctx) = ctx {
                match &obj_ty.0 {
                    TypeKind::Named(name, origin) | TypeKind::Generic(name, _, origin) => {
                        if let Some(members) = ctx
                            .get_class_members(name, origin.as_deref())
                            .or_else(|| ctx.get_interface_members(name, origin.as_deref()))
                            .or_else(|| ctx.get_namespace_members(name, origin.as_deref()))
                        {
                            if let Some(m) = members.iter().find(|m| m.name == *prop_name) {
                                return m.ty.clone();
                            }
                        }
                    }
                    TypeKind::Object(members) => {
                        for m in members {
                            match m {
                                ObjectTypeMember::Property { name, ty, .. }
                                    if name == prop_name =>
                                {
                                    return ty.clone();
                                }
                                ObjectTypeMember::Method {
                                    name,
                                    params,
                                    return_type,
                                    is_arrow,
                                    ..
                                } if name == prop_name => {
                                    return Type::fn_(crate::types::FunctionType {
                                        params: params.clone(),
                                        return_type: return_type.clone(),
                                        is_arrow: *is_arrow,
                                        type_params: vec![],
                                    });
                                }
                                _ => {}
                            }
                        }
                    }
                    TypeKind::Array(inner) => {
                        if prop_name == "length" {
                            return Type::Int;
                        }
                        if prop_name == "push" {
                            return Type::fn_(crate::types::FunctionType {
                                params: vec![crate::types::FunctionParam {
                                    name: Some("item".to_owned()),
                                    ty: (**inner).clone(),
                                    optional: false,
                                    is_rest: false,
                                }],
                                return_type: Box::new(Type::Int),
                                is_arrow: false,
                                type_params: vec![],
                            });
                        }
                    }
                    _ => {}
                }
            }
            Type::Dynamic
        }
        Expr::Unary { op, operand, .. } => {
            let inner = infer_expr_type(operand, ctx);
            match op {
                UnaryOp::Minus | UnaryOp::Plus => inner,
                UnaryOp::Not => Type::Bool,
                _ => Type::Dynamic,
            }
        }
        Expr::Binary {
            op, left, right, ..
        } => match op {
            BinaryOp::Add => {
                let l = infer_expr_type(left, ctx);
                let r = infer_expr_type(right, ctx);
                match (&l.0, &r.0) {
                    (&TypeKind::Str, _) | (_, &TypeKind::Str) => Type::Str,
                    (&TypeKind::Float, _) | (_, &TypeKind::Float) => Type::Float,
                    (&TypeKind::Int, &TypeKind::Int) => Type::Int,
                    _ => Type::Dynamic,
                }
            }
            BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod | BinaryOp::Pow => {
                let l = infer_expr_type(left, ctx);
                let r = infer_expr_type(right, ctx);
                match (&l.0, &r.0) {
                    (&TypeKind::Float, _) | (_, &TypeKind::Float) => Type::Float,
                    (&TypeKind::Int, &TypeKind::Int) => Type::Int,
                    _ => Type::Dynamic,
                }
            }
            BinaryOp::Eq
            | BinaryOp::NotEq
            | BinaryOp::Lt
            | BinaryOp::Gt
            | BinaryOp::LtEq
            | BinaryOp::GtEq
            | BinaryOp::Instanceof
            | BinaryOp::In => Type::Bool,
            _ => Type::Dynamic,
        },
        Expr::Array { elements, .. } => {
            for el in elements {
                if let ArrayEl::Expr(first) = el {
                    let elem_ty = infer_expr_type(first, ctx);
                    if !elem_ty.is_dynamic() {
                        return Type::array(widen_literal(elem_ty));
                    }
                }
            }
            Type::Dynamic
        }
        Expr::Call { callee, .. } => {
            let callee_ty = infer_expr_type(callee, ctx);
            if let TypeKind::Fn(ft) = &callee_ty.0 {
                return *ft.return_type.clone();
            }
            Type::Dynamic
        }
        Expr::New {
            callee, type_args, ..
        } => {
            if let Expr::Identifier { name, .. } = callee.as_ref() {
                if type_args.is_empty() {
                    return Type::named_with_origin(
                        name.clone(),
                        ctx.and_then(|c| c.source_file()).map(|s| s.to_owned()),
                    );
                }
                let args = type_args
                    .iter()
                    .map(|m| resolve_type_node(m, ctx))
                    .collect();
                return Type::generic_with_origin(
                    name.clone(),
                    args,
                    ctx.and_then(|c| c.source_file()).map(|s| s.to_owned()),
                );
            }

            if let Expr::Member { .. } = callee.as_ref() {
                let callee_ty = infer_expr_type(callee, ctx);
                match &callee_ty.0 {
                    TypeKind::Named(name, origin) => {
                        return Type::named_with_origin(name.clone(), origin.clone());
                    }
                    TypeKind::Generic(name, args, origin) => {
                        return Type::generic_with_origin(
                            name.clone(),
                            args.clone(),
                            origin.clone(),
                        );
                    }
                    _ => {}
                }
            }
            Type::Dynamic
        }
        Expr::Identifier { name, .. } => ctx
            .and_then(|c| c.resolve_symbol(name))
            .unwrap_or(Type::Dynamic),
        Expr::Arrow {
            params,
            return_type,
            body,
            ..
        } => {
            let mut inferred_ret = Type::Dynamic;
            if let tsn_core::ast::ArrowBody::Expr(e) = body.as_ref() {
                inferred_ret = infer_expr_type(e, ctx);
            }
            build_fn_type(params, return_type, true, ctx, inferred_ret)
        }
        Expr::Function {
            params,
            return_type,
            ..
        } => build_fn_type(params, return_type, false, ctx, Type::Dynamic),
        Expr::Object { properties, .. } => {
            let mut members = Vec::new();
            for p in properties {
                match p {
                    ObjectProp::Property { key, value, .. } => {
                        if matches!(key, PropKey::Computed(_)) {
                            let val_ty = infer_expr_type(value, ctx);
                            members.push(ObjectTypeMember::Index {
                                param_name: "_key".to_owned(),
                                key_ty: Box::new(Type::Str),
                                value_ty: Box::new(val_ty),
                            });
                            continue;
                        }
                        let name = match key {
                            PropKey::Identifier(n) | PropKey::Str(n) => n.clone(),
                            _ => continue,
                        };
                        let ty = infer_expr_type(value, ctx);
                        if let TypeKind::Fn(ft) = &ty.0 {
                            members.push(ObjectTypeMember::Method {
                                name,
                                params: ft.params.clone(),
                                return_type: ft.return_type.clone(),
                                optional: false,
                                is_arrow: ft.is_arrow,
                            });
                        } else {
                            members.push(ObjectTypeMember::Property {
                                name,
                                ty,
                                optional: false,
                                readonly: false,
                            });
                        }
                    }
                    ObjectProp::Method {
                        key,
                        params,
                        return_type,
                        ..
                    } => {
                        let name = match key {
                            PropKey::Identifier(n) | PropKey::Str(n) => n.clone(),
                            _ => continue,
                        };
                        let ps = params
                            .iter()
                            .map(|p| {
                                let name = pattern_to_string(&p.pattern);
                                let mut ty = p
                                    .type_ann
                                    .as_ref()
                                    .map(|m| resolve_type_node(m, ctx))
                                    .unwrap_or(Type::Dynamic);
                                if p.is_rest {
                                    if !matches!(ty.0, tsn_core::TypeKind::Array(_)) {
                                        ty = Type::array(ty);
                                    }
                                }
                                crate::types::FunctionParam {
                                    name: Some(name),
                                    ty,
                                    optional: p.is_optional,
                                    is_rest: p.is_rest,
                                }
                            })
                            .collect();
                        let ret = return_type
                            .as_ref()
                            .map(|m| resolve_type_node(m, ctx))
                            .unwrap_or(Type::Dynamic);
                        members.push(ObjectTypeMember::Method {
                            name,
                            params: ps,
                            return_type: Box::new(ret),
                            optional: false,
                            is_arrow: false,
                        });
                    }
                    ObjectProp::Spread { argument, .. } => {
                        let spread_ty = infer_expr_type(argument, ctx);
                        if let TypeKind::Object(spread_members) = spread_ty.0 {
                            members.extend(spread_members);
                        }
                    }
                    _ => {}
                }
            }
            Type::object(members)
        }
        _ => Type::Dynamic,
    }
}

fn build_fn_type(
    params: &[Param],
    return_type: &Option<tsn_core::ast::TypeNode>,
    is_arrow: bool,
    ctx: Option<&dyn crate::types::TypeContext>,
    inferred_ret: Type,
) -> Type {
    let ps = params
        .iter()
        .map(|p| {
            let name = pattern_to_string(&p.pattern);
            let mut ty = p
                .type_ann
                .as_ref()
                .or_else(|| match &p.pattern {
                    Pattern::Identifier { type_ann, .. } => type_ann.as_ref(),
                    _ => None,
                })
                .map(|m| resolve_type_node(m, ctx))
                .unwrap_or(Type::Dynamic);
            if p.is_rest {
                if !matches!(ty.0, tsn_core::TypeKind::Array(_)) {
                    ty = Type::array(ty);
                }
            }
            crate::types::FunctionParam {
                name: Some(name),
                ty,
                optional: p.is_optional,
                is_rest: p.is_rest,
            }
        })
        .collect();
    let ret = return_type
        .as_ref()
        .map(|m| resolve_type_node(m, ctx))
        .unwrap_or(inferred_ret);
    Type::fn_(crate::types::FunctionType {
        params: ps,
        return_type: Box::new(ret),
        is_arrow,
        type_params: vec![],
    })
}

pub fn pattern_to_string(p: &Pattern) -> String {
    match p {
        Pattern::Identifier { name, .. } => name.clone(),
        Pattern::Array { elements, rest, .. } => {
            let mut s = "[".to_owned();
            for (i, el) in elements.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                if let Some(el) = el {
                    s.push_str(&pattern_to_string(&el.pattern));
                }
            }
            if let Some(r) = rest {
                if !elements.is_empty() {
                    s.push_str(", ");
                }
                s.push_str("...");
                s.push_str(&pattern_to_string(r));
            }
            s.push(']');
            s
        }
        Pattern::Object {
            properties, rest, ..
        } => {
            let mut s = "{".to_owned();
            for (i, p) in properties.iter().enumerate() {
                if i > 0 {
                    s.push_str(", ");
                }
                if p.shorthand {
                    s.push_str(&p.key);
                } else {
                    s.push_str(&format!("{}: {}", p.key, pattern_to_string(&p.value)));
                }
            }
            if let Some(r) = rest {
                if !properties.is_empty() {
                    s.push_str(", ");
                }
                s.push_str("...");
                s.push_str(&pattern_to_string(r));
            }
            s.push('}');
            s
        }
        Pattern::Assignment { left, .. } => pattern_to_string(left),
        Pattern::Rest { argument, .. } => format!("...{}", pattern_to_string(argument)),
    }
}

/// Widen a literal type to its primitive base type (TypeScript `let` / array-element semantics).
/// `LiteralStr("x")` → `Str`, `LiteralInt(1)` → `Int`, etc.
pub fn widen_literal(ty: Type) -> Type {
    match &ty.0 {
        TypeKind::LiteralStr(_) => Type::Str,
        TypeKind::LiteralInt(_) => Type::Int,
        TypeKind::LiteralFloat(_) => Type::Float,
        TypeKind::LiteralBool(_) => Type::Bool,
        _ => ty,
    }
}

pub fn pattern_lead_name(p: &Pattern) -> &str {
    match p {
        Pattern::Identifier { name, .. } => name,
        Pattern::Array { .. } => "<array>",
        Pattern::Object { .. } => "<object>",
        Pattern::Rest { .. } => "<rest>",
        Pattern::Assignment { left, .. } => pattern_lead_name(left),
    }
}
