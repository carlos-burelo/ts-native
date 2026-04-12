use crate::types::{Type, TypeContext};
use tsn_core::ast::TypeNode;
use tsn_core::TypeKind;

use super::contexts::{AliasSubstitutionContext, InferBindingContext};
use super::resolve_type_node;

pub(super) fn resolve_conditional(
    check_node: &TypeNode,
    check: &Type,
    extends: &TypeNode,
    true_type: &TypeNode,
    false_type: &TypeNode,
    ctx: Option<&dyn TypeContext>,
) -> Type {
    // Distributive conditional: distribute over union members when check_node
    // is a naked type variable (Named("T")) whose value resolved to a union.
    if let TypeKind::Union(members) = &check.0 {
        if matches!(&check_node.kind, TypeKind::Named(_, None)) {
            if let TypeKind::Named(var_name, None) = &check_node.kind {
                let results: Vec<Type> = members
                    .iter()
                    .map(|m| {
                        // Substitute the single type variable with this member.
                        let dist_ctx = AliasSubstitutionContext {
                            inner: ctx,
                            params: vec![var_name.clone()],
                            args: vec![m.clone()],
                        };
                        let mut infer_bindings = std::collections::HashMap::new();
                        let extends_ty = resolve_extends_with_infer(
                            extends,
                            Some(&dist_ctx),
                            &mut infer_bindings,
                            m,
                        );
                        if type_satisfies_extends(m, &extends_ty) {
                            resolve_with_infer_ctx(true_type, Some(&dist_ctx), infer_bindings)
                        } else {
                            resolve_type_node(false_type, Some(&dist_ctx))
                        }
                    })
                    .collect();
                return collapse_union_never(results);
            }
        }
    }

    // Non-distributive: resolve directly.
    let mut infer_bindings = std::collections::HashMap::new();
    let extends_ty = resolve_extends_with_infer(extends, ctx, &mut infer_bindings, check);
    if type_satisfies_extends(check, &extends_ty) {
        resolve_with_infer_ctx(true_type, ctx, infer_bindings)
    } else {
        resolve_type_node(false_type, ctx)
    }
}

/// Resolve `node` within `ctx` augmented by `infer` bindings.
fn resolve_with_infer_ctx(
    node: &TypeNode,
    ctx: Option<&dyn TypeContext>,
    bindings: std::collections::HashMap<String, Type>,
) -> Type {
    if bindings.is_empty() {
        resolve_type_node(node, ctx)
    } else {
        let infer_ctx = InferBindingContext {
            inner: ctx,
            bindings,
        };
        resolve_type_node(node, Some(&infer_ctx))
    }
}

/// Collapse a union while removing `Never` members.
fn collapse_union_never(results: Vec<Type>) -> Type {
    let non_never: Vec<Type> = results
        .into_iter()
        .filter(|t| !matches!(&t.0, TypeKind::Never))
        .collect();
    match non_never.len() {
        0 => Type::Never,
        1 => non_never.into_iter().next().unwrap(),
        _ => Type::union(non_never),
    }
}

/// Resolve the `extends` type node while collecting bindings for `infer R` vars.
/// The `check` type is used for pattern matching (e.g. `Array<infer U>` vs `Array<str>`).
fn resolve_extends_with_infer(
    node: &TypeNode,
    ctx: Option<&dyn TypeContext>,
    bindings: &mut std::collections::HashMap<String, Type>,
    check: &Type,
) -> Type {
    match &node.kind {
        TypeKind::Infer(name) => {
            bindings.insert(name.clone(), check.clone());
            check.clone() // infer always matches: resolved extends == check
        }
        TypeKind::Generic(name, args, _) => {
            if let TypeKind::Generic(check_name, check_args, _) = &check.0 {
                if check_name == name && check_args.len() == args.len() {
                    for (arg_node, check_arg) in args.iter().zip(check_args.iter()) {
                        resolve_extends_with_infer(arg_node, ctx, bindings, check_arg);
                    }
                }
            }
            resolve_type_node(node, ctx)
        }
        TypeKind::Array(inner) => {
            if let TypeKind::Array(check_inner) = &check.0 {
                resolve_extends_with_infer(inner, ctx, bindings, check_inner);
            }
            resolve_type_node(node, ctx)
        }
        TypeKind::Fn((params, ret)) => {
            if let TypeKind::Fn(ft) = &check.0 {
                // Collect infer bindings from return type
                resolve_extends_with_infer(ret, ctx, bindings, &ft.return_type);
                // Collect infer bindings from params
                for (param_node, check_param) in params.iter().zip(ft.params.iter()) {
                    if let Some(constraint) = &param_node.constraint {
                        resolve_extends_with_infer(constraint, ctx, bindings, &check_param.ty);
                    }
                }
            }
            resolve_type_node(node, ctx)
        }
        _ => resolve_type_node(node, ctx),
    }
}

/// Returns true if `check` satisfies the structural constraint `extends`.
/// Uses discriminant equality for primitives, name equality for Named/Generic,
/// and any-member for Union in extends position.
fn type_satisfies_extends(check: &Type, extends: &Type) -> bool {
    // never is a subtype of everything
    if matches!(&check.0, TypeKind::Never) {
        return true;
    }
    // dynamic is compatible with everything
    if matches!(&check.0, TypeKind::Dynamic) || matches!(&extends.0, TypeKind::Dynamic) {
        return true;
    }
    match (&check.0, &extends.0) {
        // Union in extends: T extends A | B iff T extends A or T extends B
        (_, TypeKind::Union(members)) => members.iter().any(|m| type_satisfies_extends(check, m)),
        // Generics: require same name (e.g. Future<T> extends Future<U>)
        (TypeKind::Generic(cn, _, _), TypeKind::Generic(en, _, _)) => cn == en,
        // Named types: require same name
        (TypeKind::Named(cn, _), TypeKind::Named(en, _)) => cn == en,
        // Literal types: require exact value equality
        (TypeKind::LiteralStr(a), TypeKind::LiteralStr(b)) => a == b,
        (TypeKind::LiteralInt(a), TypeKind::LiteralInt(b)) => a == b,
        (TypeKind::LiteralBool(a), TypeKind::LiteralBool(b)) => a == b,
        // Literal types extend their base type
        (TypeKind::LiteralStr(_), TypeKind::Str) => true,
        (TypeKind::LiteralInt(_), TypeKind::Int) => true,
        (TypeKind::LiteralFloat(_), TypeKind::Float) => true,
        (TypeKind::LiteralBool(_), TypeKind::Bool) => true,
        // For all other types: same discriminant (e.g. Null extends Null, Str extends Str)
        _ => std::mem::discriminant(&check.0) == std::mem::discriminant(&extends.0),
    }
}
