use crate::types::{FunctionType, ObjectTypeMember, Type, TypeContext};
use tsn_core::ast::TypeNode;
use tsn_core::{well_known, TypeKind};

pub fn resolve_type_node(node: &TypeNode, ctx: Option<&dyn TypeContext>) -> Type {
    match &node.kind {
        TypeKind::Int => Type::Int,
        TypeKind::Float => Type::Float,
        TypeKind::Decimal => Type::Decimal,
        TypeKind::BigInt => Type::BigInt,
        TypeKind::Str => Type::Str,
        TypeKind::Char => Type::Char,
        TypeKind::Bool => Type::Bool,
        TypeKind::Symbol => Type::Symbol,
        TypeKind::Void => Type::Void,
        TypeKind::Null => Type::Null,
        TypeKind::Never => Type::Never,
        TypeKind::Dynamic => Type::Dynamic,
        TypeKind::This => Type::This,

        TypeKind::Nullable(inner) => Type::union(vec![resolve_type_node(inner, ctx), Type::Null]),
        TypeKind::Array(inner) => Type::array(resolve_type_node(inner, ctx)),
        TypeKind::Union(members) => {
            Type::union(members.iter().map(|m| resolve_type_node(m, ctx)).collect())
        }
        TypeKind::Generic(name, args, _origin) => {
            let resolved_args: Vec<Type> = args.iter().map(|m| resolve_type_node(m, ctx)).collect();

            // 1. Check in-scope generic type alias (from explicit imports or local definitions).
            if let Some((params, alias_node)) = ctx.and_then(|c| c.get_alias_node(name)) {
                if !params.is_empty() && params.len() == resolved_args.len() {
                    let alias_ctx = AliasSubstitutionContext {
                        inner: ctx,
                        params,
                        args: resolved_args,
                    };
                    return resolve_type_node(&alias_node, Some(&alias_ctx));
                }
            }

            // 2. Fallback: check std:types for well-known utility type aliases.
            if let Some(ty) = try_stdlib_generic_alias(name, &resolved_args, ctx) {
                return ty;
            }

            Type::generic_with_origin(
                name.clone(),
                resolved_args,
                ctx.and_then(|c| c.source_file()).map(|s| s.to_owned()),
            )
        }
        TypeKind::Named(name, __origin) => {
            // Primitives (int, str, bool, float, etc.) always resolve to their concrete TypeKind.
            // We check this first so that class symbols named "int" etc. never shadow the keyword.
            let prim = resolve_primitive(name, ctx);
            if !matches!(&prim.0, TypeKind::Named(_, _)) {
                return prim;
            }
            // Type variables (K in mapped types) and type aliases need context lookup.
            if let Some(resolved) = ctx.and_then(|c| c.resolve_symbol(name)) {
                return resolved;
            }
            // Unknown name: return Named (class / interface reference).
            prim
        }
        TypeKind::Fn((params, ret)) => {
            let resolved_params = params
                .iter()
                .map(|p| {
                    let ty = p
                        .constraint
                        .as_ref()
                        .map(|m| resolve_type_node(m, ctx))
                        .unwrap_or(Type::Dynamic);
                    crate::types::FunctionParam {
                        name: Some(p.name.clone()),
                        ty,
                        optional: false,
                        is_rest: false,
                    }
                })
                .collect();
            Type::fn_(FunctionType {
                params: resolved_params,
                return_type: Box::new(resolve_type_node(ret, ctx)),
                is_arrow: false,
                type_params: vec![],
            })
        }
        TypeKind::Object(members) => {
            let resolved_members = members
                .iter()
                .filter_map(|m| match m {
                    tsn_core::ast::InterfaceMember::Property {
                        key,
                        type_ann,
                        optional,
                        readonly,
                        ..
                    } => Some(ObjectTypeMember::Property {
                        name: key.clone(),
                        ty: resolve_type_node(type_ann, ctx),
                        optional: *optional,
                        readonly: *readonly,
                    }),
                    tsn_core::ast::InterfaceMember::Method {
                        key,
                        params,
                        return_type,
                        optional,
                        ..
                    } => {
                        let resolved_params = params
                            .iter()
                            .map(|p| {
                                let mut ty = p
                                    .type_ann
                                    .as_ref()
                                    .or_else(|| match &p.pattern {
                                        tsn_core::ast::Pattern::Identifier { type_ann, .. } => {
                                            type_ann.as_ref()
                                        }
                                        _ => None,
                                    })
                                    .map(|ann| resolve_type_node(ann, ctx))
                                    .unwrap_or(Type::Dynamic);
                                if p.is_rest && !matches!(ty.0, TypeKind::Array(_)) {
                                    ty = Type::array(ty);
                                }
                                crate::types::FunctionParam {
                                    name: Some(
                                        crate::binder::pattern_lead_name(&p.pattern).to_owned(),
                                    ),
                                    ty,
                                    optional: p.is_optional,
                                    is_rest: p.is_rest,
                                }
                            })
                            .collect::<Vec<_>>();

                        let ret = return_type
                            .as_ref()
                            .map(|m| resolve_type_node(m, ctx))
                            .unwrap_or(Type::Dynamic);
                        Some(ObjectTypeMember::Method {
                            name: key.clone(),
                            params: resolved_params,
                            return_type: Box::new(ret),
                            optional: *optional,
                            is_arrow: false,
                        })
                    }
                    tsn_core::ast::InterfaceMember::Index {
                        param, return_type, ..
                    } => {
                        let key_ty = param
                            .type_ann
                            .as_ref()
                            .or_else(|| match &param.pattern {
                                tsn_core::ast::Pattern::Identifier { type_ann, .. } => {
                                    type_ann.as_ref()
                                }
                                _ => None,
                            })
                            .map(|ann| resolve_type_node(ann, ctx))
                            .unwrap_or(Type::Str);
                        Some(ObjectTypeMember::Index {
                            param_name: crate::binder::pattern_lead_name(&param.pattern).to_owned(),
                            key_ty: Box::new(key_ty),
                            value_ty: Box::new(resolve_type_node(return_type, ctx)),
                        })
                    }
                    tsn_core::ast::InterfaceMember::Callable {
                        params,
                        return_type,
                        ..
                    } => {
                        let resolved_params = params
                            .iter()
                            .map(|p| {
                                let mut ty = p
                                    .type_ann
                                    .as_ref()
                                    .or_else(|| match &p.pattern {
                                        tsn_core::ast::Pattern::Identifier { type_ann, .. } => {
                                            type_ann.as_ref()
                                        }
                                        _ => None,
                                    })
                                    .map(|ann| resolve_type_node(ann, ctx))
                                    .unwrap_or(Type::Dynamic);
                                if p.is_rest && !matches!(ty.0, TypeKind::Array(_)) {
                                    ty = Type::array(ty);
                                }
                                crate::types::FunctionParam {
                                    name: Some(
                                        crate::binder::pattern_lead_name(&p.pattern).to_owned(),
                                    ),
                                    ty,
                                    optional: p.is_optional,
                                    is_rest: p.is_rest,
                                }
                            })
                            .collect::<Vec<_>>();
                        Some(ObjectTypeMember::Callable {
                            params: resolved_params,
                            return_type: Box::new(resolve_type_node(return_type, ctx)),
                            is_arrow: false,
                        })
                    }
                })
                .collect();
            Type::object(resolved_members)
        }
        TypeKind::TemplateLiteral(parts) => resolve_template_literal_type(parts, ctx),
        TypeKind::LiteralInt(v) => Type::literal_int(*v),
        TypeKind::LiteralFloat(v) => Type::literal_float(*v),
        TypeKind::LiteralStr(v) => Type::literal_str(v.clone()),
        TypeKind::LiteralBool(v) => Type::literal_bool(*v),

        TypeKind::Typeof(expr) => crate::binder::infer_expr_type(expr, ctx),

        TypeKind::Intersection(members) => {
            let resolved: Vec<Type> = members.iter().map(|m| resolve_type_node(m, ctx)).collect();

            let primitives: Vec<&Type> = resolved.iter().filter(|m| is_primitive_type(m)).collect();
            if primitives.len() > 1 {
                let first = primitives[0];
                let incompatible = primitives
                    .iter()
                    .any(|m| std::mem::discriminant(&m.0) != std::mem::discriminant(&first.0));
                if incompatible {
                    return Type::Never;
                }
            }

            let parts_opt: Option<Vec<Vec<ObjectTypeMember>>> = resolved
                .iter()
                .map(|m| match &m.0 {
                    TypeKind::Object(members) => Some(members.clone()),
                    TypeKind::Named(name, origin) => {
                        let ctx = ctx?;
                        let members = ctx
                            .get_class_members(name, origin.as_deref())
                            .or_else(|| ctx.get_interface_members(name, origin.as_deref()))?;
                        Some(
                            members
                                .iter()
                                .map(|cm| {
                                    use crate::types::ClassMemberKind;
                                    match cm.kind {
                                        ClassMemberKind::Method => {
                                            if let TypeKind::Fn(ft) = &cm.ty.0 {
                                                ObjectTypeMember::Method {
                                                    name: cm.name.clone(),
                                                    params: ft.params.clone(),
                                                    return_type: ft.return_type.clone(),
                                                    optional: cm.is_optional,
                                                    is_arrow: ft.is_arrow,
                                                }
                                            } else {
                                                ObjectTypeMember::Property {
                                                    name: cm.name.clone(),
                                                    ty: cm.ty.clone(),
                                                    optional: cm.is_optional,
                                                    readonly: cm.is_readonly,
                                                }
                                            }
                                        }
                                        _ => ObjectTypeMember::Property {
                                            name: cm.name.clone(),
                                            ty: cm.ty.clone(),
                                            optional: cm.is_optional,
                                            readonly: cm.is_readonly,
                                        },
                                    }
                                })
                                .collect(),
                        )
                    }
                    _ => None,
                })
                .collect();

            if let Some(parts) = parts_opt {
                return Type::object(parts.into_iter().flatten().collect());
            }
            Type(TypeKind::Intersection(resolved))
        }

        TypeKind::KeyOf(inner) => {
            let resolved = resolve_type_node(inner, ctx);
            resolve_keyof(resolved, ctx)
        }

        TypeKind::IndexedAccess { object, index } => {
            let obj = resolve_type_node(object, ctx);
            let idx = resolve_type_node(index, ctx);
            resolve_indexed_access(obj, idx, ctx)
        }

        TypeKind::Mapped {
            key_var,
            source,
            value,
            optional,
            readonly,
        } => {
            let resolved_source = resolve_type_node(source, ctx);
            // For homomorphic mapped types (source = keyof T), track the original T
            // so we can preserve per-member optionality when no explicit ? modifier.
            let source_obj = if !*optional {
                if let TypeKind::KeyOf(inner) = &source.kind {
                    Some(resolve_type_node(inner, ctx))
                } else {
                    None
                }
            } else {
                None
            };
            resolve_mapped(
                key_var,
                resolved_source,
                value,
                *optional,
                *readonly,
                source_obj,
                ctx,
            )
        }

        TypeKind::Conditional {
            check,
            extends,
            true_type,
            false_type,
        } => {
            let check_ty = resolve_type_node(check, ctx);
            resolve_conditional(check, &check_ty, extends, true_type, false_type, ctx)
        }

        // `infer R` outside a conditional — not meaningful, return Dynamic.
        TypeKind::Infer(_) => Type::Dynamic,

        _ => Type::Dynamic,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Collects all string literal names from a type (for Pick/Omit key sets).
fn collect_string_literals(ty: &Type) -> Vec<String> {
    match &ty.0 {
        TypeKind::LiteralStr(s) => vec![s.clone()],
        TypeKind::Union(members) => members.iter().flat_map(collect_string_literals).collect(),
        _ => vec![],
    }
}

fn collect_template_literal_variants(parts: &[Type]) -> Option<Vec<String>> {
    let mut variants: Vec<String> = vec![String::new()];
    for (idx, part) in parts.iter().enumerate() {
        if idx % 2 == 0 {
            if let TypeKind::LiteralStr(s) = &part.0 {
                for v in &mut variants {
                    v.push_str(s);
                }
            } else {
                return None;
            }
            continue;
        }

        match &part.0 {
            TypeKind::LiteralStr(s) => {
                for v in &mut variants {
                    v.push_str(s);
                }
            }
            TypeKind::LiteralInt(i) => {
                let lit = i.to_string();
                for v in &mut variants {
                    v.push_str(&lit);
                }
            }
            TypeKind::LiteralBool(b) => {
                let lit = b.to_string();
                for v in &mut variants {
                    v.push_str(&lit);
                }
            }
            TypeKind::Union(members) => {
                let mut additions: Vec<String> = vec![];
                for m in members {
                    let lit = match &m.0 {
                        TypeKind::LiteralStr(s) => s.clone(),
                        TypeKind::LiteralInt(i) => i.to_string(),
                        TypeKind::LiteralBool(b) => b.to_string(),
                        _ => return None,
                    };
                    additions.push(lit);
                }
                let mut next = vec![];
                for existing in &variants {
                    for add in &additions {
                        next.push(format!("{}{}", existing, add));
                    }
                }
                variants = next;
            }
            _ => return None,
        }
    }
    Some(variants)
}

fn resolve_template_literal_type(parts: &[TypeNode], ctx: Option<&dyn TypeContext>) -> Type {
    let resolved_parts: Vec<Type> = parts.iter().map(|p| resolve_type_node(p, ctx)).collect();
    if let Some(variants) = collect_template_literal_variants(&resolved_parts) {
        let literal_types: Vec<Type> = variants.into_iter().map(Type::literal_str).collect();
        return match literal_types.len() {
            0 => Type::literal_str(String::new()),
            1 => literal_types.into_iter().next().unwrap(),
            _ => Type::union(literal_types),
        };
    }
    Type(TypeKind::TemplateLiteral(resolved_parts))
}

// ── KeyOf / IndexedAccess / Mapped helpers ────────────────────────────────────

/// Returns a union of string literal types for every property/method key of `ty`.
fn resolve_keyof(ty: Type, ctx: Option<&dyn TypeContext>) -> Type {
    if let TypeKind::Object(members) = &ty.0 {
        let mut key_types: Vec<Type> = vec![];
        for member in members {
            if let ObjectTypeMember::Index { key_ty, .. } = member {
                if !key_types.iter().any(|k| k == key_ty.as_ref()) {
                    key_types.push((**key_ty).clone());
                }
            }
        }
        if !key_types.is_empty() {
            return if key_types.len() == 1 {
                key_types.into_iter().next().unwrap()
            } else {
                Type::union(key_types)
            };
        }
    }

    let keys = collect_type_keys(&ty, ctx);
    match keys.len() {
        0 => Type::Never,
        1 => Type::literal_str(keys.into_iter().next().unwrap()),
        _ => Type::union(keys.into_iter().map(Type::literal_str).collect()),
    }
}

/// Collects all property/method name strings from an object-like type.
fn collect_type_keys(ty: &Type, ctx: Option<&dyn TypeContext>) -> Vec<String> {
    match &ty.0 {
        TypeKind::Object(members) => members
            .iter()
            .filter_map(|m| match m {
                ObjectTypeMember::Property { name, .. } => Some(name.clone()),
                ObjectTypeMember::Method { name, .. } => Some(name.clone()),
                _ => None,
            })
            .collect(),
        TypeKind::Named(name, _) => ctx
            .and_then(|c| {
                c.get_interface_members(name, None)
                    .or_else(|| c.get_class_members(name, None))
            })
            .map(|members| members.iter().map(|m| m.name.clone()).collect())
            .unwrap_or_default(),
        TypeKind::Intersection(parts) => {
            // Union of keys from all intersection members
            let mut all_keys: Vec<String> = vec![];
            for part in parts {
                for key in collect_type_keys(part, ctx) {
                    if !all_keys.contains(&key) {
                        all_keys.push(key);
                    }
                }
            }
            all_keys
        }
        TypeKind::Union(parts) => {
            // Intersection of keys common to ALL union members
            if parts.is_empty() {
                return vec![];
            }
            let first = collect_type_keys(&parts[0], ctx);
            first
                .into_iter()
                .filter(|k| {
                    parts[1..]
                        .iter()
                        .all(|p| collect_type_keys(p, ctx).contains(k))
                })
                .collect()
        }
        _ => vec![],
    }
}

/// `T[K]` — resolves the type of property `K` in `T`.
fn resolve_indexed_access(obj: Type, index: Type, ctx: Option<&dyn TypeContext>) -> Type {
    if let TypeKind::Object(members) = &obj.0 {
        let value_from_index = |idx: &Type| {
            members.iter().find_map(|m| match m {
                ObjectTypeMember::Index {
                    key_ty, value_ty, ..
                } if crate::checker::compat::types_compatible(key_ty, idx, None) => {
                    Some((**value_ty).clone())
                }
                _ => None,
            })
        };

        match &index.0 {
            TypeKind::LiteralStr(key) => {
                if let Some(prop) = lookup_property_type(&obj, key, ctx) {
                    return prop;
                }
                if let Some(v) = value_from_index(&Type::Str) {
                    return v;
                }
            }
            TypeKind::LiteralInt(_) => {
                if let Some(v) = value_from_index(&Type::Int) {
                    return v;
                }
            }
            TypeKind::Union(members) => {
                let resolved: Vec<Type> = members
                    .iter()
                    .map(|m| resolve_indexed_access(obj.clone(), m.clone(), ctx))
                    .filter(|m| !m.is_dynamic())
                    .collect();
                return match resolved.len() {
                    0 => Type::Dynamic,
                    1 => resolved.into_iter().next().unwrap(),
                    _ => Type::union(resolved),
                };
            }
            _ => {
                if let Some(v) = value_from_index(&index) {
                    return v;
                }
            }
        }
    }

    match &index.0 {
        TypeKind::LiteralStr(key) => lookup_property_type(&obj, key, ctx).unwrap_or(Type::Dynamic),
        TypeKind::Union(members) => {
            let types: Vec<Type> = members
                .iter()
                .filter_map(|m| {
                    if let TypeKind::LiteralStr(key) = &m.0 {
                        lookup_property_type(&obj, key, ctx)
                    } else {
                        None
                    }
                })
                .collect();
            match types.len() {
                0 => Type::Dynamic,
                1 => types.into_iter().next().unwrap(),
                _ => Type::union(types),
            }
        }
        _ => Type::Dynamic,
    }
}

fn lookup_property_type(ty: &Type, key: &str, ctx: Option<&dyn TypeContext>) -> Option<Type> {
    match &ty.0 {
        TypeKind::Object(members) => members.iter().find_map(|m| match m {
            ObjectTypeMember::Property { name, ty, .. } if name == key => Some(ty.clone()),
            ObjectTypeMember::Method {
                name,
                params,
                return_type,
                is_arrow,
                ..
            } if name == key => Some(Type::fn_(FunctionType {
                params: params.clone(),
                return_type: return_type.clone(),
                is_arrow: *is_arrow,
                type_params: vec![],
            })),
            ObjectTypeMember::Index {
                key_ty, value_ty, ..
            } if crate::checker::compat::types_compatible(key_ty, &Type::Str, None) => {
                Some((**value_ty).clone())
            }
            _ => None,
        }),
        TypeKind::Named(name, _) => ctx.and_then(|c| {
            c.get_interface_members(name, None)
                .or_else(|| c.get_class_members(name, None))
                .and_then(|members| members.iter().find(|m| m.name == key).map(|m| m.ty.clone()))
        }),
        _ => None,
    }
}

/// `{ [K in Source]: Value }` — expands to an object type with one property per key.
///
/// `source_obj`: when `optional=false` and the source was `keyof T`, this is the resolved T.
/// Used for homomorphic mapped types to preserve per-member optionality.
fn resolve_mapped(
    key_var: &str,
    source: Type,
    value_node: &tsn_core::ast::TypeNode,
    optional: bool,
    readonly: bool,
    source_obj: Option<Type>,
    ctx: Option<&dyn TypeContext>,
) -> Type {
    let keys = collect_string_literals(&source);
    if keys.is_empty() {
        let key_ty = match &source.0 {
            TypeKind::Str => Some(Type::Str),
            TypeKind::Int => Some(Type::Int),
            TypeKind::LiteralStr(_) => Some(Type::Str),
            TypeKind::LiteralInt(_) => Some(Type::Int),
            _ => None,
        };
        if let Some(key_ty) = key_ty {
            let mapped_ctx = MappedContext {
                inner: ctx,
                key_var: key_var.to_owned(),
                key_value: key_ty.clone(),
            };
            let value_ty = resolve_type_node(value_node, Some(&mapped_ctx));
            return Type::object(vec![ObjectTypeMember::Index {
                param_name: key_var.to_owned(),
                key_ty: Box::new(key_ty),
                value_ty: Box::new(value_ty),
            }]);
        }
        return Type::Dynamic;
    }
    let members: Vec<ObjectTypeMember> = keys
        .into_iter()
        .map(|key| {
            let mapped_ctx = MappedContext {
                inner: ctx,
                key_var: key_var.to_owned(),
                key_value: Type::literal_str(key.clone()),
            };
            let value_ty = resolve_type_node(value_node, Some(&mapped_ctx));
            // For homomorphic mapped types (no explicit ? modifier), preserve source optionality.
            let member_optional = if optional {
                true
            } else if let Some(ref obj) = source_obj {
                is_member_optional(obj, &key, ctx)
            } else {
                false
            };
            ObjectTypeMember::Property {
                name: key,
                ty: value_ty,
                optional: member_optional,
                readonly,
            }
        })
        .collect();
    Type::object(members)
}

// ── Conditional type resolution ───────────────────────────────────────────────

/// `Check extends Extends ? TrueType : FalseType`
///
/// Handles distributivity: when check is a union type and the check_node is a
/// naked type variable, each union member is evaluated independently.
fn resolve_conditional(
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

// ── Infer binding context ─────────────────────────────────────────────────────

/// TypeContext that adds `infer` bindings on top of an existing context.
struct InferBindingContext<'a> {
    inner: Option<&'a dyn TypeContext>,
    bindings: std::collections::HashMap<String, Type>,
}

impl TypeContext for InferBindingContext<'_> {
    fn resolve_symbol(&self, name: &str) -> Option<Type> {
        if let Some(ty) = self.bindings.get(name) {
            return Some(ty.clone());
        }
        self.inner.and_then(|c| c.resolve_symbol(name))
    }

    fn get_interface_members(
        &self,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Vec<crate::types::ClassMemberInfo>> {
        self.inner
            .and_then(|c| c.get_interface_members(name, origin))
    }

    fn get_class_members(
        &self,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Vec<crate::types::ClassMemberInfo>> {
        self.inner.and_then(|c| c.get_class_members(name, origin))
    }

    fn get_namespace_members(
        &self,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Vec<crate::types::ClassMemberInfo>> {
        self.inner
            .and_then(|c| c.get_namespace_members(name, origin))
    }

    fn source_file(&self) -> Option<&str> {
        self.inner.and_then(|c| c.source_file())
    }

    fn get_alias_node(&self, name: &str) -> Option<(Vec<String>, TypeNode)> {
        self.inner.and_then(|c| c.get_alias_node(name))
    }
}

/// Returns true if the member `key` in type `ty` is optional.
fn is_member_optional(ty: &Type, key: &str, ctx: Option<&dyn TypeContext>) -> bool {
    match &ty.0 {
        TypeKind::Object(members) => members.iter().any(|m| match m {
            ObjectTypeMember::Property { name, optional, .. } => name == key && *optional,
            _ => false,
        }),
        TypeKind::Named(name, _) => ctx
            .and_then(|c| {
                c.get_interface_members(name, None)
                    .or_else(|| c.get_class_members(name, None))
            })
            .and_then(|members| {
                members
                    .iter()
                    .find(|m| m.name == key)
                    .map(|m| m.is_optional)
            })
            .unwrap_or(false),
        _ => false,
    }
}

/// TypeContext wrapper that substitutes a single type variable with a concrete type.
/// Used during mapped type expansion to replace `K` with each key literal.
struct MappedContext<'a> {
    inner: Option<&'a dyn TypeContext>,
    key_var: String,
    key_value: Type,
}

impl TypeContext for MappedContext<'_> {
    fn resolve_symbol(&self, name: &str) -> Option<Type> {
        if name == self.key_var {
            return Some(self.key_value.clone());
        }
        self.inner.and_then(|c| c.resolve_symbol(name))
    }

    fn get_interface_members(
        &self,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Vec<crate::types::ClassMemberInfo>> {
        self.inner
            .and_then(|c| c.get_interface_members(name, origin))
    }

    fn get_class_members(
        &self,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Vec<crate::types::ClassMemberInfo>> {
        self.inner.and_then(|c| c.get_class_members(name, origin))
    }

    fn get_namespace_members(
        &self,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Vec<crate::types::ClassMemberInfo>> {
        self.inner
            .and_then(|c| c.get_namespace_members(name, origin))
    }

    fn source_file(&self) -> Option<&str> {
        self.inner.and_then(|c| c.source_file())
    }
}

// ── Generic type alias resolution ─────────────────────────────────────────────

/// Substitution context for generic type alias expansion.
/// Replaces type param names (e.g. "T", "K") with the provided concrete types.
struct AliasSubstitutionContext<'a> {
    inner: Option<&'a dyn TypeContext>,
    params: Vec<String>,
    args: Vec<Type>,
}

impl TypeContext for AliasSubstitutionContext<'_> {
    fn resolve_symbol(&self, name: &str) -> Option<Type> {
        if let Some(pos) = self.params.iter().position(|p| p == name) {
            return Some(self.args[pos].clone());
        }
        self.inner.and_then(|c| c.resolve_symbol(name))
    }

    fn get_interface_members(
        &self,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Vec<crate::types::ClassMemberInfo>> {
        self.inner
            .and_then(|c| c.get_interface_members(name, origin))
    }

    fn get_class_members(
        &self,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Vec<crate::types::ClassMemberInfo>> {
        self.inner.and_then(|c| c.get_class_members(name, origin))
    }

    fn get_namespace_members(
        &self,
        name: &str,
        origin: Option<&str>,
    ) -> Option<Vec<crate::types::ClassMemberInfo>> {
        self.inner
            .and_then(|c| c.get_namespace_members(name, origin))
    }

    fn source_file(&self) -> Option<&str> {
        self.inner.and_then(|c| c.source_file())
    }

    fn get_alias_node(&self, name: &str) -> Option<(Vec<String>, TypeNode)> {
        self.inner.and_then(|c| c.get_alias_node(name))
    }
}

/// Checks `std:types` module for a generic type alias matching `name` and expands it.
///
/// Uses a dedicated OnceLock to avoid deadlocking with MODULE_BIND_CACHE
/// (which is held while binding builtins, and binding calls resolve_type_node).
fn try_stdlib_generic_alias(
    name: &str,
    args: &[Type],
    ctx: Option<&dyn TypeContext>,
) -> Option<Type> {
    use super::super::binder::BindResult;
    use std::sync::OnceLock;

    static STD_TYPES: OnceLock<Option<BindResult>> = OnceLock::new();

    let bind = STD_TYPES
        .get_or_init(|| {
            // Bind std:types directly WITHOUT going through MODULE_BIND_CACHE
            // to avoid deadlocking with it (it may already be locked by a parent binding).
            let path = crate::module_resolver::stdlib_path_for("std:types")?;
            let source = std::fs::read_to_string(&path).ok()?;
            let abs = path.to_string_lossy().into_owned();
            let tokens = tsn_lexer::scan(&source, &abs);
            let program = tsn_parser::parse(tokens, &abs).ok()?;
            Some(crate::binder::Binder::bind(&program))
        })
        .as_ref()?;

    let (params, alias_node) = bind.get_alias_node(name)?;
    if params.is_empty() || params.len() != args.len() {
        return None;
    }
    let alias_ctx = AliasSubstitutionContext {
        inner: ctx,
        params,
        args: args.to_vec(),
    };
    Some(resolve_type_node(&alias_node, Some(&alias_ctx)))
}

fn is_primitive_type(ty: &Type) -> bool {
    matches!(
        &ty.0,
        TypeKind::Int
            | TypeKind::Float
            | TypeKind::Str
            | TypeKind::Bool
            | TypeKind::Char
            | TypeKind::Null
            | TypeKind::Void
            | TypeKind::Never
    )
}

pub fn resolve_primitive(name: &str, ctx: Option<&dyn TypeContext>) -> Type {
    match name {
        well_known::INT => Type::Int,
        well_known::FLOAT => Type::Float,
        well_known::STR => Type::Str,
        well_known::BOOL => Type::Bool,
        well_known::CHAR => Type::Char,
        well_known::VOID => Type::Void,
        well_known::NULL => Type::Null,
        well_known::NEVER => Type::Never,
        well_known::DYNAMIC => Type::Dynamic,
        _ => Type::named_with_origin(
            name.to_owned(),
            ctx.and_then(|c| c.source_file()).map(|s| s.to_owned()),
        ),
    }
}
