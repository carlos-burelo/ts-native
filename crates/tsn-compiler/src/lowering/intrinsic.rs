use crate::emit::Compiler;
use tsn_core::ast::{Arg, Expr};
use tsn_core::{resolve_intrinsic, OpCode};

impl Compiler {
    pub(crate) fn try_lower_intrinsic(
        &mut self,
        callee: &Expr,
        args: &[Arg],
    ) -> Result<bool, String> {
        if let Expr::Identifier { name, .. } = callee {
            if let Some(id) = resolve_intrinsic(name) {
                let (arg_count, has_spread) = self.compile_args(args)?;
                self.emit2(
                    if has_spread {
                        OpCode::OpCallIntrinsicSpread
                    } else {
                        OpCode::OpCallIntrinsic
                    },
                    id as u16,
                    arg_count,
                );
                return Ok(true);
            }
        }
        Ok(false)
    }
}
