use tsn_types::value::{alloc_set, new_array, NativeFn, Value};

pub fn as_set_mut(v: &Value) -> Result<*mut std::collections::HashSet<Value>, String> {
    match v {
        Value::Set(s) => Ok(*s),
        _ => Err(format!("expected Set, got {}", v.type_name())),
    }
}

pub fn set_new(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    Ok(Value::Set(alloc_set()))
}

pub fn set_add(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let s = as_set_mut(args.first().ok_or("set_add: no receiver")?)?;
    let val = args.get(1).cloned().unwrap_or(Value::Null);
    unsafe { &mut *s }.insert(val);
    Ok(Value::Null)
}

pub fn set_has(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let s = as_set_mut(args.first().ok_or("set_has: no receiver")?)?;
    let val = args.get(1).cloned().unwrap_or(Value::Null);
    Ok(Value::Bool(unsafe { &*s }.contains(&val)))
}

pub fn set_delete(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let s = as_set_mut(args.first().ok_or("set_delete: no receiver")?)?;
    let val = args.get(1).cloned().unwrap_or(Value::Null);
    Ok(Value::Bool(unsafe { &mut *s }.remove(&val)))
}

pub fn set_clear(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let s = as_set_mut(args.first().ok_or("set_clear: no receiver")?)?;
    unsafe { &mut *s }.clear();
    Ok(Value::Null)
}

pub fn set_values(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let s = as_set_mut(args.first().ok_or("set_values: no receiver")?)?;
    let vals: Vec<Value> = unsafe { &*s }.iter().cloned().collect();
    Ok(new_array(vals))
}

pub fn set_size(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let s = as_set_mut(args.first().ok_or("set_size: no receiver")?)?;
    Ok(Value::Int(unsafe { &*s }.len() as i64))
}

pub fn find_set_method(key: &str) -> Option<(NativeFn, &'static str)> {
    match key {
        "add" => Some((set_add, "add")),
        "has" => Some((set_has, "has")),
        "delete" => Some((set_delete, "delete")),
        "clear" => Some((set_clear, "clear")),
        "values" => Some((set_values, "values")),
        _ => None,
    }
}

pub fn get_property(obj: &Value, _s: tsn_types::value::SetRef, key: &str) -> Result<Value, String> {
    if key == "size" {
        let s = as_set_mut(obj)?;
        return Ok(Value::Int(unsafe { &*s }.len() as i64));
    }
    if let Some((func, name)) = find_set_method(key) {
        Ok(Value::native_bound(obj.clone(), func, name))
    } else {
        Err(format!("Set property '{}' not found", key))
    }
}
