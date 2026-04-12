use std::sync::Arc;
use tsn_core::well_known;
use tsn_runtime::modules::primitives as rt_prim;
use tsn_runtime::modules::{map, set};
use tsn_types::chunk::CacheEntry;
use tsn_types::value::{find_method_with_owner, BoundMethod, Value};
use tsn_types::ClassObj;

use super::array;
use super::bool;
use super::decimal;
use super::float;
use super::future;
use super::generator;
use super::getter_setter;
use super::int;
use super::range;
use super::str;
use super::symbol;
use crate::Vm;

fn get_char_property(obj: &Value, key: &str) -> Result<Value, String> {
    match key {
        "toString" => Ok(Value::native_bound(
            obj.clone(),
            rt_prim::char_to_str,
            "toString",
        )),
        "charCodeAt" => Ok(Value::native_bound(
            obj.clone(),
            rt_prim::char_code_at,
            "charCodeAt",
        )),
        _ => Err(format!("property '{}' not found on primitive char", key)),
    }
}

pub fn get_primitive_property(
    vm: &Vm,
    obj: &Value,
    class_name: &str,
    key: &str,
) -> Result<Value, String> {
    let globals = vm.globals.read();
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

pub fn get_property_cached(
    vm: &mut Vm,
    obj: &Value,
    key: &str,
    cache_idx: usize,
) -> Result<Value, String> {
    match obj {
        Value::Object(obj_arc) => {
            let entry = vm.frames.last().unwrap().ic_slots[cache_idx];
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
                    let n = vm.frames.len() - 1;
                    vm.frames[n].ic_slots[cache_idx] = CacheEntry {
                        id: class_id,
                        slot: slot as u16,
                        is_class: true,
                    };
                    return Ok(v);
                }

                if let Some(getter) = cls_arc.find_getter(key) {
                    let receiver = obj.clone();
                    return getter_setter::invoke_getter(vm, getter, receiver);
                }

                return get_property(vm, obj, key);
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

            let v = get_property(vm, obj, key)?;
            let guard2 = unsafe { &**obj_arc };
            if let Some(&slot) = guard2.fields.shape.property_names.get(key) {
                let new_id = guard2.fields.shape.id;
                let n = vm.frames.len() - 1;
                vm.frames[n].ic_slots[cache_idx] = CacheEntry {
                    id: new_id,
                    slot: slot as u16,
                    is_class: false,
                };
            }
            Ok(v)
        }
        Value::Class(cls) => {
            if let Some(getter) = cls.find_static_getter(key) {
                return getter_setter::invoke_static_getter(vm, getter);
            }
            get_property(vm, obj, key)
        }
        _ => get_property(vm, obj, key),
    }
}

pub fn set_property_cached(
    vm: &mut Vm,
    obj: &Value,
    key: &str,
    value: Value,
    cache_idx: usize,
) -> Result<(), String> {
    match obj {
        Value::Object(obj_arc) => {
            let entry = vm.frames.last().unwrap().ic_slots[cache_idx];
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
                    let n = vm.frames.len() - 1;
                    vm.frames[n].ic_slots[cache_idx] = CacheEntry {
                        id: class_id,
                        slot: slot as u16,
                        is_class: true,
                    };
                    return Ok(());
                }

                if let Some(setter) = cls_arc.find_setter(key) {
                    let receiver = Value::Object(*obj_arc);
                    return getter_setter::invoke_setter(vm, setter, receiver, value);
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
                let n = vm.frames.len() - 1;
                vm.frames[n].ic_slots[cache_idx] = CacheEntry {
                    id: new_id,
                    slot: slot as u16,
                    is_class: false,
                };
            }
            Ok(())
        }
        Value::Class(cls) => {
            if let Some(setter) = cls.find_static_setter(key) {
                return getter_setter::invoke_static_setter(vm, setter, value);
            }
            set_property(vm, obj, key, value)
        }
        _ => set_property(vm, obj, key, value),
    }
}

pub fn get_property(_vm: &Vm, obj: &Value, key: &str) -> Result<Value, String> {
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
            get_primitive_property(_vm, obj, well_known::ARRAY, key)
        }
        Value::Str(s) => {
            let v = str::get_property(obj, s, key);
            if v.is_ok() {
                return v;
            }
            get_primitive_property(_vm, obj, well_known::STR, key)
        }
        Value::Int(_) => {
            let v = int::get_property(obj, key);
            if v.is_ok() {
                return v;
            }
            get_primitive_property(_vm, obj, well_known::INT, key)
        }
        Value::Float(_) => {
            let v = float::get_property(obj, key);
            if v.is_ok() {
                return v;
            }
            get_primitive_property(_vm, obj, well_known::FLOAT, key)
        }
        Value::Decimal(_) => {
            let v = decimal::get_property(obj, key);
            if v.is_ok() {
                return v;
            }
            get_primitive_property(_vm, obj, well_known::DECIMAL, key)
        }
        Value::Char(_) => get_char_property(obj, key),
        Value::Bool(_) => {
            let v = bool::get_property(obj, key);
            if v.is_ok() {
                return v;
            }
            get_primitive_property(_vm, obj, well_known::BOOL, key)
        }
        Value::Symbol(s) => symbol::get_symbol_property(_vm, obj, s.clone()),
        Value::Map(m) => map::get_property(obj, *m, key),
        Value::Set(s) => set::get_property(obj, *s, key),
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

pub fn set_property(_vm: &Vm, obj: &Value, key: &str, value: Value) -> Result<(), String> {
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
            unsafe { &mut *(Arc::as_ptr(cls) as *mut ClassObj) }
                .statics
                .insert(Arc::from(key), value);
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
