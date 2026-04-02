use super::super::math::{div_op, negate, numeric_op, pow_op, to_i64};
use tsn_core::OpCode;
use tsn_types::value::Value;

impl super::super::Vm {
    pub(super) fn exec_arith_op(&mut self, op: OpCode) -> Result<(), String> {
        match op {
            OpCode::OpAdd => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(a.num_add(&b)?);
            }
            OpCode::OpAddI32 => {
                let b = self.stack.pop().ok_or("stack underflow")?;
                let a = self.stack.pop().ok_or("stack underflow")?;
                match (a, b) {
                    (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Int(x + y)),
                    (a, b) => self.stack.push(a.num_add(&b)?),
                }
            }
            OpCode::OpAddF64 => {
                let b = self.stack.pop().ok_or("stack underflow")?;
                let a = self.stack.pop().ok_or("stack underflow")?;
                match (a, b) {
                    (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Float(x + y)),
                    (a, b) => self.stack.push(a.num_add(&b)?),
                }
            }

            OpCode::OpSub => {
                let b = self.stack.pop().ok_or("stack underflow")?;
                let a = self.stack.pop().ok_or("stack underflow")?;
                self.stack
                    .push(numeric_op(a, b, |x, y| x - y, |x, y| x - y, |x, y| x - y)?);
            }
            OpCode::OpSubI32 => {
                let b = self.stack.pop().ok_or("stack underflow")?;
                let a = self.stack.pop().ok_or("stack underflow")?;
                match (a, b) {
                    (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Int(x - y)),
                    (a, b) => {
                        self.stack
                            .push(numeric_op(a, b, |x, y| x - y, |x, y| x - y, |x, y| x - y)?)
                    }
                }
            }
            OpCode::OpSubF64 => {
                let b = self.stack.pop().ok_or("stack underflow")?;
                let a = self.stack.pop().ok_or("stack underflow")?;
                match (a, b) {
                    (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Float(x - y)),
                    (a, b) => {
                        self.stack
                            .push(numeric_op(a, b, |x, y| x - y, |x, y| x - y, |x, y| x - y)?)
                    }
                }
            }

            OpCode::OpMul => {
                let b = self.stack.pop().ok_or("stack underflow")?;
                let a = self.stack.pop().ok_or("stack underflow")?;
                self.stack
                    .push(numeric_op(a, b, |x, y| x * y, |x, y| x * y, |x, y| x * y)?);
            }
            OpCode::OpMulI32 => {
                let b = self.stack.pop().ok_or("stack underflow")?;
                let a = self.stack.pop().ok_or("stack underflow")?;
                match (a, b) {
                    (Value::Int(x), Value::Int(y)) => self.stack.push(Value::Int(x * y)),
                    (a, b) => {
                        self.stack
                            .push(numeric_op(a, b, |x, y| x * y, |x, y| x * y, |x, y| x * y)?)
                    }
                }
            }
            OpCode::OpMulF64 => {
                let b = self.stack.pop().ok_or("stack underflow")?;
                let a = self.stack.pop().ok_or("stack underflow")?;
                match (a, b) {
                    (Value::Float(x), Value::Float(y)) => self.stack.push(Value::Float(x * y)),
                    (a, b) => {
                        self.stack
                            .push(numeric_op(a, b, |x, y| x * y, |x, y| x * y, |x, y| x * y)?)
                    }
                }
            }

            OpCode::OpDiv => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(div_op(a, b)?);
            }
            OpCode::OpDivI32 => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(div_op(a, b)?);
            }
            OpCode::OpDivF64 => {
                let b = self.pop()?;
                let a = self.pop()?;
                let af = match &a {
                    Value::Float(f) => *f,
                    Value::Int(i) => *i as f64,
                    _ => return Err(format!("OpDivF64: expected numeric, got {}", a)),
                };
                let bf = match &b {
                    Value::Float(f) => *f,
                    Value::Int(i) => *i as f64,
                    _ => return Err(format!("OpDivF64: expected numeric, got {}", b)),
                };
                self.push(Value::Float(af / bf));
            }

            OpCode::OpMod => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(numeric_op(a, b, |x, y| x % y, |x, y| x % y, |x, y| x % y)?);
            }
            OpCode::OpPow => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(pow_op(a, b)?);
            }
            OpCode::OpNegate => {
                let v = self.pop()?;
                self.push(negate(v)?);
            }

            OpCode::OpBitAnd => {
                let b = to_i64(self.pop()?)?;
                let a = to_i64(self.pop()?)?;
                self.push(Value::Int(a & b));
            }
            OpCode::OpBitOr => {
                let b = to_i64(self.pop()?)?;
                let a = to_i64(self.pop()?)?;
                self.push(Value::Int(a | b));
            }
            OpCode::OpBitXor => {
                let b = to_i64(self.pop()?)?;
                let a = to_i64(self.pop()?)?;
                self.push(Value::Int(a ^ b));
            }
            OpCode::OpShl => {
                let b = to_i64(self.pop()?)? as u32;
                let a = to_i64(self.pop()?)?;
                self.push(Value::Int((a << (b & 31)) as i64));
            }
            OpCode::OpShr => {
                let b = to_i64(self.pop()?)? as u32;
                let a = to_i64(self.pop()?)?;
                self.push(Value::Int(a >> (b & 31)));
            }
            OpCode::OpUshr => {
                let b = to_i64(self.pop()?)? as u32;
                let a = to_i64(self.pop()?)? as u64;
                self.push(Value::Int((a >> (b & 31)) as i64));
            }

            _ => unreachable!("exec_arith_op called with non-arithmetic opcode: {:?}", op),
        }
        Ok(())
    }
}
