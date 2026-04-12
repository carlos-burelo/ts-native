use std::sync::Arc;
use tsn_types::Value;

pub fn int_parse(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        return Ok(Value::Int(s.trim().parse::<i64>().unwrap_or(0)));
    }
    Ok(Value::Int(0))
}

pub fn int_to_str(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Int(n)) = args.first() {
        return Ok(Value::Str(Arc::from(n.to_string())));
    }
    Ok(Value::Str(Arc::from("0")))
}

pub fn int_to_fixed(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Int(n)) = args.first() {
        let decimals = match args.get(1) {
            Some(Value::Int(d)) => *d as usize,
            _ => 0,
        };
        return Ok(Value::Str(Arc::from(format!(
            "{:.prec$}",
            *n as f64,
            prec = decimals
        ))));
    }
    Ok(Value::Null)
}

pub fn int_abs(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Int(n)) = args.first() {
        return Ok(Value::Int(n.abs()));
    }
    Ok(Value::Int(0))
}

pub fn int_sign(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Int(n)) = args.first() {
        return Ok(Value::Int(n.signum()));
    }
    Ok(Value::Int(0))
}

pub fn int_negate(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Int(n)) = args.first() {
        return Ok(Value::Int(-n));
    }
    Ok(Value::Int(0))
}

pub fn int_bitwise_not(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Int(n)) = args.first() {
        return Ok(Value::Int(!n));
    }
    Ok(Value::Int(0))
}

pub fn int_min(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match (args.first(), args.get(1)) {
        (Some(Value::Int(a)), Some(Value::Int(b))) => Ok(Value::Int(*a.min(b))),
        _ => Ok(Value::Int(0)),
    }
}

pub fn int_max(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match (args.first(), args.get(1)) {
        (Some(Value::Int(a)), Some(Value::Int(b))) => Ok(Value::Int(*a.max(b))),
        _ => Ok(Value::Int(0)),
    }
}

pub fn int_clamp(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match (args.first(), args.get(1), args.get(2)) {
        (Some(Value::Int(n)), Some(Value::Int(lo)), Some(Value::Int(hi))) => {
            Ok(Value::Int((*n).clamp(*lo, *hi)))
        }
        _ => Ok(Value::Int(0)),
    }
}

pub fn int_to_hex(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Int(n)) = args.first() {
        return Ok(Value::Str(Arc::from(format!("{:x}", n))));
    }
    Ok(Value::Null)
}

pub fn int_to_binary(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Int(n)) = args.first() {
        return Ok(Value::Str(Arc::from(format!("{:b}", n))));
    }
    Ok(Value::Null)
}

pub fn int_to_octal(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Int(n)) = args.first() {
        return Ok(Value::Str(Arc::from(format!("{:o}", n))));
    }
    Ok(Value::Null)
}

pub fn int_to_float(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Int(n)) = args.first() {
        return Ok(Value::Float(*n as f64));
    }
    Ok(Value::Float(0.0))
}

pub fn int_pow(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match (args.first(), args.get(1)) {
        (Some(Value::Int(base)), Some(Value::Int(exp))) => {
            if *exp < 0 {
                return Err("exponent must be non-negative".into());
            }
            Ok(Value::Int(base.pow(*exp as u32)))
        }
        _ => Ok(Value::Int(0)),
    }
}

pub fn int_is_integer(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(val) = args.first() {
        return Ok(Value::Bool(matches!(val, Value::Int(_))));
    }
    Ok(Value::Bool(false))
}
