use crate::binder::BindResult;
use crate::checker::Checker;
use crate::types::Type;
use tsn_core::TypeKind;

impl Checker {
    pub(crate) fn member_exists_cached(
        &mut self,
        obj_ty: &Type,
        prop_name: &str,
        bind: &BindResult,
    ) -> bool {
        if let Some(type_name) = canonical_member_cache_name(obj_ty) {
            let key = (type_name, prop_name.to_owned());
            if let Some(cached) = self.member_exists_cache.get(&key) {
                return *cached;
            }

            let result = self.member_exists(obj_ty, prop_name, bind);
            self.member_exists_cache.insert(key, result);
            return result;
        }

        self.member_exists(obj_ty, prop_name, bind)
    }
}

fn canonical_member_cache_name(obj_ty: &Type) -> Option<String> {
    match &obj_ty.0 {
        TypeKind::Named(name, origin) | TypeKind::Generic(name, _, origin) => match origin {
            Some(origin) => Some(format!("{origin}::{name}")),
            None => Some(name.clone()),
        },
        TypeKind::Array(_) => Some(tsn_core::well_known::ARRAY.to_owned()),
        _ => obj_ty.stdlib_key().map(String::from),
    }
}
