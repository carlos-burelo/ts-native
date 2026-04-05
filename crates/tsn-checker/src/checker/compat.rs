use crate::binder::BindResult;
use crate::types::{ClassMemberInfo, ClassMemberKind, ObjectTypeMember, Type};
use rustc_hash::{FxHashMap, FxHashSet};
use tsn_core::TypeKind;

pub(crate) fn types_compatible(
    declared: &Type,
    inferred: &Type,
    bind: Option<&BindResult>,
) -> bool {
    let mut cache = FxHashMap::default();
    types_compatible_with_cache(declared, inferred, bind, &mut cache)
}

pub(crate) fn types_compatible_with_cache(
    declared: &Type,
    inferred: &Type,
    bind: Option<&BindResult>,
    cache: &mut FxHashMap<(usize, usize, usize), bool>,
) -> bool {
    let mut in_progress = FxHashSet::default();
    types_compatible_impl(declared, inferred, bind, cache, &mut in_progress)
}

fn types_compatible_impl(
    declared: &Type,
    inferred: &Type,
    bind: Option<&BindResult>,
    cache: &mut FxHashMap<(usize, usize, usize), bool>,
    in_progress: &mut FxHashSet<(usize, usize, usize)>,
) -> bool {
    let key = (
        declared as *const Type as usize,
        inferred as *const Type as usize,
        bind.map_or(0usize, |b| b as *const BindResult as usize),
    );
    if let Some(cached) = cache.get(&key) {
        return *cached;
    }
    if !in_progress.insert(key) {
        return true;
    }

    let result = match (&declared.0, &inferred.0) {
        (a, b) if a == b => true,

        (TypeKind::Int, TypeKind::LiteralInt(_)) => true,
        (TypeKind::Float, TypeKind::LiteralFloat(_)) => true,
        (TypeKind::Float, TypeKind::Int) => true,
        (TypeKind::Str, TypeKind::LiteralStr(_)) => true,
        (TypeKind::Bool, TypeKind::LiteralBool(_)) => true,
        (TypeKind::Char, TypeKind::LiteralStr(s)) if s.chars().count() == 1 => true,
        (TypeKind::Str, TypeKind::TemplateLiteral(_)) => true,
        (TypeKind::TemplateLiteral(parts), TypeKind::LiteralStr(s)) => {
            template_matches_literal(parts, s)
        }
        (TypeKind::TemplateLiteral(a), TypeKind::TemplateLiteral(b)) => a == b,

        (TypeKind::Array(decl_elem), TypeKind::Array(inf_elem)) => {
            types_compatible_impl(decl_elem, inf_elem, bind, cache, in_progress)
        }

        (TypeKind::Generic(name, args, _origin), TypeKind::Array(inner))
            if name == tsn_core::well_known::ARRAY && args.len() == 1 =>
        {
            types_compatible_impl(&args[0], inner, bind, cache, in_progress)
        }
        (TypeKind::Array(inner), TypeKind::Generic(name, args, _origin))
            if name == tsn_core::well_known::ARRAY && args.len() == 1 =>
        {
            types_compatible_impl(inner, &args[0], bind, cache, in_progress)
        }

        (TypeKind::Generic(n1, a1, _o1), TypeKind::Generic(n2, a2, _o2))
            if n1 == tsn_core::well_known::ARRAY
                && n2 == tsn_core::well_known::ARRAY
                && a1.len() == 1
                && a2.len() == 1 =>
        {
            types_compatible_impl(&a1[0], &a2[0], bind, cache, in_progress)
        }

        (TypeKind::Generic(n1, a1, _o1), TypeKind::Generic(n2, a2, _o2)) if n1 == n2 => {
            a1.len() == a2.len()
                && a1
                    .iter()
                    .zip(a2.iter())
                    .all(|(x, y)| types_compatible_impl(x, y, bind, cache, in_progress))
        }

        (TypeKind::Union(members), _) => members
            .iter()
            .any(|m| types_compatible_impl(m, inferred, bind, cache, in_progress)),
        (_, TypeKind::Union(inf_members)) => inf_members
            .iter()
            .all(|m| types_compatible_impl(declared, m, bind, cache, in_progress)),
        (_, TypeKind::Never) => true,
        (TypeKind::Dynamic, _) | (_, TypeKind::Dynamic) => true,
        (TypeKind::Named(dn, _), TypeKind::Named(in_, _))
        | (TypeKind::Named(dn, _), TypeKind::Generic(in_, _, _))
        | (TypeKind::Generic(dn, _, _), TypeKind::Named(in_, _))
        | (TypeKind::Generic(dn, _, _), TypeKind::Generic(in_, _, _)) => {
            compatible_named(dn, in_, bind, cache, in_progress)
        }
        (TypeKind::Named(dn, _), TypeKind::Object(inf_fields))
        | (TypeKind::Generic(dn, _, _), TypeKind::Object(inf_fields)) => {
            if let Some(bind) = bind {
                if let Some(decl_members) = named_members(bind, dn) {
                    return class_members_match_object(
                        decl_members,
                        inf_fields,
                        bind,
                        cache,
                        in_progress,
                    );
                }
                return !is_known_named(bind, dn);
            }
            true
        }
        (TypeKind::Object(decl_fields), TypeKind::Named(in_, _))
        | (TypeKind::Object(decl_fields), TypeKind::Generic(in_, _, _)) => {
            if let Some(bind) = bind {
                if let Some(inf_members) = named_members(bind, in_) {
                    return object_matches_class_members(
                        decl_fields,
                        inf_members,
                        bind,
                        cache,
                        in_progress,
                    );
                }
                return !is_known_named(bind, in_);
            }
            true
        }
        (TypeKind::Named(_, _), _) | (_, TypeKind::Named(_, _)) => true,
        (TypeKind::Generic(name, args, _origin), _)
            if name == tsn_core::well_known::FUTURE && args.len() == 1 =>
        {
            types_compatible_impl(&args[0], inferred, bind, cache, in_progress)
        }
        (TypeKind::Fn(ft1), TypeKind::Fn(ft2)) => {
            ft2.params.len() <= ft1.params.len()
                && types_compatible_impl(
                    &ft1.return_type,
                    &ft2.return_type,
                    bind,
                    cache,
                    in_progress,
                )
                && ft1.params.iter().zip(ft2.params.iter()).all(|(t1, t2)| {
                    types_compatible_impl(&t2.ty, &t1.ty, bind, cache, in_progress)
                        && t1.optional == t2.optional
                })
        }

        (TypeKind::Object(decl_fields), TypeKind::Object(inf_fields)) => {
            for dm in decl_fields {
                match dm {
                    ObjectTypeMember::Property {
                        name, ty, optional, ..
                    } => {
                        let found = inf_fields.iter().find_map(|im| match im {
                            ObjectTypeMember::Property {
                                name: iname,
                                ty: ity,
                                ..
                            } if iname == name => Some(ity),
                            _ => None,
                        });
                        match found {
                            Some(inf_ty) => {
                                if !types_compatible_impl(ty, inf_ty, bind, cache, in_progress) {
                                    return false;
                                }
                            }
                            None if !*optional => return false,
                            None => {}
                        }
                    }
                    ObjectTypeMember::Method {
                        name,
                        params: p1,
                        return_type: r1,
                        optional,
                        ..
                    } => {
                        if *optional {
                            continue;
                        }
                        let found = inf_fields.iter().find_map(|im| match im {
                            ObjectTypeMember::Method {
                                name: iname,
                                params: p2,
                                return_type: r2,
                                optional: o2,
                                ..
                            } if iname == name => Some((p2, r2, o2)),
                            _ => None,
                        });
                        match found {
                            Some((p2, r2, o2)) => {
                                if *optional != *o2
                                    || !types_compatible_impl(r1, r2, bind, cache, in_progress)
                                    || p1.len() != p2.len()
                                    || p1.iter().zip(p2.iter()).any(|(t1, t2)| {
                                        !types_compatible_impl(
                                            &t1.ty,
                                            &t2.ty,
                                            bind,
                                            cache,
                                            in_progress,
                                        ) || t1.optional != t2.optional
                                    })
                                {
                                    return false;
                                }
                            }
                            None => return false,
                        }
                    }
                    ObjectTypeMember::Index {
                        key_ty, value_ty, ..
                    } => {
                        let has_compatible_index = inf_fields.iter().any(|im| match im {
                            ObjectTypeMember::Index {
                                key_ty: ikey,
                                value_ty: ivalue,
                                ..
                            } => {
                                types_compatible_impl(key_ty, ikey, bind, cache, in_progress)
                                    && types_compatible_impl(
                                        value_ty,
                                        ivalue,
                                        bind,
                                        cache,
                                        in_progress,
                                    )
                            }
                            _ => false,
                        });
                        if has_compatible_index {
                            continue;
                        }

                        let explicit_members_compatible = inf_fields.iter().all(|im| match im {
                            ObjectTypeMember::Property { ty, .. } => {
                                types_compatible_impl(value_ty, ty, bind, cache, in_progress)
                            }
                            ObjectTypeMember::Method {
                                params,
                                return_type,
                                is_arrow,
                                ..
                            } => types_compatible_with_fn_signature(
                                value_ty,
                                params,
                                return_type,
                                *is_arrow,
                                bind,
                                cache,
                                in_progress,
                            ),
                            _ => true,
                        });
                        if !explicit_members_compatible {
                            return false;
                        }
                    }
                    _ => {}
                }
            }
            true
        }

        (TypeKind::Tuple(decl_elems), TypeKind::Array(inf_elem)) => decl_elems
            .iter()
            .all(|d| types_compatible_impl(d, inf_elem, bind, cache, in_progress)),

        (TypeKind::Tuple(decl_elems), TypeKind::Tuple(inf_elems)) => {
            decl_elems.len() == inf_elems.len()
                && decl_elems
                    .iter()
                    .zip(inf_elems)
                    .all(|(d, i)| types_compatible_impl(d, i, bind, cache, in_progress))
        }

        (TypeKind::Intersection(decl_members), _) => decl_members
            .iter()
            .all(|m| types_compatible_impl(m, inferred, bind, cache, in_progress)),

        (_, TypeKind::Intersection(inf_members)) => inf_members
            .iter()
            .any(|m| types_compatible_impl(declared, m, bind, cache, in_progress)),

        _ => false,
    };

    in_progress.remove(&key);
    cache.insert(key, result);
    result
}

fn is_known_named(bind: &BindResult, name: &str) -> bool {
    bind.class_members.contains_key(name)
        || bind.interface_members.contains_key(name)
        || bind.namespace_members.contains_key(name)
        || bind.enum_members.contains_key(name)
        || bind
            .scopes
            .get(bind.global_scope)
            .resolve(name, &bind.scopes)
            .is_some()
}

fn named_members<'a>(bind: &'a BindResult, name: &str) -> Option<&'a [ClassMemberInfo]> {
    bind.interface_members
        .get(name)
        .or_else(|| bind.class_members.get(name))
        .or_else(|| bind.namespace_members.get(name))
        .map(|v| v.as_slice())
}

fn compatible_named(
    declared: &str,
    inferred: &str,
    bind: Option<&BindResult>,
    cache: &mut FxHashMap<(usize, usize, usize), bool>,
    in_progress: &mut FxHashSet<(usize, usize, usize)>,
) -> bool {
    if declared == inferred {
        return true;
    }
    let Some(bind) = bind else {
        return true;
    };

    let decl_members = named_members(bind, declared);
    let inf_members = named_members(bind, inferred);

    match (decl_members, inf_members) {
        (Some(decl), Some(inf)) => class_members_compatible(decl, inf, bind, cache, in_progress),
        _ => {
            // If either side is not a known nominal type, treat as generic/type-var.
            !is_known_named(bind, declared) || !is_known_named(bind, inferred)
        }
    }
}

fn class_members_compatible(
    decl_members: &[ClassMemberInfo],
    inf_members: &[ClassMemberInfo],
    bind: &BindResult,
    cache: &mut FxHashMap<(usize, usize, usize), bool>,
    in_progress: &mut FxHashSet<(usize, usize, usize)>,
) -> bool {
    for dm in decl_members {
        match dm.kind {
            ClassMemberKind::Property | ClassMemberKind::Getter | ClassMemberKind::Setter => {
                let found = inf_members
                    .iter()
                    .find(|im| im.name == dm.name)
                    .map(|m| &m.ty);
                match found {
                    Some(inf_ty) => {
                        if !types_compatible_impl(&dm.ty, inf_ty, Some(bind), cache, in_progress) {
                            return false;
                        }
                    }
                    None if !dm.is_optional => return false,
                    None => {}
                }
            }
            ClassMemberKind::Method => {
                if dm.is_optional {
                    continue;
                }
                let Some(inf_m) = inf_members.iter().find(|im| im.name == dm.name) else {
                    return false;
                };
                if inf_m.kind != ClassMemberKind::Method {
                    return false;
                }
                if !types_compatible_impl(&dm.ty, &inf_m.ty, Some(bind), cache, in_progress) {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

fn class_members_match_object(
    decl_members: &[ClassMemberInfo],
    inf_fields: &[ObjectTypeMember],
    bind: &BindResult,
    cache: &mut FxHashMap<(usize, usize, usize), bool>,
    in_progress: &mut FxHashSet<(usize, usize, usize)>,
) -> bool {
    for dm in decl_members {
        match dm.kind {
            ClassMemberKind::Property | ClassMemberKind::Getter | ClassMemberKind::Setter => {
                let found = inf_fields.iter().find_map(|im| match im {
                    ObjectTypeMember::Property { name, ty, .. } if name == &dm.name => Some(ty),
                    _ => None,
                });
                match found {
                    Some(inf_ty) => {
                        if !types_compatible_impl(&dm.ty, inf_ty, Some(bind), cache, in_progress) {
                            return false;
                        }
                    }
                    None if !dm.is_optional => return false,
                    None => {}
                }
            }
            ClassMemberKind::Method => {
                if dm.is_optional {
                    continue;
                }
                let found = inf_fields.iter().find_map(|im| match im {
                    ObjectTypeMember::Method {
                        name,
                        params,
                        return_type,
                        is_arrow,
                        ..
                    } if name == &dm.name => {
                        Some((params.as_slice(), return_type.as_ref(), *is_arrow))
                    }
                    _ => None,
                });
                let Some((params, return_type, is_arrow)) = found else {
                    return false;
                };
                if !types_compatible_with_fn_signature(
                    &dm.ty,
                    params,
                    return_type,
                    is_arrow,
                    Some(bind),
                    cache,
                    in_progress,
                ) {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

fn object_matches_class_members(
    decl_fields: &[ObjectTypeMember],
    inf_members: &[ClassMemberInfo],
    bind: &BindResult,
    cache: &mut FxHashMap<(usize, usize, usize), bool>,
    in_progress: &mut FxHashSet<(usize, usize, usize)>,
) -> bool {
    for dm in decl_fields {
        match dm {
            ObjectTypeMember::Property {
                name, ty, optional, ..
            } => {
                let found = inf_members
                    .iter()
                    .find(|im| &im.name == name)
                    .map(|m| &m.ty);
                match found {
                    Some(inf_ty) => {
                        if !types_compatible_impl(ty, inf_ty, Some(bind), cache, in_progress) {
                            return false;
                        }
                    }
                    None if !*optional => return false,
                    None => {}
                }
            }
            ObjectTypeMember::Method {
                name,
                params,
                return_type,
                optional,
                is_arrow,
            } => {
                if *optional {
                    continue;
                }
                let Some(inf_m) = inf_members.iter().find(|im| &im.name == name) else {
                    return false;
                };
                if !fn_signature_compatible_type(
                    params,
                    return_type,
                    *is_arrow,
                    &inf_m.ty,
                    Some(bind),
                    cache,
                    in_progress,
                ) {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

fn interp_accepts(ty: &Type, segment: &str) -> bool {
    match &ty.0 {
        TypeKind::Str => true,
        TypeKind::Char => segment.chars().count() == 1,
        TypeKind::Int => segment.parse::<i64>().is_ok(),
        TypeKind::Float | TypeKind::Decimal => segment.parse::<f64>().is_ok(),
        TypeKind::Bool => segment == "true" || segment == "false",
        TypeKind::LiteralStr(s) => segment == s,
        TypeKind::LiteralInt(i) => segment == i.to_string(),
        TypeKind::LiteralBool(b) => segment == b.to_string(),
        TypeKind::TemplateLiteral(parts) => template_matches_literal(parts, segment),
        TypeKind::Union(members) => members.iter().any(|m| interp_accepts(m, segment)),
        TypeKind::Dynamic => true,
        _ => false,
    }
}

fn fn_signature_compatible_type(
    params: &[crate::types::FunctionParam],
    return_type: &Type,
    is_arrow: bool,
    inferred: &Type,
    bind: Option<&BindResult>,
    cache: &mut FxHashMap<(usize, usize, usize), bool>,
    in_progress: &mut FxHashSet<(usize, usize, usize)>,
) -> bool {
    match &inferred.0 {
        TypeKind::Fn(ft2) => {
            ft2.params.len() <= params.len()
                && types_compatible_impl(return_type, &ft2.return_type, bind, cache, in_progress)
                && params.iter().zip(ft2.params.iter()).all(|(t1, t2)| {
                    types_compatible_impl(&t2.ty, &t1.ty, bind, cache, in_progress)
                        && t1.optional == t2.optional
                })
        }
        _ => {
            let declared = Type::fn_(crate::types::FunctionType {
                params: params.to_vec(),
                return_type: Box::new(return_type.clone()),
                is_arrow,
                type_params: vec![],
            });
            types_compatible_impl(&declared, inferred, bind, cache, in_progress)
        }
    }
}

fn types_compatible_with_fn_signature(
    declared: &Type,
    params: &[crate::types::FunctionParam],
    return_type: &Type,
    is_arrow: bool,
    bind: Option<&BindResult>,
    cache: &mut FxHashMap<(usize, usize, usize), bool>,
    in_progress: &mut FxHashSet<(usize, usize, usize)>,
) -> bool {
    match &declared.0 {
        TypeKind::Fn(ft1) => {
            params.len() <= ft1.params.len()
                && types_compatible_impl(&ft1.return_type, return_type, bind, cache, in_progress)
                && ft1.params.iter().zip(params.iter()).all(|(t1, t2)| {
                    types_compatible_impl(&t2.ty, &t1.ty, bind, cache, in_progress)
                        && t1.optional == t2.optional
                })
        }
        _ => {
            let inferred = Type::fn_(crate::types::FunctionType {
                params: params.to_vec(),
                return_type: Box::new(return_type.clone()),
                is_arrow,
                type_params: vec![],
            });
            types_compatible_impl(declared, &inferred, bind, cache, in_progress)
        }
    }
}

fn template_matches_literal(parts: &[Type], value: &str) -> bool {
    fn rec(parts: &[Type], idx: usize, remaining: &str) -> bool {
        if idx >= parts.len() {
            return remaining.is_empty();
        }

        if idx % 2 == 0 {
            let TypeKind::LiteralStr(prefix) = &parts[idx].0 else {
                return false;
            };
            if let Some(rest) = remaining.strip_prefix(prefix) {
                return rec(parts, idx + 1, rest);
            }
            return false;
        }

        let next_literal = if idx + 1 < parts.len() {
            if let TypeKind::LiteralStr(s) = &parts[idx + 1].0 {
                Some(s.as_str())
            } else {
                None
            }
        } else {
            None
        };

        match next_literal {
            Some(lit) if !lit.is_empty() => {
                let mut starts = vec![];
                if remaining.starts_with(lit) {
                    starts.push(0);
                }
                starts.extend(
                    remaining
                        .match_indices(lit)
                        .map(|(i, _)| i)
                        .filter(|i| *i > 0),
                );
                starts.into_iter().any(|i| {
                    interp_accepts(&parts[idx], &remaining[..i])
                        && rec(parts, idx + 1, &remaining[i..])
                })
            }
            _ => interp_accepts(&parts[idx], remaining) && rec(parts, idx + 1, ""),
        }
    }

    rec(parts, 0, value)
}
