use tsn_types::{value::new_array, Value};

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
