use tsn_op_macros::op;
use tsn_types::Value;

#[op("abs")]
pub fn math_abs(args: &[Value]) -> Result<Value, String> {
    match args_get_float(args, 0) {
        Ok(f) => Ok(Value::Float(f.abs())),
        Err(_) => match args.get(0) {
            Some(Value::Int(i)) => Ok(Value::Int(i.abs())),
            _ => Err("Math.abs: expected number".into()),
        },
    }
}

#[op("acos")]
pub fn math_acos(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.acos()))
}

#[op("asin")]
pub fn math_asin(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.asin()))
}

#[op("atan")]
pub fn math_atan(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.atan()))
}

#[op("atan2")]
pub fn math_atan2(args: &[Value]) -> Result<Value, String> {
    let y = args_get_float(args, 0)?;
    let x = args_get_float(args, 1)?;
    Ok(Value::Float(y.atan2(x)))
}

#[op("ceil")]
pub fn math_ceil(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.ceil()))
}

#[op("cos")]
pub fn math_cos(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.cos()))
}

#[op("exp")]
pub fn math_exp(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.exp()))
}

#[op("floor")]
pub fn math_floor(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.floor()))
}

#[op("log")]
pub fn math_log(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.ln()))
}

#[op("max")]
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

#[op("min")]
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

#[op("pow")]
pub fn math_pow(args: &[Value]) -> Result<Value, String> {
    let b = args_get_float(args, 0)?;
    let e = args_get_float(args, 1)?;
    Ok(Value::Float(b.powf(e)))
}

#[op("random")]
pub fn math_random(_args: &[Value]) -> Result<Value, String> {
    use rand::Rng;
    Ok(Value::Float(rand::thread_rng().gen()))
}

#[op("round")]
pub fn math_round(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.round()))
}

#[op("sin")]
pub fn math_sin(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.sin()))
}

#[op("sqrt")]
pub fn math_sqrt(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.sqrt()))
}

#[op("tan")]
pub fn math_tan(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.tan()))
}

#[op("trunc")]
pub fn math_trunc(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(args_get_float(args, 0)?.trunc()))
}

#[op("sign")]
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

pub const OPS: &[crate::host_ops::HostOp] = &[
    math_abs_OP,
    math_acos_OP,
    math_asin_OP,
    math_atan_OP,
    math_atan2_OP,
    math_ceil_OP,
    math_cos_OP,
    math_exp_OP,
    math_floor_OP,
    math_log_OP,
    math_max_OP,
    math_min_OP,
    math_pow_OP,
    math_random_OP,
    math_round_OP,
    math_sin_OP,
    math_sqrt_OP,
    math_tan_OP,
    math_trunc_OP,
    math_sign_OP,
];

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
