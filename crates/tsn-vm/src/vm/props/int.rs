use crate::tsn_types::value::Value;

pub(super) fn get_property(obj: &Value, key: &str) -> Result<Value, String> {
    if key == "rawValue" {
        return Ok(obj.clone());
    }
    Err(format!("property '{}' not found on primitive int", key))
}
