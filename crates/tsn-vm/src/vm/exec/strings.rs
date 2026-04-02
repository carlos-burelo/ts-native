use std::fmt::Write as FmtWrite;
use std::sync::Arc;
use tsn_core::OpCode;
use tsn_types::value::Value;

impl super::super::Vm {
    pub(super) fn exec_string_op(&mut self, op: OpCode) -> Result<(), String> {
        match op {
            OpCode::OpStrConcat => {
                let b = self.stack.pop().ok_or("stack underflow")?;
                let a = self.stack.pop().ok_or("stack underflow")?;
                let s = match (a, b) {
                    (Value::Str(sa), Value::Str(sb)) => {
                        let mut s = String::with_capacity(sa.len() + sb.len());
                        s.push_str(&sa);
                        s.push_str(&sb);
                        s
                    }
                    (Value::Str(sa), other) => {
                        let mut s = sa.to_string();
                        write!(&mut s, "{}", other).unwrap();
                        s
                    }
                    (other, Value::Str(sb)) => {
                        let mut s = other.to_string();
                        s.push_str(&sb);
                        s
                    }
                    (a, b) => format!("{}{}", a, b),
                };
                self.stack.push(Value::Str(Arc::from(s)));
            }
            OpCode::OpToString => {
                let v = self.pop()?;
                self.push(Value::Str(Arc::from(v.to_string())));
            }
            OpCode::OpStrLength => {
                let v = self.pop()?;
                let len = match &v {
                    Value::Str(s) => {
                        if s.is_ascii() {
                            s.len() as i64
                        } else {
                            s.chars().count() as i64
                        }
                    }
                    _ => return Err(format!("expected string, got {}", v.type_name())),
                };
                self.push(Value::Int(len));
            }
            OpCode::OpStrSlice => {
                let end = self.pop()?;
                let start = self.pop()?;
                let s = self.pop()?;
                match (&s, &start, &end) {
                    (Value::Str(s), Value::Int(a), Value::Int(b)) => {
                        let chars: Vec<char> = s.chars().collect();
                        let from = (*a as usize).min(chars.len());
                        let to = (*b as usize).min(chars.len());
                        let slice: String = chars[from..to].iter().collect();
                        self.push(Value::Str(Arc::from(slice)));
                    }
                    _ => self.push(Value::Str(Arc::from(""))),
                }
            }

            _ => unreachable!("exec_string_op called with non-string opcode: {:?}", op),
        }
        Ok(())
    }
}
