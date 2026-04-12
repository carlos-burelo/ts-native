use super::Compiler;
use tsn_core::ast::Stmt;
use tsn_core::OpCode;

impl Compiler {
    pub(super) fn compile_stmt_basic(&mut self, stmt: &Stmt) -> Result<bool, String> {
        match stmt {
            Stmt::Empty { .. } => {}

            Stmt::Expr { expression, .. } => {
                self.compile_expr(expression)?;
                self.emit_smart_pop();
            }

            Stmt::Block { stmts, .. } => {
                self.scope.push_block();
                for s in stmts {
                    self.compile_stmt(s)?;
                }
                self.emit_dispose_cleanup()?;
                let (_count, captured) = self.scope.pop_block();
                for is_cap in captured {
                    if is_cap {
                        self.emit(OpCode::OpCloseUpvalue);
                    } else {
                        self.emit(OpCode::OpPop);
                    }
                }
            }

            Stmt::Decl(decl) => self.compile_decl(decl)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}
