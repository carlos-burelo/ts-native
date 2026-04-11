use crate::tsn_types::value::Value;

pub(super) fn get_property(obj: &Value, key: &str) -> Result<Value, String> {
    match key {
        "toString" => Ok(Value::native_bound(obj.clone(), bool_to_string, "toString")),
        "valueOf" => Ok(Value::native_bound(obj.clone(), bool_identity, "valueOf")),
        _ => Err(format!("method '{}' not found on bool", key)),
    }
}

fn bool_to_string(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::Bool(true)) => Ok(Value::Str(std::sync::Arc::from("true"))),
        Some(Value::Bool(false)) => Ok(Value::Str(std::sync::Arc::from("false"))),
        _ => Ok(Value::Str(std::sync::Arc::from("false"))),
    }
}

fn bool_identity(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(args.first().cloned().unwrap_or(Value::Null))
}
