use crate::tsn_types::value::Value;
use tsn_types::value::ArrayRef;

pub(super) fn get_property(_obj: &Value, arr: &ArrayRef, key: &str) -> Result<Value, String> {
    if key == "length" {
        return Ok(Value::Int(unsafe { &**arr }.len() as i64));
    }
    if let Ok(n) = key.parse::<usize>() {
        return unsafe { &**arr }
            .get(n)
            .cloned()
            .ok_or_else(|| format!("index {} out of bounds for array", n));
    }

    // All other methods are now resolved via the 'Array' class in primitives.tsn
    Err(format!(
        "method '{}' not found on array (fallback to class)",
        key
    ))
}
