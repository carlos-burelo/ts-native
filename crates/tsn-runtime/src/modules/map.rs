use tsn_types::value::{alloc_map, new_array, NativeFn, Value};

pub fn as_map_mut(v: &Value) -> Result<*mut std::collections::HashMap<Value, Value>, String> {
    match v {
        Value::Map(m) => Ok(*m),
        _ => Err(format!("expected Map, got {}", v.type_name())),
    }
}

pub fn map_new(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    Ok(Value::Map(alloc_map()))
}

pub fn map_get(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let m = as_map_mut(args.first().ok_or("map_get: no receiver")?)?;
    let key = args.get(1).cloned().unwrap_or(Value::Null);
    Ok(unsafe { &*m }.get(&key).cloned().unwrap_or(Value::Null))
}

pub fn map_set(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let m = as_map_mut(args.first().ok_or("map_set: no receiver")?)?;
    let key = args.get(1).cloned().unwrap_or(Value::Null);
    let val = args.get(2).cloned().unwrap_or(Value::Null);
    unsafe { &mut *m }.insert(key, val);
    Ok(Value::Null)
}

pub fn map_has(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let m = as_map_mut(args.first().ok_or("map_has: no receiver")?)?;
    let key = args.get(1).cloned().unwrap_or(Value::Null);
    Ok(Value::Bool(unsafe { &*m }.contains_key(&key)))
}

pub fn map_delete(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let m = as_map_mut(args.first().ok_or("map_delete: no receiver")?)?;
    let key = args.get(1).cloned().unwrap_or(Value::Null);
    Ok(Value::Bool(unsafe { &mut *m }.remove(&key).is_some()))
}

pub fn map_clear(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let m = as_map_mut(args.first().ok_or("map_clear: no receiver")?)?;
    unsafe { &mut *m }.clear();
    Ok(Value::Null)
}

pub fn map_keys(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let m = as_map_mut(args.first().ok_or("map_keys: no receiver")?)?;
    let keys: Vec<Value> = unsafe { &*m }.keys().cloned().collect();
    Ok(new_array(keys))
}

pub fn map_values(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let m = as_map_mut(args.first().ok_or("map_values: no receiver")?)?;
    let vals: Vec<Value> = unsafe { &*m }.values().cloned().collect();
    Ok(new_array(vals))
}

pub fn map_size(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let m = as_map_mut(args.first().ok_or("map_size: no receiver")?)?;
    Ok(Value::Int(unsafe { &*m }.len() as i64))
}

pub fn find_map_method(key: &str) -> Option<(NativeFn, &'static str)> {
    match key {
        "get" => Some((map_get, "get")),
        "set" => Some((map_set, "set")),
        "has" => Some((map_has, "has")),
        "delete" => Some((map_delete, "delete")),
        "clear" => Some((map_clear, "clear")),
        "keys" => Some((map_keys, "keys")),
        "values" => Some((map_values, "values")),
        _ => None,
    }
}

pub fn get_property(obj: &Value, _m: tsn_types::value::MapRef, key: &str) -> Result<Value, String> {
    if key == "size" {
        let m = as_map_mut(obj)?;
        return Ok(Value::Int(unsafe { &*m }.len() as i64));
    }
    if let Some((func, name)) = find_map_method(key) {
        Ok(Value::native_bound(obj.clone(), func, name))
    } else {
        Err(format!("Map property '{}' not found", key))
    }
}
