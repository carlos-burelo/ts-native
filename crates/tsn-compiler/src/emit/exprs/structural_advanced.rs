use super::Compiler;
use tsn_core::ast::{Expr, Modifiers, Param, Pattern, Stmt};
use tsn_core::OpCode;
use tsn_core::SourceRange;

impl Compiler {
    pub(super) fn compile_expr_structural_advanced(&mut self, expr: &Expr) -> Result<bool, String> {
        match expr {
            Expr::Pipeline { left, right, .. } => {
                if pipeline_has_placeholder(right) {
                    let range = SourceRange::default();
                    let param = Param {
                        pattern: Pattern::Identifier {
                            name: "_".to_owned(),
                            type_ann: None,
                            range: range.clone(),
                        },
                        type_ann: None,
                        default: None,
                        is_rest: false,
                        is_optional: false,
                        modifiers: Modifiers::default(),
                        range: range.clone(),
                    };
                    let body = Stmt::Return {
                        argument: Some(Box::new(right.as_ref().clone())),
                        range,
                    };
                    let (proto, upvalues) = super::super::compile_function_with_parent(
                        "<pipe>",
                        &[param],
                        &body,
                        false,
                        false,
                        false,
                        self,
                    )?;
                    self.emit_closure(proto, upvalues);
                    self.compile_expr(left)?;
                    self.emit1(OpCode::OpCall, 1);
                } else {
                    self.compile_expr(right)?;
                    self.compile_expr(left)?;
                    self.emit1(OpCode::OpCall, 1);
                }
            }

            Expr::ClassExpr { declaration, .. } => {
                self.compile_class_decl(declaration)?;
            }

            Expr::Spread { argument, .. } => {
                self.compile_expr(argument)?;
            }

            Expr::Match { subject, cases, .. } => {
                self.compile_expr(subject)?;
                let mut end_jumps = vec![];
                for case in cases {
                    let locals_before = self.scope.local_count();

                    let skip = match &case.pattern {
                        tsn_core::ast::MatchPattern::Wildcard => {
                            self.scope.declare_local("__match_subj__");
                            None
                        }
                        tsn_core::ast::MatchPattern::Literal(lit) => {
                            self.emit(OpCode::OpDup);
                            self.compile_expr(lit)?;
                            self.emit(OpCode::OpEq);
                            let j = self.emit_jump(OpCode::OpJumpIfFalse);
                            self.emit(OpCode::OpPop);

                            self.scope.declare_local("__match_subj__");
                            Some(j)
                        }
                        tsn_core::ast::MatchPattern::Identifier(name) => {
                            self.scope.declare_local(name.as_str());
                            None
                        }
                        tsn_core::ast::MatchPattern::Record { fields, .. } => {
                            let variant_name = fields.first().and_then(|(key, sub)| {
                                if key == "__variant__" {
                                    if let Some(tsn_core::ast::MatchPattern::Identifier(n)) = sub {
                                        return Some(n.as_str());
                                    }
                                }
                                None
                            });

                            if let Some(vname) = variant_name {
                                self.emit(OpCode::OpDup);
                                let tag_key = self.add_str("__tag");
                                let cs = self.alloc_cache_slot();
                                self.emit2(OpCode::OpGetProperty, tag_key, cs);
                                let vname_idx = self.add_str(vname);
                                self.emit1(OpCode::OpPushConst, vname_idx);
                                self.emit(OpCode::OpEq);
                                let j = self.emit_jump(OpCode::OpJumpIfFalse);
                                self.emit(OpCode::OpPop);

                                let subj_slot = self.scope.declare_local("__match_subj__");

                                for (field_name, sub_pat) in fields.iter().skip(1) {
                                    self.emit1(OpCode::OpGetLocal, subj_slot);
                                    let fkey = self.add_str(field_name);
                                    let cs2 = self.alloc_cache_slot();
                                    self.emit2(OpCode::OpGetProperty, fkey, cs2);

                                    let binding = match sub_pat {
                                        Some(tsn_core::ast::MatchPattern::Identifier(n)) => {
                                            n.as_str()
                                        }
                                        _ => field_name.as_str(),
                                    };
                                    self.scope.declare_local(binding);
                                }

                                Some(j)
                            } else {
                                let subj_slot = self.scope.declare_local("__match_subj__");
                                for (field_name, sub_pat) in fields {
                                    self.emit1(OpCode::OpGetLocal, subj_slot);
                                    let fkey = self.add_str(field_name);
                                    let cs = self.alloc_cache_slot();
                                    self.emit2(OpCode::OpGetProperty, fkey, cs);
                                    let binding = match sub_pat {
                                        Some(tsn_core::ast::MatchPattern::Identifier(n)) => {
                                            n.as_str()
                                        }
                                        _ => field_name.as_str(),
                                    };
                                    self.scope.declare_local(binding);
                                }
                                None
                            }
                        }
                        _ => None,
                    };

                    if let Some(guard) = &case.guard {
                        self.compile_expr(guard)?;
                        let gj = self.emit_jump(OpCode::OpJumpIfFalse);
                        self.emit(OpCode::OpPop);
                        let s2 = self.emit_jump(OpCode::OpJump);
                        self.patch_jump(gj);
                        self.emit(OpCode::OpPop);
                        let _ = s2;
                    }

                    match &case.body {
                        tsn_core::ast::MatchBody::Block(s) => self.compile_stmt(s)?,
                        tsn_core::ast::MatchBody::Expr(e) => self.compile_expr(e)?,
                    }

                    let locals_added = self.scope.local_count() - locals_before;

                    let fields_to_pop = if locals_added > 1 {
                        locals_added - 1
                    } else {
                        0
                    };
                    for _ in 0..fields_to_pop {
                        self.emit(OpCode::OpSwap);
                        self.emit(OpCode::OpPop);
                    }

                    self.scope.locals.truncate(locals_before);

                    let end = self.emit_jump(OpCode::OpJump);
                    end_jumps.push(end);

                    if let Some(s) = skip {
                        self.patch_jump(s);
                        self.emit(OpCode::OpPop);
                    }
                }
                self.emit(OpCode::OpPushNull);
                for j in end_jumps {
                    self.patch_jump(j);
                }
                self.emit(OpCode::OpSwap);
                self.emit(OpCode::OpPop);
            }

            Expr::TaggedTemplate { tag, template, .. } => {
                self.compile_expr(tag)?;
                self.compile_expr(template)?;
                self.emit1(OpCode::OpCall, 1);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}

fn pipeline_has_placeholder(expr: &Expr) -> bool {
    match expr {
        Expr::Identifier { name, .. } => name == "_",
        Expr::Call { callee, args, .. } => {
            use tsn_core::ast::Arg;
            pipeline_has_placeholder(callee)
                || args.iter().any(|a| match a {
                    Arg::Positional(e) | Arg::Spread(e) => pipeline_has_placeholder(e),
                    Arg::Named { value, .. } => pipeline_has_placeholder(value),
                })
        }
        Expr::Member { object, .. } => pipeline_has_placeholder(object),
        Expr::Paren { expression, .. } => pipeline_has_placeholder(expression),
        Expr::Binary { left, right, .. } | Expr::Logical { left, right, .. } => {
            pipeline_has_placeholder(left) || pipeline_has_placeholder(right)
        }
        Expr::Unary { operand, .. } => pipeline_has_placeholder(operand),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            pipeline_has_placeholder(test)
                || pipeline_has_placeholder(consequent)
                || pipeline_has_placeholder(alternate)
        }
        Expr::Array { elements, .. } => {
            use tsn_core::ast::ArrayEl;
            elements.iter().any(|el| match el {
                ArrayEl::Expr(e) | ArrayEl::Spread(e) => pipeline_has_placeholder(e),
                ArrayEl::Hole => false,
            })
        }
        _ => false,
    }
}
