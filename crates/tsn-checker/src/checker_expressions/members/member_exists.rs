use std::sync::Arc;

use crate::binder::BindResult;
use crate::checker::Checker;
use crate::module_resolver::{find_module_bind_for_type_ref, resolve_module_bind_ref};
use crate::symbol::SymbolKind;
use crate::types::{well_known, Type};
use tsn_core::TypeKind;

use super::origin_modules_for_bind;

impl Checker {
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
                    if let Some(rb) = resolve_module_bind_ref(origin_path) {
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
                let mut external_bind: Option<Arc<BindResult>> = None;
                let mut local_origin_modules: Option<Vec<String>> = None;
                let mut external_origin_modules: Option<Vec<String>> = None;
                let mut _current_origin = _origin;

                loop {
                    let b: &BindResult = match current_bind {
                        Some(b) => b,
                        None => external_bind.as_deref().unwrap(),
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
                                if let Some(rb) = resolve_module_bind_ref(&origin) {
                                    external_bind = Some(rb);
                                    external_origin_modules = None;
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

                        let origin_modules = if current_bind.is_some() {
                            local_origin_modules.get_or_insert_with(|| origin_modules_for_bind(b))
                        } else {
                            external_origin_modules
                                .get_or_insert_with(|| origin_modules_for_bind(b))
                        };
                        if !origin_modules.is_empty() {
                            if let Some(rb) = find_module_bind_for_type_ref(current, origin_modules)
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
                false
            }
            TypeKind::Union(members) => members
                .iter()
                .filter(|m| **m != Type::Null)
                .all(|m| self.member_exists(m, prop_name, bind)),

            _ => false,
        }
    }
}
