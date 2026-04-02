use tsn_types::value::{alloc_set, new_array, Value};

fn as_set_mut(v: &Value) -> Result<*mut std::collections::HashSet<Value>, String> {
    match v {
        Value::Set(s) => Ok(*s),
        _ => Err(format!("expected Set, got {}", v.type_name())),
    }
}

pub fn set_new(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Set(alloc_set()))
}

pub fn set_add(args: &[Value]) -> Result<Value, String> {
    let s = as_set_mut(args.first().ok_or("set_add: no receiver")?)?;
    let val = args.get(1).cloned().unwrap_or(Value::Null);
    unsafe { &mut *s }.insert(val);
    Ok(Value::Null)
}

pub fn set_has(args: &[Value]) -> Result<Value, String> {
    let s = as_set_mut(args.first().ok_or("set_has: no receiver")?)?;
    let val = args.get(1).cloned().unwrap_or(Value::Null);
    Ok(Value::Bool(unsafe { &*s }.contains(&val)))
}

pub fn set_delete(args: &[Value]) -> Result<Value, String> {
    let s = as_set_mut(args.first().ok_or("set_delete: no receiver")?)?;
    let val = args.get(1).cloned().unwrap_or(Value::Null);
    Ok(Value::Bool(unsafe { &mut *s }.remove(&val)))
}

pub fn set_clear(args: &[Value]) -> Result<Value, String> {
    let s = as_set_mut(args.first().ok_or("set_clear: no receiver")?)?;
    unsafe { &mut *s }.clear();
    Ok(Value::Null)
}

pub fn set_values(args: &[Value]) -> Result<Value, String> {
    let s = as_set_mut(args.first().ok_or("set_values: no receiver")?)?;
    let vals: Vec<Value> = unsafe { &*s }.iter().cloned().collect();
    Ok(new_array(vals))
}

pub fn set_size(args: &[Value]) -> Result<Value, String> {
    let s = as_set_mut(args.first().ok_or("set_size: no receiver")?)?;
    Ok(Value::Int(unsafe { &*s }.len() as i64))
}

pub fn get_property(obj: &Value, _s: tsn_types::value::SetRef, key: &str) -> Result<Value, String> {
    match key {
        "size" => set_size(std::slice::from_ref(obj)),
        "add" => Ok(Value::native_bound(obj.clone(), |_, a| set_add(a), "add")),
        "has" => Ok(Value::native_bound(obj.clone(), |_, a| set_has(a), "has")),
        "delete" => Ok(Value::native_bound(
            obj.clone(),
            |_, a| set_delete(a),
            "delete",
        )),
        "clear" => Ok(Value::native_bound(
            obj.clone(),
            |_, a| set_clear(a),
            "clear",
        )),
        "values" => Ok(Value::native_bound(
            obj.clone(),
            |_, a| set_values(a),
            "values",
        )),
        _ => Err(format!("property '{}' not found on Set", key)),
    }
}
