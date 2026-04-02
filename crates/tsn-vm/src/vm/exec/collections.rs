use std::sync::Arc;
use tsn_core::OpCode;
use tsn_types::value::{new_array, new_object, ObjData, RangeData, Value};

impl super::super::Vm {
    pub(super) fn exec_collection_op(&mut self, op: OpCode) -> Result<(), String> {
        match op {
            OpCode::OpBuildArray => {
                let count = self.read_u16() as usize;
                let start = self.stack.len().saturating_sub(count);
                let elems: Vec<Value> = self.stack.drain(start..).collect();
                self.push(new_array(elems));
            }
            OpCode::OpArrayLength => {
                let v = self.pop()?;
                let len = match &v {
                    Value::Array(a) => unsafe { &**a }.len() as i64,
                    Value::Str(s) => {
                        if s.is_ascii() {
                            s.len() as i64
                        } else {
                            s.chars().count() as i64
                        }
                    }
                    _ => 0,
                };
                self.push(Value::Int(len));
            }
            OpCode::OpArrayPush => {
                let item = self.pop()?;
                let arr = self.pop()?;
                if let Value::Array(a) = &arr {
                    unsafe { &mut **a }.push(item);
                    let len = unsafe { &**a }.len() as i64;
                    self.push(arr.clone());
                    self.push(Value::Int(len));
                } else {
                    return Err("OpArrayPush on non-array".to_owned());
                }
            }
            OpCode::OpArrayPop => {
                let arr = self.pop()?;
                if let Value::Array(a) = &arr {
                    let v = unsafe { &mut **a }.pop().unwrap_or(Value::Null);
                    self.push(arr.clone());
                    self.push(v);
                } else {
                    return Err("OpArrayPop on non-array".to_owned());
                }
            }
            OpCode::OpArrayExtend => {
                let spread_src = self.pop()?;
                let target = self.stack.last_mut().ok_or("stack underflow")?;
                if let (Value::Array(tgt), Value::Array(src)) = (target, &spread_src) {
                    let elems: Vec<Value> = unsafe { &**src }.iter().cloned().collect();
                    unsafe { &mut **tgt }.extend(elems);
                }
            }
            OpCode::OpIsArray => {
                let v = self.pop()?;
                self.push(Value::Bool(matches!(v, Value::Array(_))));
            }

            OpCode::OpBuildObject => {
                let count = self.read_u16() as usize;
                let start = self.stack.len().saturating_sub(count * 2);
                let pairs: Vec<Value> = self.stack.drain(start..).collect();
                let mut obj = ObjData::new();
                let mut i = 0;
                while i + 1 < pairs.len() {
                    let k = pairs[i].to_string();
                    let v = pairs[i + 1].clone();
                    obj.fields.insert(Arc::from(k), v);
                    i += 2;
                }
                self.push(new_object(obj));
            }
            OpCode::OpObjectRest => {
                let spread = self.pop()?;
                let target = self.pop()?;
                match (&target, &spread) {
                    (Value::Object(t), Value::Object(s)) => {
                        let src = unsafe { &**s };

                        let slot_pairs: Vec<(Arc<str>, Value)> = if let Some(cls) = &src.class {
                            let mut pairs: Vec<(Arc<str>, usize)> = cls
                                .field_map
                                .iter()
                                .map(|(k, &slot)| (k.clone(), slot))
                                .collect();
                            pairs.sort_unstable_by_key(|(_, slot)| *slot);
                            pairs
                                .into_iter()
                                .map(|(k, slot)| {
                                    let v = src.slots.get(slot).cloned().unwrap_or(Value::Null);
                                    (k, v)
                                })
                                .collect()
                        } else {
                            vec![]
                        };
                        let dyn_pairs: Vec<(Arc<str>, Value)> = src
                            .fields
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                        let dst = unsafe { &mut **t };
                        for (k, v) in slot_pairs {
                            dst.fields.insert(k, v);
                        }
                        for (k, v) in dyn_pairs {
                            dst.fields.insert(k, v);
                        }
                        self.push(target.clone());
                    }
                    _ => self.push(target.clone()),
                }
            }

            OpCode::OpInvokeRuntimeStatic => {
                let idx = self.read_u16();
                let method = self.get_str_const(idx);
                let flag = self.read_u16();
                if method.as_ref() == "__range__" {
                    let end = self.pop()?;
                    let start = self.pop()?;
                    match (start, end) {
                        (Value::Int(s), Value::Int(e)) => {
                            self.push(Value::Range(Box::new(RangeData {
                                start: s,
                                end: e,
                                inclusive: flag == 1,
                            })));
                        }
                        (s, e) => {
                            return Err(format!(
                                "range operator requires integer operands, got {} and {}",
                                s.type_name(),
                                e.type_name()
                            ));
                        }
                    }
                }
            }

            _ => unreachable!(
                "exec_collection_op called with non-collection opcode: {:?}",
                op
            ),
        }
        Ok(())
    }
}
