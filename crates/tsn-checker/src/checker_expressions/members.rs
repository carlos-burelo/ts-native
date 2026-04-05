use std::collections::HashSet;

use crate::binder::BindResult;
use crate::checker::Checker;
use crate::module_resolver::{find_module_bind_for_type, resolve_module_bind};
use crate::symbol::SymbolKind;
use crate::types::{well_known, ClassMemberKind, FunctionType, ObjectTypeMember, Type};
use tsn_core::TypeKind;

impl Checker {
    pub(crate) fn find_member(
        &self,
        ty: &Type,
        name: &str,
        bind: &BindResult,
    ) -> Option<ObjectTypeMember> {
        match &ty.0 {
            TypeKind::Object(members) => {
                if let Some(found) = members.iter().find(|m| match m {
                    ObjectTypeMember::Property { name: n, .. } => n == name,
                    ObjectTypeMember::Method { name: n, .. } => n == name,
                    _ => false,
                }) {
                    return Some(found.clone());
                }
                // Fall back to index signature for computed-key objects
                members
                    .iter()
                    .find(|m| match m {
                        ObjectTypeMember::Index { key_ty, .. } => {
                            crate::checker::compat::types_compatible(key_ty, &Type::Str, None)
                        }
                        _ => false,
                    })
                    .cloned()
            }
            TypeKind::Union(members) => {
                for m in members {
                    if let Some(res) = self.find_member(m, name, bind) {
                        return Some(res);
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub(crate) fn member_exists(&self, obj_ty: &Type, prop_name: &str, bind: &BindResult) -> bool {
        if obj_ty.is_dynamic() {
            return true;
        }

        if let Some(tn) = obj_ty.descriptor_key() {
            if bind
                .extension_methods
                .get(tn)
                .map_or(false, |m| m.contains_key(prop_name))
                || bind
                    .extension_getters
                    .get(tn)
                    .map_or(false, |m| m.contains_key(prop_name))
                || bind
                    .extension_setters
                    .get(tn)
                    .map_or(false, |m| m.contains_key(prop_name))
            {
                return true;
            }
        }

        if let TypeKind::Union(members) = &obj_ty.0 {
            return members
                .iter()
                .filter(|m| !m.is_nullable())
                .all(|m| self.member_exists(m, prop_name, bind));
        }

        // Intersection: property exists if it exists on ANY constituent member
        if let TypeKind::Intersection(members) = &obj_ty.0 {
            return members
                .iter()
                .any(|m| self.member_exists(m, prop_name, bind));
        }

        if let TypeKind::Named(cn, origin) = &obj_ty.0 {
            let scope = bind.scopes.get(bind.global_scope);
            let resolved = scope
                .resolve(cn.as_str(), &bind.scopes)
                .map(|sid| bind.arena.get(sid));
            if let Some(sym) = resolved {
                if matches!(sym.kind, SymbolKind::TypeAlias) {
                    if let Some(aliased) = &sym.ty {
                        if aliased.is_dynamic() {
                            return true;
                        }
                        return self.member_exists(aliased, prop_name, bind);
                    }
                }
            }

            if let Some(origin_path) = origin {
                if origin_path != &bind.source_file {
                    if let Some(rb) = resolve_module_bind(origin_path) {
                        let origin_scope = rb.scopes.get(rb.global_scope);
                        if let Some(sid) = origin_scope.resolve(cn.as_str(), &rb.scopes) {
                            let sym = rb.arena.get(sid);
                            if matches!(sym.kind, SymbolKind::TypeAlias) {
                                if let Some(aliased) = &sym.ty {
                                    if aliased.is_dynamic() {
                                        return true;
                                    }
                                    return self.member_exists(aliased, prop_name, &rb);
                                }
                            }
                        }
                    }
                }
            }
        }

        let (lookup_name, _origin, _is_generic) = match &obj_ty.0 {
            TypeKind::Named(cn, origin) => (Some(cn.as_str()), origin.as_deref(), false),
            TypeKind::Generic(cn, _, origin) => (Some(cn.as_str()), origin.as_deref(), true),
            TypeKind::Array(_) => (Some(well_known::ARRAY), None, true),
            TypeKind::Char => (Some(well_known::CHAR), None, true),
            _ => (obj_ty.stdlib_key(), None, false),
        };

        if let Some(name) = lookup_name {
            if bind
                .class_members
                .get(name)
                .map_or(false, |m| m.iter().any(|m| m.name == prop_name))
                || bind
                    .interface_members
                    .get(name)
                    .map_or(false, |m| m.iter().any(|m| m.name == prop_name))
            {
                return true;
            }
        }

        if self.find_member(obj_ty, prop_name, bind).is_some() {
            return true;
        }

        match &obj_ty.0 {
            TypeKind::Named(cn, _origin2) | TypeKind::Generic(cn, _, _origin2) => {
                if bind
                    .enum_members
                    .get(cn.as_str())
                    .map_or(false, |m| m.iter().any(|m| m.name == prop_name))
                {
                    return true;
                }

                if bind.enum_members.contains_key(cn.as_str()) && prop_name == "rawValue" {
                    return true;
                }
                let mut current_name = cn.clone();
                let mut current_bind = Some(bind);
                let mut external_bind: Option<BindResult> = None;
                let mut _current_origin = _origin;

                loop {
                    let b = match current_bind {
                        Some(b) => b,
                        None => external_bind.as_ref().unwrap(),
                    };
                    let current = current_name.as_str();

                    if b.class_members
                        .get(current)
                        .map_or(false, |m| m.iter().any(|m| m.name == prop_name))
                        || b.interface_members
                            .get(current)
                            .map_or(false, |m| m.iter().any(|m| m.name == prop_name))
                        || b.namespace_members
                            .get(current)
                            .map_or(false, |m| m.iter().any(|m| m.name == prop_name))
                        || b.enum_members
                            .get(current)
                            .map_or(false, |m| m.iter().any(|m| m.name == prop_name))
                    {
                        return true;
                    }

                    if let Some(parent) = b.class_parents.get(current) {
                        current_name = parent.clone();
                    } else {
                        let scope = b.scopes.get(b.global_scope);
                        let sid = scope.resolve(current, &b.scopes);
                        if let Some(sid) = sid {
                            let sym = b.arena.get(sid);
                            if let Some(origin) = sym.origin_module.as_deref() {
                                let origin = origin.to_owned();
                                let target_name = sym.original_name.clone();
                                if let Some(rb) = resolve_module_bind(&origin) {
                                    external_bind = Some(rb);
                                    current_bind = None;
                                    _current_origin = Some(origin.as_str());
                                    if let Some(orig) = target_name {
                                        current_name = orig;
                                    }

                                    continue;
                                }
                            } else if let Some(alias_ty) = sym.ty.as_ref().filter(|t| {
                                !t.is_dynamic()
                                    && matches!(
                                        t.0,
                                        TypeKind::Object(_)
                                            | TypeKind::Union(_)
                                            | TypeKind::Intersection(_)
                                            | TypeKind::Tuple(_)
                                    )
                            }) {
                                let alias_ty = alias_ty.clone();
                                return self.member_exists(&alias_ty, prop_name, bind);
                            }
                        }

                        let has_imports = b.global_symbols().any(|s| s.origin_module.is_some());
                        if has_imports {
                            let origin_modules: HashSet<String> = b
                                .global_symbols()
                                .filter_map(|s| s.origin_module.clone())
                                .collect();
                            if let Some(rb) = find_module_bind_for_type(
                                current,
                                &origin_modules.into_iter().collect::<Vec<_>>(),
                            ) {
                                external_bind = Some(rb);
                                current_bind = None;
                                continue;
                            }
                        }
                        break;
                    }
                }
                false
            }
            TypeKind::Union(members) => members
                .iter()
                .filter(|m| **m != Type::Null)
                .all(|m| self.member_exists(m, prop_name, bind)),

            _ => false,
        }
    }

    pub(crate) fn find_member_type(&self, ty: &Type, name: &str, bind: &BindResult) -> Type {
        if let Some(tn) = ty.descriptor_key() {
            if let Some(method_map) = bind.extension_methods.get(tn) {
                if let Some(mangled) = method_map.get(name) {
                    let scope = bind.scopes.get(bind.global_scope);
                    if let Some(sid) = scope.resolve(mangled, &bind.scopes) {
                        let sym = bind.arena.get(sid);
                        if let Some(ft) = &sym.ty {
                            if let TypeKind::Fn(func) = &ft.0 {
                                let mut trimmed = func.clone();
                                if trimmed.params.first().map(|p| p.name.as_deref())
                                    == Some(Some("this"))
                                {
                                    trimmed.params.remove(0);
                                }
                                if sym.is_async
                                    && !matches!(&trimmed.return_type.0, TypeKind::Generic(name, _, _) if name == tsn_core::well_known::FUTURE)
                                    && *trimmed.return_type != Type::Void
                                    && !trimmed.return_type.is_dynamic()
                                {
                                    trimmed.return_type = Box::new(Type::generic(
                                        tsn_core::well_known::FUTURE.to_owned(),
                                        vec![(*trimmed.return_type).clone()],
                                    ));
                                }
                                return Type::fn_(trimmed);
                            }
                            return ft.clone();
                        }
                    }
                }
            }
            if let Some(getter_map) = bind.extension_getters.get(tn) {
                if let Some(mangled) = getter_map.get(name) {
                    let scope = bind.scopes.get(bind.global_scope);
                    if let Some(sid) = scope.resolve(mangled, &bind.scopes) {
                        if let Some(ft) = &bind.arena.get(sid).ty {
                            if let TypeKind::Fn(func) = &ft.0 {
                                return (*func.return_type).clone();
                            }
                            return ft.clone();
                        }
                    }
                }
            }
        }

        if let Some(m) = self.find_member(ty, name, bind) {
            return match m {
                ObjectTypeMember::Property { ty, .. } => ty.clone(),
                ObjectTypeMember::Method {
                    params,
                    return_type,
                    is_arrow,
                    ..
                } => Type::fn_(FunctionType {
                    params: params.clone(),
                    return_type: return_type.clone(),
                    is_arrow,
                    type_params: vec![],
                }),
                ObjectTypeMember::Index { value_ty, .. } => *value_ty,
                _ => Type::Dynamic,
            };
        }

        match &ty.0 {
            TypeKind::Named(cn, origin) | TypeKind::Generic(cn, _, origin) => {
                let mut current_bind = Some(bind);
                let mut external_bind: Option<BindResult> = None;

                if let Some(orig) = origin {
                    if orig != &bind.source_file {
                        if let Some(rb) = resolve_module_bind(orig) {
                            external_bind = Some(rb);
                            current_bind = None;
                        }
                    }
                }

                let mut current_name = cn.clone();
                let current_origin = origin.clone();

                loop {
                    let b = match current_bind {
                        Some(b) => b,
                        None => external_bind.as_ref().unwrap(),
                    };
                    if let Some(members) = b.class_members.get(&current_name) {
                        if let Some(m) = members.iter().find(|m| m.name == name) {
                            if !m.members.is_empty() {
                                return Type::object(
                                    m.members
                                        .iter()
                                        .map(|mi| {
                                            let ty = if let Some(origin) = current_origin.as_ref() {
                                                mi.ty.with_origin_recursive(origin)
                                            } else {
                                                mi.ty.clone()
                                            };
                                            match mi.kind {
                                                ClassMemberKind::Method => {
                                                    if let TypeKind::Fn(ft) = &ty.0 {
                                                        ObjectTypeMember::Method {
                                                            name: mi.name.clone(),
                                                            params: ft.params.clone(),
                                                            return_type: ft.return_type.clone(),
                                                            optional: mi.is_optional,
                                                            is_arrow: ft.is_arrow,
                                                        }
                                                    } else {
                                                        ObjectTypeMember::Property {
                                                            name: mi.name.clone(),
                                                            ty,
                                                            optional: mi.is_optional,
                                                            readonly: mi.is_readonly,
                                                        }
                                                    }
                                                }
                                                _ => ObjectTypeMember::Property {
                                                    name: mi.name.clone(),
                                                    ty,
                                                    optional: mi.is_optional,
                                                    readonly: mi.is_readonly,
                                                },
                                            }
                                        })
                                        .collect(),
                                );
                            }
                            let mut m_ty = m.ty.clone();
                            if let Some(origin) = current_origin.as_ref() {
                                m_ty = m_ty.with_origin_recursive(origin);
                            }
                            return m_ty;
                        }
                    }
                    if let Some(members) = b.interface_members.get(&current_name) {
                        if let Some(m) = members.iter().find(|m| m.name == name) {
                            if !m.members.is_empty() {
                                return Type::object(
                                    m.members
                                        .iter()
                                        .map(|mi| {
                                            let ty = if let Some(origin) = current_origin.as_ref() {
                                                mi.ty.with_origin_recursive(origin)
                                            } else {
                                                mi.ty.clone()
                                            };
                                            match mi.kind {
                                                ClassMemberKind::Method => {
                                                    if let TypeKind::Fn(ft) = &ty.0 {
                                                        ObjectTypeMember::Method {
                                                            name: mi.name.clone(),
                                                            params: ft.params.clone(),
                                                            return_type: ft.return_type.clone(),
                                                            optional: mi.is_optional,
                                                            is_arrow: ft.is_arrow,
                                                        }
                                                    } else {
                                                        ObjectTypeMember::Property {
                                                            name: mi.name.clone(),
                                                            ty,
                                                            optional: mi.is_optional,
                                                            readonly: mi.is_readonly,
                                                        }
                                                    }
                                                }
                                                _ => ObjectTypeMember::Property {
                                                    name: mi.name.clone(),
                                                    ty,
                                                    optional: mi.is_optional,
                                                    readonly: mi.is_readonly,
                                                },
                                            }
                                        })
                                        .collect(),
                                );
                            }
                            let mut m_ty = m.ty.clone();
                            if let Some(origin) = current_origin.as_ref() {
                                m_ty = m_ty.with_origin_recursive(origin);
                            }
                            return m_ty;
                        }
                    }
                    if let Some(members) = b.namespace_members.get(&current_name) {
                        if let Some(m) = members.iter().find(|m| m.name == name) {
                            if !m.members.is_empty() {
                                return Type::object(
                                    m.members
                                        .iter()
                                        .map(|mi| {
                                            let mut m_ty = mi.ty.clone();
                                            if let Some(origin) = current_origin.as_ref() {
                                                m_ty = m_ty.with_origin_recursive(origin);
                                            }
                                            match mi.kind {
                                                ClassMemberKind::Method => {
                                                    if let TypeKind::Fn(ft) = &m_ty.clone().0 {
                                                        ObjectTypeMember::Method {
                                                            name: mi.name.clone(),
                                                            params: ft.params.clone(),
                                                            return_type: ft.return_type.clone(),
                                                            optional: mi.is_optional,
                                                            is_arrow: ft.is_arrow,
                                                        }
                                                    } else {
                                                        ObjectTypeMember::Property {
                                                            name: mi.name.clone(),
                                                            ty: m_ty,
                                                            optional: mi.is_optional,
                                                            readonly: mi.is_readonly,
                                                        }
                                                    }
                                                }
                                                _ => ObjectTypeMember::Property {
                                                    name: mi.name.clone(),
                                                    ty: m_ty,
                                                    optional: mi.is_optional,
                                                    readonly: mi.is_readonly,
                                                },
                                            }
                                        })
                                        .collect(),
                                );
                            }
                            let mut m_ty = m.ty.clone();
                            if let Some(origin) = current_origin.as_ref() {
                                m_ty = m_ty.with_origin_recursive(origin);
                            }
                            return m_ty;
                        }
                    }
                    if let Some(members) = b.enum_members.get(&current_name) {
                        if name == "rawValue" {
                            return Type::Int;
                        }
                        if let Some(m) = members.iter().find(|m| m.name == name) {
                            if !m.members.is_empty() {
                                return Type::object(
                                    m.members
                                        .iter()
                                        .map(|mi| ObjectTypeMember::Property {
                                            name: mi.name.clone(),
                                            ty: if let Some(origin) = current_origin.as_ref() {
                                                mi.ty.with_origin_recursive(origin)
                                            } else {
                                                mi.ty.clone()
                                            },
                                            optional: mi.is_optional,
                                            readonly: mi.is_readonly,
                                        })
                                        .collect(),
                                );
                            }
                            let mut m_ty = m.ty.clone();
                            if let Some(origin) = current_origin.as_ref() {
                                m_ty = m_ty.with_origin_recursive(origin);
                            }
                            return m_ty;
                        }
                    }

                    if let Some(parent) = b.class_parents.get(&current_name) {
                        current_name = parent.clone();
                    } else {
                        let scope = b.scopes.get(b.global_scope);
                        let sid = scope.resolve(&current_name, &b.scopes);
                        if let Some(sid) = sid {
                            let sym = b.arena.get(sid);
                            if let Some(origin) = sym.origin_module.as_deref() {
                                let origin = origin.to_owned();
                                let target_name = sym.original_name.clone();
                                if let Some(rb) = resolve_module_bind(&origin) {
                                    external_bind = Some(rb);
                                    current_bind = None;
                                    if let Some(orig) = target_name {
                                        current_name = orig;
                                    }
                                    continue;
                                }
                            } else if let Some(alias_ty) = sym.ty.as_ref().filter(|t| {
                                !t.is_dynamic()
                                    && matches!(
                                        t.0,
                                        TypeKind::Object(_)
                                            | TypeKind::Union(_)
                                            | TypeKind::Intersection(_)
                                            | TypeKind::Tuple(_)
                                    )
                            }) {
                                let alias_ty = alias_ty.clone();
                                return self.find_member_type(&alias_ty, name, bind);
                            }
                        }
                        let has_imports = b.global_symbols().any(|s| s.origin_module.is_some());
                        if has_imports {
                            let origin_modules: HashSet<String> = b
                                .global_symbols()
                                .filter_map(|s| s.origin_module.clone())
                                .collect();
                            if let Some(rb) = find_module_bind_for_type(
                                &current_name,
                                &origin_modules.into_iter().collect::<Vec<_>>(),
                            ) {
                                external_bind = Some(rb);
                                current_bind = None;
                                continue;
                            }
                        }
                        break;
                    }
                }
            }
            TypeKind::Intersection(members) => {
                for m in members {
                    let t = self.find_member_type(m, name, bind);
                    if !t.is_dynamic() {
                        return t;
                    }
                }
            }
            TypeKind::Array(inner) => {
                return self.find_member_type(
                    &Type::generic(well_known::ARRAY.to_owned(), vec![(**inner).clone()]),
                    name,
                    bind,
                )
            }
            _ => {
                if let Some(key) = ty.stdlib_key() {
                    if !matches!(&ty.0, TypeKind::Array(_)) {
                        return self.find_member_type(&Type::named(key.to_owned()), name, bind);
                    }
                }
            }
        }

        Type::Dynamic
    }
}
