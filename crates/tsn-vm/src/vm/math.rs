use rust_decimal::Decimal;
use tsn_types::value::Value;

pub(crate) fn to_i64(v: Value) -> Result<i64, String> {
    match v {
        Value::Int(n) => Ok(n),
        Value::Float(f) => Ok(f as i64),
        Value::Bool(b) => Ok(if b { 1 } else { 0 }),
        _ => Err(format!("expected integer, got {}", v.type_name())),
    }
}

pub(crate) fn numeric_op(
    a: Value,
    b: Value,
    int_op: impl Fn(i64, i64) -> i64,
    float_op: impl Fn(f64, f64) -> f64,
    decimal_op: impl Fn(Decimal, Decimal) -> Decimal,
) -> Result<Value, String> {
    match (&a, &b) {
        (Value::Int(x), Value::Int(y)) => Ok(Value::Int(int_op(*x, *y))),
        (Value::Float(x), Value::Float(y)) => Ok(Value::Float(float_op(*x, *y))),
        (Value::Int(x), Value::Float(y)) => Ok(Value::Float(float_op(*x as f64, *y))),
        (Value::Float(x), Value::Int(y)) => Ok(Value::Float(float_op(*x, *y as f64))),
        (Value::Decimal(x), Value::Decimal(y)) => {
            Ok(Value::Decimal(Box::new(decimal_op(**x, **y))))
        }
        (Value::Decimal(x), Value::Int(y)) => {
            Ok(Value::Decimal(Box::new(decimal_op(**x, Decimal::from(*y)))))
        }
        (Value::Int(x), Value::Decimal(y)) => {
            Ok(Value::Decimal(Box::new(decimal_op(Decimal::from(*x), **y))))
        }
        (Value::Decimal(x), Value::Float(y)) => {
            let d = Decimal::try_from(*y).unwrap_or_default();
            Ok(Value::Decimal(Box::new(decimal_op(**x, d))))
        }
        (Value::Float(x), Value::Decimal(y)) => {
            let d = Decimal::try_from(*x).unwrap_or_default();
            Ok(Value::Decimal(Box::new(decimal_op(d, **y))))
        }
        _ => Err(format!(
            "cannot apply numeric op to {} and {}",
            a.type_name(),
            b.type_name()
        )),
    }
}

pub(crate) fn div_op(a: Value, b: Value) -> Result<Value, String> {
    match (&a, &b) {
        (Value::Int(x), Value::Int(y)) => {
            if *y == 0 {
                Ok(Value::Float(f64::INFINITY))
            } else {
                Ok(Value::Float(*x as f64 / *y as f64))
            }
        }
        (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x / y)),
        (Value::Int(x), Value::Float(y)) => Ok(Value::Float(*x as f64 / y)),
        (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x / *y as f64)),
        (Value::Decimal(x), Value::Decimal(y)) => {
            if y.is_zero() {
                return Err("decimal division by zero".to_string());
            }
            Ok(Value::Decimal(Box::new(**x / **y)))
        }
        (Value::Decimal(x), Value::Int(y)) => {
            if *y == 0 {
                return Err("decimal division by zero".to_string());
            }
            Ok(Value::Decimal(Box::new(**x / Decimal::from(*y))))
        }
        (Value::Int(x), Value::Decimal(y)) => {
            if y.is_zero() {
                return Err("decimal division by zero".to_string());
            }
            Ok(Value::Decimal(Box::new(Decimal::from(*x) / **y)))
        }
        (Value::Decimal(x), Value::Float(y)) => {
            let d = Decimal::try_from(*y).unwrap_or_default();
            if d.is_zero() {
                return Err("decimal division by zero".to_string());
            }
            Ok(Value::Decimal(Box::new(**x / d)))
        }
        (Value::Float(x), Value::Decimal(y)) => {
            if y.is_zero() {
                return Err("decimal division by zero".to_string());
            }
            let d = Decimal::try_from(*x).unwrap_or_default();
            Ok(Value::Decimal(Box::new(d / **y)))
        }
        _ => Err(format!(
            "cannot divide {} by {}",
            a.type_name(),
            b.type_name()
        )),
    }
}

pub(crate) fn pow_op(a: Value, b: Value) -> Result<Value, String> {
    match (&a, &b) {
        (Value::Int(x), Value::Int(y)) => {
            if *y >= 0 {
                Ok(Value::Int(x.pow(*y as u32)))
            } else {
                Ok(Value::Float((*x as f64).powf(*y as f64)))
            }
        }
        (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x.powf(*y))),
        (Value::Int(x), Value::Float(y)) => Ok(Value::Float((*x as f64).powf(*y))),
        (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x.powf(*y as f64))),
        (Value::Decimal(x), Value::Int(y)) => {
            let base = f64::try_from(**x).unwrap_or(0.0);
            let r = base.powi(*y as i32);
            Ok(Value::Decimal(Box::new(
                Decimal::try_from(r).unwrap_or(**x),
            )))
        }
        (Value::Decimal(x), Value::Float(y)) => {
            let base = f64::try_from(**x).unwrap_or(0.0);
            let r = base.powf(*y);
            Ok(Value::Decimal(Box::new(
                Decimal::try_from(r).unwrap_or(**x),
            )))
        }
        (Value::Float(x), Value::Decimal(y)) => {
            let exp = f64::try_from(**y).unwrap_or(0.0);
            let r = x.powf(exp);
            Ok(Value::Decimal(Box::new(
                Decimal::try_from(r).unwrap_or(**y),
            )))
        }
        _ => Err(format!("cannot pow {} ** {}", a.type_name(), b.type_name())),
    }
}

pub(crate) fn negate(v: Value) -> Result<Value, String> {
    match v {
        Value::Int(n) => Ok(Value::Int(-n)),
        Value::Float(f) => Ok(Value::Float(-f)),
        Value::Decimal(d) => Ok(Value::Decimal(Box::new(-*d))),
        _ => Err(format!("cannot negate {}", v.type_name())),
    }
}

pub(crate) fn cmp_lt(a: &Value, b: &Value) -> Result<bool, String> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Ok(x < y),
        (Value::Float(x), Value::Float(y)) => Ok(x < y),
        (Value::Int(x), Value::Float(y)) => Ok((*x as f64) < *y),
        (Value::Float(x), Value::Int(y)) => Ok(*x < (*y as f64)),
        (Value::Str(x), Value::Str(y)) => Ok(**x < **y),
        (Value::Decimal(x), Value::Decimal(y)) => Ok(x < y),
        (Value::Decimal(x), Value::Int(y)) => Ok(**x < Decimal::from(*y)),
        (Value::Int(x), Value::Decimal(y)) => Ok(Decimal::from(*x) < **y),
        (Value::Decimal(x), Value::Float(y)) => Ok(f64::try_from(**x).unwrap_or(0.0) < *y),
        (Value::Float(x), Value::Decimal(y)) => Ok(*x < f64::try_from(**y).unwrap_or(0.0)),
        _ => Err(format!(
            "cannot compare {} < {}",
            a.type_name(),
            b.type_name()
        )),
    }
}

pub(crate) fn cmp_lte(a: &Value, b: &Value) -> Result<bool, String> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Ok(x <= y),
        (Value::Float(x), Value::Float(y)) => Ok(x <= y),
        (Value::Int(x), Value::Float(y)) => Ok((*x as f64) <= *y),
        (Value::Float(x), Value::Int(y)) => Ok(*x <= (*y as f64)),
        (Value::Str(x), Value::Str(y)) => Ok(**x <= **y),
        (Value::Decimal(x), Value::Decimal(y)) => Ok(x <= y),
        (Value::Decimal(x), Value::Int(y)) => Ok(**x <= Decimal::from(*y)),
        (Value::Int(x), Value::Decimal(y)) => Ok(Decimal::from(*x) <= **y),
        (Value::Decimal(x), Value::Float(y)) => Ok(f64::try_from(**x).unwrap_or(0.0) <= *y),
        (Value::Float(x), Value::Decimal(y)) => Ok(*x <= f64::try_from(**y).unwrap_or(0.0)),
        _ => Err(format!(
            "cannot compare {} <= {}",
            a.type_name(),
            b.type_name()
        )),
    }
}
