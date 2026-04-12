mod helpers;

use crate::binder::BindResult;
use crate::types::{ObjectTypeMember, Type};
use rustc_hash::{FxHashMap, FxHashSet};
use tsn_core::TypeKind;

use self::helpers::{
    class_members_match_object, compatible_named, is_known_named, named_members,
    object_matches_class_members, template_matches_literal, types_compatible_with_fn_signature,
};

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

pub(super) fn types_compatible_impl(
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
