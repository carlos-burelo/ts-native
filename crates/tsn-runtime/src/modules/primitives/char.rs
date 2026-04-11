use std::sync::Arc;
use tsn_types::Value;

pub fn char_to_str(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Char(c)) = args.first() {
        return Ok(Value::Str(Arc::from(c.to_string())));
    }
    Ok(Value::Str(Arc::from("")))
}

pub fn char_code_at(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Char(c)) = args.first() {
        return Ok(Value::Int(*c as u32 as i64));
    }
    Ok(Value::Null)
}