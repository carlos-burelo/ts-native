use crate::tsn_types::value::{new_array, Value};
use crate::tsn_types::Context;
use std::sync::Arc;

pub(super) fn get_property(
    obj: &Value,
    start: i64,
    end: i64,
    inclusive: bool,
    key: &str,
) -> Result<Value, String> {
    let len: i64 = if inclusive {
        (end - start + 1).max(0)
    } else {
        (end - start).max(0)
    };

    fn range_to_string(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        if let Some(Value::Range(r)) = args.first() {
            let s = if r.inclusive {
                format!("{}..={}", r.start, r.end)
            } else {
                format!("{}..{}", r.start, r.end)
            };
            Ok(Value::Str(Arc::from(s)))
        } else {
            Err("toString: expected range receiver".to_owned())
        }
    }

    fn range_contains(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        let (range, n) = match (args.first(), args.get(1)) {
            (Some(Value::Range(r)), Some(Value::Int(n))) => (r, *n),
            _ => return Err("contains: expected (range, int)".to_owned()),
        };
        let result = if range.inclusive {
            n >= range.start && n <= range.end
        } else {
            n >= range.start && n < range.end
        };
        Ok(Value::Bool(result))
    }

    fn range_to_array(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        if let Some(Value::Range(r)) = args.first() {
            let end = if r.inclusive { r.end + 1 } else { r.end };
            let items: Vec<Value> = (r.start..end).map(Value::Int).collect();
            Ok(new_array(items))
        } else {
            Err("toArray: expected range receiver".to_owned())
        }
    }

    fn range_step(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        let (range, step) = match (args.first(), args.get(1)) {
            (Some(Value::Range(r)), Some(Value::Int(s))) if *s > 0 => (r, *s as usize),
            (Some(Value::Range(_)), _) => {
                return Err("step: step must be a positive int".to_owned())
            }
            _ => return Err("step: expected (range, int)".to_owned()),
        };
        let end = if range.inclusive {
            range.end + 1
        } else {
            range.end
        };
        let items: Vec<Value> = (range.start..end).step_by(step).map(Value::Int).collect();
        Ok(new_array(items))
    }

    match key {
        "length" => Ok(Value::Int(len)),
        "start" => Ok(Value::Int(start)),
        "end" => Ok(Value::Int(end)),
        "toString" => Ok(Value::native_bound(
            obj.clone(),
            range_to_string,
            "toString",
        )),
        "contains" => Ok(Value::native_bound(obj.clone(), range_contains, "contains")),
        "toArray" => Ok(Value::native_bound(obj.clone(), range_to_array, "toArray")),
        "step" => Ok(Value::native_bound(obj.clone(), range_step, "step")),
        _ => Err(format!("property '{}' not found on range", key)),
    }
}
