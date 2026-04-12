mod member_exists;
mod member_type;

use std::collections::HashSet;

use crate::binder::BindResult;
use crate::checker::Checker;
use crate::types::{ObjectTypeMember, Type};
use tsn_core::TypeKind;

pub(super) fn origin_modules_for_bind(bind: &BindResult) -> Vec<String> {
    bind.global_symbols()
        .filter_map(|s| s.origin_module.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

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
}
