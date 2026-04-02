use std::sync::Arc;
use tsn_core::OpCode;
use tsn_types::value::Value;

impl super::super::Vm {
    pub(super) fn exec_var_op(&mut self, op: OpCode) -> Result<(), String> {
        match op {
            OpCode::OpGetGlobal => {
                let idx = self.read_u16();
                let name = self.get_str_const(idx);
                let v = self
                    .globals
                    .read()
                    .get(name.as_ref())
                    .cloned()
                    .unwrap_or(Value::Null);
                self.push(v);
            }
            OpCode::OpSetGlobal => {
                let idx = self.read_u16();
                let name = self.get_str_const(idx);
                let v = self.stack.last().ok_or("stack underflow")?.clone();
                self.globals.write().insert(Arc::from(name.as_ref()), v);
            }
            OpCode::OpDefineGlobal => {
                let idx = self.read_u16();
                let name = self.get_str_const(idx);
                let v = self.pop()?;
                self.globals.write().insert(Arc::from(name.as_ref()), v);
            }

            OpCode::OpGetLocal => {
                let last_idx = self.frames.len() - 1;
                let frame = &mut self.frames[last_idx];
                let slot = frame.closure.proto.chunk.code[frame.ip] as usize;
                frame.ip += 1;
                let base = frame.base;
                let idx = base + slot;
                if idx >= self.stack.len() {
                    return Err(format!(
                        "stack underflow: get local {} at base {} (stack len {})",
                        slot,
                        base,
                        self.stack.len()
                    ));
                }
                let v = unsafe { self.stack.get_unchecked(idx).clone() };
                self.stack.push(v);
            }
            OpCode::OpSetLocal => {
                let last_idx = self.frames.len() - 1;
                let frame = &mut self.frames[last_idx];
                let slot = frame.closure.proto.chunk.code[frame.ip] as usize;
                frame.ip += 1;
                let base = frame.base;
                let v = self.stack.last().ok_or("stack underflow")?.clone();
                let idx = base + slot;
                if idx >= self.stack.len() {
                    while self.stack.len() <= idx {
                        self.stack.push(tsn_types::Value::Null);
                    }
                }
                unsafe {
                    *self.stack.get_unchecked_mut(idx) = v;
                }
            }
            OpCode::OpSetLocalDrop => {
                let last_idx = self.frames.len() - 1;
                let frame = &mut self.frames[last_idx];
                let slot = frame.closure.proto.chunk.code[frame.ip] as usize;
                frame.ip += 1;
                let base = frame.base;
                let v = self.pop()?;
                let idx = base + slot;
                if idx >= self.stack.len() {
                    while self.stack.len() <= idx {
                        self.stack.push(tsn_types::Value::Null);
                    }
                }
                unsafe {
                    *self.stack.get_unchecked_mut(idx) = v;
                }
            }

            OpCode::OpGetUpvalue => {
                let idx = self.read_u16() as usize;
                let v = {
                    let frame = self.frame();
                    let up = frame.closure.upvalues.get(idx).cloned();
                    if let Some(up) = up {
                        let inner = up.inner.lock();
                        if let Some(loc) = inner.location {
                            self.stack[loc].clone()
                        } else {
                            inner.value.clone()
                        }
                    } else {
                        Value::Null
                    }
                };
                self.push(v);
            }
            OpCode::OpSetUpvalue => {
                let idx = self.read_u16() as usize;
                let v = self.stack.last().ok_or("stack underflow")?.clone();
                let up = self.frame().closure.upvalues.get(idx).cloned();
                if let Some(up) = up {
                    let mut inner = up.inner.lock();
                    if let Some(loc) = inner.location {
                        self.stack[loc] = v;
                    } else {
                        inner.value = v;
                    }
                }
            }
            OpCode::OpCloseUpvalue => {
                let last_idx = self.stack.len() - 1;
                self.close_upvalues_on_stack(last_idx);
                self.pop()?;
            }

            _ => unreachable!("exec_var_op called with non-variable opcode: {:?}", op),
        }
        Ok(())
    }
}
