use std::sync::Arc;
use tsn_compiler::chunk::PoolEntry;
use tsn_types::value::{find_method_with_owner, BoundMethod, Closure, Upvalue, Value};
use tsn_types::Context;

use super::modules::short_val;

impl super::super::Vm {
    #[inline(always)]
    pub(super) fn op_closure(&mut self) -> Result<(), String> {
        let fn_idx = self.read_u16();
        let proto_arc = match self
            .frame()
            .closure
            .proto
            .chunk
            .constants
            .get(fn_idx as usize)
        {
            Some(PoolEntry::Function(p)) => Arc::new(p.clone()),
            _ => return Err(format!("OpClosure: const #{} is not a function", fn_idx)),
        };
        let upvalue_count = proto_arc.upvalue_count;
        let mut upvalues = Vec::with_capacity(upvalue_count);
        for _ in 0..upvalue_count {
            let is_local = self.read_u16() != 0;
            let index = self.read_u16() as usize;
            if is_local {
                let base = self.frame().base;
                let abs_idx = base + index;

                // Keep track of open upvalues to avoid duplicates for the same slot
                let existing = self
                    .open_upvalues
                    .iter()
                    .find(|u| u.inner.lock().location == Some(abs_idx))
                    .cloned();

                if let Some(up) = existing {
                    upvalues.push((*up).clone());
                } else {
                    let new_up = Arc::new(Upvalue {
                        inner: Arc::new(parking_lot::Mutex::new(tsn_types::value::UpvalueInner {
                            value: Value::Null,
                            location: Some(abs_idx),
                        })),
                    });
                    self.open_upvalues.push(new_up.clone());
                    upvalues.push((*new_up).clone());
                }
            } else {
                let up = self
                    .frame()
                    .closure
                    .upvalues
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| Upvalue {
                        inner: Arc::new(parking_lot::Mutex::new(tsn_types::value::UpvalueInner {
                            value: Value::Null,
                            location: None,
                        })),
                    });
                upvalues.push(up);
            }
        }
        let closure = Arc::new(Closure {
            proto: proto_arc,
            upvalues,
        });
        self.push(Value::Closure(closure));
        Ok(())
    }

    #[inline(always)]
    pub(super) fn op_bind_method(&mut self) -> Result<(), String> {
        let method = self.pop()?;
        let receiver = self.pop()?;
        match method {
            Value::Closure(c) => {
                self.push(Value::BoundMethod(Arc::new(BoundMethod {
                    receiver: Box::new(receiver),
                    method: c,
                    owner_class: None,
                })));
                Ok(())
            }
            Value::NativeFn(b) => {
                self.push(Value::native_bound(receiver, b.0, b.1));
                Ok(())
            }
            other => Err(format!(
                "cannot bind non-callable value of type {}",
                other.type_name()
            )),
        }
    }

    #[inline(always)]
    pub(super) fn op_wrap_spread(&mut self) -> Result<(), String> {
        let value = self.pop()?;
        self.push(Value::Spread(Box::new(value)));
        Ok(())
    }

    #[inline(always)]
    pub(super) fn op_return(&mut self, stop_at: usize) -> Result<Option<Value>, String> {
        let return_val = self.pop()?;
        if self.calls {
            let name = self
                .frame()
                .closure
                .proto
                .name
                .as_deref()
                .unwrap_or("<anon>")
                .to_owned();
            let depth = self.frames.len().saturating_sub(1);
            eprintln!(
                "  {}<- {}  = {}",
                "  ".repeat(depth),
                name,
                short_val(&return_val)
            );
        }
        let frame = self.frames.pop().ok_or("return from empty frames")?;

        // Close all upvalues above or at the base of this frame
        self.close_upvalues_on_stack(frame.base);

        while self.stack.len() > frame.base {
            self.stack.pop();
        }
        if self.frames.is_empty() {
            return Ok(Some(return_val));
        }
        if !self.stack.is_empty() {
            self.stack.pop();
        }

        if self.frames.len() <= stop_at {
            return Ok(Some(return_val));
        }
        self.push(return_val);
        Ok(None)
    }

    #[inline(always)]
    pub(super) fn op_inherit(&mut self) -> Result<(), String> {
        let mut class = self.pop()?;
        let superclass = self.pop()?;
        if let (Value::Class(c), Value::Class(s)) = (&mut class, &superclass) {
            let c_write = Arc::get_mut(c).unwrap();
            c_write.superclass = Some(s.clone());

            c_write.vtable = s.vtable.clone();
            c_write.method_map = s.method_map.clone();

            c_write.field_map = s.field_map.clone();
            c_write.field_count = s.field_count;
        }
        self.push(class);
        Ok(())
    }

    #[inline(always)]
    pub(super) fn op_get_super(&mut self) -> Result<(), String> {
        let idx = self.read_u16();
        let key = self.get_str_const(idx);
        let base = self.frame().base;
        let this_val = self.stack.get(base).cloned().unwrap_or(Value::Null);
        let current_class = self.frame().current_class.clone();
        let super_cls = current_class.as_ref().and_then(|c| c.superclass.clone());
        let lookup: &str = if key.as_ref() == "super" {
            "constructor"
        } else {
            key.as_ref()
        };
        let method_val = super_cls
            .as_ref()
            .and_then(|s| find_method_with_owner(s, lookup));
        match method_val {
            Some((Value::Closure(c), owner)) => {
                self.push(Value::BoundMethod(Arc::new(BoundMethod {
                    receiver: Box::new(this_val),
                    method: c,
                    owner_class: Some(owner),
                })));
            }
            Some((Value::NativeFn(b), _)) => {
                self.push(Value::native_bound(this_val, b.0, b.1));
            }
            _ => {
                fn noop(_ctx: &mut dyn Context, _args: &[Value]) -> Result<Value, String> {
                    Ok(Value::Null)
                }
                self.push(Value::native(noop, "super"));
            }
        }
        Ok(())
    }

    #[inline(always)]
    pub(super) fn op_object_keys(&mut self) -> Result<(), String> {
        let obj = self.pop()?;
        let keys: Vec<Value> = match &obj {
            Value::Object(obj_arc) => {
                let guard = unsafe { &**obj_arc };
                let mut all_keys: Vec<Value> = Vec::new();

                if let Some(cls) = &guard.class {
                    let mut pairs: Vec<(Arc<str>, usize)> = cls
                        .field_map
                        .iter()
                        .map(|(k, &slot)| (k.clone(), slot))
                        .collect();
                    pairs.sort_unstable_by_key(|(_, slot)| *slot);
                    all_keys.extend(pairs.into_iter().map(|(k, _)| Value::Str(k)));
                }

                all_keys.extend(guard.fields.keys().map(Value::Str));
                all_keys
            }
            _ => vec![],
        };
        self.push(tsn_types::value::new_array(keys));
        Ok(())
    }
}
