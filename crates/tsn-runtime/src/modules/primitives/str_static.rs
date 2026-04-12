use std::sync::Arc;
use tsn_types::{value::new_array, Value};

pub fn str_from_value(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(v) = args.first() {
        let s = match v {
            Value::Str(s) => s.as_ref().to_owned(),
            Value::Int(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Bool(b) => {
                if *b {
                    "true".to_owned()
                } else {
                    "false".to_owned()
                }
            }
            Value::Null => "null".to_owned(),
            Value::Char(c) => c.to_string(),
            other => format!("{}", other),
        };
        return Ok(Value::Str(Arc::from(s)));
    }
    Ok(Value::Str(Arc::from("")))
}

pub fn str_from_char_code(
    _ctx: &mut dyn tsn_types::Context,
    args: &[Value],
) -> Result<Value, String> {
    let mut s = String::new();
    for arg in args {
        if let Value::Int(code) = arg {
            if let Some(c) = std::char::from_u32(*code as u32) {
                s.push(c);
                continue;
            }
        }
    }
    Ok(Value::Str(Arc::from(s)))
}

pub fn str_char_code(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        return Ok(s
            .chars()
            .next()
            .map(|c| Value::Int(c as u32 as i64))
            .unwrap_or(Value::Int(-1)));
    }
    Ok(Value::Int(-1))
}

pub fn str_join(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let arr = match args.first() {
        Some(Value::Array(a)) => unsafe { &**a }.clone(),
        _ => return Ok(Value::Str(Arc::from(""))),
    };
    let sep = match args.get(1) {
        Some(Value::Str(s)) => s.as_ref().to_owned(),
        _ => String::new(),
    };
    let parts: Vec<String> = arr
        .iter()
        .map(|v| match v {
            Value::Str(s) => s.as_ref().to_owned(),
            other => other.to_string(),
        })
        .collect();
    Ok(Value::Str(Arc::from(parts.join(&sep))))
}

pub fn str_lines(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        let lines: Vec<Value> = s.lines().map(|l| Value::Str(Arc::from(l))).collect();
        return Ok(new_array(lines));
    }
    Ok(Value::empty_array())
}

pub fn str_words(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        let words: Vec<Value> = s
            .split_whitespace()
            .map(|w| Value::Str(Arc::from(w)))
            .collect();
        return Ok(new_array(words));
    }
    Ok(Value::empty_array())
}
