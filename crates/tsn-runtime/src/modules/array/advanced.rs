use tsn_types::{value::new_array, Value};

pub fn array_reverse(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Array(arr)) = args.first() {
        let arr_ptr = *arr;
        let arr_ref = unsafe { &mut *arr_ptr };
        arr_ref.reverse();
        return Ok(Value::Array(arr_ptr));
    }
    Ok(Value::Null)
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
    let mapped = super::array_map(ctx, args)?;
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
