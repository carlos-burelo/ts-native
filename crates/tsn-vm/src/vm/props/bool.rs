use crate::tsn_types::value::Value;

pub(super) fn get_property(_obj: &Value, key: &str) -> Result<Value, String> {
    Err(format!("method '{}' not found on bool", key))
}
