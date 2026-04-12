use super::Compiler;
use tsn_core::ast::{ArrayEl, ArrowBody, Expr, ObjectProp, PropKey, Stmt, TemplatePart};
use tsn_core::OpCode;

impl Compiler {
    pub(super) fn compile_expr_structural_core(&mut self, expr: &Expr) -> Result<bool, String> {
        match expr {
            Expr::Array { elements, .. } => {
                let has_spread = elements.iter().any(|e| matches!(e, ArrayEl::Spread(_)));
                if has_spread {
                    // incremental build: start with empty array, push/extend each element
                    self.emit1(OpCode::OpBuildArray, 0);
                    for el in elements {
                        match el {
                            ArrayEl::Hole => {
                                self.emit(OpCode::OpPushNull);
                                self.emit(OpCode::OpArrayPush);
                                self.emit(OpCode::OpPop); // discard length
                            }
                            ArrayEl::Expr(e) => {
                                self.compile_expr(e)?;
                                self.emit(OpCode::OpArrayPush);
                                self.emit(OpCode::OpPop); // discard length
                            }
                            ArrayEl::Spread(e) => {
                                self.compile_expr(e)?;
                                self.emit(OpCode::OpArrayExtend);
                            }
                        }
                    }
                } else {
                    let mut count = 0u16;
                    for el in elements {
                        match el {
                            ArrayEl::Hole => self.emit(OpCode::OpPushNull),
                            ArrayEl::Expr(e) => self.compile_expr(e)?,
                            ArrayEl::Spread(_) => unreachable!(),
                        }
                        count += 1;
                    }
                    self.emit1(OpCode::OpBuildArray, count);
                }
            }

            Expr::Object { properties, .. } => {
                let mut in_first_segment = true;
                let mut segment_count = 0u16;

                for prop in properties {
                    match prop {
                        ObjectProp::Property { key, value, .. } => {
                            match key {
                                PropKey::Identifier(s) | PropKey::Str(s) => {
                                    let idx = self.add_str(s);
                                    self.emit1(OpCode::OpPushConst, idx);
                                }
                                PropKey::Int(n) => {
                                    let s = n.to_string();
                                    let idx = self.add_str(&s);
                                    self.emit1(OpCode::OpPushConst, idx);
                                }
                                PropKey::Computed(e) => {
                                    self.compile_expr(e)?;
                                    self.emit(OpCode::OpToString);
                                }
                            }
                            self.compile_expr(value)?;
                            segment_count += 1;
                        }
                        ObjectProp::Method {
                            key,
                            params,
                            body,
                            is_async,
                            is_generator,
                            ..
                        } => {
                            let key_str = match key {
                                PropKey::Identifier(s) | PropKey::Str(s) => {
                                    let idx = self.add_str(s);
                                    self.emit1(OpCode::OpPushConst, idx);
                                    s.clone()
                                }
                                PropKey::Int(n) => {
                                    let s = n.to_string();
                                    let idx = self.add_str(&s);
                                    self.emit1(OpCode::OpPushConst, idx);
                                    s
                                }
                                PropKey::Computed(e) => {
                                    self.compile_expr(e)?;
                                    self.emit(OpCode::OpToString);
                                    "<computed>".to_owned()
                                }
                            };
                            let (proto, upvalues) = super::super::compile_function_with_parent(
                                &key_str,
                                params,
                                body,
                                *is_async,
                                *is_generator,
                                true,
                                self,
                            )?;
                            self.emit_closure(proto, upvalues);
                            segment_count += 1;
                        }
                        ObjectProp::Spread { argument, .. } => {
                            if in_first_segment {
                                self.emit1(OpCode::OpBuildObject, segment_count);
                                in_first_segment = false;
                            } else if segment_count > 0 {
                                self.emit1(OpCode::OpBuildObject, segment_count);
                                self.emit(OpCode::OpObjectRest);
                            }
                            segment_count = 0;

                            self.compile_expr(argument)?;
                            self.emit(OpCode::OpObjectRest);
                        }
                        _ => {}
                    }
                }

                if in_first_segment {
                    self.emit1(OpCode::OpBuildObject, segment_count);
                } else if segment_count > 0 {
                    self.emit1(OpCode::OpBuildObject, segment_count);
                    self.emit(OpCode::OpObjectRest);
                }
            }

            Expr::Function {
                id,
                params,
                body,
                is_async,
                is_generator,
                ..
            } => {
                let name = id.clone().unwrap_or_else(|| "<anonymous>".to_owned());
                let (proto, upvalues) = super::super::compile_function_with_parent(
                    &name,
                    params,
                    body,
                    *is_async,
                    *is_generator,
                    false,
                    self,
                )?;
                self.emit_closure(proto, upvalues);
            }

            Expr::Arrow {
                params,
                body,
                is_async,
                ..
            } => {
                let stmt_body = match body.as_ref() {
                    ArrowBody::Block(s) => s.clone(),
                    ArrowBody::Expr(e) => Stmt::Return {
                        argument: Some(Box::new(e.clone())),
                        range: e.range().clone(),
                    },
                };
                let (proto, upvalues) = super::super::compile_function_with_parent(
                    "<arrow>", params, &stmt_body, *is_async, false, false, self,
                )?;
                self.emit_closure(proto, upvalues);
            }

            Expr::Await { argument, .. } => {
                self.compile_expr(argument)?;
                self.emit(OpCode::OpAwait);
            }

            Expr::Yield {
                argument,
                delegate: _,
                ..
            } => {
                if let Some(val) = argument {
                    self.compile_expr(val)?;
                } else {
                    self.emit(OpCode::OpPushNull);
                }
                self.emit(OpCode::OpYield);
            }

            Expr::Sequence { expressions, .. } => {
                for (i, e) in expressions.iter().enumerate() {
                    self.compile_expr(e)?;
                    if i + 1 < expressions.len() {
                        self.emit(OpCode::OpPop);
                    }
                }
            }

            Expr::Paren { expression, .. } => self.compile_expr(expression)?,

            Expr::Template { parts, .. } => {
                let mut count = 0u16;
                for part in parts {
                    match part {
                        TemplatePart::Literal(s) => {
                            let idx = self.add_str(s);
                            self.emit1(OpCode::OpPushConst, idx);
                        }
                        TemplatePart::Interpolation(e) => {
                            self.compile_expr(e)?;
                            self.emit(OpCode::OpToString);
                        }
                    }
                    count += 1;
                }
                if count > 0 {
                    for _ in 1..count {
                        self.emit(OpCode::OpStrConcat);
                    }
                } else {
                    let idx = self.add_str("");
                    self.emit1(OpCode::OpPushConst, idx);
                }
            }

            Expr::As { expression, .. } => {
                self.compile_expr(expression)?;
            }
            Expr::Satisfies { expression, .. } => {
                self.compile_expr(expression)?;
            }

            Expr::NonNull { expression, .. } => {
                self.compile_expr(expression)?;
                self.emit(OpCode::OpAssertNotNull);
            }

            Expr::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                self.compile_expr(start)?;
                self.compile_expr(end)?;
                let flag = if *inclusive { 1u16 } else { 0u16 };
                let method = self.add_str("__range__");
                self.emit2(OpCode::OpInvokeRuntimeStatic, method, flag);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
