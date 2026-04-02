use super::Checker;
use crate::binder::widen_literal;
use crate::binder::BindResult;
use tsn_core::ast::{Decl, ExportDecl, ExportDefaultDecl, ImportSpecifier, VarKind};
use tsn_core::Diagnostic;

impl Checker {
    pub(crate) fn check_decl(&mut self, decl: &Decl, bind: &BindResult) {
        match decl {
            Decl::Variable(v) => {
                for d in &v.declarators {
                    if matches!(&d.id, tsn_core::ast::Pattern::Identifier { name, .. } if name == "_")
                    {
                        self.diagnostics.push(Diagnostic::warning(
                            "binding to '_' discards the value; use a bare expression statement instead"
                                .to_string(),
                            d.range.clone(),
                        ));
                    }

                    let ann = d.type_ann.as_ref().or_else(|| match &d.id {
                        tsn_core::ast::Pattern::Identifier { type_ann, .. } => type_ann.as_ref(),
                        _ => None,
                    });
                    let ann_ty_opt = ann.map(|node| self.resolve_type_node_cached(node, bind));

                    if let Some(init) = &d.init {
                        // Propagate the declared type as expected context into the init expression.
                        self.with_expected(ann_ty_opt.clone(), |c| c.check_expr(init, bind));

                        let init_ty = self.infer_type(init, bind);
                        let has_ann = ann_ty_opt.is_some();
                        if !init_ty.is_dynamic() {
                            // For `let` without explicit annotation, widen literal types
                            let stored_ty = if v.kind == VarKind::Let && !has_ann {
                                widen_literal(init_ty.clone())
                            } else {
                                init_ty.clone()
                            };
                            self.check_pattern(&d.id, &stored_ty, bind);
                        }

                        if let Some(ann_ty) = &ann_ty_opt {
                            if !init_ty.is_dynamic()
                                && !self.types_compatible_cached(ann_ty, &init_ty, Some(bind))
                            {
                                self.diagnostics.push(Diagnostic::error(
                                    format!(
                                        "type mismatch: declared as '{}' but initialised with '{}'",
                                        ann_ty, init_ty
                                    ),
                                    d.range.clone(),
                                ));
                            }
                        }
                    }

                    if let Some(ann_ty) = &ann_ty_opt {
                        self.check_pattern(&d.id, ann_ty, bind);
                    }
                }
            }

            Decl::Function(f) => {
                // Register the function name token with its symbol_id so rename works.
                let scope = bind.scopes.get(self.current_scope);
                if let Some(sym_id) = scope.resolve(&f.id, &bind.scopes) {
                    if let Some(ty) = bind.arena.get(sym_id).ty.clone() {
                        self.record_type_with_symbol(f.id_offset, ty, sym_id);
                    }
                }

                let saved_expected = self.expected_return_type.take();
                self.expected_return_type = f
                    .return_type
                    .as_ref()
                    .map(|rt| self.resolve_type_node_cached(rt, bind));

                let saved_scope = self.current_scope;
                if let Some(fn_scope) = self.next_child_scope(bind) {
                    self.current_scope = fn_scope;
                }

                self.check_stmt(&f.body, bind);

                self.current_scope = saved_scope;
                self.expected_return_type = saved_expected;
            }

            Decl::Class(c) => {
                let class_name = c.id.clone().unwrap_or_default();
                // Register the class name token with its symbol_id so rename works.
                if !class_name.is_empty() && c.id_offset != 0 {
                    let scope = bind.scopes.get(self.current_scope);
                    if let Some(sym_id) = scope.resolve(&class_name, &bind.scopes) {
                        if let Some(ty) = bind.arena.get(sym_id).ty.clone() {
                            self.record_type_with_symbol(c.id_offset, ty, sym_id);
                        }
                    }
                }

                if let Some(members) = bind.class_members.get(&class_name) {
                    let has_abstract = members.iter().any(|m| {
                        m.is_abstract && m.kind != crate::binder::ClassMemberKind::Constructor
                    });
                    if has_abstract {
                        self.abstract_classes.insert(class_name.clone());
                    }
                }

                for (member_name, child_cls, parent_cls, line, col) in &bind.override_errors {
                    if child_cls == &class_name {
                        let loc = tsn_core::source::SourceLocation {
                            line: *line,
                            column: *col,
                            offset: 0,
                        };
                        self.diagnostics.push(Diagnostic::error(
                            format!(
                                "no overrideable member '{}' found in parent class '{}'",
                                member_name, parent_cls
                            ),
                            tsn_core::source::SourceRange::zero(loc),
                        ));
                    }
                }

                let saved_class = self.current_class.take();
                self.current_class = Some(class_name.clone());

                for member in &c.body {
                    use tsn_core::ast::ClassMember;
                    match member {
                        ClassMember::Constructor { body, .. } => {
                            let saved_scope = self.current_scope;
                            if let Some(fn_scope) = self.next_child_scope(bind) {
                                self.current_scope = fn_scope;
                            }
                            self.check_stmt(body, bind);
                            self.current_scope = saved_scope;
                        }
                        ClassMember::Method {
                            return_type,
                            body: Some(body),
                            ..
                        }
                        | ClassMember::Getter {
                            return_type,
                            body: Some(body),
                            ..
                        } => {
                            let saved_ret = self.expected_return_type.take();
                            self.expected_return_type = return_type
                                .as_ref()
                                .map(|rt| self.resolve_type_node_cached(rt, bind));
                            let saved_scope = self.current_scope;
                            if let Some(fn_scope) = self.next_child_scope(bind) {
                                self.current_scope = fn_scope;
                            }
                            self.check_stmt(body, bind);
                            self.current_scope = saved_scope;
                            self.expected_return_type = saved_ret;
                        }
                        ClassMember::Setter {
                            body: Some(body), ..
                        } => {
                            let saved_scope = self.current_scope;
                            if let Some(fn_scope) = self.next_child_scope(bind) {
                                self.current_scope = fn_scope;
                            }
                            self.check_stmt(body, bind);
                            self.current_scope = saved_scope;
                        }
                        _ => {}
                    }
                }

                self.current_class = saved_class;
            }

            Decl::Namespace(n) => {
                let saved_scope = self.current_scope;
                if let Some(ns_scope) = self.next_child_scope(bind) {
                    self.current_scope = ns_scope;
                }
                for d in &n.body {
                    self.check_decl(d, bind);
                }
                self.current_scope = saved_scope;
            }

            Decl::Export(e) => match e {
                ExportDecl::Decl { declaration, .. } => self.check_decl(declaration, bind),
                ExportDecl::Default { declaration, .. } => match declaration.as_ref() {
                    ExportDefaultDecl::Function(f) => {
                        let saved_expected = self.expected_return_type.take();
                        self.expected_return_type = f
                            .return_type
                            .as_ref()
                            .map(|rt| self.resolve_type_node_cached(rt, bind));
                        let saved_scope = self.current_scope;
                        if let Some(fn_scope) = self.next_child_scope(bind) {
                            self.current_scope = fn_scope;
                        }
                        self.check_stmt(&f.body, bind);
                        self.current_scope = saved_scope;
                        self.expected_return_type = saved_expected;
                    }
                    _ => {}
                },
                _ => {}
            },

            Decl::Import(imp) => {
                // Register import specifier tokens with symbol_ids so rename works.
                let scope = bind.scopes.get(self.current_scope);
                for spec in &imp.specifiers {
                    let (local, offset) = match spec {
                        ImportSpecifier::Named { local, range, .. } => {
                            (local.as_str(), range.start.offset)
                        }
                        ImportSpecifier::Default { local, range } => {
                            (local.as_str(), range.start.offset)
                        }
                        ImportSpecifier::Namespace { local, range } => {
                            (local.as_str(), range.start.offset)
                        }
                    };
                    if let Some(sym_id) = scope.resolve(local, &bind.scopes) {
                        if let Some(ty) = bind.arena.get(sym_id).ty.clone() {
                            self.record_type_with_symbol(offset, ty, sym_id);
                        }
                    }
                }
            }

            Decl::SumType(_) => {}

            Decl::Extension(ext) => {
                for member in &ext.members {
                    let saved_expected = self.expected_return_type.take();
                    let body = match member {
                        tsn_core::ast::ExtensionMember::Method(method) => {
                            self.expected_return_type = method
                                .return_type
                                .as_ref()
                                .map(|rt| self.resolve_type_node_cached(rt, bind));
                            &method.body
                        }
                        tsn_core::ast::ExtensionMember::Getter {
                            return_type, body, ..
                        } => {
                            self.expected_return_type = return_type
                                .as_ref()
                                .map(|rt| self.resolve_type_node_cached(rt, bind));
                            body
                        }
                        tsn_core::ast::ExtensionMember::Setter { body, .. } => body,
                    };

                    let saved_scope = self.current_scope;
                    if let Some(fn_scope) = self.next_child_scope(bind) {
                        self.current_scope = fn_scope;
                    }

                    self.check_stmt(body, bind);

                    self.current_scope = saved_scope;
                    self.expected_return_type = saved_expected;
                }
            }

            _ => {}
        }
    }
}
