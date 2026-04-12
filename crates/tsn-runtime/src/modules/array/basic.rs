use std::sync::Arc;
use tsn_types::{value::new_array, Value};

pub fn array_is_array(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let val = match (args.first(), args.get(1)) {
        (Some(Value::Array(_)), _) => args.first().unwrap_or(&Value::Null),
        (_, Some(v)) => v,
        _ => &Value::Null,
    };
    Ok(Value::Bool(matches!(val, Value::Array(_))))
}

pub fn array_length(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Array(arr)) = args.first() {
        return Ok(Value::Int(unsafe { &**arr }.len() as i64));
    }
    Ok(Value::Int(0))
}

pub fn array_push(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Array(arr)) = args.first() {
        if let Some(item) = args.get(1) {
            unsafe { &mut **arr }.push(item.clone());
        }
    }
    Ok(Value::Null)
}

pub fn array_pop(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Array(arr)) = args.first() {
        return Ok(unsafe { &mut **arr }.pop().unwrap_or(Value::Null));
    }
    Ok(Value::Null)
}

pub fn array_shift(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Array(arr)) = args.first() {
        let arr = unsafe { &mut **arr };
        if !arr.is_empty() {
            return Ok(arr.remove(0));
        }
    }
    Ok(Value::Null)
}

pub fn array_unshift(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Array(arr)) = args.first() {
        if let Some(item) = args.get(1) {
            unsafe { &mut **arr }.insert(0, item.clone());
        }
    }
    Ok(Value::Null)
}

pub fn array_join(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Array(arr)) = args.first() {
        let sep = match args.get(1) {
            Some(Value::Str(s)) => s.as_ref(),
            _ => ",",
        };
        let arr = unsafe { &**arr };
        let joined = arr
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<String>>()
            .join(sep);
        return Ok(Value::Str(Arc::from(joined)));
    }
    Ok(Value::Str(Arc::from("")))
}

pub fn array_includes(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match (args.first(), args.get(1)) {
        (Some(Value::Array(arr)), Some(search)) => {
            Ok(Value::Bool(unsafe { &**arr }.contains(search)))
        }
        _ => Ok(Value::Bool(false)),
    }
}

pub fn array_index_of(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match (args.first(), args.get(1)) {
        (Some(Value::Array(arr)), Some(search)) => {
            let arr = unsafe { &**arr };
            Ok(Value::Int(
                arr.iter()
                    .position(|v| v == search)
                    .map(|i| i as i64)
                    .unwrap_or(-1),
            ))
        }
        _ => Ok(Value::Int(-1)),
    }
}

pub fn array_slice(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Array(arr)) = args.first() {
        let arr = unsafe { &**arr };
        let len = arr.len() as i64;
        let mut start = match args.get(1) {
            Some(Value::Int(n)) => *n,
            _ => 0,
        };
        let mut end = match args.get(2) {
            Some(Value::Int(n)) => *n,
            _ => len,
        };
        if start < 0 {
            start = (len + start).max(0);
        }
        if end < 0 {
            end = (len + end).max(0);
        }
        start = start.min(len);
        end = end.min(len);
        if start >= end {
            return Ok(Value::empty_array());
        }
        let sliced = arr[start as usize..end as usize].to_vec();
        return Ok(new_array(sliced));
    }
    Ok(Value::Null)
}

pub fn array_at(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let (Some(Value::Array(arr)), Some(Value::Int(pos))) = (args.first(), args.get(1)) {
        let arr = unsafe { &**arr };
        let len = arr.len() as i64;
        let mut idx = *pos;
        if idx < 0 {
            idx += len;
        }
        if idx < 0 || idx >= len {
            return Ok(Value::Null);
        }
        return Ok(arr[idx as usize].clone());
    }
    Ok(Value::Null)
}

pub fn array_concat(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Array(arr)) = args.first() {
        let mut result = unsafe { &**arr }.to_vec();
        for arg in args.iter().skip(1) {
            match arg {
                Value::Array(other) => result.extend_from_slice(unsafe { &**other }),
                _ => result.push(arg.clone()),
            }
        }
        return Ok(new_array(result));
    }
    Ok(Value::empty_array())
}

pub fn array_fill(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Array(arr)) = args.first() {
        if let Some(val) = args.get(1) {
            let arr_ref = unsafe { &mut **arr };
            let len = arr_ref.len() as i64;
            let mut start = match args.get(2) {
                Some(Value::Int(n)) => *n,
                _ => 0,
            };
            let mut end = match args.get(3) {
                Some(Value::Int(n)) => *n,
                _ => len,
            };
            if start < 0 {
                start = (len + start).max(0);
            }
            if end < 0 {
                end = (len + end).max(0);
            }
            for i in start.min(len) as usize..end.min(len) as usize {
                arr_ref[i] = val.clone();
            }
            return Ok(Value::Array(*arr));
        }
    }
    Ok(Value::Null)
}
