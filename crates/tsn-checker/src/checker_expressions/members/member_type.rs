use std::sync::Arc;

use crate::binder::BindResult;
use crate::checker::Checker;
use crate::module_resolver::{find_module_bind_for_type_ref, resolve_module_bind_ref};
use crate::types::{well_known, ClassMemberKind, FunctionType, ObjectTypeMember, Type};
use tsn_core::TypeKind;

use super::origin_modules_for_bind;

impl Checker {
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
                let mut external_bind: Option<Arc<BindResult>> = None;
                let mut local_origin_modules: Option<Vec<String>> = None;
                let mut external_origin_modules: Option<Vec<String>> = None;

                if let Some(orig) = origin {
                    if orig != &bind.source_file {
                        if let Some(rb) = resolve_module_bind_ref(orig) {
                            external_bind = Some(rb);
                            external_origin_modules = None;
                            current_bind = None;
                        }
                    }
                }

                let mut current_name = cn.clone();
                let current_origin = origin.clone();

                loop {
                    let b: &BindResult = match current_bind {
                        Some(b) => b,
                        None => external_bind.as_deref().unwrap(),
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
                                if let Some(rb) = resolve_module_bind_ref(&origin) {
                                    external_bind = Some(rb);
                                    external_origin_modules = None;
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
                        let origin_modules = if current_bind.is_some() {
                            local_origin_modules.get_or_insert_with(|| origin_modules_for_bind(b))
                        } else {
                            external_origin_modules
                                .get_or_insert_with(|| origin_modules_for_bind(b))
                        };
                        if !origin_modules.is_empty() {
                            if let Some(rb) =
                                find_module_bind_for_type_ref(&current_name, origin_modules)
                            {
                                external_bind = Some(rb);
                                external_origin_modules = None;
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
