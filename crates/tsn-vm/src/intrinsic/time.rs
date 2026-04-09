use std::sync::Arc;
use tsn_op_macros::op;
use tsn_types::value::Value;

#[op("now")]
pub fn time_now(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    Ok(Value::Int(unix_millis()))
}

fn unix_millis() -> i64 {
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

/// Exposed for use inside tsn-stdlib TSN wrappers that call this intrinsic.
pub fn unix_secs_f64() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// Format unix milliseconds as ISO 8601 string.
pub fn unix_ms_to_iso(ms: i64) -> Arc<str> {
    let secs = (ms / 1000) as u64;
    let millis = (ms % 1000) as u64;
    tsn_core::time::unix_to_iso(secs, millis).into()
}

#[op("fromISO")]
pub fn time_from_iso(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let s = args.get(0).map(|v| v.to_string()).unwrap_or_default();
    if let Some((secs, ms)) = tsn_core::time::iso_to_unix(&s) {
        Ok(Value::Int((secs * 1000 + ms) as i64))
    } else {
        Err(format!("Invalid ISO date string: {}", s))
    }
}

#[op("toParts")]
pub fn time_to_parts(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let ms = match args.get(0) {
        Some(Value::Int(i)) => *i,
        Some(Value::Float(f)) => *f as i64,
        _ => 0,
    };
    let secs = (ms / 1000) as u64;
    let millis = (ms % 1000) as u64;
    let (y, mo, d, h, min, s, wd) = tsn_core::time::secs_to_calendar(secs);

    let mut data = tsn_types::value::ObjData::new();
    data.fields.insert("year".into(), Value::Int(y as i64));
    data.fields.insert("month".into(), Value::Int(mo as i64));
    data.fields.insert("day".into(), Value::Int(d as i64));
    data.fields.insert("hour".into(), Value::Int(h as i64));
    data.fields.insert("minute".into(), Value::Int(min as i64));
    data.fields.insert("second".into(), Value::Int(s as i64));
    data.fields
        .insert("millisecond".into(), Value::Int(millis as i64));
    data.fields.insert("weekday".into(), Value::Int(wd as i64));

    Ok(tsn_types::value::new_object(data))
}

#[op("toISOString")]
pub fn time_to_iso(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let ms = match args.get(0) {
        Some(Value::Int(i)) => *i,
        Some(Value::Float(f)) => *f as i64,
        _ => 0,
    };
    Ok(Value::Str(unix_ms_to_iso(ms)))
}

#[op("fromParts")]
pub fn time_from_parts(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let y = args
        .get(0)
        .and_then(|v| match v {
            Value::Int(i) => Some(*i as u64),
            _ => None,
        })
        .unwrap_or(1970);
    let mo = args
        .get(1)
        .and_then(|v| match v {
            Value::Int(i) => Some(*i as u64),
            _ => None,
        })
        .unwrap_or(1);
    let d = args
        .get(2)
        .and_then(|v| match v {
            Value::Int(i) => Some(*i as u64),
            _ => None,
        })
        .unwrap_or(1);
    let h = args
        .get(3)
        .and_then(|v| match v {
            Value::Int(i) => Some(*i as u64),
            _ => None,
        })
        .unwrap_or(0);
    let min = args
        .get(4)
        .and_then(|v| match v {
            Value::Int(i) => Some(*i as u64),
            _ => None,
        })
        .unwrap_or(0);
    let s = args
        .get(5)
        .and_then(|v| match v {
            Value::Int(i) => Some(*i as u64),
            _ => None,
        })
        .unwrap_or(0);
    let ms = args
        .get(6)
        .and_then(|v| match v {
            Value::Int(i) => Some(*i as u64),
            _ => None,
        })
        .unwrap_or(0);

    Ok(Value::Int(
        tsn_core::time::calendar_to_millis(y, mo, d, h, min, s, ms) as i64,
    ))
}

pub const OPS: &[crate::host_ops::HostOp] = &[
    time_now_OP,
    time_from_iso_OP,
    time_to_parts_OP,
    time_to_iso_OP,
    time_from_parts_OP,
];
