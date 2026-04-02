use crate::binder::BindResult;
use crate::checker::Checker;
use crate::types::{ObjectTypeMember, Type};
use tsn_core::ast::operators::BinaryOp;
use tsn_core::ast::Expr;
use tsn_core::TypeKind;

impl Checker {
    pub(crate) fn can_extract_narrowings(&self, expr: &Expr) -> bool {
        matches!(
            expr,
            Expr::Binary {
                op: BinaryOp::Eq | BinaryOp::NotEq | BinaryOp::Instanceof,
                ..
            } | Expr::Logical { .. }
        )
    }

    pub(crate) fn extract_narrowings(
        &mut self,
        expr: &Expr,
        bind: &BindResult,
        is_true_branch: bool,
    ) -> Vec<(crate::symbol::SymbolId, Type)> {
        let cache_key = (
            expr as *const Expr as usize,
            is_true_branch,
            self.current_scope,
        );
        if let Some(cached) = self.narrowings_cache.get(&cache_key) {
            return cached.clone();
        }

        let mut narrowings = Vec::new();

        match expr {
            Expr::Binary {
                left, right, op, ..
            } => {
                let is_eq = *op == tsn_core::ast::operators::BinaryOp::Eq;
                let is_neq = *op == tsn_core::ast::operators::BinaryOp::NotEq;

                if let (
                    Expr::Unary {
                        op: tsn_core::ast::operators::UnaryOp::Typeof,
                        operand: typeof_op,
                        ..
                    },
                    Expr::StrLiteral { value, .. },
                ) = (left.as_ref(), right.as_ref())
                {
                    if (is_eq && is_true_branch) || (is_neq && !is_true_branch) {
                        if let Expr::Identifier { name, .. } = typeof_op.as_ref() {
                            let scope = bind.scopes.get(self.current_scope);
                            if let Some(id) = scope.resolve(name, &bind.scopes) {
                                let narrowed_ty =
                                    crate::binder::resolve_primitive(value, Some(bind));
                                narrowings.push((id, narrowed_ty));
                            }
                        }
                    }
                }

                if let (Expr::Identifier { name, .. }, Expr::NullLiteral { .. }) =
                    (left.as_ref(), right.as_ref())
                {
                    let scope = bind.scopes.get(self.current_scope);
                    if let Some(id) = scope.resolve(name, &bind.scopes) {
                        if (is_neq && is_true_branch) || (is_eq && !is_true_branch) {
                            if let Some(original_ty) = &bind.arena.get(id).ty {
                                narrowings.push((id, original_ty.non_nullified()));
                            }
                        } else if is_eq && is_true_branch {
                            narrowings.push((id, Type::Null));
                        }
                    }
                }

                if is_eq || is_neq {
                    if let Expr::Member {
                        object,
                        property,
                        computed: false,
                        ..
                    } = left.as_ref()
                    {
                        if let (
                            Expr::Identifier { name: obj_name, .. },
                            Expr::Identifier {
                                name: prop_name, ..
                            },
                        ) = (object.as_ref(), property.as_ref())
                        {
                            let disc_ty: Option<Type> = match right.as_ref() {
                                Expr::StrLiteral { value, .. } => {
                                    Some(Type::literal_str(value.clone()))
                                }
                                Expr::IntLiteral { value, .. } => Some(Type::literal_int(*value)),
                                _ => None,
                            };
                            if let Some(disc_ty) = disc_ty {
                                let scope = bind.scopes.get(self.current_scope);
                                if let Some(id) = scope.resolve(obj_name, &bind.scopes) {
                                    let original_ty = bind.arena.get(id).ty.clone();
                                    if let Some(Type(TypeKind::Union(members))) = &original_ty {
                                        let mut matched: Vec<Type> = Vec::new();
                                        let mut unmatched: Vec<Type> = Vec::new();
                                        for m in members.iter() {
                                            let hits =
                                                if let TypeKind::Object(fields) = &m.0 {
                                                    fields.iter().any(|f| matches!(f,
                                                    ObjectTypeMember::Property { name, ty, .. }
                                                    if name == prop_name && ty == &disc_ty
                                                ))
                                                } else {
                                                    false
                                                };
                                            if hits {
                                                matched.push(m.clone());
                                            } else {
                                                unmatched.push(m.clone());
                                            }
                                        }
                                        let make_ty = |v: Vec<Type>| match v.len() {
                                            0 => None,
                                            1 => Some(v.into_iter().next().unwrap()),
                                            _ => Some(Type::union(v)),
                                        };
                                        if (is_eq && is_true_branch) || (is_neq && !is_true_branch)
                                        {
                                            if let Some(t) = make_ty(matched) {
                                                narrowings.push((id, t));
                                            }
                                        } else if (is_neq && is_true_branch)
                                            || (is_eq && !is_true_branch)
                                        {
                                            if let Some(t) = make_ty(unmatched) {
                                                narrowings.push((id, t));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if *op == BinaryOp::Instanceof {
                    if let (
                        Expr::Identifier { name, .. },
                        Expr::Identifier {
                            name: class_name, ..
                        },
                    ) = (left.as_ref(), right.as_ref())
                    {
                        let scope = bind.scopes.get(self.current_scope);
                        if let Some(id) = scope.resolve(name, &bind.scopes) {
                            if is_true_branch {
                                narrowings.push((id, Type::named(class_name.clone())));
                            } else if let Some(ty) = &bind.arena.get(id).ty {
                                let narrowed = ty.minus_named(class_name);
                                if narrowed != *ty {
                                    narrowings.push((id, narrowed));
                                }
                            }
                        }
                    }
                }
            }

            Expr::Logical {
                left,
                right,
                op: tsn_core::ast::operators::LogicalOp::And,
                ..
            } if is_true_branch => {
                narrowings.extend(self.extract_narrowings(left, bind, true));
                narrowings.extend(self.extract_narrowings(right, bind, true));
            }

            Expr::Logical {
                left,
                right,
                op: tsn_core::ast::operators::LogicalOp::Or,
                ..
            } if !is_true_branch => {
                narrowings.extend(self.extract_narrowings(left, bind, false));
                narrowings.extend(self.extract_narrowings(right, bind, false));
            }

            _ => {}
        }
        self.narrowings_cache.insert(cache_key, narrowings.clone());
        narrowings
    }
}
