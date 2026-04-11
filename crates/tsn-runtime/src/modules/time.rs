use std::sync::Arc;
use tsn_types::{value::{new_object, ObjData}, Value};
use tsn_types::NativeFn;

pub fn unix_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn unix_millis_f64() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or(0.0)
}

pub fn unix_ms_to_iso(ms: i64) -> Arc<str> {
    let secs = (ms / 1000) as u64;
    let millis = (ms % 1000) as u64;
    tsn_core::time::unix_to_iso(secs, millis).into()
}

pub fn time_now(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    Ok(Value::Int(unix_millis()))
}

pub fn time_to_iso(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let ms = int_arg(args, 0);
    Ok(Value::Str(unix_ms_to_iso(ms)))
}

pub fn time_from_iso(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let s = args.first().map(|v| v.to_string()).unwrap_or_default();
    if let Some((secs, ms)) = tsn_core::time::iso_to_unix(&s) {
        Ok(Value::Int((secs * 1000 + ms) as i64))
    } else {
        Err(format!("Invalid ISO date string: {}", s))
    }
}

pub fn time_to_parts(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let ms = int_arg(args, 0);
    let secs = (ms / 1000) as u64;
    let millis = (ms % 1000) as u64;
    let (y, mo, d, h, min, s, wd) = tsn_core::time::secs_to_calendar(secs);
    let mut data = tsn_types::value::ObjData::new();
    data.fields.insert("year".into(),        Value::Int(y as i64));
    data.fields.insert("month".into(),       Value::Int(mo as i64));
    data.fields.insert("day".into(),         Value::Int(d as i64));
    data.fields.insert("hour".into(),        Value::Int(h as i64));
    data.fields.insert("minute".into(),      Value::Int(min as i64));
    data.fields.insert("second".into(),      Value::Int(s as i64));
    data.fields.insert("millisecond".into(), Value::Int(millis as i64));
    data.fields.insert("weekday".into(),     Value::Int(wd as i64));
    Ok(tsn_types::value::new_object(data))
}

pub fn time_from_parts(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let get = |i: usize, default: u64| -> u64 {
        args.get(i).and_then(|v| match v {
            Value::Int(n) => Some(*n as u64),
            _ => None,
        }).unwrap_or(default)
    };
    let ms = tsn_core::time::calendar_to_millis(
        get(0, 1970), get(1, 1), get(2, 1),
        get(3, 0), get(4, 0), get(5, 0), get(6, 0),
    );
    Ok(Value::Int(ms as i64))
}

pub fn int_arg(args: &[Value], idx: usize) -> i64 {
    match args.get(idx) {
        Some(Value::Int(i)) => *i,
        Some(Value::Float(f)) => *f as i64,
        _ => 0,
    }
}

pub fn build() -> Value {
    let mut ns = ObjData::new();
    ns.set_field(Arc::from("now"),         Value::NativeFn(Box::new((time_now        as NativeFn, "now"))));
    ns.set_field(Arc::from("millis"),      Value::NativeFn(Box::new((time_now        as NativeFn, "millis"))));
    ns.set_field(Arc::from("toISOString"), Value::NativeFn(Box::new((time_to_iso     as NativeFn, "toISOString"))));
    ns.set_field(Arc::from("fromISO"),     Value::NativeFn(Box::new((time_from_iso   as NativeFn, "fromISO"))));
    ns.set_field(Arc::from("toParts"),     Value::NativeFn(Box::new((time_to_parts   as NativeFn, "toParts"))));
    ns.set_field(Arc::from("fromParts"),   Value::NativeFn(Box::new((time_from_parts as NativeFn, "fromParts"))));

    let mut exports = ObjData::new();
    exports.set_field(Arc::from("Time"), new_object(ns));
    new_object(exports)
}
