use std::sync::Arc;
use tsn_types::value::Value;

use super::property::{get_property, set_property};
use crate::Vm;

pub fn get_index(vm: &Vm, obj: &Value, idx: &Value) -> Result<Value, String> {
    match obj {
        Value::Array(a) => {
            let n = match idx {
                Value::Int(n) => {
                    if *n < 0 {
                        return Err(format!("negative index {}", n));
                    }
                    *n as usize
                }
                _ => {
                    return Err(format!(
                        "array index must be an integer, got {}",
                        idx.type_name()
                    ))
                }
            };
            unsafe { &**a }
                .get(n)
                .cloned()
                .ok_or_else(|| format!("index {} out of bounds for array", n))
        }
        Value::Str(s) => {
            let n = match idx {
                Value::Int(n) => {
                    if *n < 0 {
                        return Err(format!("negative index {} for string", n));
                    }
                    *n as usize
                }
                _ => {
                    return Err(format!(
                        "string index must be an integer, got {}",
                        idx.type_name()
                    ))
                }
            };
            s.chars()
                .nth(n)
                .map(|c| Value::Str(Arc::from(c.to_string())))
                .ok_or_else(|| format!("index {} out of bounds for string", n))
        }
        Value::Object(_) => {
            let key = idx.to_string();
            get_property(vm, obj, &key)
        }
        Value::Range(r) => match idx {
            Value::Int(n) => {
                let start = r.start;
                let end = r.end;
                let inclusive = r.inclusive;
                if *n < 0 {
                    return Err(format!("negative index {} for range", n));
                }
                let len: i64 = if inclusive {
                    (end - start + 1).max(0)
                } else {
                    (end - start).max(0)
                };
                if *n >= len {
                    return Err(format!("range index {} out of bounds (length {})", n, len));
                }
                Ok(Value::Int(start + n))
            }
            _ => Err(format!(
                "range index must be an integer, got {}",
                idx.type_name()
            )),
        },
        Value::Char(_) => Err("char is not indexable".to_owned()),
        _ => Err(format!("cannot index {} with []", obj.type_name())),
    }
}

pub fn set_index(vm: &Vm, obj: &Value, idx: &Value, value: Value) -> Result<(), String> {
    match obj {
        Value::Array(a) => {
            let n = match idx {
                Value::Int(n) => *n as usize,
                Value::Float(f) => *f as usize,
                _ => return Ok(()),
            };
            let arr = unsafe { &mut **a };
            while arr.len() <= n {
                arr.push(Value::Null);
            }
            arr[n] = value;
            Ok(())
        }
        Value::Object(_) => {
            let key = idx.to_string();
            set_property(vm, obj, &key, value)
        }
        Value::Char(_) | Value::Int(_) | Value::Float(_) | Value::Str(_) => {
            Err(format!("cannot set index on primitive {}", obj.type_name()))
        }
        _ => Err(format!("cannot index {} with []", obj.type_name())),
    }
}
