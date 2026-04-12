use super::Compiler;
use tsn_core::ast::Stmt;
use tsn_core::OpCode;

impl Compiler {
    pub(super) fn compile_stmt_exceptions(&mut self, stmt: &Stmt) -> Result<bool, String> {
        match stmt {
            Stmt::Return { argument, .. } => {
                if let Some(val) = argument {
                    self.compile_expr(val)?;
                } else {
                    self.emit(OpCode::OpPushNull);
                }
                // Inline pending finally blocks (innermost → outermost) and pop
                // try handlers before leaving the function frame early.
                let try_depth = self.try_stack.len();
                let stashed: Vec<Option<Stmt>> = self.try_stack.clone();
                for entry in stashed.iter().rev() {
                    if let Some(fin_stmt) = entry {
                        self.compile_stmt(fin_stmt)?;
                    }
                }
                for _ in 0..try_depth {
                    self.emit(OpCode::OpPopTry);
                }
                self.emit(OpCode::OpReturn);
            }

            Stmt::Break { .. } => {
                if let Some(ctx) = self.loop_stack.last() {
                    let to_pop = self.scope.local_count() - ctx.locals_before_hidden;
                    for _ in 0..to_pop {
                        self.emit(OpCode::OpPop);
                    }
                    let p = self.chunk.emit_jump(OpCode::OpJump, self.line);
                    self.loop_stack.last_mut().unwrap().break_patches.push(p);
                }
            }

            Stmt::Continue { .. } => {
                if let Some(ctx) = self.loop_stack.last() {
                    let to_pop = self.scope.local_count() - ctx.locals_at_body_start;
                    for _ in 0..to_pop {
                        self.emit(OpCode::OpPop);
                    }
                    let p = self.chunk.emit_jump(OpCode::OpJump, self.line);
                    self.loop_stack.last_mut().unwrap().continue_patches.push(p);
                }
            }

            Stmt::Throw { argument, .. } => {
                self.compile_expr(argument)?;
                self.emit(OpCode::OpThrow);
            }

            Stmt::Try {
                block,
                catch,
                finally,
                ..
            } => {
                // Push the finally body (if any) so that early returns inside the
                // try block can inline it before the OpReturn.
                self.try_stack.push(finally.as_deref().cloned());
                let try_start = self.emit_jump(OpCode::OpTry);
                self.compile_stmt(block)?;
                self.try_stack.pop();
                self.emit(OpCode::OpPopTry);
                let finally_jump = self.emit_jump(OpCode::OpJump);
                self.patch_jump(try_start);

                if let Some(catch_clause) = catch {
                    self.scope.push_block();
                    if let Some(param) = &catch_clause.param {
                        self.declare_pattern_local(param)?;
                    } else {
                        self.emit(OpCode::OpPop);
                    }
                    self.compile_stmt(&catch_clause.body)?;
                    let (_count, captured) = self.scope.pop_block();
                    for is_cap in captured {
                        if is_cap {
                            self.emit(OpCode::OpCloseUpvalue);
                        } else {
                            self.emit(OpCode::OpPop);
                        }
                    }
                } else {
                    self.emit(OpCode::OpPop);
                }

                self.patch_jump(finally_jump);

                if let Some(fin) = finally {
                    self.compile_stmt(fin)?;
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
