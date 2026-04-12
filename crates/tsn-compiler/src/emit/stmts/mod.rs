mod decls_namespace;
mod stmt_basic;
mod stmt_control;
mod stmt_exceptions;
mod stmt_loops;
use super::Compiler;
use tsn_core::ast::{Program, Stmt};
use tsn_core::OpCode::{OpPushNull, OpReturn};

impl Compiler {
    pub fn compile_program(&mut self, program: &Program) -> Result<(), String> {
        for stmt in &program.body {
            self.compile_stmt(stmt)?;
        }
        self.emit(OpPushNull);
        self.emit(OpReturn);
        Ok(())
    }

    pub(super) fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        self.line = stmt.range().start.line;

        if self.compile_stmt_basic(stmt)? {
            return Ok(());
        }
        if self.compile_stmt_loops(stmt)? {
            return Ok(());
        }
        if self.compile_stmt_control(stmt)? {
            return Ok(());
        }
        if self.compile_stmt_exceptions(stmt)? {
            return Ok(());
        }

        Err("unsupported statement variant".to_owned())
    }
}
