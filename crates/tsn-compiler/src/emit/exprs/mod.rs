mod assign;
mod basic_ops;
mod member_call;
mod structural_advanced;
mod structural_core;

use super::Compiler;
use tsn_core::ast::Expr;
use tsn_core::OpCode;

impl Compiler {
    fn emit_call_opcode(&mut self, arg_count: u16, has_spread: bool) {
        self.emit1(
            if has_spread {
                OpCode::OpCallSpread
            } else {
                OpCode::OpCall
            },
            arg_count,
        );
    }

    pub fn compile_expr(&mut self, expr: &Expr) -> Result<(), String> {
        self.line = expr.range().start.line;

        if self.compile_expr_basic_ops(expr)? {
            return Ok(());
        }
        if self.compile_expr_member_call(expr)? {
            return Ok(());
        }
        if self.compile_expr_structural_core(expr)? {
            return Ok(());
        }
        if self.compile_expr_structural_advanced(expr)? {
            return Ok(());
        }

        Err("unsupported expression variant".to_owned())
    }
}
