use tsn_types::Value;

pub fn math_abs(args: &[Value]) -> Result<Value, String> {
    match args_get_float(args, 0) {
        Ok(f) => Ok(Value::Float(f.abs())),
        Err(_) => match args.get(0) {
            Some(Value::Int(i)) => Ok(Value::Int(i.abs())),
            _ => Err("Math.abs: expected number".into()),
        },
    }
}

pub fn math_acos(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.acos()))
}

pub fn math_asin(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.asin()))
}

pub fn math_atan(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.atan()))
}

pub fn math_atan2(args: &[Value]) -> Result<Value, String> {
    let y = args_get_float(args, 0)?;
    let x = args_get_float(args, 1)?;
    Ok(Value::Float(y.atan2(x)))
}

pub fn math_ceil(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.ceil()))
}

pub fn math_cos(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.cos()))
}

pub fn math_exp(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.exp()))
}

pub fn math_floor(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.floor()))
}

pub fn math_log(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.ln()))
}

pub fn math_max(args: &[Value]) -> Result<Value, String> {
    let mut max = f64::NEG_INFINITY;
    if args.is_empty() {
        return Ok(Value::Float(max));
    }
    for arg in args {
        match arg {
            Value::Array(ptr) => {
                let elements = unsafe { &**ptr };
                for item in elements {
                    let val = to_float(item)?;
                    if val > max {
                        max = val;
                    }
                }
            }
            _ => {
                let val = to_float(arg)?;
                if val > max {
                    max = val;
                }
            }
        }
    }
    Ok(Value::Float(max))
}

pub fn math_min(args: &[Value]) -> Result<Value, String> {
    let mut min = f64::INFINITY;
    if args.is_empty() {
        return Ok(Value::Float(min));
    }
    for arg in args {
        match arg {
            Value::Array(ptr) => {
                let elements = unsafe { &**ptr };
                for item in elements {
                    let val = to_float(item)?;
                    if val < min {
                        min = val;
                    }
                }
            }
            _ => {
                let val = to_float(arg)?;
                if val < min {
                    min = val;
                }
            }
        }
    }
    Ok(Value::Float(min))
}

pub fn math_pow(args: &[Value]) -> Result<Value, String> {
    let b = args_get_float(args, 0)?;
    let e = args_get_float(args, 1)?;
    Ok(Value::Float(b.powf(e)))
}

pub fn math_random(_args: &[Value]) -> Result<Value, String> {
    use rand::Rng;
    Ok(Value::Float(rand::thread_rng().gen()))
}

pub fn math_round(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.round()))
}

pub fn math_sin(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.sin()))
}

pub fn math_sqrt(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.sqrt()))
}

pub fn math_tan(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.tan()))
}

pub fn math_trunc(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.trunc()))
}

pub fn math_sign(args: &[Value]) -> Result<Value, String> {
    let f = args_get_float(args, 0)?;
    if f == 0.0 {
        Ok(Value::Float(0.0))
    } else if f > 0.0 {
        Ok(Value::Float(1.0))
    } else {
        Ok(Value::Float(-1.0))
    }
}

pub fn math_nan(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(f64::NAN))
}

pub fn math_infinity(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(f64::INFINITY))
}

// Helpers
fn to_float(v: &Value) -> Result<f64, String> {
    match v {
        Value::Int(i) => Ok(*i as f64),
        Value::Float(f) => Ok(*f),
        _ => Err(format!("expected number, got {}", v.type_name())),
    }
}

fn args_get_float(args: &[Value], idx: usize) -> Result<f64, String> {
    args.get(idx)
        .ok_or_else(|| format!("missing argument at index {}", idx))
        .and_then(|v| to_float(v))
}
