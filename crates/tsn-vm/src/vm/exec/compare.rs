use super::super::math::{cmp_lt, cmp_lte};
use std::sync::Arc;
use tsn_core::OpCode;
use tsn_types::value::{new_object, ClassObj, ObjData, Value};

fn is_subclass_of(candidate: &Arc<ClassObj>, target: &Arc<ClassObj>) -> bool {
    if Arc::ptr_eq(candidate, target) {
        return true;
    }
    let super_cls = candidate.superclass.clone();
    match super_cls {
        Some(s) => is_subclass_of(&s, target),
        None => false,
    }
}

impl super::super::Vm {
    pub(super) fn exec_compare_op(&mut self, op: OpCode) -> Result<(), String> {
        match op {
            OpCode::OpEq => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(Value::Bool(a == b));
            }
            OpCode::OpNeq => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(Value::Bool(a != b));
            }
            OpCode::OpLt => {
                let (a, b) = self.pop2()?;
                self.push(Value::Bool(cmp_lt(&a, &b)?));
            }
            OpCode::OpLte => {
                let (a, b) = self.pop2()?;
                self.push(Value::Bool(cmp_lte(&a, &b)?));
            }
            OpCode::OpGt => {
                let (a, b) = self.pop2()?;
                self.push(Value::Bool(cmp_lt(&b, &a)?));
            }
            OpCode::OpGte => {
                let (a, b) = self.pop2()?;
                self.push(Value::Bool(cmp_lte(&b, &a)?));
            }

            OpCode::OpNot => {
                let v = self.pop()?;
                self.push(Value::Bool(!v.is_truthy()?));
            }
            OpCode::OpIsNull => {
                let v = self.pop()?;
                self.push(Value::Bool(v.is_null()));
            }
            OpCode::OpAssertNotNull => {
                let v = self.pop()?;
                if v.is_null() {
                    let msg = "null assertion failed: value is null";
                    let mut obj = ObjData::new();
                    obj.fields
                        .insert(Arc::from("message"), Value::Str(Arc::from(msg)));
                    obj.fields.insert(
                        Arc::from("name"),
                        Value::Str(Arc::from("NullAssertionError")),
                    );
                    obj.fields
                        .insert(Arc::from("stack"), Value::Str(Arc::from("")));
                    self.dispatch_value(new_object(obj))?;
                } else {
                    self.push(v);
                }
            }
            OpCode::OpTypeof => {
                let v = self.pop()?;
                self.push(Value::Str(Arc::from(v.type_name())));
            }

            OpCode::OpInstanceof => {
                let cls = self.pop()?;
                let obj = self.pop()?;
                let result = match (&obj, &cls) {
                    (Value::Object(obj_arc), Value::Class(c)) => {
                        if let Some(inst_class) = unsafe { &**obj_arc }.class.clone() {
                            is_subclass_of(&inst_class, c)
                        } else {
                            false
                        }
                    }
                    _ => matches!(&cls, Value::Class(c) if obj.type_name() == c.name.as_str()),
                };
                self.push(Value::Bool(result));
            }
            OpCode::OpIn => {
                let obj = self.pop()?;
                let key = self.pop()?;
                let key_str = key.to_string();
                let result = match &obj {
                    Value::Object(obj_arc) => {
                        let guard = unsafe { &**obj_arc };
                        if guard.fields.contains_key(key_str.as_str()) {
                            true
                        } else if let Some(class) = &guard.class {
                            class.method_map.contains_key(key_str.as_str())
                        } else {
                            false
                        }
                    }
                    Value::Array(a) => {
                        if let Ok(n) = key_str.parse::<usize>() {
                            n < unsafe { &**a }.len()
                        } else {
                            false
                        }
                    }
                    _ => false,
                };
                self.push(Value::Bool(result));
            }

            _ => unreachable!("exec_compare_op called with non-compare opcode: {:?}", op),
        }
        Ok(())
    }
}
