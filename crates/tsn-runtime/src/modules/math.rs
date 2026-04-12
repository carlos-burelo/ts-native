use std::sync::Arc;
use tsn_types::NativeFn;
use tsn_types::{
    value::{new_object, ObjData},
    Value,
};

pub fn get_f(args: &[Value], idx: usize) -> Result<f64, String> {
    match args.get(idx) {
        Some(Value::Float(f)) => Ok(*f),
        Some(Value::Int(i)) => Ok(*i as f64),
        _ => Err("Math: expected number".into()),
    }
}

pub fn math_abs(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::Float(f)) => Ok(Value::Float(f.abs())),
        Some(Value::Int(i)) => Ok(Value::Int(i.abs())),
        _ => Err("Math.abs: expected number".into()),
    }
}

pub fn math_sign(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::Float(f)) => Ok(Value::Float(f.signum())),
        Some(Value::Int(i)) => Ok(Value::Int(i.signum())),
        _ => Err("Math.sign: expected number".into()),
    }
}

pub fn math_floor(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match args.first() {
        Some(Value::Float(f)) => Ok(Value::Float(f.floor())),
        Some(Value::Int(i)) => Ok(Value::Int(*i)),
        _ => Err("Math.floor: expected number".into()),
    }
}

pub fn math_ceil(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(get_f(args, 0)?.ceil()))
}

pub fn math_round(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(get_f(args, 0)?.round()))
}

pub fn math_trunc(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(get_f(args, 0)?.trunc()))
}

pub fn math_sqrt(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(get_f(args, 0)?.sqrt()))
}

pub fn math_cbrt(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(get_f(args, 0)?.cbrt()))
}

pub fn math_pow(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(get_f(args, 0)?.powf(get_f(args, 1)?)))
}

pub fn math_exp(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(get_f(args, 0)?.exp()))
}

pub fn math_log(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(get_f(args, 0)?.ln()))
}

pub fn math_log2(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(get_f(args, 0)?.log2()))
}

pub fn math_log10(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(get_f(args, 0)?.log10()))
}

pub fn math_sin(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(get_f(args, 0)?.sin()))
}

pub fn math_cos(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(get_f(args, 0)?.cos()))
}

pub fn math_tan(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(get_f(args, 0)?.tan()))
}

pub fn math_asin(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(get_f(args, 0)?.asin()))
}

pub fn math_acos(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(get_f(args, 0)?.acos()))
}

pub fn math_atan(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(get_f(args, 0)?.atan()))
}

pub fn math_atan2(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(get_f(args, 0)?.atan2(get_f(args, 1)?)))
}

pub fn math_hypot(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(Value::Float(get_f(args, 0)?.hypot(get_f(args, 1)?)))
}

pub fn math_max(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let mut m = f64::NEG_INFINITY;
    for v in args {
        m = m.max(get_f(std::slice::from_ref(v), 0)?);
    }
    Ok(Value::Float(m))
}

pub fn math_min(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let mut m = f64::INFINITY;
    for v in args {
        m = m.min(get_f(std::slice::from_ref(v), 0)?);
    }
    Ok(Value::Float(m))
}

pub fn math_random(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    use rand::Rng;
    Ok(Value::Float(rand::thread_rng().gen()))
}

pub fn build() -> Value {
    let mut ns = ObjData::new();
    ns.set_field(Arc::from("E"), Value::Float(std::f64::consts::E));
    ns.set_field(Arc::from("LN2"), Value::Float(std::f64::consts::LN_2));
    ns.set_field(Arc::from("LN10"), Value::Float(std::f64::consts::LN_10));
    ns.set_field(Arc::from("LOG2E"), Value::Float(std::f64::consts::LOG2_E));
    ns.set_field(Arc::from("LOG10E"), Value::Float(std::f64::consts::LOG10_E));
    ns.set_field(Arc::from("PI"), Value::Float(std::f64::consts::PI));
    ns.set_field(
        Arc::from("SQRT1_2"),
        Value::Float(std::f64::consts::FRAC_1_SQRT_2),
    );
    ns.set_field(Arc::from("SQRT2"), Value::Float(std::f64::consts::SQRT_2));

    ns.set_field(
        Arc::from("abs"),
        Value::NativeFn(Box::new((math_abs as NativeFn, "abs"))),
    );
    ns.set_field(
        Arc::from("sign"),
        Value::NativeFn(Box::new((math_sign as NativeFn, "sign"))),
    );
    ns.set_field(
        Arc::from("floor"),
        Value::NativeFn(Box::new((math_floor as NativeFn, "floor"))),
    );
    ns.set_field(
        Arc::from("ceil"),
        Value::NativeFn(Box::new((math_ceil as NativeFn, "ceil"))),
    );
    ns.set_field(
        Arc::from("round"),
        Value::NativeFn(Box::new((math_round as NativeFn, "round"))),
    );
    ns.set_field(
        Arc::from("trunc"),
        Value::NativeFn(Box::new((math_trunc as NativeFn, "trunc"))),
    );
    ns.set_field(
        Arc::from("sqrt"),
        Value::NativeFn(Box::new((math_sqrt as NativeFn, "sqrt"))),
    );
    ns.set_field(
        Arc::from("cbrt"),
        Value::NativeFn(Box::new((math_cbrt as NativeFn, "cbrt"))),
    );
    ns.set_field(
        Arc::from("pow"),
        Value::NativeFn(Box::new((math_pow as NativeFn, "pow"))),
    );
    ns.set_field(
        Arc::from("exp"),
        Value::NativeFn(Box::new((math_exp as NativeFn, "exp"))),
    );
    ns.set_field(
        Arc::from("log"),
        Value::NativeFn(Box::new((math_log as NativeFn, "log"))),
    );
    ns.set_field(
        Arc::from("log2"),
        Value::NativeFn(Box::new((math_log2 as NativeFn, "log2"))),
    );
    ns.set_field(
        Arc::from("log10"),
        Value::NativeFn(Box::new((math_log10 as NativeFn, "log10"))),
    );
    ns.set_field(
        Arc::from("sin"),
        Value::NativeFn(Box::new((math_sin as NativeFn, "sin"))),
    );
    ns.set_field(
        Arc::from("cos"),
        Value::NativeFn(Box::new((math_cos as NativeFn, "cos"))),
    );
    ns.set_field(
        Arc::from("tan"),
        Value::NativeFn(Box::new((math_tan as NativeFn, "tan"))),
    );
    ns.set_field(
        Arc::from("asin"),
        Value::NativeFn(Box::new((math_asin as NativeFn, "asin"))),
    );
    ns.set_field(
        Arc::from("acos"),
        Value::NativeFn(Box::new((math_acos as NativeFn, "acos"))),
    );
    ns.set_field(
        Arc::from("atan"),
        Value::NativeFn(Box::new((math_atan as NativeFn, "atan"))),
    );
    ns.set_field(
        Arc::from("atan2"),
        Value::NativeFn(Box::new((math_atan2 as NativeFn, "atan2"))),
    );
    ns.set_field(
        Arc::from("hypot"),
        Value::NativeFn(Box::new((math_hypot as NativeFn, "hypot"))),
    );
    ns.set_field(
        Arc::from("max"),
        Value::NativeFn(Box::new((math_max as NativeFn, "max"))),
    );
    ns.set_field(
        Arc::from("min"),
        Value::NativeFn(Box::new((math_min as NativeFn, "min"))),
    );
    ns.set_field(
        Arc::from("random"),
        Value::NativeFn(Box::new((math_random as NativeFn, "random"))),
    );

    let mut exports = ObjData::new();
    exports.set_field(Arc::from("Math"), new_object(ns));
    new_object(exports)
}
