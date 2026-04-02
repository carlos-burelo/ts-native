use crate::tsn_types::value::Value;
use crate::tsn_types::{Context, NativeFn};
use std::sync::Arc;

macro_rules! get_decimal {
    ($args:expr, $name:expr) => {
        match $args.first() {
            Some(Value::Decimal(d)) => **d,
            _ => return Err(format!("{}: expected decimal receiver", $name)),
        }
    };
}

pub(super) fn get_property(obj: &Value, key: &str) -> Result<Value, String> {
    fn dec_to_string(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        let d = get_decimal!(args, "toString");
        Ok(Value::Str(Arc::from(d.to_string())))
    }

    fn dec_to_float(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        let d = get_decimal!(args, "toFloat");
        Ok(Value::Float(f64::try_from(d).unwrap_or(f64::NAN)))
    }

    fn dec_to_int(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        let d = get_decimal!(args, "toInt");
        Ok(Value::Int(f64::try_from(d).unwrap_or(0.0) as i64))
    }

    fn dec_abs(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        let d = get_decimal!(args, "abs");
        Ok(Value::Decimal(Box::new(d.abs())))
    }

    fn dec_ceil(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        let d = get_decimal!(args, "ceil");
        Ok(Value::Decimal(Box::new(d.ceil())))
    }

    fn dec_floor(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        let d = get_decimal!(args, "floor");
        Ok(Value::Decimal(Box::new(d.floor())))
    }

    fn dec_round(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        let d = get_decimal!(args, "round");
        Ok(Value::Decimal(Box::new(d.round())))
    }

    fn dec_negate(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        let d = get_decimal!(args, "negate");
        Ok(Value::Decimal(Box::new(-d)))
    }

    fn dec_to_fixed(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        let d = get_decimal!(args, "toFixed");
        let places = match args.get(1) {
            Some(Value::Int(n)) => *n as usize,
            _ => 2,
        };
        Ok(Value::Str(Arc::from(format!(
            "{:.prec$}",
            d,
            prec = places
        ))))
    }

    fn dec_trunc(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        let d = get_decimal!(args, "trunc");
        Ok(Value::Decimal(Box::new(d.trunc())))
    }

    fn dec_fract(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        let d = get_decimal!(args, "fract");
        Ok(Value::Decimal(Box::new(d.fract())))
    }

    fn dec_scale(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        let d = get_decimal!(args, "scale");
        Ok(Value::Int(d.scale() as i64))
    }

    fn dec_is_zero(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        let d = get_decimal!(args, "isZero");
        Ok(Value::Bool(d.is_zero()))
    }

    fn dec_is_sign_positive(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        let d = get_decimal!(args, "isPositive");
        Ok(Value::Bool(d.is_sign_positive()))
    }

    fn dec_is_sign_negative(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        let d = get_decimal!(args, "isNegative");
        Ok(Value::Bool(d.is_sign_negative()))
    }

    let method_fn: Option<(NativeFn, &'static str)> = match key {
        "toString" | "toStr" | "valueOf" => Some((dec_to_string as _, "toString")),
        "toFloat" => Some((dec_to_float as _, "toFloat")),
        "toInt" => Some((dec_to_int as _, "toInt")),
        "abs" => Some((dec_abs as _, "abs")),
        "ceil" => Some((dec_ceil as _, "ceil")),
        "floor" => Some((dec_floor as _, "floor")),
        "round" => Some((dec_round as _, "round")),
        "negate" => Some((dec_negate as _, "negate")),
        "toFixed" => Some((dec_to_fixed as _, "toFixed")),
        "trunc" => Some((dec_trunc as _, "trunc")),
        "fract" => Some((dec_fract as _, "fract")),
        "scale" => Some((dec_scale as _, "scale")),
        "isZero" => Some((dec_is_zero as _, "isZero")),
        "isPositive" => Some((dec_is_sign_positive as _, "isPositive")),
        "isNegative" => Some((dec_is_sign_negative as _, "isNegative")),
        _ => None,
    };
    match method_fn {
        Some((f, name)) => Ok(Value::native_bound(obj.clone(), f, name)),
        None => Err(format!("method '{}' not found on decimal", key)),
    }
}
