pub(crate) mod array;
mod bool;
mod decimal;
mod float;
mod future;
pub(super) mod generator;
mod int;
mod range;
mod str;

use std::sync::Arc;

use tsn_core::well_known;
use tsn_types::chunk::CacheEntry;
use tsn_types::value::{find_method_with_owner, BoundMethod, Closure, ObjData, Value};

fn range_symbol_iterator(
    _ctx: &mut dyn tsn_types::Context,
    args: &[Value],
) -> Result<Value, String> {
    let (cur, end_excl) = match args.first() {
        Some(Value::Range(r)) => (r.start, if r.inclusive { r.end + 1 } else { r.end }),
        _ => return Err("range_symbol_iterator: invalid receiver".into()),
    };
    let mut iter_obj = ObjData::new();
    iter_obj.fields.insert(Arc::from("__cur"), Value::Int(cur));
    iter_obj
        .fields
        .insert(Arc::from("__end"), Value::Int(end_excl));
    let iter_val = tsn_types::value::new_object(iter_obj);
    let next_method = Value::native_bound(iter_val.clone(), range_iter_next, "next");
    if let Value::Object(o) = &iter_val {
        unsafe { &mut **o }
            .fields
            .insert(Arc::from("next"), next_method);
    }
    Ok(iter_val)
}

fn range_iter_next(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let iter_ptr = match args.first() {
        Some(Value::Object(o)) => *o,
        _ => return Err("range_iter_next: invalid receiver".into()),
    };
    let iter_obj = unsafe { &mut *iter_ptr };
    let cur = match iter_obj.fields.get("__cur") {
        Some(Value::Int(i)) => *i,
        _ => return Err("range_iter_next: __cur not an int".into()),
    };
    let end = match iter_obj.fields.get("__end") {
        Some(Value::Int(i)) => *i,
        _ => return Err("range_iter_next: __end not an int".into()),
    };
    if cur >= end {
        let mut done = ObjData::new();
        done.fields.insert(Arc::from("value"), Value::Null);
        done.fields.insert(Arc::from("done"), Value::Bool(true));
        return Ok(tsn_types::value::new_object(done));
    }
    iter_obj
        .fields
        .insert(Arc::from("__cur"), Value::Int(cur + 1));
    let mut result = ObjData::new();
    result.fields.insert(Arc::from("value"), Value::Int(cur));
    result.fields.insert(Arc::from("done"), Value::Bool(false));
    Ok(tsn_types::value::new_object(result))
}

fn array_symbol_iterator(
    _ctx: &mut dyn tsn_types::Context,
    args: &[Value],
) -> Result<Value, String> {
    let arr = args.first().cloned().unwrap_or(Value::Null);
    let mut iter_obj = ObjData::new();
    iter_obj.fields.insert(Arc::from("__arr"), arr);
    iter_obj.fields.insert(Arc::from("__idx"), Value::Int(0));
    let iter_val = tsn_types::value::new_object(iter_obj);
    let next_method = Value::native_bound(iter_val.clone(), array_iter_next, "next");
    if let Value::Object(o) = &iter_val {
        unsafe { &mut **o }
            .fields
            .insert(Arc::from("next"), next_method);
    }
    Ok(iter_val)
}

fn array_iter_next(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let iter_ptr = match args.first() {
        Some(Value::Object(o)) => *o,
        _ => return Err("array_iter_next: invalid receiver".into()),
    };
    let iter_obj = unsafe { &mut *iter_ptr };
    let arr_val = iter_obj.fields.get("__arr").cloned().unwrap_or(Value::Null);
    let idx = match iter_obj.fields.get("__idx") {
        Some(Value::Int(i)) => *i,
        _ => 0,
    };
    let arr_len = match &arr_val {
        Value::Array(a) => unsafe { &**a }.len(),
        _ => return Err("array_iter_next: __arr not an array".into()),
    };
    if idx as usize >= arr_len {
        let mut done = ObjData::new();
        done.fields.insert(Arc::from("value"), Value::Null);
        done.fields.insert(Arc::from("done"), Value::Bool(true));
        return Ok(tsn_types::value::new_object(done));
    }
    let item = match &arr_val {
        Value::Array(a) => {
            let slice = unsafe { &**a };
            slice[idx as usize].clone()
        }
        _ => unreachable!(),
    };
    iter_obj
        .fields
        .insert(Arc::from("__idx"), Value::Int(idx + 1));
    let mut result = ObjData::new();
    result.fields.insert(Arc::from("value"), item);
    result.fields.insert(Arc::from("done"), Value::Bool(false));
    Ok(tsn_types::value::new_object(result))
}

impl super::Vm {
    fn invoke_getter(&mut self, getter: Arc<Closure>, receiver: Value) -> Result<Value, String> {
        let depth_before = self.frames.len();
        let callee = Value::Closure(getter);
        self.push(callee.clone());
        self.push(receiver);
        self.call_value(callee, 1)?;
        self.run_until(depth_before)
    }

    fn invoke_setter(
        &mut self,
        setter: Arc<Closure>,
        receiver: Value,
        new_val: Value,
    ) -> Result<(), String> {
        let depth_before = self.frames.len();
        let callee = Value::Closure(setter);
        self.push(callee.clone());
        self.push(receiver);
        self.push(new_val);
        self.call_value(callee, 2)?;
        self.run_until(depth_before)?;
        Ok(())
    }

    fn invoke_static_getter(&mut self, getter: Arc<Closure>) -> Result<Value, String> {
        let depth_before = self.frames.len();
        let callee = Value::Closure(getter);
        self.push(callee.clone());
        self.call_value(callee, 0)?;
        self.run_until(depth_before)
    }

    fn invoke_static_setter(&mut self, setter: Arc<Closure>, new_val: Value) -> Result<(), String> {
        let depth_before = self.frames.len();
        let callee = Value::Closure(setter);
        self.push(callee.clone());
        self.push(new_val);
        self.call_value(callee, 1)?;
        self.run_until(depth_before)?;
        Ok(())
    }

    pub(super) fn get_property_cached(
        &mut self,
        obj: &Value,
        key: &str,
        cache_idx: usize,
    ) -> Result<Value, String> {
        match obj {
            Value::Object(obj_arc) => {
                let entry = self.frames.last().unwrap().ic_slots[cache_idx];
                let guard = unsafe { &**obj_arc };

                if guard.class.is_some() {
                    let cls_id = guard.class.as_ref().unwrap().id;
                    if entry.is_class && entry.id == cls_id {
                        return Ok(guard
                            .slots
                            .get(entry.slot as usize)
                            .cloned()
                            .unwrap_or(Value::Null));
                    }

                    let cls_arc = guard.class.as_ref().unwrap().clone();
                    if let Some(&slot) = cls_arc.field_map.get(key) {
                        let class_id = cls_arc.id;
                        let v = guard.slots.get(slot).cloned().unwrap_or(Value::Null);
                        let n = self.frames.len() - 1;
                        self.frames[n].ic_slots[cache_idx] = CacheEntry {
                            id: class_id,
                            slot: slot as u16,
                            is_class: true,
                        };
                        return Ok(v);
                    }

                    if let Some(getter) = cls_arc.find_getter(key) {
                        let receiver = obj.clone();
                        return self.invoke_getter(getter, receiver);
                    }

                    return self.get_property(obj, key);
                }

                let shape_id = guard.fields.shape.id;
                if !entry.is_class && entry.id == shape_id {
                    let raw = guard.fields.values[entry.slot as usize].clone();
                    return Ok(match raw {
                        Value::Closure(ref c) if c.proto.has_this => {
                            Value::BoundMethod(Arc::new(BoundMethod {
                                receiver: Box::new(obj.clone()),
                                method: c.clone(),
                                owner_class: None,
                            }))
                        }
                        other => other,
                    });
                }

                let v = self.get_property(obj, key)?;
                let guard2 = unsafe { &**obj_arc };
                if let Some(&slot) = guard2.fields.shape.property_names.get(key) {
                    let new_id = guard2.fields.shape.id;
                    let n = self.frames.len() - 1;
                    self.frames[n].ic_slots[cache_idx] = CacheEntry {
                        id: new_id,
                        slot: slot as u16,
                        is_class: false,
                    };
                }
                Ok(v)
            }
            Value::Class(cls) => {
                if let Some(getter) = cls.find_static_getter(key) {
                    return self.invoke_static_getter(getter);
                }
                self.get_property(obj, key)
            }
            _ => self.get_property(obj, key),
        }
    }

    pub(super) fn set_property_cached(
        &mut self,
        obj: &Value,
        key: &str,
        value: Value,
        cache_idx: usize,
    ) -> Result<(), String> {
        match obj {
            Value::Object(obj_arc) => {
                let entry = self.frames.last().unwrap().ic_slots[cache_idx];
                let obj = unsafe { &**obj_arc };

                if obj.class.is_some() {
                    let cls_arc = obj.class.as_ref().unwrap().clone();

                    if entry.is_class && entry.id == cls_arc.id {
                        let slot = entry.slot as usize;
                        let w = unsafe { &mut **obj_arc };
                        if slot < w.slots.len() {
                            w.slots[slot] = value;
                        }
                        return Ok(());
                    }

                    if let Some(&slot) = cls_arc.field_map.get(key) {
                        let class_id = cls_arc.id;
                        unsafe { &mut **obj_arc }.slots[slot] = value;
                        let n = self.frames.len() - 1;
                        self.frames[n].ic_slots[cache_idx] = CacheEntry {
                            id: class_id,
                            slot: slot as u16,
                            is_class: true,
                        };
                        return Ok(());
                    }

                    if let Some(setter) = cls_arc.find_setter(key) {
                        let receiver = Value::Object(*obj_arc);
                        return self.invoke_setter(setter, receiver, value);
                    }

                    let is_native = cls_arc.is_native;
                    let cls_name = if !is_native {
                        cls_arc.name.clone()
                    } else {
                        String::new()
                    };

                    if is_native {
                        unsafe { &mut **obj_arc }
                            .fields
                            .insert(Arc::from(key), value);
                        return Ok(());
                    } else {
                        return Err(format!(
                            "cannot set undeclared field '{}' on instance of '{}'",
                            key, cls_name
                        ));
                    }
                }

                if !entry.is_class && entry.id == obj.fields.shape.id {
                    unsafe { &mut **obj_arc }.fields.values[entry.slot as usize] = value;
                    return Ok(());
                }

                unsafe { &mut **obj_arc }
                    .fields
                    .insert(Arc::from(key), value);
                let obj2 = unsafe { &**obj_arc };
                if let Some(&slot) = obj2.fields.shape.property_names.get(key) {
                    let new_id = obj2.fields.shape.id;
                    let n = self.frames.len() - 1;
                    self.frames[n].ic_slots[cache_idx] = CacheEntry {
                        id: new_id,
                        slot: slot as u16,
                        is_class: false,
                    };
                }
                Ok(())
            }
            Value::Class(cls) => {
                if let Some(setter) = cls.find_static_setter(key) {
                    return self.invoke_static_setter(setter, value);
                }
                self.set_property(obj, key, value)
            }
            _ => self.set_property(obj, key, value),
        }
    }

    pub(super) fn get_property(&self, obj: &Value, key: &str) -> Result<Value, String> {
        match obj {
            Value::Object(obj_arc) => {
                let guard = unsafe { &**obj_arc };

                if let Some(cls) = &guard.class {
                    if let Some(&slot) = cls.field_map.get(key) {
                        return Ok(guard.slots.get(slot).cloned().unwrap_or(Value::Null));
                    }

                    if let Some(v) = guard.fields.get(key) {
                        return Ok(v.clone());
                    }

                    let class = cls.clone();
                    if let Some((m, owner)) = find_method_with_owner(&class, key) {
                        return Ok(match m {
                            Value::Closure(c) => Value::BoundMethod(Arc::new(BoundMethod {
                                receiver: Box::new(obj.clone()),
                                method: c,
                                owner_class: Some(owner),
                            })),
                            Value::NativeFn(b) => Value::native_bound(obj.clone(), b.0, b.1),
                            other => other,
                        });
                    }
                    return Err(format!("property '{}' not found on instance", key));
                }

                // Plain object (no class) — missing field returns null, matching JS/TS semantics.
                // Methods (has_this=true) are wrapped as BoundMethod so the receiver is passed
                // automatically at the call site, just like class instance methods.
                let v = guard.fields.get(key).cloned().unwrap_or(Value::Null);
                Ok(match v {
                    Value::Closure(ref c) if c.proto.has_this => {
                        Value::BoundMethod(Arc::new(BoundMethod {
                            receiver: Box::new(obj.clone()),
                            method: c.clone(),
                            owner_class: None,
                        }))
                    }
                    other => other,
                })
            }

            Value::Class(cls) => {
                // Expose built-in class metadata properties before checking statics.
                if key == "name" {
                    return Ok(Value::Str(Arc::from(cls.name.as_str())));
                }
                let v = cls
                    .statics
                    .get(key)
                    .cloned()
                    .or_else(|| cls.find_method(key));
                v.ok_or_else(|| format!("static property or method '{}' not found on class", key))
            }
            Value::Array(arr) => {
                let v = array::get_property(obj, arr, key);
                if v.is_ok() {
                    return v;
                }
                self.get_primitive_property(obj, well_known::ARRAY, key)
            }
            Value::Str(s) => {
                let v = str::get_property(obj, s, key);
                if v.is_ok() {
                    return v;
                }
                self.get_primitive_property(obj, well_known::STR, key)
            }
            Value::Int(_) => {
                let v = int::get_property(obj, key);
                if v.is_ok() {
                    return v;
                }
                self.get_primitive_property(obj, well_known::INT, key)
            }
            Value::Float(_) => {
                let v = float::get_property(obj, key);
                if v.is_ok() {
                    return v;
                }
                self.get_primitive_property(obj, well_known::FLOAT, key)
            }
            Value::Decimal(_) => {
                let v = decimal::get_property(obj, key);
                if v.is_ok() {
                    return v;
                }
                self.get_primitive_property(obj, well_known::DECIMAL, key)
            }
            Value::Char(_) => self.get_primitive_property(obj, well_known::CHAR, key),
            Value::Bool(_) => {
                let v = bool::get_property(obj, key);
                if v.is_ok() {
                    return v;
                }
                self.get_primitive_property(obj, well_known::BOOL, key)
            }
            Value::Symbol(s) => self.get_symbol_property(obj, s.clone()),
            Value::Map(m) => crate::intrinsic::map::get_property(obj, *m, key),
            Value::Set(s) => crate::intrinsic::set::get_property(obj, *s, key),
            Value::Future(_) => future::get_property(obj, key),
            Value::Range(r) => range::get_property(obj, r.start, r.end, r.inclusive, key),
            Value::Generator(gen) => generator::get_property(obj, gen, key),

            _ => Err(format!(
                "cannot get property '{}' on {}",
                key,
                obj.type_name()
            )),
        }
    }

    fn get_primitive_property(
        &self,
        obj: &Value,
        class_name: &str,
        key: &str,
    ) -> Result<Value, String> {
        let globals = self.globals.read();
        if let Some(Value::Class(cls)) = globals.get(class_name) {
            if let Some(_getter) = cls.find_getter(key) {
                return Err(format!(
                    "getter '{}' on primitive '{}' not accessible via immutable borrow",
                    key, class_name
                ));
            }
            if let Some(m) = cls.find_method(key) {
                return Ok(match m {
                    Value::Closure(c) => Value::BoundMethod(Arc::new(BoundMethod {
                        receiver: Box::new(obj.clone()),
                        method: c,
                        owner_class: Some(cls.clone()),
                    })),
                    Value::NativeFn(b) => Value::native_bound(obj.clone(), b.0, b.1),
                    other => other,
                });
            }
        }
        Err(format!(
            "property '{}' not found on primitive {}",
            key,
            obj.type_name()
        ))
    }

    pub(super) fn get_symbol_property(
        &self,
        obj: &Value,
        symbol: tsn_types::value::SymbolKind,
    ) -> Result<Value, String> {
        match obj {
            Value::Array(_) => {
                if matches!(symbol, tsn_types::value::SymbolKind::Iterator) {
                    return Ok(Value::native_bound(
                        obj.clone(),
                        array_symbol_iterator,
                        "[Symbol.iterator]",
                    ));
                }
                Err(format!("symbol property {} not found on array", symbol))
            }
            Value::Object(obj_arc) => {
                let guard = unsafe { &**obj_arc };
                if let Some(v) = guard.fields.get(&symbol.to_string()) {
                    return Ok(v.clone());
                }
                Err(format!("symbol property {} not found on object", symbol))
            }
            Value::Range(_) => {
                if matches!(symbol, tsn_types::value::SymbolKind::Iterator) {
                    return Ok(Value::native_bound(
                        obj.clone(),
                        range_symbol_iterator,
                        "[Symbol.iterator]",
                    ));
                }
                Err(format!("symbol property {} not found on range", symbol))
            }
            Value::Generator(gen) => generator::get_symbol(obj, gen, symbol),
            Value::AsyncQueue(_) => generator::asyncqueue_get_symbol(obj, symbol),
            _ => self.get_property(obj, symbol.name()),
        }
    }

    pub(super) fn set_property(&self, obj: &Value, key: &str, value: Value) -> Result<(), String> {
        match obj {
            Value::Object(obj_arc) => {
                let guard = unsafe { &**obj_arc };
                let (slot, is_native, cls_name) = if let Some(cls) = &guard.class {
                    (
                        cls.field_map.get(key).copied(),
                        cls.is_native,
                        cls.name.clone(),
                    )
                } else {
                    (None, true, String::new())
                };
                if let Some(slot) = slot {
                    unsafe { &mut **obj_arc }.slots[slot] = value;
                } else if is_native {
                    unsafe { &mut **obj_arc }
                        .fields
                        .insert(Arc::from(key), value);
                } else {
                    return Err(format!(
                        "cannot set undeclared field '{}' on instance of '{}'",
                        key, cls_name
                    ));
                }
                Ok(())
            }
            Value::Class(cls) => {
                unsafe { &mut *(std::sync::Arc::as_ptr(cls) as *mut tsn_types::value::ClassObj) }
                    .statics
                    .insert(std::sync::Arc::from(key), value);
                Ok(())
            }
            Value::Char(_) | Value::Int(_) | Value::Float(_) | Value::Str(_) | Value::Bool(_) => {
                Err(format!(
                    "cannot set property '{}' on primitive {}",
                    key,
                    obj.type_name()
                ))
            }
            _ => Err(format!(
                "cannot set property '{}' on {}",
                key,
                obj.type_name()
            )),
        }
    }

    pub(super) fn get_index(&self, obj: &Value, idx: &Value) -> Result<Value, String> {
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
                self.get_property(obj, &key)
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

    pub(super) fn set_index(&self, obj: &Value, idx: &Value, value: Value) -> Result<(), String> {
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
                self.set_property(obj, &key, value)
            }
            Value::Char(_) | Value::Int(_) | Value::Float(_) | Value::Str(_) => {
                Err(format!("cannot set index on primitive {}", obj.type_name()))
            }
            _ => Err(format!("cannot index {} with []", obj.type_name())),
        }
    }
}
