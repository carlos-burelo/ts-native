use super::Compiler;
use crate::chunk::{Literal, PoolEntry};
use crate::scope::LoopContext;
use tsn_core::ast::{ForInit, Stmt};
use tsn_core::OpCode;

impl Compiler {
    pub(super) fn compile_stmt_loops(&mut self, stmt: &Stmt) -> Result<bool, String> {
        match stmt {
            Stmt::While { test, body, .. } => {
                let loop_start = self.chunk_len();
                self.compile_expr(test)?;
                let exit_jump = self.emit_jump(OpCode::OpJumpIfFalse);
                self.emit(OpCode::OpPop);

                let locals_base = self.scope.local_count();
                let ctx = LoopContext {
                    break_patches: vec![],
                    continue_patches: vec![],
                    locals_before_hidden: locals_base,
                    locals_at_body_start: locals_base,
                };
                self.loop_stack.push(ctx);
                self.compile_stmt(body)?;
                let ctx = self.loop_stack.pop().unwrap();

                for p in ctx.continue_patches {
                    self.patch_jump(p);
                }
                self.emit_loop(loop_start);
                self.patch_jump(exit_jump);
                self.emit(OpCode::OpPop);

                for p in ctx.break_patches {
                    self.patch_jump(p);
                }
            }

            Stmt::DoWhile { body, test, .. } => {
                let loop_start = self.chunk_len();
                let locals_base = self.scope.local_count();
                let ctx = LoopContext {
                    break_patches: vec![],
                    continue_patches: vec![],
                    locals_before_hidden: locals_base,
                    locals_at_body_start: locals_base,
                };
                self.loop_stack.push(ctx);
                self.compile_stmt(body)?;
                let ctx = self.loop_stack.pop().unwrap();

                for p in ctx.continue_patches {
                    self.patch_jump(p);
                }
                self.compile_expr(test)?;
                let exit_jump = self.emit_jump(OpCode::OpJumpIfFalse);
                self.emit(OpCode::OpPop);
                self.emit_loop(loop_start);
                self.patch_jump(exit_jump);
                self.emit(OpCode::OpPop);

                for p in ctx.break_patches {
                    self.patch_jump(p);
                }
            }

            Stmt::For {
                init,
                test,
                update,
                body,
                ..
            } => {
                let locals_before_for = self.scope.local_count();
                self.scope.push_block();

                if let Some(init) = init {
                    match init.as_ref() {
                        ForInit::Var {
                            kind: _,
                            declarators,
                        } => {
                            for d in declarators {
                                if let Some(init_expr) = &d.init {
                                    self.compile_expr(init_expr)?;
                                } else {
                                    self.emit(OpCode::OpPushNull);
                                }
                                self.declare_pattern_local(&d.id)?;
                            }
                        }
                        ForInit::Expr(expr) => {
                            self.compile_expr(expr)?;
                            self.emit(OpCode::OpPop);
                        }
                    }
                }

                let loop_start = self.chunk_len();
                let mut exit_jump = None;
                if let Some(test_expr) = test {
                    self.compile_expr(test_expr)?;
                    let j = self.emit_jump(OpCode::OpJumpIfFalse);
                    exit_jump = Some(j);
                    self.emit(OpCode::OpPop);
                }

                let locals_after_init = self.scope.local_count();
                let ctx = LoopContext {
                    break_patches: vec![],
                    continue_patches: vec![],
                    locals_before_hidden: locals_before_for,
                    locals_at_body_start: locals_after_init,
                };
                self.loop_stack.push(ctx);
                self.compile_stmt(body)?;
                let ctx = self.loop_stack.pop().unwrap();

                for p in ctx.continue_patches {
                    self.patch_jump(p);
                }

                if let Some(update_expr) = update {
                    self.compile_expr(update_expr)?;
                    self.emit(OpCode::OpPop);
                }

                self.emit_loop(loop_start);

                if let Some(j) = exit_jump {
                    self.patch_jump(j);
                    self.emit(OpCode::OpPop);
                }

                let (_count, captured) = self.scope.pop_block();
                for is_cap in captured {
                    if is_cap {
                        self.emit(OpCode::OpCloseUpvalue);
                    } else {
                        self.emit(OpCode::OpPop);
                    }
                }

                for p in ctx.break_patches {
                    self.patch_jump(p);
                }
            }

            Stmt::ForIn {
                kind: _,
                left,
                right,
                body,
                ..
            } => {
                self.compile_expr(right)?;
                self.emit(OpCode::OpObjectKeys);

                let arr_slot = self.scope.declare_local("__for_keys__");

                let zero = self.add_const(PoolEntry::Literal(Literal::Int(0)));
                self.emit1(OpCode::OpPushConst, zero);
                let idx_slot = self.scope.declare_local("__for_kidx__");

                let loop_start = self.chunk_len();
                self.emit1(OpCode::OpGetLocal, idx_slot);
                self.emit1(OpCode::OpGetLocal, arr_slot);
                let length_key = self.add_str("length");
                let cs = self.alloc_cache_slot();
                self.emit2(OpCode::OpGetProperty, length_key, cs);
                self.emit(OpCode::OpLt);
                let exit_jump = self.emit_jump(OpCode::OpJumpIfFalse);
                self.emit(OpCode::OpPop);

                self.emit1(OpCode::OpGetLocal, arr_slot);
                self.emit1(OpCode::OpGetLocal, idx_slot);
                self.emit(OpCode::OpGetIndex);

                let scope_was = self.scope.local_count();
                self.assign_pattern(left)?;

                let ctx = LoopContext {
                    break_patches: vec![],
                    continue_patches: vec![],

                    locals_before_hidden: arr_slot as usize,

                    locals_at_body_start: scope_was,
                };
                self.loop_stack.push(ctx);
                self.compile_stmt(body)?;
                let ctx = self.loop_stack.pop().unwrap();

                let added = self.scope.local_count() - scope_was;
                for _ in 0..added {
                    self.emit(OpCode::OpPop);
                }

                for p in ctx.continue_patches {
                    self.patch_jump(p);
                }

                self.emit1(OpCode::OpGetLocal, idx_slot);
                let one = self.add_const(PoolEntry::Literal(Literal::Int(1)));
                self.emit1(OpCode::OpPushConst, one);
                self.emit(OpCode::OpAdd);
                self.emit1(OpCode::OpSetLocalDrop, idx_slot);

                self.emit_loop(loop_start);

                self.patch_jump(exit_jump);
                self.emit(OpCode::OpPop);
                self.emit(OpCode::OpPop);
                self.emit(OpCode::OpPop);

                self.scope.locals.truncate(arr_slot as usize);

                for p in ctx.break_patches {
                    self.patch_jump(p);
                }
            }

            Stmt::ForOf {
                kind: _,
                left,
                right,
                body,
                is_await,
                ..
            } => {
                use tsn_types::value::SymbolKind;

                self.compile_expr(right)?;
                let iterable_slot = self.scope.declare_local("__for_iterable__");

                self.emit1(OpCode::OpGetLocal, iterable_slot);
                self.emit1(OpCode::OpGetLocal, iterable_slot);
                let symbol_kind = if *is_await {
                    SymbolKind::AsyncIterator
                } else {
                    SymbolKind::Iterator
                };
                let sym_idx = self.chunk.add_symbol(symbol_kind);
                self.emit1(OpCode::OpGetSymbol, sym_idx);
                self.emit(OpCode::OpSwap);
                self.emit1(OpCode::OpCall, 1);
                let iterator_slot = self.scope.declare_local("__for_iterator__");

                let loop_start = self.chunk_len();

                self.emit1(OpCode::OpGetLocal, iterator_slot);
                self.emit1(OpCode::OpGetLocal, iterator_slot);
                let next_key = self.add_str("next");
                let cs_next = self.alloc_cache_slot();
                self.emit2(OpCode::OpGetProperty, next_key, cs_next);
                self.emit(OpCode::OpSwap);
                self.emit1(OpCode::OpCall, 1);

                if *is_await {
                    self.emit(OpCode::OpAwait);
                }

                self.emit(OpCode::OpDup);

                let done_key = self.add_str("done");
                let cs_done = self.alloc_cache_slot();
                self.emit2(OpCode::OpGetProperty, done_key, cs_done);

                let exit_jump = self.emit_jump(OpCode::OpJumpIfTrue);

                self.emit(OpCode::OpPop);

                let value_key = self.add_str("value");
                let cs_val = self.alloc_cache_slot();
                self.emit2(OpCode::OpGetProperty, value_key, cs_val);

                self.scope.push_block();
                let scope_was = self.scope.local_count();
                self.assign_pattern(left)?;

                let ctx = LoopContext {
                    break_patches: vec![],
                    continue_patches: vec![],
                    locals_before_hidden: iterable_slot as usize,
                    locals_at_body_start: scope_was,
                };
                self.loop_stack.push(ctx);
                self.compile_stmt(body)?;
                let ctx = self.loop_stack.pop().unwrap();

                let (_count, captured) = self.scope.pop_block();
                for is_cap in captured {
                    if is_cap {
                        self.emit(OpCode::OpCloseUpvalue);
                    } else {
                        self.emit(OpCode::OpPop);
                    }
                }

                for p in ctx.continue_patches {
                    self.patch_jump(p);
                }

                self.emit_loop(loop_start);

                self.patch_jump(exit_jump);
                self.emit(OpCode::OpPop);
                self.emit(OpCode::OpPop);

                self.scope.locals.truncate(iterable_slot as usize);
                self.emit(OpCode::OpPop);
                self.emit(OpCode::OpPop);

                for p in ctx.break_patches {
                    self.patch_jump(p);
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
