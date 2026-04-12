use super::assign::compound_op_to_arith;
use super::Compiler;
use crate::chunk::{Literal, PoolEntry};
use tsn_core::ast::{AssignOp, BinaryOp, Expr, LogicalOp, UnaryOp, UpdateOp};
use tsn_core::{NumericKind, OpCode};

impl Compiler {
    pub(super) fn compile_expr_basic_ops(&mut self, expr: &Expr) -> Result<bool, String> {
        match expr {
            Expr::NullLiteral { .. } => self.emit(OpCode::OpPushNull),
            Expr::BoolLiteral { value, .. } => {
                if *value {
                    self.emit(OpCode::OpPushTrue);
                } else {
                    self.emit(OpCode::OpPushFalse);
                }
            }
            Expr::IntLiteral { value, .. } => {
                let idx = self.add_const(PoolEntry::Literal(Literal::Int(*value)));
                self.emit1(OpCode::OpPushConst, idx);
            }
            Expr::FloatLiteral { value, .. } => {
                let idx = self.add_const(PoolEntry::Literal(Literal::Float(*value)));
                self.emit1(OpCode::OpPushConst, idx);
            }
            Expr::BigIntLiteral { raw, .. } => {
                let s = raw.trim_end_matches('n').replace('_', "");
                let num: i128 = if let Some(rest) =
                    s.strip_prefix("0x").or_else(|| s.strip_prefix("0X"))
                {
                    i128::from_str_radix(rest, 16).unwrap_or(0)
                } else if let Some(rest) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
                    i128::from_str_radix(rest, 8).unwrap_or(0)
                } else if let Some(rest) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
                    i128::from_str_radix(rest, 2).unwrap_or(0)
                } else {
                    s.parse().unwrap_or(0)
                };
                let idx = self.add_const(PoolEntry::Literal(Literal::BigInt(num)));
                self.emit1(OpCode::OpPushConst, idx);
            }
            Expr::DecimalLiteral { raw, .. } => {
                use std::str::FromStr;
                let d = rust_decimal::Decimal::from_str(raw.trim_end_matches('d'))
                    .unwrap_or(rust_decimal::Decimal::ZERO);
                let idx = self.add_const(PoolEntry::Literal(Literal::Decimal(d)));
                self.emit1(OpCode::OpPushConst, idx);
            }
            Expr::StrLiteral { value, .. } => {
                let idx = self.add_str(value);
                self.emit1(OpCode::OpPushConst, idx);
            }
            Expr::CharLiteral { value, .. } => {
                let idx = self.add_str(value.to_string());
                self.emit1(OpCode::OpPushConst, idx);
            }
            Expr::RegexLiteral { pattern, flags, .. } => {
                let raw = format!("/{}/{}", pattern, flags);
                let idx = self.add_str(&raw);
                self.emit1(OpCode::OpPushConst, idx);
            }

            Expr::Identifier { name, .. } => {
                self.emit_get_var(name);
            }

            Expr::This { .. } => {
                self.emit_get_var("this");
            }
            Expr::Super { .. } => {
                let idx = self.add_str("super");
                self.emit1(OpCode::OpGetSuper, idx);
            }

            Expr::Unary { op, operand, .. } => {
                self.compile_expr(operand)?;
                match op {
                    UnaryOp::Minus => self.emit(OpCode::OpNegate),
                    UnaryOp::Plus => {}
                    UnaryOp::Not => self.emit(OpCode::OpNot),
                    UnaryOp::BitNot => {
                        let neg1 = self.add_const(PoolEntry::Literal(Literal::Int(-1)));
                        self.emit1(OpCode::OpPushConst, neg1);
                        self.emit(OpCode::OpBitXor);
                    }
                    UnaryOp::Typeof => self.emit(OpCode::OpTypeof),
                }
            }

            Expr::Update {
                op,
                prefix,
                operand,
                ..
            } => match operand.as_ref() {
                Expr::Identifier { name, .. } => {
                    self.emit_get_var(name);
                    if !prefix {
                        self.emit(OpCode::OpDup);
                    }
                    let one = self.add_const(PoolEntry::Literal(Literal::Int(1)));
                    self.emit1(OpCode::OpPushConst, one);
                    match op {
                        UpdateOp::Increment => self.emit(OpCode::OpAdd),
                        UpdateOp::Decrement => self.emit(OpCode::OpSub),
                    }
                    self.emit_set_var(name);
                    if !prefix {
                        self.emit(OpCode::OpPop);
                    }
                }
                _ => {
                    self.compile_expr(operand)?;
                }
            },

            Expr::Binary {
                op,
                left,
                right,
                range,
            } => {
                self.compile_expr(left)?;
                self.compile_expr(right)?;
                match op {
                    BinaryOp::Add => {
                        let op = match self.lookup_numeric(range.start.offset) {
                            Some(NumericKind::Int) => OpCode::OpAddI32,
                            Some(NumericKind::Float) => OpCode::OpAddF64,
                            None => OpCode::OpAdd,
                        };
                        self.emit(op)
                    }
                    BinaryOp::Sub => {
                        let op = match self.lookup_numeric(range.start.offset) {
                            Some(NumericKind::Int) => OpCode::OpSubI32,
                            Some(NumericKind::Float) => OpCode::OpSubF64,
                            None => OpCode::OpSub,
                        };
                        self.emit(op)
                    }
                    BinaryOp::Mul => {
                        let op = match self.lookup_numeric(range.start.offset) {
                            Some(NumericKind::Int) => OpCode::OpMulI32,
                            Some(NumericKind::Float) => OpCode::OpMulF64,
                            None => OpCode::OpMul,
                        };
                        self.emit(op)
                    }
                    BinaryOp::Div => {
                        let op = match self.lookup_numeric(range.start.offset) {
                            Some(NumericKind::Float) | Some(NumericKind::Int) => OpCode::OpDivF64,
                            None => OpCode::OpDiv,
                        };
                        self.emit(op)
                    }
                    BinaryOp::Mod => self.emit(OpCode::OpMod),
                    BinaryOp::Pow => self.emit(OpCode::OpPow),
                    BinaryOp::Eq => self.emit(OpCode::OpEq),
                    BinaryOp::NotEq => self.emit(OpCode::OpNeq),
                    BinaryOp::Lt => self.emit(OpCode::OpLt),
                    BinaryOp::LtEq => self.emit(OpCode::OpLte),
                    BinaryOp::Gt => self.emit(OpCode::OpGt),
                    BinaryOp::GtEq => self.emit(OpCode::OpGte),
                    BinaryOp::BitAnd => self.emit(OpCode::OpBitAnd),
                    BinaryOp::BitOr => self.emit(OpCode::OpBitOr),
                    BinaryOp::BitXor => self.emit(OpCode::OpBitXor),
                    BinaryOp::Shl => self.emit(OpCode::OpShl),
                    BinaryOp::Shr => self.emit(OpCode::OpShr),
                    BinaryOp::UShr => self.emit(OpCode::OpUshr),
                    BinaryOp::Instanceof => self.emit(OpCode::OpInstanceof),
                    BinaryOp::In => self.emit(OpCode::OpIn),
                }
            }

            Expr::Logical {
                op, left, right, ..
            } => {
                self.compile_expr(left)?;
                match op {
                    LogicalOp::And => {
                        let skip = self.emit_jump(OpCode::OpJumpIfFalse);
                        self.emit(OpCode::OpPop);
                        self.compile_expr(right)?;
                        self.patch_jump(skip);
                    }
                    LogicalOp::Or => {
                        let skip = self.emit_jump(OpCode::OpJumpIfTrue);
                        self.emit(OpCode::OpPop);
                        self.compile_expr(right)?;
                        self.patch_jump(skip);
                    }
                    LogicalOp::Nullish => {
                        self.emit(OpCode::OpDup);
                        self.emit(OpCode::OpIsNull);
                        let null_jump = self.emit_jump(OpCode::OpJumpIfTrue);
                        self.emit(OpCode::OpPop);
                        let end_jump = self.emit_jump(OpCode::OpJump);
                        self.patch_jump(null_jump);
                        self.emit(OpCode::OpPop);
                        self.emit(OpCode::OpPop);
                        self.compile_expr(right)?;
                        self.patch_jump(end_jump);
                    }
                }
            }

            Expr::Conditional {
                test,
                consequent,
                alternate,
                ..
            } => {
                self.compile_expr(test)?;
                let else_jump = self.emit_jump(OpCode::OpJumpIfFalse);
                self.emit(OpCode::OpPop);
                self.compile_expr(consequent)?;
                let end_jump = self.emit_jump(OpCode::OpJump);
                self.patch_jump(else_jump);
                self.emit(OpCode::OpPop);
                self.compile_expr(alternate)?;
                self.patch_jump(end_jump);
            }

            Expr::Assign {
                op, target, value, ..
            } => match op {
                AssignOp::Assign => {
                    self.compile_expr(value)?;
                    self.emit_assign_target(target)?;
                }

                AssignOp::AndAssign => {
                    self.compile_expr(target)?;
                    self.emit(OpCode::OpDup);
                    let skip = self.emit_jump(OpCode::OpJumpIfFalse);
                    self.emit(OpCode::OpPop);
                    self.compile_expr(value)?;
                    self.emit(OpCode::OpDup);
                    self.emit_assign_target(target)?;
                    self.patch_jump(skip);
                }
                AssignOp::OrAssign => {
                    self.compile_expr(target)?;
                    self.emit(OpCode::OpDup);
                    let skip = self.emit_jump(OpCode::OpJumpIfTrue);
                    self.emit(OpCode::OpPop);
                    self.compile_expr(value)?;
                    self.emit(OpCode::OpDup);
                    self.emit_assign_target(target)?;
                    self.patch_jump(skip);
                }
                AssignOp::NullishAssign => {
                    self.compile_expr(target)?;
                    self.emit(OpCode::OpDup);
                    self.emit(OpCode::OpIsNull);
                    let is_null = self.emit_jump(OpCode::OpJumpIfFalse);
                    self.emit(OpCode::OpPop);
                    self.compile_expr(value)?;
                    self.emit(OpCode::OpDup);
                    self.emit_assign_target(target)?;
                    self.patch_jump(is_null);
                }

                _ => {
                    let arith_op =
                        compound_op_to_arith(*op).expect("unhandled compound assignment operator");
                    self.compile_expr(target)?;
                    self.compile_expr(value)?;
                    self.emit(arith_op);
                    self.emit_assign_target(target)?;
                }
            },
            _ => return Ok(false),
        }
        Ok(true)
    }
}
