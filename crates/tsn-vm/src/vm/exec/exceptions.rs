use tsn_core::OpCode;

impl super::super::Vm {
    pub(super) fn exec_exception_op(&mut self, op: OpCode) -> Result<(), String> {
        match op {
            OpCode::OpTry => {
                let offset = self.read_u16() as usize;
                let catch_ip = self.frame().ip + offset;
                self.try_handlers.push(super::super::frame::TryEntry {
                    catch_ip,
                    frame_depth: self.frames.len(),
                    stack_depth: self.stack.len(),
                });
            }
            OpCode::OpPopTry => {
                self.try_handlers.pop();
            }
            OpCode::OpThrow => {
                let v = self.pop()?;
                self.dispatch_value(v)?;
            }

            _ => unreachable!(
                "exec_exception_op called with non-exception opcode: {:?}",
                op
            ),
        }
        Ok(())
    }
}
