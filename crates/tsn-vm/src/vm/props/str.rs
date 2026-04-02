use crate::tsn_types::value::Value;
use std::sync::Arc;

pub(super) fn get_property(_obj: &Value, s: &Arc<str>, key: &str) -> Result<Value, String> {
    if key == "length" {
        return Ok(Value::Int(if s.is_ascii() {
            s.len() as i64
        } else {
            s.chars().count() as i64
        }));
    }
    if let Ok(n) = key.parse::<usize>() {
        return s
            .chars()
            .nth(n)
            .map(|c| Value::Str(Arc::from(c.to_string())))
            .ok_or_else(|| format!("index {} out of bounds for string", n));
    }

    // All other methods are now resolved via the 'str' class in primitives.tsn
    Err(format!(
        "method '{}' not found on string (fallback to class)",
        key
    ))
}
