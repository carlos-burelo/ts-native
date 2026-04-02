use std::sync::Arc;
use tsn_types::value::{alloc_map, new_array, Value};

fn as_map_mut(v: &Value) -> Result<*mut std::collections::HashMap<Value, Value>, String> {
    match v {
        Value::Map(m) => Ok(*m),
        _ => Err(format!("expected Map, got {}", v.type_name())),
    }
}

pub fn map_new(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Map(alloc_map()))
}

pub fn map_get(args: &[Value]) -> Result<Value, String> {
    let m = as_map_mut(args.first().ok_or("map_get: no receiver")?)?;
    let key = args.get(1).cloned().unwrap_or(Value::Null);
    Ok(unsafe { &*m }.get(&key).cloned().unwrap_or(Value::Null))
}

pub fn map_set(args: &[Value]) -> Result<Value, String> {
    let m = as_map_mut(args.first().ok_or("map_set: no receiver")?)?;
    let key = args.get(1).cloned().unwrap_or(Value::Null);
    let val = args.get(2).cloned().unwrap_or(Value::Null);
    unsafe { &mut *m }.insert(key, val);
    Ok(Value::Null)
}

pub fn map_has(args: &[Value]) -> Result<Value, String> {
    let m = as_map_mut(args.first().ok_or("map_has: no receiver")?)?;
    let key = args.get(1).cloned().unwrap_or(Value::Null);
    Ok(Value::Bool(unsafe { &*m }.contains_key(&key)))
}

pub fn map_delete(args: &[Value]) -> Result<Value, String> {
    let m = as_map_mut(args.first().ok_or("map_delete: no receiver")?)?;
    let key = args.get(1).cloned().unwrap_or(Value::Null);
    Ok(Value::Bool(unsafe { &mut *m }.remove(&key).is_some()))
}

pub fn map_clear(args: &[Value]) -> Result<Value, String> {
    let m = as_map_mut(args.first().ok_or("map_clear: no receiver")?)?;
    unsafe { &mut *m }.clear();
    Ok(Value::Null)
}

pub fn map_keys(args: &[Value]) -> Result<Value, String> {
    let m = as_map_mut(args.first().ok_or("map_keys: no receiver")?)?;
    let keys: Vec<Value> = unsafe { &*m }.keys().cloned().collect();
    Ok(new_array(keys))
}

pub fn map_values(args: &[Value]) -> Result<Value, String> {
    let m = as_map_mut(args.first().ok_or("map_values: no receiver")?)?;
    let vals: Vec<Value> = unsafe { &*m }.values().cloned().collect();
    Ok(new_array(vals))
}

pub fn map_size(args: &[Value]) -> Result<Value, String> {
    let m = as_map_mut(args.first().ok_or("map_size: no receiver")?)?;
    Ok(Value::Int(unsafe { &*m }.len() as i64))
}

pub fn get_property(obj: &Value, m: tsn_types::value::MapRef, key: &str) -> Result<Value, String> {
    match key {
        "size" => map_size(std::slice::from_ref(obj)),
        "get" => Ok(Value::native_bound(obj.clone(), |_, a| map_get(a), "get")),
        "set" => Ok(Value::native_bound(obj.clone(), |_, a| map_set(a), "set")),
        "has" => Ok(Value::native_bound(obj.clone(), |_, a| map_has(a), "has")),
        "delete" => Ok(Value::native_bound(
            obj.clone(),
            |_, a| map_delete(a),
            "delete",
        )),
        "clear" => Ok(Value::native_bound(
            obj.clone(),
            |_, a| map_clear(a),
            "clear",
        )),
        "keys" => Ok(Value::native_bound(obj.clone(), |_, a| map_keys(a), "keys")),
        "values" => Ok(Value::native_bound(
            obj.clone(),
            |_, a| map_values(a),
            "values",
        )),
        _ => {
            // Fallback: treat as key lookup
            Ok(unsafe { &*m }
                .get(&Value::Str(Arc::from(key)))
                .cloned()
                .unwrap_or(Value::Null))
        }
    }
}
