use crate::binder::BindResult;
use crate::checker::Checker;
use crate::types::ObjectTypeMember;
use tsn_core::ast::operators::Visibility;
use tsn_core::ast::Expr;
use tsn_core::source::SourceRange;
use tsn_core::{Diagnostic, TypeKind};

impl Checker {
    pub(super) fn check_extension_assignment(&mut self, target: &Expr, bind: &BindResult) {
        let Expr::Member {
            object,
            property,
            computed: false,
            ..
        } = target
        else {
            return;
        };
        let Expr::Identifier {
            name: prop_name, ..
        } = property.as_ref()
        else {
            return;
        };

        let obj_ty = self.infer_type(object, bind);
        if let Some(tn) = extension_type_name(&obj_ty.non_nullified()) {
            if let Some(setter_map) = bind.extension_setters.get(&tn) {
                if let Some(mangled) = setter_map.get(prop_name.as_str()) {
                    self.extension_set_members
                        .insert(target.range().start.offset, mangled.clone());
                }
            }
        }
        if let Some(ObjectTypeMember::Property { readonly: true, .. }) =
            self.find_member(&obj_ty, prop_name, bind)
        {
            self.diagnostics.push(Diagnostic::error(
                format!("cannot assign to readonly property '{}'", prop_name),
                target.range().clone(),
            ));
        }
    }

    pub(super) fn check_member_expr(
        &mut self,
        expr: &Expr,
        object: &Expr,
        property: &Expr,
        computed: bool,
        optional: bool,
        range: &SourceRange,
        bind: &BindResult,
    ) {
        self.check_expr(object, bind);
        if computed {
            self.check_expr(property, bind);
            return;
        }

        let prop_ty = self.infer_type(expr, bind);
        self.record_type(property.range().start.offset, prop_ty);

        let Expr::Identifier {
            name: prop_name, ..
        } = property
        else {
            return;
        };

        let obj_ty = self.infer_type(object, bind);
        let check_ty = if optional {
            obj_ty.non_nullified()
        } else {
            obj_ty.clone()
        };
        let should_check = !matches!(check_ty.0, TypeKind::Never);

        if let Some(tn) = extension_type_name(&check_ty) {
            if let Some(getter_map) = bind.extension_getters.get(&tn) {
                if let Some(mangled) = getter_map.get(prop_name.as_str()) {
                    self.extension_members
                        .insert(range.start.offset, mangled.clone());
                }
            } else if let Some(method_map) = bind.extension_methods.get(&tn) {
                if let Some(mangled) = method_map.get(prop_name.as_str()) {
                    self.extension_members
                        .insert(range.start.offset, mangled.clone());
                }
            }
        }

        if should_check && !self.member_exists_cached(&check_ty, prop_name, bind) {
            self.diagnostics.push(Diagnostic::error(
                format!(
                    "property '{}' does not exist on type '{}'",
                    prop_name, check_ty
                ),
                range.clone(),
            ));
        }

        let class_name = match &obj_ty.0 {
            TypeKind::Named(n, _origin) | TypeKind::Generic(n, _, _origin) => Some(n.as_str()),
            _ => None,
        };
        if let Some(class_name) = class_name {
            self.check_member_visibility(class_name, prop_name, range, bind);
        }
    }

    fn check_member_visibility(
        &mut self,
        class_name: &str,
        prop_name: &str,
        range: &SourceRange,
        bind: &BindResult,
    ) {
        let Some(members) = bind.class_members.get(class_name) else {
            return;
        };
        let Some(m) = members.iter().find(|m| m.name == *prop_name) else {
            return;
        };

        match m.visibility {
            Some(Visibility::Private) => {
                if self.current_class.as_deref() != Some(class_name) {
                    self.diagnostics.push(Diagnostic::error(
                        format!(
                            "cannot access private member '{}' outside class '{}'",
                            prop_name, class_name
                        ),
                        range.clone(),
                    ));
                }
            }
            Some(Visibility::Protected) => {
                let ok = self.current_class.as_ref().is_some_and(|cc| {
                    self.is_subclass_or_same(cc, class_name, &bind.class_parents)
                });
                if !ok {
                    self.diagnostics.push(Diagnostic::error(
                        format!(
                            "cannot access protected member '{}' outside class hierarchy of '{}'",
                            prop_name, class_name
                        ),
                        range.clone(),
                    ));
                }
            }
            _ => {}
        }
    }
}

pub(super) fn extension_type_name(ty: &crate::types::Type) -> Option<String> {
    ty.descriptor_key().map(String::from)
}
