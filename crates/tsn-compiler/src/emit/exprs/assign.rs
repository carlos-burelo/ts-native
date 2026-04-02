use super::super::Compiler;
use tsn_core::ast::{Arg, AssignOp, Expr};
use tsn_core::OpCode;

impl Compiler {
    pub(super) fn emit_assign_target(&mut self, target: &Expr) -> Result<(), String> {
        match target {
            Expr::Identifier { name, .. } => {
                self.emit_set_var(name);
            }
            Expr::Member {
                object,
                property,
                computed,
                ..
            } => {
                if !*computed {
                    if matches!(object.as_ref(), Expr::This { .. }) {
                        if let Expr::Identifier { name: prop, .. } = property.as_ref() {
                            if self.class_field_layout_valid {
                                if let Some(slot) = self
                                    .class_field_layout
                                    .as_ref()
                                    .and_then(|fl| fl.get(prop.as_str()))
                                    .copied()
                                {
                                    self.compile_expr(object)?;
                                    self.emit(OpCode::OpSwap);
                                    self.emit1(OpCode::OpSetFixedField, slot as u16);
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
                self.compile_expr(object)?;
                if *computed {
                    self.compile_expr(property)?;
                    self.emit(OpCode::OpRot3);
                    self.emit(OpCode::OpSetIndex);
                } else {
                    if let Some(mangled) = self
                        .lookup_extension_set_member(target.range().start.offset)
                        .map(|s| s.to_owned())
                    {
                        self.emit_get_var(&mangled);
                        self.emit(OpCode::OpRot);
                        self.emit(OpCode::OpRot3);
                        self.emit1(OpCode::OpCall, 2);
                        return Ok(());
                    }
                    let name = match property.as_ref() {
                        Expr::Identifier { name, .. } => name.clone(),
                        _ => return Err("non-identifier in member assign".to_owned()),
                    };
                    let idx = self.add_str(&name);
                    let cs = self.alloc_cache_slot();
                    self.emit(OpCode::OpSwap);
                    self.emit2(OpCode::OpSetProperty, idx, cs);
                }
            }
            _ => {
                self.emit(OpCode::OpPop);
            }
        }
        Ok(())
    }

    pub(crate) fn compile_args(&mut self, args: &[Arg]) -> Result<(u16, bool), String> {
        let mut count = 0u16;
        let mut has_spread = false;
        for arg in args {
            match arg {
                Arg::Positional(e) => self.compile_expr(e)?,
                Arg::Spread(e) => {
                    self.compile_expr(e)?;
                    self.emit(OpCode::OpWrapSpread);
                    has_spread = true;
                }
                Arg::Named { value, .. } => self.compile_expr(value)?,
            }
            count += 1;
        }
        Ok((count, has_spread))
    }
}

pub(super) fn compound_op_to_arith(op: AssignOp) -> Option<OpCode> {
    match op {
        AssignOp::AddAssign => Some(OpCode::OpAdd),
        AssignOp::SubAssign => Some(OpCode::OpSub),
        AssignOp::MulAssign => Some(OpCode::OpMul),
        AssignOp::DivAssign => Some(OpCode::OpDiv),
        AssignOp::ModAssign => Some(OpCode::OpMod),
        AssignOp::PowAssign => Some(OpCode::OpPow),
        AssignOp::BitAndAssign => Some(OpCode::OpBitAnd),
        AssignOp::BitOrAssign => Some(OpCode::OpBitOr),
        AssignOp::BitXorAssign => Some(OpCode::OpBitXor),
        AssignOp::ShlAssign => Some(OpCode::OpShl),
        AssignOp::ShrAssign => Some(OpCode::OpShr),
        AssignOp::UShrAssign => Some(OpCode::OpUshr),
        AssignOp::AndAssign | AssignOp::OrAssign | AssignOp::NullishAssign => None,
        AssignOp::Assign => None,
    }
}
