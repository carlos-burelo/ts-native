use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use tsn_op_macros::op;
use tsn_types::{value::new_array, Value};

use std::sync::OnceLock;
static METADATA: OnceLock<Mutex<HashMap<String, HashMap<String, Value>>>> = OnceLock::new();

fn metadata() -> &'static Mutex<HashMap<String, HashMap<String, Value>>> {
    METADATA.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Extract a stable string key from a target value.
/// For class values, uses the class name directly instead of "[class Foo]".
fn target_key(v: &Value) -> String {
    match v {
        Value::Class(c) => c.name.clone(),
        Value::Str(s) => s.to_string(),
        other => other.to_string(),
    }
}

#[op("defineMetadata")]
pub fn reflect_define_meta(args: &[Value]) -> Result<Value, String> {
    let key = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    let val = args.get(1).cloned().unwrap_or(Value::Null);
    let target = args.get(2).map(target_key).unwrap_or_default();

    let mut meta = metadata().lock();
    meta.entry(target).or_default().insert(key, val);

    Ok(Value::Null)
}

#[op("getMetadata")]
pub fn reflect_get_meta(args: &[Value]) -> Result<Value, String> {
    let key = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    let target = args.get(1).map(target_key).unwrap_or_default();

    let meta = metadata().lock();
    let val = meta
        .get(&target)
        .and_then(|m| m.get(&key))
        .cloned()
        .unwrap_or(Value::Null);

    Ok(val)
}

#[op("hasMetadata")]
pub fn reflect_has_meta(args: &[Value]) -> Result<Value, String> {
    let key = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    let target = args.get(1).map(target_key).unwrap_or_default();

    let meta = metadata().lock();
    let has = meta
        .get(&target)
        .map(|m| m.contains_key(&key))
        .unwrap_or(false);

    Ok(Value::Bool(has))
}

#[op("getMetadataKeys")]
pub fn reflect_get_meta_keys(args: &[Value]) -> Result<Value, String> {
    let target = args.get(0).map(target_key).unwrap_or_default();

    let meta = metadata().lock();
    let keys: Vec<Value> = meta
        .get(&target)
        .map(|m| {
            m.keys()
                .map(|k| Value::Str(Arc::from(k.as_str())))
                .collect()
        })
        .unwrap_or_default();

    Ok(new_array(keys))
}

pub const OPS: &[crate::host_ops::HostOp] = &[
    reflect_define_meta_OP,
    reflect_get_meta_OP,
    reflect_has_meta_OP,
    reflect_get_meta_keys_OP,
];
