use std::sync::Arc;
use tsn_types::{value::new_array, Value};

pub fn array_is_array(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    // args[0] is `this` (the Array class itself, ignored), args[1] is the value to check
    let val = args.get(1).unwrap_or(&Value::Null);
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

pub fn array_filter(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let (Some(Value::Array(arr)), Some(cb)) = (args.first(), args.get(1)) {
        let arr_ptr = *arr;
        let arr_ref = unsafe { &*arr_ptr };
        let mut result = Vec::new();
        for (i, val) in arr_ref.iter().enumerate() {
            let res = ctx.call(
                cb.clone(),
                &[val.clone(), Value::Int(i as i64), Value::Array(arr_ptr)],
            )?;
            if res.is_truthy()? {
                result.push(val.clone());
            }
        }
        return Ok(new_array(result));
    }
    Ok(Value::empty_array())
}

pub fn array_find(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let (Some(Value::Array(arr)), Some(cb)) = (args.first(), args.get(1)) {
        let arr_ptr = *arr;
        let arr_ref = unsafe { &*arr_ptr };
        for (i, val) in arr_ref.iter().enumerate() {
            let res = ctx.call(
                cb.clone(),
                &[val.clone(), Value::Int(i as i64), Value::Array(arr_ptr)],
            )?;
            if res.is_truthy()? {
                return Ok(val.clone());
            }
        }
    }
    Ok(Value::Null)
}

pub fn array_find_index(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let (Some(Value::Array(arr)), Some(cb)) = (args.first(), args.get(1)) {
        let arr_ptr = *arr;
        let arr_ref = unsafe { &*arr_ptr };
        for (i, val) in arr_ref.iter().enumerate() {
            let res = ctx.call(
                cb.clone(),
                &[val.clone(), Value::Int(i as i64), Value::Array(arr_ptr)],
            )?;
            if res.is_truthy()? {
                return Ok(Value::Int(i as i64));
            }
        }
    }
    Ok(Value::Int(-1))
}

pub fn array_for_each(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let (Some(Value::Array(arr)), Some(cb)) = (args.first(), args.get(1)) {
        let arr_ptr = *arr;
        let arr_ref = unsafe { &*arr_ptr };
        for (i, val) in arr_ref.iter().enumerate() {
            ctx.call(
                cb.clone(),
                &[val.clone(), Value::Int(i as i64), Value::Array(arr_ptr)],
            )?;
        }
    }
    Ok(Value::Null)
}

pub fn array_map(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let (Some(Value::Array(arr)), Some(cb)) = (args.first(), args.get(1)) {
        let arr_ptr = *arr;
        let arr_ref = unsafe { &*arr_ptr };
        let mut result = Vec::with_capacity(arr_ref.len());
        for (i, val) in arr_ref.iter().enumerate() {
            result.push(ctx.call(
                cb.clone(),
                &[val.clone(), Value::Int(i as i64), Value::Array(arr_ptr)],
            )?);
        }
        return Ok(new_array(result));
    }
    Ok(Value::empty_array())
}

pub fn array_reduce(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let (Some(Value::Array(arr)), Some(cb)) = (args.first(), args.get(1)) {
        let arr_ptr = *arr;
        let arr_ref = unsafe { &*arr_ptr };
        let mut acc = match args.get(2) {
            Some(initial) => initial.clone(),
            None => {
                if arr_ref.is_empty() {
                    return Err("Reduce of empty array with no initial value".into());
                }
                arr_ref[0].clone()
            }
        };
        let start = if args.get(2).is_some() { 0 } else { 1 };
        for (i, val) in arr_ref.iter().enumerate().skip(start) {
            acc = ctx.call(
                cb.clone(),
                &[
                    acc,
                    val.clone(),
                    Value::Int(i as i64),
                    Value::Array(arr_ptr),
                ],
            )?;
        }
        return Ok(acc);
    }
    Ok(Value::Null)
}

pub fn array_reverse(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Array(arr)) = args.first() {
        let arr_ptr = *arr;
        let arr_ref = unsafe { &mut *arr_ptr };
        arr_ref.reverse();
        return Ok(Value::Array(arr_ptr));
    }
    Ok(Value::Null)
}

pub fn array_every(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let (Some(Value::Array(arr)), Some(cb)) = (args.first(), args.get(1)) {
        let arr_ptr = *arr;
        let arr_ref = unsafe { &*arr_ptr };
        for (i, val) in arr_ref.iter().enumerate() {
            let res = ctx.call(
                cb.clone(),
                &[val.clone(), Value::Int(i as i64), Value::Array(arr_ptr)],
            )?;
            if !res.is_truthy()? {
                return Ok(Value::Bool(false));
            }
        }
        return Ok(Value::Bool(true));
    }
    Ok(Value::Bool(false))
}

pub fn array_some(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let (Some(Value::Array(arr)), Some(cb)) = (args.first(), args.get(1)) {
        let arr_ptr = *arr;
        let arr_ref = unsafe { &*arr_ptr };
        for (i, val) in arr_ref.iter().enumerate() {
            let res = ctx.call(
                cb.clone(),
                &[val.clone(), Value::Int(i as i64), Value::Array(arr_ptr)],
            )?;
            if res.is_truthy()? {
                return Ok(Value::Bool(true));
            }
        }
    }
    Ok(Value::Bool(false))
}

pub fn array_splice(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Array(arr)) = args.first() {
        let arr_ptr = *arr;
        let arr_mut = unsafe { &mut *arr_ptr };
        let len = arr_mut.len() as i64;
        let mut start = match args.get(1) {
            Some(Value::Int(n)) => *n,
            _ => 0,
        };
        if start < 0 {
            start = (len + start).max(0);
        }
        start = start.min(len);
        let delete_count = match args.get(2) {
            Some(Value::Int(n)) => (*n).max(0).min(len - start),
            _ => len - start,
        };
        let items = &args[3..];
        let removed: Vec<Value> = arr_mut
            .drain(start as usize..(start + delete_count) as usize)
            .collect();
        for (i, item) in items.iter().enumerate() {
            arr_mut.insert((start + i as i64) as usize, item.clone());
        }
        return Ok(new_array(removed));
    }
    Ok(Value::empty_array())
}

pub fn array_flat(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Array(arr)) = args.first() {
        let arr_ptr = *arr;
        let depth = match args.get(1) {
            Some(Value::Int(d)) => *d,
            _ => 1,
        };
        let mut result = Vec::new();
        fn flatten(src: &[Value], dest: &mut Vec<Value>, depth: i64) {
            for val in src {
                if depth > 0 {
                    if let Value::Array(nested) = val {
                        flatten(unsafe { &**nested }, dest, depth - 1);
                        continue;
                    }
                }
                dest.push(val.clone());
            }
        }
        flatten(unsafe { &*arr_ptr }, &mut result, depth);
        return Ok(new_array(result));
    }
    Ok(Value::empty_array())
}

pub fn array_flat_map(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let mapped = array_map(ctx, args)?;
    if let Value::Array(arr) = mapped {
        let arr_ptr = arr;
        let mut result = Vec::new();
        for val in unsafe { &*arr_ptr }.iter() {
            if let Value::Array(nested) = val {
                result.extend_from_slice(unsafe { &**nested });
            } else {
                result.push(val.clone());
            }
        }
        return Ok(new_array(result));
    }
    Ok(mapped)
}

pub fn array_sort(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Array(arr)) = args.first() {
        let arr_ptr = *arr;
        let arr_mut = unsafe { &mut *arr_ptr };
        let compare_fn = args.get(1).filter(|v| !v.is_null());
        let mut err = None;
        arr_mut.sort_by(|a, b| {
            if err.is_some() {
                return std::cmp::Ordering::Equal;
            }
            if let Some(cb) = compare_fn {
                match ctx.call(cb.clone(), &[a.clone(), b.clone()]) {
                    Ok(Value::Int(n)) => {
                        if n < 0 {
                            std::cmp::Ordering::Less
                        } else if n > 0 {
                            std::cmp::Ordering::Greater
                        } else {
                            std::cmp::Ordering::Equal
                        }
                    }
                    Err(e) => {
                        err = Some(e);
                        std::cmp::Ordering::Equal
                    }
                    _ => std::cmp::Ordering::Equal,
                }
            } else {
                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
            }
        });
        if let Some(e) = err {
            return Err(e);
        }
        return Ok(Value::Array(arr_ptr));
    }
    Ok(Value::Null)
}
