use tsn_core::OpCode;
use tsn_types::value::Value;

impl super::super::Vm {
    pub(super) fn exec_prop_op(&mut self, op: OpCode) -> Result<(), String> {
        match op {
            OpCode::OpGetProperty => {
                let key_idx = self.read_u16();
                let cache_idx = self.read_u16() as usize;
                let key = self.get_str_const(key_idx);
                let obj = self.pop()?;
                let v = self.get_property_cached(&obj, key.as_ref(), cache_idx)?;
                self.push(v);
            }
            OpCode::OpGetPropertyMaybe => {
                let key_idx = self.read_u16();
                let cache_idx = self.read_u16() as usize;
                let key = self.get_str_const(key_idx);
                let obj = self.pop()?;
                let v = self
                    .get_property_cached(&obj, key.as_ref(), cache_idx)
                    .unwrap_or(Value::Null);
                self.push(v);
            }
            OpCode::OpSetProperty => {
                let key_idx = self.read_u16();
                let cache_idx = self.read_u16() as usize;
                let key = self.get_str_const(key_idx);
                let value = self.pop()?;
                let obj = self.pop()?;
                self.set_property_cached(&obj, key.as_ref(), value.clone(), cache_idx)?;
                self.push(value);
            }

            OpCode::OpGetIndex => {
                let idx = self.pop()?;
                let obj = self.pop()?;
                let v = self.get_index(&obj, &idx)?;
                self.push(v);
            }
            OpCode::OpSetIndex => {
                let value = self.pop()?;
                let idx = self.pop()?;
                let obj = self.pop()?;
                self.set_index(&obj, &idx, value.clone())?;
                self.push(value);
            }

            OpCode::OpGetFixedField => {
                let slot = self.read_u16() as usize;
                let obj = self.pop()?;
                match obj {
                    Value::Object(arc) => {
                        let v = unsafe { &*arc }
                            .slots
                            .get(slot)
                            .cloned()
                            .unwrap_or(Value::Null);
                        self.push(v);
                    }
                    _ => {
                        return Err(format!(
                            "OpGetFixedField: expected object, got {}",
                            obj.type_name()
                        ))
                    }
                }
            }
            OpCode::OpSetFixedField => {
                let slot = self.read_u16() as usize;
                let value = self.pop()?;
                let obj = self.pop()?;
                match &obj {
                    Value::Object(arc) => {
                        let obj_data = unsafe { &mut **arc };
                        if slot < obj_data.slots.len() {
                            obj_data.slots[slot] = value.clone();
                        } else {
                            return Err(format!(
                                "OpSetFixedField: slot {} out of bounds (len={})",
                                slot,
                                obj_data.slots.len()
                            ));
                        }
                    }
                    _ => {
                        return Err(format!(
                            "OpSetFixedField: expected object, got {}",
                            obj.type_name()
                        ))
                    }
                }
                self.push(value);
            }

            OpCode::OpGetSymbol => {
                let idx = self.read_u16();
                let entry = self.read_const(idx);
                let symbol = match entry {
                    Value::Symbol(s) => s,
                    _ => {
                        return Err(format!(
                            "OpGetSymbol: expected symbol constant, got {}",
                            entry.type_name()
                        ))
                    }
                };
                let obj = self.pop()?;
                let v = self.get_symbol_property(&obj, symbol)?;
                self.push(v);
            }

            _ => unreachable!("exec_prop_op called with non-property opcode: {:?}", op),
        }
        Ok(())
    }
}
