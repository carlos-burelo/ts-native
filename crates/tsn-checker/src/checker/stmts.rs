use super::Checker;
use crate::binder::BindResult;
use crate::types::Type;
use tsn_core::ast::{ForInit, Stmt};
use tsn_core::Diagnostic;

impl Checker {
    pub(crate) fn check_stmts(&mut self, stmts: &[Stmt], bind: &BindResult) {
        let mut terminated = false;
        for stmt in stmts {
            if terminated {
                self.diagnostics.push(Diagnostic::warning(
                    "unreachable code".to_string(),
                    stmt.range().clone(),
                ));
                break;
            }
            self.check_stmt(stmt, bind);

            if matches!(stmt, Stmt::Throw { .. } | Stmt::Return { .. }) {
                terminated = true;
            }
        }
    }

    pub(crate) fn check_stmt(&mut self, stmt: &Stmt, bind: &BindResult) {
        match stmt {
            Stmt::Decl(decl) => self.check_decl(decl, bind),

            Stmt::Block { stmts, .. } => {
                self.with_next_child_scope(bind, |checker| checker.check_stmts(stmts, bind));
            }

            Stmt::Expr { expression, .. } => {
                self.check_expr(expression, bind);
            }

            Stmt::Return { argument, range } => {
                let actual = if let Some(arg) = argument {
                    let expected_ret = self.expected_return_type.clone();
                    self.with_expected(expected_ret, |c| c.check_expr(arg, bind));
                    self.infer_type(arg, bind)
                } else {
                    Type::Void
                };

                if let Some(expected) = self.expected_return_type.clone() {
                    if !expected.is_dynamic()
                        && !actual.is_dynamic()
                        && !self.types_compatible_cached(&expected, &actual, Some(bind))
                    {
                        self.diagnostics.push(Diagnostic::error(
                            format!(
                                "type mismatch: function is declared to return '{}', but returns '{}'",
                                expected, actual
                            ),
                            range.clone(),
                        ));
                    }
                }
            }

            Stmt::If {
                test,
                consequent,
                alternate,
                ..
            } => {
                self.check_expr(test, bind);
                if self.can_extract_narrowings(test) {
                    let narrow_true = self.extract_narrowings(test, bind, true);
                    self.with_narrowings(&narrow_true, |checker| {
                        checker.check_stmt(consequent, bind)
                    });

                    if let Some(alt) = alternate {
                        let narrow_false = self.extract_narrowings(test, bind, false);
                        self.with_narrowings(&narrow_false, |checker| {
                            checker.check_stmt(alt, bind)
                        });
                    }
                } else {
                    self.check_stmt(consequent, bind);
                    if let Some(alt) = alternate {
                        self.check_stmt(alt, bind);
                    }
                }
            }

            Stmt::While { test, body, .. } | Stmt::DoWhile { test, body, .. } => {
                self.check_expr(test, bind);
                if self.can_extract_narrowings(test) {
                    let narrow_true = self.extract_narrowings(test, bind, true);
                    self.with_narrowings(&narrow_true, |checker| checker.check_stmt(body, bind));
                } else {
                    self.check_stmt(body, bind);
                }
            }

            Stmt::For {
                init,
                test,
                update,
                body,
                ..
            } => {
                if let Some(i) = init {
                    match i.as_ref() {
                        ForInit::Var { declarators, .. } => {
                            self.check_for_var_init(declarators, bind)
                        }
                        ForInit::Expr(e) => self.check_expr(e, bind),
                    }
                }
                if let Some(t) = test {
                    self.check_expr(t, bind);
                }
                if let Some(u) = update {
                    self.check_expr(u, bind);
                }
                self.check_stmt(body, bind);
            }

            Stmt::ForOf {
                left, right, body, ..
            } => {
                self.check_expr(right, bind);
                let right_ty = self.infer_type(right, bind);
                let elem_ty = match &right_ty.0 {
                    tsn_core::TypeKind::Array(inner) => (**inner).clone(),
                    _ => Type::Dynamic,
                };
                self.check_pattern(left, &elem_ty, bind);
                self.check_stmt(body, bind);
            }

            Stmt::ForIn {
                left, right, body, ..
            } => {
                self.check_expr(right, bind);
                self.check_pattern(left, &Type::Str, bind);
                self.check_stmt(body, bind);
            }

            Stmt::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.check_expr(discriminant, bind);
                for case in cases {
                    if let Some(t) = &case.test {
                        self.check_expr(t, bind);
                    }
                    self.check_stmts(&case.body, bind);
                }
            }

            Stmt::Try {
                block,
                catch,
                finally,
                ..
            } => {
                self.check_stmt(block, bind);
                if let Some(clause) = catch {
                    self.check_stmt(&clause.body, bind);
                }
                if let Some(fin) = finally {
                    self.check_stmt(fin, bind);
                }
            }

            Stmt::Throw { argument, .. } => {
                self.check_expr(argument, bind);
            }

            Stmt::Labeled { body, .. } => {
                self.check_stmt(body, bind);
            }

            Stmt::Using {
                declarations,
                is_await,
                ..
            } => {
                let dispose_method = if *is_await { "disposeAsync" } else { "dispose" };
                let interface_name = if *is_await {
                    tsn_core::well_known::ASYNC_DISPOSABLE
                } else {
                    tsn_core::well_known::DISPOSABLE
                };
                for d in declarations {
                    if d.init.is_none() {
                        self.diagnostics.push(Diagnostic::error(
                            "'using' declaration must have an initializer".to_string(),
                            d.range.clone(),
                        ));
                        continue;
                    }
                    let init = d.init.as_ref().unwrap();
                    self.check_expr(init, bind);
                    let init_ty = self.infer_type(init, bind);
                    if !init_ty.is_dynamic()
                        && !self.member_exists_cached(&init_ty, dispose_method, bind)
                    {
                        self.diagnostics.push(Diagnostic::error(
                            format!(
                                "type '{}' does not implement {}: missing '{}()' method",
                                init_ty, interface_name, dispose_method
                            ),
                            d.range.clone(),
                        ));
                    }
                    self.check_pattern(&d.id, &init_ty, bind);
                }
            }

            _ => {}
        }
    }

    fn check_for_var_init(
        &mut self,
        declarators: &[tsn_core::ast::VarDeclarator],
        bind: &BindResult,
    ) {
        for declarator in declarators {
            let ann = declarator
                .type_ann
                .as_ref()
                .or_else(|| match &declarator.id {
                    tsn_core::ast::Pattern::Identifier { type_ann, .. } => type_ann.as_ref(),
                    _ => None,
                });
            let ann_ty_opt = ann.map(|node| self.resolve_type_node_cached(node, bind));

            if let Some(init_expr) = &declarator.init {
                self.with_expected(ann_ty_opt.clone(), |c| c.check_expr(init_expr, bind));

                if let Some(ann_ty) = &ann_ty_opt {
                    let init_ty = self.infer_type(init_expr, bind);
                    if !init_ty.is_dynamic()
                        && !self.types_compatible_cached(ann_ty, &init_ty, Some(bind))
                    {
                        self.diagnostics.push(Diagnostic::error(
                            format!(
                                "type mismatch: declared as '{}' but initialised with '{}'",
                                ann_ty, init_ty
                            ),
                            declarator.range.clone(),
                        ));
                    }
                }
            }
        }
    }

    fn with_next_child_scope(&mut self, bind: &BindResult, f: impl FnOnce(&mut Self)) {
        let saved = self.current_scope;
        if let Some(child) = self.next_child_scope(bind) {
            self.current_scope = child;
        }
        f(self);
        self.current_scope = saved;
    }

    fn with_narrowings(
        &mut self,
        narrowings: &[(crate::symbol::SymbolId, Type)],
        f: impl FnOnce(&mut Self),
    ) {
        if narrowings.is_empty() {
            f(self);
            return;
        }

        self.push_narrowings(narrowings);
        f(self);
        self.pop_narrowings(narrowings);
    }

    fn push_narrowings(&mut self, narrowings: &[(crate::symbol::SymbolId, Type)]) {
        for (id, ty) in narrowings {
            self.narrowed_types.entry(*id).or_default().push(ty.clone());
        }
        self.mark_infer_env_dirty();
    }

    fn pop_narrowings(&mut self, narrowings: &[(crate::symbol::SymbolId, Type)]) {
        for (id, _) in narrowings {
            if let Some(stack) = self.narrowed_types.get_mut(id) {
                stack.pop();
            }
        }
        self.mark_infer_env_dirty();
    }
}
