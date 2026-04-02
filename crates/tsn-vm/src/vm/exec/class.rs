use std::sync::Arc;
use tsn_core::OpCode;
use tsn_types::value::{ClassObj, Value};

impl super::super::Vm {
    pub(super) fn exec_class_op(&mut self, op: OpCode) -> Result<(), String> {
        match op {
            OpCode::OpClass => {
                let idx = self.read_u16();
                let name = self.get_str_const(idx);
                self.push(Value::Class(Arc::new(ClassObj::new(name.as_ref()))));
            }
            OpCode::OpMethod => {
                let idx = self.read_u16();
                let key = self.get_str_const(idx);
                let method = self.pop()?;
                let class = self.stack.last_mut().ok_or("no class on stack")?;
                if let Value::Class(c) = class {
                    Arc::get_mut(c).unwrap().add_method(key.as_ref(), method);
                }
            }
            OpCode::OpDefineStatic => {
                let idx = self.read_u16();
                let key = self.get_str_const(idx);
                let val = self.pop()?;
                let class = self.stack.last_mut().ok_or("no class on stack")?;
                if let Value::Class(c) = class {
                    Arc::get_mut(c)
                        .unwrap()
                        .statics
                        .insert(Arc::from(key.as_ref()), val);
                }
            }
            OpCode::OpDeclareField => {
                let name_idx = self.read_u16();
                let name = self.get_str_const(name_idx);
                if let Some(Value::Class(cls_arc)) = self.stack.last_mut() {
                    Arc::get_mut(cls_arc).unwrap().declare_field(name);
                }
            }

            OpCode::OpDefineGetter => {
                let idx = self.read_u16();
                let key = self.get_str_const(idx);
                let val = self.pop()?;
                if let Value::Closure(c) = val {
                    if let Some(Value::Class(cls)) = self.stack.last_mut() {
                        Arc::get_mut(cls).unwrap().add_getter(key.as_ref(), c);
                    }
                }
            }
            OpCode::OpDefineSetter => {
                let idx = self.read_u16();
                let key = self.get_str_const(idx);
                let val = self.pop()?;
                if let Value::Closure(c) = val {
                    if let Some(Value::Class(cls)) = self.stack.last_mut() {
                        Arc::get_mut(cls).unwrap().add_setter(key.as_ref(), c);
                    }
                }
            }
            OpCode::OpDefineStaticGetter => {
                let idx = self.read_u16();
                let key = self.get_str_const(idx);
                let val = self.pop()?;
                if let Value::Closure(c) = val {
                    if let Some(Value::Class(cls)) = self.stack.last_mut() {
                        Arc::get_mut(cls)
                            .unwrap()
                            .add_static_getter(key.as_ref(), c);
                    }
                }
            }
            OpCode::OpDefineStaticSetter => {
                let idx = self.read_u16();
                let key = self.get_str_const(idx);
                let val = self.pop()?;
                if let Value::Closure(c) = val {
                    if let Some(Value::Class(cls)) = self.stack.last_mut() {
                        Arc::get_mut(cls)
                            .unwrap()
                            .add_static_setter(key.as_ref(), c);
                    }
                }
            }

            OpCode::OpInvokeVirtual => {
                let vtable_idx = self.read_u16() as usize;
                let arg_count = self.read_u16() as usize;

                let obj = {
                    let idx = self.stack.len() - 1 - arg_count;
                    self.stack
                        .get(idx)
                        .cloned()
                        .ok_or("no receiver for virtual call")?
                };

                match obj {
                    Value::Object(obj_arc) => {
                        let class = unsafe { &*obj_arc }.class.clone().ok_or_else(|| {
                            "OpInvokeVirtual on plain object (no class)".to_owned()
                        })?;
                        let method = class.vtable.get(vtable_idx).cloned().ok_or_else(|| {
                            format!(
                                "vtable index {} out of bounds for class {}",
                                vtable_idx, class.name
                            )
                        })?;

                        let this_idx = self.stack.len() - 1 - arg_count;
                        self.stack.insert(this_idx, method.clone());
                        self.call_value(method, arg_count + 1)?;
                    }
                    _ => {
                        return Err(format!(
                            "OpInvokeVirtual on non-instance: {}",
                            obj.type_name()
                        ))
                    }
                }
            }

            _ => unreachable!("exec_class_op called with non-class opcode: {:?}", op),
        }
        Ok(())
    }
}
