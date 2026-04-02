use std::sync::Arc;
use tsn_types::value::Value;
use tsn_types::value::{new_array, new_object, ObjData};

pub fn json_parse(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::Str(s)) => parse_str(s),
        Some(v) => parse_str(&v.to_string()),
        None => Err("__json_parse: missing argument".into()),
    }
}

fn parse_str(s: &str) -> Result<Value, String> {
    let sv: serde_json::Value =
        serde_json::from_str(s).map_err(|e| format!("JSON.parse: {}", e))?;
    Ok(serde_to_tsn(sv))
}

fn serde_to_tsn(sv: serde_json::Value) -> Value {
    match sv {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Null
            }
        }
        serde_json::Value::String(s) => Value::Str(Arc::from(s)),
        serde_json::Value::Array(arr) => {
            let items = arr.into_iter().map(serde_to_tsn).collect();
            new_array(items)
        }
        serde_json::Value::Object(map) => {
            let mut obj = ObjData::new();
            for (k, v) in map {
                obj.fields.insert(Arc::from(k), serde_to_tsn(v));
            }
            new_object(obj)
        }
    }
}

pub fn json_stringify(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let v = args.first().unwrap_or(&Value::Null);
    let sv = tsn_to_serde(v);
    let s = serde_json::to_string(&sv).unwrap_or_else(|_| "null".to_owned());
    Ok(Value::Str(Arc::from(s)))
}

fn tsn_to_serde(v: &Value) -> serde_json::Value {
    match v {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(n) => serde_json::Value::Number((*n).into()),
        Value::Float(f) => {
            if let Some(n) = serde_json::Number::from_f64(*f) {
                serde_json::Value::Number(n)
            } else {
                serde_json::Value::Null
            }
        }
        Value::Str(s) => serde_json::Value::String(s.to_string()),
        Value::BigInt(n) => serde_json::Value::String(n.to_string()),
        Value::Array(a) => {
            let arr: Vec<serde_json::Value> = unsafe { &**a }.iter().map(tsn_to_serde).collect();
            serde_json::Value::Array(arr)
        }
        Value::Object(o) => {
            let map: serde_json::Map<String, serde_json::Value> = unsafe { &**o }
                .fields
                .iter()
                .map(|(k, v)| (k.to_string(), tsn_to_serde(&v)))
                .collect();
            serde_json::Value::Object(map)
        }
        _ => serde_json::Value::Null,
    }
}
