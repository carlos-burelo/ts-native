use std::sync::Arc;
use tsn_types::Value;

pub fn float_parse(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        return Ok(Value::Float(s.trim().parse::<f64>().unwrap_or(0.0)));
    }
    Ok(Value::Float(0.0))
}

pub fn float_to_str(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Float(n)) = args.first() {
        return Ok(Value::Str(Arc::from(n.to_string())));
    }
    Ok(Value::Str(Arc::from("0.0")))
}

pub fn float_to_fixed(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Float(n)) = args.first() {
        let decimals = match args.get(1) {
            Some(Value::Int(d)) => *d as usize,
            _ => 0,
        };
        return Ok(Value::Str(Arc::from(format!(
            "{:.prec$}",
            n,
            prec = decimals
        ))));
    }
    Ok(Value::Null)
}

pub fn float_abs(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Float(n)) = args.first() {
        return Ok(Value::Float(n.abs()));
    }
    Ok(Value::Float(0.0))
}

pub fn float_sign(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Float(n)) = args.first() {
        return Ok(Value::Int(n.signum() as i64));
    }
    Ok(Value::Int(0))
}

pub fn float_negate(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Float(n)) = args.first() {
        return Ok(Value::Float(-n));
    }
    Ok(Value::Float(0.0))
}

pub fn float_min(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match (args.first(), args.get(1)) {
        (Some(Value::Float(a)), Some(Value::Float(b))) => Ok(Value::Float(a.min(*b))),
        _ => Ok(Value::Float(0.0)),
    }
}

pub fn float_max(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match (args.first(), args.get(1)) {
        (Some(Value::Float(a)), Some(Value::Float(b))) => Ok(Value::Float(a.max(*b))),
        _ => Ok(Value::Float(0.0)),
    }
}

pub fn float_pow(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match (args.first(), args.get(1)) {
        (Some(Value::Float(base)), Some(Value::Float(exp))) => Ok(Value::Float(base.powf(*exp))),
        _ => Ok(Value::Float(0.0)),
    }
}

pub fn float_is_nan(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Float(f)) = args.first() {
        return Ok(Value::Bool(f.is_nan()));
    }
    Ok(Value::Bool(false))
}

pub fn float_is_finite(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Float(f)) = args.first() {
        return Ok(Value::Bool(f.is_finite()));
    }
    Ok(Value::Bool(false))
}

pub fn float_is_integer(
    _ctx: &mut dyn tsn_types::Context,
    args: &[Value],
) -> Result<Value, String> {
    if let Some(Value::Float(f)) = args.first() {
        return Ok(Value::Bool(f.fract() == 0.0));
    }
    Ok(Value::Bool(false))
}
