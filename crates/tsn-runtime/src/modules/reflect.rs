use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::sync::atomic::{AtomicU64, Ordering};
use tsn_types::{value::{new_array, new_object, ObjData}, Value};
use tsn_types::NativeFn;

static METADATA: OnceLock<Mutex<HashMap<String, HashMap<String, Value>>>> = OnceLock::new();
static META_KEY_COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn metadata() -> &'static Mutex<HashMap<String, HashMap<String, Value>>> {
    METADATA.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn target_key(v: &Value) -> String {
    match v {
        Value::Class(c) => c.name.clone(),
        Value::Str(s) => s.to_string(),
        other => other.to_string(),
    }
}

pub fn reflect_define_meta(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let key = args.first().map(|v| v.to_string()).unwrap_or_default();
    let val = args.get(1).cloned().unwrap_or(Value::Null);
    let target = args.get(2).map(target_key).unwrap_or_default();
    metadata().lock().entry(target).or_default().insert(key, val);
    Ok(Value::Null)
}

pub fn reflect_get_meta(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let key = args.first().map(|v| v.to_string()).unwrap_or_default();
    let target = args.get(1).map(target_key).unwrap_or_default();
    let val = metadata().lock().get(&target)
        .and_then(|m| m.get(&key)).cloned().unwrap_or(Value::Null);
    Ok(val)
}

pub fn reflect_has_meta(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let key = args.first().map(|v| v.to_string()).unwrap_or_default();
    let target = args.get(1).map(target_key).unwrap_or_default();
    let has = metadata().lock().get(&target)
        .map(|m| m.contains_key(&key)).unwrap_or(false);
    Ok(Value::Bool(has))
}

pub fn reflect_get_meta_keys(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let target = args.first().map(target_key).unwrap_or_default();
    let keys: Vec<Value> = metadata().lock().get(&target)
        .map(|m| m.keys().map(|k| Value::Str(Arc::from(k.as_str()))).collect())
        .unwrap_or_default();
    Ok(new_array(keys))
}

fn meta_key_receiver(args: &[Value]) -> Result<Arc<str>, String> {
    let recv = args.first().ok_or("MetaKey: missing receiver")?;
    let recv_obj = match recv {
        Value::Object(o) => unsafe { &**o },
        _ => return Err("MetaKey: invalid receiver".into()),
    };
    match recv_obj.fields.get("__key") {
        Some(Value::Str(s)) => Ok(s.clone()),
        _ => Err("MetaKey: missing internal key".into()),
    }
}

pub fn meta_key_create(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    let key_id = META_KEY_COUNTER.fetch_add(1, Ordering::Relaxed);
    let key_name = Arc::from(format!("__meta_key_{}", key_id));

    let mut obj = ObjData::new();
    obj.fields.insert(Arc::from("__key"), Value::Str(key_name));
    let key_obj = new_object(obj);

    if let Value::Object(o) = &key_obj {
        let target = key_obj.clone();
        unsafe { &mut **o }.fields.insert(
            Arc::from("set"),
            Value::native_bound(target.clone(), meta_key_set, "set"),
        );
        unsafe { &mut **o }.fields.insert(
            Arc::from("get"),
            Value::native_bound(target.clone(), meta_key_get, "get"),
        );
        unsafe { &mut **o }.fields.insert(
            Arc::from("has"),
            Value::native_bound(target.clone(), meta_key_has, "has"),
        );
        unsafe { &mut **o }.fields.insert(
            Arc::from("keys"),
            Value::native_bound(target, meta_key_keys, "keys"),
        );
    }

    Ok(key_obj)
}

pub fn meta_key_set(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let key = meta_key_receiver(args)?;
    let target = args.get(1).map(target_key).unwrap_or_default();
    let value = args.get(2).cloned().unwrap_or(Value::Null);
    metadata()
        .lock()
        .entry(target)
        .or_default()
        .insert(key.to_string(), value);
    Ok(Value::Null)
}

pub fn meta_key_get(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let key = meta_key_receiver(args)?;
    let target = args.get(1).map(target_key).unwrap_or_default();
    let value = metadata().lock().get(&target)
        .and_then(|m| m.get(key.as_ref()))
        .cloned()
        .unwrap_or(Value::Null);
    Ok(value)
}

pub fn meta_key_has(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let key = meta_key_receiver(args)?;
    let target = args.get(1).map(target_key).unwrap_or_default();
    let has = metadata().lock().get(&target)
        .map(|m| m.contains_key(key.as_ref()))
        .unwrap_or(false);
    Ok(Value::Bool(has))
}

pub fn meta_key_keys(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let key = meta_key_receiver(args)?;
    let target = args.get(1).map(target_key).unwrap_or_default();
    let keys = metadata().lock().get(&target)
        .and_then(|m| m.get(key.as_ref()))
        .map(|_| vec![Value::Str(Arc::from(target))])
        .unwrap_or_default();
    Ok(new_array(keys))
}

pub fn build() -> Value {
    let mut ns = ObjData::new();
    ns.set_field(Arc::from("defineMetadata"),   Value::NativeFn(Box::new((reflect_define_meta   as NativeFn, "defineMetadata"))));
    ns.set_field(Arc::from("getMetadata"),      Value::NativeFn(Box::new((reflect_get_meta      as NativeFn, "getMetadata"))));
    ns.set_field(Arc::from("hasMetadata"),      Value::NativeFn(Box::new((reflect_has_meta      as NativeFn, "hasMetadata"))));
    ns.set_field(Arc::from("getMetadataKeys"),  Value::NativeFn(Box::new((reflect_get_meta_keys as NativeFn, "getMetadataKeys"))));

    let mut exports = ObjData::new();
    exports.set_field(Arc::from("Reflect"), new_object(ns));

    let mut meta_key = ObjData::new();
    meta_key.set_field(
        Arc::from("create"),
        Value::NativeFn(Box::new((meta_key_create as NativeFn, "create"))),
    );
    exports.set_field(Arc::from("MetaKey"), new_object(meta_key));

    exports.set_field(Arc::from("MethodContext"), Value::plain_object());
    new_object(exports)
}
