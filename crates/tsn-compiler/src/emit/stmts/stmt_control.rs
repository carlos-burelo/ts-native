use super::Compiler;
use tsn_core::ast::{Pattern, Stmt};
use tsn_core::OpCode;

impl Compiler {
    pub(super) fn compile_stmt_control(&mut self, stmt: &Stmt) -> Result<bool, String> {
        match stmt {
            Stmt::If {
                test,
                consequent,
                alternate,
                ..
            } => {
                self.compile_expr(test)?;
                let then_jump = self.emit_jump(OpCode::OpJumpIfFalse);
                self.emit(OpCode::OpPop);
                self.compile_stmt(consequent)?;

                if let Some(alt) = alternate {
                    let else_jump = self.emit_jump(OpCode::OpJump);
                    self.patch_jump(then_jump);
                    self.emit(OpCode::OpPop);
                    self.compile_stmt(alt)?;
                    self.patch_jump(else_jump);
                } else {
                    let over_jump = self.emit_jump(OpCode::OpJump);
                    self.patch_jump(then_jump);
                    self.emit(OpCode::OpPop);
                    self.patch_jump(over_jump);
                }
            }

            Stmt::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.compile_expr(discriminant)?;
                let exit_jumps: Vec<usize> = vec![];
                let mut next_jumps: Vec<usize> = vec![];

                for case in cases {
                    for p in next_jumps.drain(..) {
                        self.patch_jump(p);
                    }

                    if let Some(test) = &case.test {
                        self.emit(OpCode::OpDup);
                        self.compile_expr(test)?;
                        self.emit(OpCode::OpEq);
                        let skip = self.emit_jump(OpCode::OpJumpIfFalse);
                        self.emit(OpCode::OpPop);
                        next_jumps.push(skip);
                    }

                    for s in &case.body {
                        self.compile_stmt(s)?;
                    }
                }

                for p in next_jumps {
                    self.patch_jump(p);
                }
                self.emit(OpCode::OpPop);

                for p in exit_jumps {
                    self.patch_jump(p);
                }
            }

            Stmt::Labeled { body, .. } => {
                self.compile_stmt(body)?;
            }

            Stmt::Using {
                declarations,
                is_await,
                ..
            } => {
                for d in declarations {
                    if let Some(init) = &d.init {
                        self.compile_expr(init)?;
                    } else {
                        self.emit(OpCode::OpPushNull);
                    }
                    let name = match &d.id {
                        Pattern::Identifier { name, .. } => name.clone(),
                        _ => {
                            return Err("using only supports simple identifier patterns".to_owned())
                        }
                    };
                    let slot = self.scope.declare_local(&name);
                    self.scope
                        .disposables
                        .push((slot, *is_await, self.scope.depth));
                }
            }

            Stmt::Debugger { .. } => {}
            _ => return Ok(false),
        }
        Ok(true)
    }
}
