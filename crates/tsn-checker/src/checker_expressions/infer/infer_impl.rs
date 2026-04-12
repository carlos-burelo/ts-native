use crate::binder::BindResult;
use crate::checker::Checker;
use crate::checker_generics::build_call_mapping;
use crate::types::{FunctionParam, FunctionType, ObjectTypeMember, Type};
use tsn_core::ast::Expr;
use tsn_core::{Diagnostic, TypeKind};

use super::collect_checked_return_types;
use super::member_binary::{infer_binary_type, infer_member_type};

impl Checker {
    pub(super) fn infer_type_impl(&mut self, expr: &Expr, bind: &BindResult) -> Type {
        match expr {
            Expr::Identifier { name, .. } => {
                let scope = bind.scopes.get(self.current_scope);
                let opt_id = scope.resolve(name, &bind.scopes);
                if let Some(id) = opt_id {
                    if let Some(narrowed_stack) = self.narrowed_types.get(&id) {
                        if let Some(narrowed_ty) = narrowed_stack.last() {
                            return narrowed_ty.clone();
                        }
                    }

                    // Prefer checker-resolved type (has generic substitution) over binder's raw type
                    if let Some(inferred) = self.var_types.get(&id).cloned() {
                        return inferred;
                    }

                    let ty_opt = &bind.arena.get(id).ty;
                    if let Some(ty) = ty_opt {
                        if !ty.is_dynamic() {
                            return ty.clone();
                        }
                    }
                }

                Type::Dynamic
            }

            Expr::This { .. } => {
                if let Some(cn) = &self.current_class {
                    return Type::named(cn.clone());
                }
                Type::Dynamic
            }

            Expr::New {
                callee, type_args, ..
            } => {
                if let Expr::Identifier { name, .. } = callee.as_ref() {
                    if !type_args.is_empty() {
                        let args: Vec<Type> = type_args
                            .iter()
                            .map(|a| self.resolve_type_node_cached(a, bind))
                            .collect();
                        return Type::generic(name.clone(), args);
                    }
                    return Type::named(name.clone());
                }
                crate::binder::infer_expr_type(expr, Some(bind))
            }

            Expr::Call {
                callee,
                type_args,
                args,
                ..
            } => {
                let callee_ty = self.infer_type(callee, bind).non_nullified();
                if let TypeKind::Fn(ft) = &callee_ty.0 {
                    let mapping = build_call_mapping(callee, type_args, args, ft, self, bind);
                    let ret = if mapping.is_empty() {
                        *ft.return_type.clone()
                    } else {
                        ft.return_type.substitute(&mapping)
                    };

                    let ret = if matches!(ret.0, TypeKind::This) {
                        if let Expr::Member { object, .. } = callee.as_ref() {
                            let receiver_ty = self.infer_type(object, bind);
                            if !receiver_ty.is_dynamic() {
                                receiver_ty
                            } else {
                                ret
                            }
                        } else {
                            ret
                        }
                    } else {
                        ret
                    };

                    let is_async_callee = if let Expr::Identifier { name, .. } = callee.as_ref() {
                        let scope = bind.scopes.get(self.current_scope);
                        scope
                            .resolve(name, &bind.scopes)
                            .map(|id| bind.arena.get(id).is_async)
                            .unwrap_or(false)
                    } else if self
                        .extension_calls
                        .contains_key(&expr.range().start.offset)
                    {
                        self.extension_calls
                            .get(&expr.range().start.offset)
                            .and_then(|mangled| {
                                let scope = bind.scopes.get(bind.global_scope);
                                scope.resolve(mangled, &bind.scopes)
                            })
                            .map(|id| bind.arena.get(id).is_async)
                            .unwrap_or(false)
                    } else {
                        false
                    };

                    if is_async_callee
                        && !matches!(&ret.0, TypeKind::Generic(n, _, _) if n == tsn_core::well_known::FUTURE)
                        && !ret.is_dynamic()
                        && ret != Type::Void
                    {
                        Type::generic(tsn_core::well_known::FUTURE.to_owned(), vec![ret])
                    } else {
                        ret
                    }
                } else {
                    Type::Dynamic
                }
            }

            Expr::Conditional {
                consequent,
                alternate,
                ..
            } => {
                let t_ty = self.infer_type(consequent, bind);
                let f_ty = self.infer_type(alternate, bind);
                if t_ty == f_ty {
                    t_ty
                } else if t_ty.is_dynamic() {
                    f_ty
                } else if f_ty.is_dynamic() {
                    t_ty
                } else {
                    Type::union(vec![t_ty, f_ty])
                }
            }

            Expr::Member {
                object,
                property,
                computed: false,
                ..
            } => infer_member_type(self, expr, object, property, bind),

            Expr::Member {
                object,
                property,
                computed: true,
                ..
            } => {
                let obj_ty = self.infer_type(object, bind);
                let idx_ty = self.infer_type(property, bind);
                match &obj_ty.0 {
                    tsn_core::TypeKind::Array(elem) => *elem.clone(),
                    tsn_core::TypeKind::Object(members) => {
                        for member in members {
                            if let ObjectTypeMember::Index {
                                key_ty, value_ty, ..
                            } = member
                            {
                                if self.types_compatible_cached(key_ty, &idx_ty, None) {
                                    return Type::make_nullable((**value_ty).clone());
                                }
                            }
                        }
                        Type::Dynamic
                    }
                    _ => crate::binder::infer_expr_type(expr, Some(bind)),
                }
            }

            Expr::Arrow {
                params,
                return_type,
                body,
                ..
            } => {
                if return_type.is_some() {
                    return crate::binder::infer_expr_type(expr, Some(bind));
                }

                let ret_ty = match body.as_ref() {
                    tsn_core::ast::ArrowBody::Expr(e) => self
                        .expr_types
                        .get(&e.range().start.offset)
                        .map(|info| info.ty.clone())
                        .unwrap_or(Type::Dynamic),
                    tsn_core::ast::ArrowBody::Block(block) => {
                        let return_tys = collect_checked_return_types(block, &self.expr_types);
                        match return_tys.len() {
                            0 => Type::Void,
                            1 => return_tys.into_iter().next().unwrap(),
                            _ => Type::union(return_tys),
                        }
                    }
                };
                let ps = params
                    .iter()
                    .map(|p| {
                        let name = crate::binder::pattern_lead_name(&p.pattern).to_owned();
                        let mut ty = p
                            .type_ann
                            .as_ref()
                            .or_else(|| match &p.pattern {
                                tsn_core::ast::Pattern::Identifier { type_ann, .. } => {
                                    type_ann.as_ref()
                                }
                                _ => None,
                            })
                            .map(|m| self.resolve_type_node_cached(m, bind))
                            .unwrap_or_else(|| {
                                if self.warn_implicit_dynamic && !name.is_empty() && name != "_" {
                                    self.diagnostics.push(Diagnostic::hint(
                                        format!("el parámetro '{name}' no tiene anotación de tipo — se asumió 'dynamic'"),
                                        p.pattern.range().clone(),
                                    ));
                                }
                                Type::Dynamic
                            });
                        if p.is_rest && !matches!(ty.0, tsn_core::TypeKind::Array(_)) {
                            ty = Type::array(ty);
                        }
                        FunctionParam {
                            name: Some(name),
                            ty,
                            optional: p.is_optional,
                            is_rest: p.is_rest,
                        }
                    })
                    .collect();
                Type::fn_(FunctionType {
                    params: ps,
                    return_type: Box::new(ret_ty),
                    is_arrow: true,
                    type_params: vec![],
                })
            }

            Expr::Object { properties, .. } => {
                // Only override the binder's inference when there are spreads — the binder can't
                // resolve local variables so spreads always produce nothing in its pass.
                let has_spread = properties
                    .iter()
                    .any(|p| matches!(p, tsn_core::ast::ObjectProp::Spread { .. }));
                if !has_spread {
                    return crate::binder::infer_expr_type(expr, Some(bind));
                }
                let mut members: Vec<ObjectTypeMember> = Vec::new();
                for prop in properties {
                    match prop {
                        tsn_core::ast::ObjectProp::Spread { argument, .. } => {
                            let spread_ty = self.infer_type(argument, bind);
                            match &spread_ty.non_nullified().0 {
                                TypeKind::Object(ms) => members.extend(ms.clone()),
                                TypeKind::Named(_, _) | TypeKind::Generic(_, _, _) => {
                                    // Named type — flatten its known members into the object.
                                    let key = match &spread_ty.non_nullified().0 {
                                        TypeKind::Named(n, _) => Some(n.clone()),
                                        TypeKind::Generic(n, _, _) => Some(n.clone()),
                                        _ => None,
                                    };
                                    if let Some(cls) = key {
                                        if let Some(ms) = bind.class_members.get(&cls) {
                                            for m in ms {
                                                members.push(ObjectTypeMember::Property {
                                                    name: m.name.clone(),
                                                    ty: m.ty.clone(),
                                                    optional: m.is_optional,
                                                    readonly: m.is_readonly,
                                                });
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        tsn_core::ast::ObjectProp::Property { key, value, .. } => {
                            use tsn_core::ast::PropKey;
                            let name = match key {
                                PropKey::Identifier(n) | PropKey::Str(n) => n.clone(),
                                PropKey::Computed(_) | PropKey::Int(_) => continue,
                            };
                            let ty = self.infer_type(value, bind);
                            members.push(ObjectTypeMember::Property {
                                name,
                                ty,
                                optional: false,
                                readonly: false,
                            });
                        }
                        _ => {}
                    }
                }
                Type::object(members)
            }

            Expr::Satisfies { expression, .. } => self.infer_type(expression, bind),

            Expr::Await { argument, .. } => {
                let inner = self.infer_type(argument, bind);
                match &inner.0 {
                    TypeKind::Generic(name, args, _origin)
                        if name == tsn_core::well_known::FUTURE && args.len() == 1 =>
                    {
                        args[0].clone()
                    }
                    _ => inner,
                }
            }

            Expr::Binary {
                op, left, right, ..
            } => infer_binary_type(self, op, left, right, bind),

            Expr::Unary { op, operand, .. } => {
                use tsn_core::ast::operators::UnaryOp;
                let inner = self.infer_type(operand, bind);
                match op {
                    UnaryOp::Not => Type::Bool,
                    UnaryOp::Minus | UnaryOp::Plus => inner,
                    _ => Type::Dynamic,
                }
            }

            Expr::Update { operand, .. } => {
                let inner = self.infer_type(operand, bind);
                match &inner.0 {
                    TypeKind::Int | TypeKind::Float => inner,
                    _ => Type::Int,
                }
            }

            Expr::Yield { argument, .. } => {
                if let Some(arg) = argument {
                    self.infer_type(arg, bind)
                } else {
                    Type::Void
                }
            }

            Expr::Template { .. } => Type::Str,

            Expr::Logical {
                op, left, right, ..
            } => {
                use tsn_core::ast::operators::LogicalOp;
                match op {
                    LogicalOp::Nullish => {
                        let r = self.infer_type(right, bind);
                        if r.is_dynamic() {
                            self.infer_type(left, bind)
                        } else {
                            r
                        }
                    }
                    LogicalOp::And | LogicalOp::Or => {
                        let l = self.infer_type(left, bind);
                        let r = self.infer_type(right, bind);
                        if l == r {
                            l
                        } else if l.is_dynamic() {
                            r
                        } else {
                            l
                        }
                    }
                }
            }

            _ => crate::binder::infer_expr_type(expr, Some(bind)),
        }
    }
}
