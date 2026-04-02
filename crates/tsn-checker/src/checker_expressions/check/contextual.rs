use crate::binder::{pattern_lead_name, BindResult};
use crate::checker::Checker;
use crate::types::{FunctionType, ObjectTypeMember, Type};
use tsn_core::ast::{ArrayEl, ObjectProp, Param, PropKey};
use tsn_core::{Diagnostic, TypeKind};

impl Checker {
    /// When checking an `Arrow` expression that has an `expected_type` of `Fn(ft)`,
    /// inject the expected parameter types for any untyped arrow params into `var_types`.
    /// Must be called AFTER the arrow's child scope has been entered.
    pub(super) fn apply_contextual_arrow_params(
        &mut self,
        params: &[Param],
        expected_fn: &FunctionType,
        bind: &BindResult,
    ) {
        for (ap, ep) in params.iter().zip(expected_fn.params.iter()) {
            // Skip params that already have an explicit annotation
            let has_ann = ap.type_ann.is_some()
                || matches!(
                    &ap.pattern,
                    tsn_core::ast::Pattern::Identifier {
                        type_ann: Some(_),
                        ..
                    }
                );
            if has_ann || ep.ty.is_dynamic() {
                continue;
            }

            let name = pattern_lead_name(&ap.pattern);
            let scope = bind.scopes.get(self.current_scope);
            if let Some(sym_id) = scope.resolve(name, &bind.scopes) {
                self.var_types.insert(sym_id, ep.ty.clone());
                self.mark_infer_env_dirty();
            }
        }
    }

    /// Check array elements with contextual element type propagation.
    /// When `self.expected_type` is `Array(T)`, each element is checked with `expected_type = T`
    /// and validated against T, emitting a type mismatch error on incompatible elements.
    pub(super) fn check_array_with_context(&mut self, elements: &[ArrayEl], bind: &BindResult) {
        let elem_expected = self.expected_type.as_ref().and_then(|t| match &t.0 {
            TypeKind::Array(inner) => Some(*inner.clone()),
            TypeKind::Generic(name, args, _)
                if name == tsn_core::well_known::ARRAY && args.len() == 1 =>
            {
                Some(args[0].clone())
            }
            _ => None,
        });

        for el in elements {
            match el {
                ArrayEl::Expr(e) => {
                    self.with_expected(elem_expected.clone(), |c| c.check_expr(e, bind));
                    if let Some(expected) = &elem_expected {
                        let actual = self.infer_type(e, bind);
                        if !actual.is_dynamic()
                            && !self.types_compatible_cached(expected, &actual, Some(bind))
                        {
                            self.diagnostics.push(Diagnostic::error(
                                format!(
                                    "type mismatch: array element is '{}', expected '{}'",
                                    actual, expected
                                ),
                                e.range().clone(),
                            ));
                        }
                    }
                }
                ArrayEl::Spread(e) => self.check_expr(e, bind),
                ArrayEl::Hole => {}
            }
        }
    }

    /// Check object properties with contextual property type propagation.
    /// When `self.expected_type` is `Object(members)`, each property value is checked
    /// with `expected_type` set to the matching declared member type, and validated.
    pub(super) fn check_object_with_context(
        &mut self,
        properties: &[ObjectProp],
        bind: &BindResult,
    ) {
        let expected_members: Vec<ObjectTypeMember> = self
            .expected_type
            .as_ref()
            .and_then(|t| {
                if let TypeKind::Object(m) = &t.0 {
                    Some(m.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        for prop in properties {
            match prop {
                ObjectProp::Property { key, value, .. } => {
                    let key_str = prop_key_str(key);
                    let prop_expected = key_str.and_then(|k| {
                        expected_members.iter().find_map(|m| match m {
                            ObjectTypeMember::Property { name, ty, .. } if name == k => {
                                Some(ty.clone())
                            }
                            _ => None,
                        })
                    });
                    self.with_expected(prop_expected.clone(), |c| c.check_expr(value, bind));
                    if let Some(expected) = &prop_expected {
                        let actual = self.infer_type(value, bind);
                        if !actual.is_dynamic()
                            && !self.types_compatible_cached(expected, &actual, Some(bind))
                        {
                            self.diagnostics.push(Diagnostic::error(
                                format!(
                                    "type mismatch: property '{}' is '{}', expected '{}'",
                                    key_str.unwrap_or("?"),
                                    actual,
                                    expected
                                ),
                                value.range().clone(),
                            ));
                        }
                    }
                }
                ObjectProp::Method {
                    return_type, body, ..
                } => {
                    let saved_expected = self.expected_return_type.take();
                    self.expected_return_type = return_type
                        .as_ref()
                        .map(|rt| self.resolve_type_node_cached(rt, bind));

                    let saved_scope = self.current_scope;
                    if let Some(fn_scope) = self.next_child_scope(bind) {
                        self.current_scope = fn_scope;
                    }
                    self.check_stmt(body, bind);
                    self.current_scope = saved_scope;
                    self.expected_return_type = saved_expected;
                }
                ObjectProp::Getter { body, .. } | ObjectProp::Setter { body, .. } => {
                    self.check_stmt(body, bind)
                }
                ObjectProp::Spread { argument, .. } => self.check_expr(argument, bind),
            }
        }
    }

    /// Extract the expected `FunctionType` from `self.expected_type` if it is a `Fn(_)`.
    pub(super) fn expected_fn_type(&self) -> Option<FunctionType> {
        self.expected_type.as_ref().and_then(|t| {
            if let TypeKind::Fn(ft) = &t.0 {
                Some(ft.clone())
            } else {
                None
            }
        })
    }

    /// Extract the expected return type from `self.expected_type` if it is a `Fn(_)`.
    pub(super) fn expected_return_from_fn_type(&self) -> Option<Type> {
        self.expected_fn_type()
            .map(|ft| *ft.return_type)
            .filter(|t| !t.is_dynamic())
    }
}

fn prop_key_str(key: &PropKey) -> Option<&str> {
    match key {
        PropKey::Identifier(s) | PropKey::Str(s) => Some(s.as_str()),
        _ => None,
    }
}
