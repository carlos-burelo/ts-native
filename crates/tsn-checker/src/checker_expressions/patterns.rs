use crate::binder::BindResult;
use crate::checker::Checker;
use crate::types::Type;

impl Checker {
    pub(crate) fn check_pattern(
        &mut self,
        pattern: &tsn_core::ast::Pattern,
        value_ty: &Type,
        bind: &BindResult,
    ) {
        use tsn_core::ast::Pattern;
        match pattern {
            Pattern::Identifier { name, range, .. } => {
                let scope = bind.scopes.get(self.current_scope);
                if let Some(id) = scope.resolve(name, &bind.scopes) {
                    self.record_type_with_symbol(range.start.offset, value_ty.clone(), id);
                } else {
                    self.record_type(range.start.offset, value_ty.clone());
                }
            }
            Pattern::Object {
                properties, rest, ..
            } => {
                for prop in properties {
                    let member_ty = self.find_member_type(value_ty, &prop.key, bind);
                    self.check_pattern(&prop.value, &member_ty, bind);
                }
                if let Some(r) = rest {
                    self.check_pattern(r, &Type::Dynamic, bind);
                }
            }
            Pattern::Array { elements, rest, .. } => {
                let elem_ty = match &value_ty.0 {
                    tsn_core::TypeKind::Array(inner) => (**inner).clone(),
                    _ => Type::Dynamic,
                };
                for el in elements.iter().flatten() {
                    self.check_pattern(&el.pattern, &elem_ty, bind);
                }
                if let Some(r) = rest {
                    self.check_pattern(r, value_ty, bind);
                }
            }
            Pattern::Assignment { left, .. } => {
                self.check_pattern(left, value_ty, bind);
            }
            Pattern::Rest { argument, .. } => {
                self.check_pattern(argument, value_ty, bind);
            }
        }
    }
}
