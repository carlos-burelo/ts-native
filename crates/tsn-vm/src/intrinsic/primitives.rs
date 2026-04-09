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

pub fn str_char_code(
    _ctx: &mut dyn tsn_types::Context,
    args: &[Value],
) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        return Ok(s
            .chars()
            .next()
            .map(|c| Value::Int(c as u32 as i64))
            .unwrap_or(Value::Int(-1)));
    }
    Ok(Value::Int(-1))
}

pub fn str_join(
    _ctx: &mut dyn tsn_types::Context,
    args: &[Value],
) -> Result<Value, String> {
    let arr = match args.first() {
        Some(Value::Array(a)) => unsafe { &**a }.clone(),
        _ => return Ok(Value::Str(Arc::from(""))),
    };
    let sep = match args.get(1) {
        Some(Value::Str(s)) => s.as_ref().to_owned(),
        _ => String::new(),
    };
    let parts: Vec<String> = arr.iter().map(|v| match v {
        Value::Str(s) => s.as_ref().to_owned(),
        other => other.to_string(),
    }).collect();
    Ok(Value::Str(Arc::from(parts.join(&sep))))
}

pub fn str_length(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        return Ok(Value::Int(s.chars().count() as i64));
    }
    Ok(Value::Int(0))
}


pub fn str_to_lower(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        return Ok(Value::Str(Arc::from(s.to_lowercase())));
    }
    Ok(Value::Null)
}

pub fn str_to_upper(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        return Ok(Value::Str(Arc::from(s.to_uppercase())));
    }
    Ok(Value::Null)
}

pub fn str_trim(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        return Ok(Value::Str(Arc::from(s.trim())));
    }
    Ok(Value::Null)
}

pub fn str_trim_start(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        return Ok(Value::Str(Arc::from(s.trim_start())));
    }
    Ok(Value::Null)
}

pub fn str_trim_end(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        return Ok(Value::Str(Arc::from(s.trim_end())));
    }
    Ok(Value::Null)
}

pub fn str_includes(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match (args.first(), args.get(1)) {
        (Some(Value::Str(s)), Some(Value::Str(search))) => {
            Ok(Value::Bool(s.contains(search.as_ref())))
        }
        _ => Ok(Value::Bool(false)),
    }
}

pub fn str_starts_with(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match (args.first(), args.get(1)) {
        (Some(Value::Str(s)), Some(Value::Str(search))) => {
            Ok(Value::Bool(s.starts_with(search.as_ref())))
        }
        _ => Ok(Value::Bool(false)),
    }
}

pub fn str_ends_with(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match (args.first(), args.get(1)) {
        (Some(Value::Str(s)), Some(Value::Str(search))) => {
            Ok(Value::Bool(s.ends_with(search.as_ref())))
        }
        _ => Ok(Value::Bool(false)),
    }
}

pub fn str_index_of(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match (args.first(), args.get(1)) {
        (Some(Value::Str(s)), Some(Value::Str(search))) => Ok(Value::Int(
            s.find(search.as_ref()).map(|i| i as i64).unwrap_or(-1),
        )),
        _ => Ok(Value::Int(-1)),
    }
}

pub fn str_last_index_of(
    _ctx: &mut dyn tsn_types::Context,
    args: &[Value],
) -> Result<Value, String> {
    match (args.first(), args.get(1)) {
        (Some(Value::Str(s)), Some(Value::Str(search))) => Ok(Value::Int(
            s.rfind(search.as_ref()).map(|i| i as i64).unwrap_or(-1),
        )),
        _ => Ok(Value::Int(-1)),
    }
}

pub fn str_substring(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        let chars: Vec<char> = s.chars().collect();
        let len = chars.len();
        let start = match args.get(1) {
            Some(Value::Int(n)) => (*n as usize).min(len),
            _ => 0,
        };
        let end = match args.get(2) {
            Some(Value::Int(n)) => (*n as usize).min(len),
            _ => len,
        };
        let (a, b) = (start.min(end), start.max(end));
        return Ok(Value::Str(Arc::from(
            chars[a..b].iter().collect::<String>(),
        )));
    }
    Ok(Value::Null)
}

pub fn str_slice(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        let chars: Vec<char> = s.chars().collect();
        let len = chars.len() as i64;
        let mut start = match args.get(1) {
            Some(Value::Int(n)) => *n,
            _ => 0,
        };
        let mut end = match args.get(2) {
            Some(Value::Int(n)) => *n,
            _ => len,
        };
        if start < 0 {
            start = (len + start).max(0);
        }
        if end < 0 {
            end = (len + end).max(0);
        }
        start = start.min(len);
        end = end.min(len);
        if start >= end {
            return Ok(Value::Str(Arc::from("")));
        }
        return Ok(Value::Str(Arc::from(
            chars[start as usize..end as usize]
                .iter()
                .collect::<String>(),
        )));
    }
    Ok(Value::Null)
}

pub fn str_replace(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match (args.first(), args.get(1), args.get(2)) {
        (Some(Value::Str(s)), Some(Value::Str(from)), Some(Value::Str(to))) => Ok(Value::Str(
            Arc::from(s.replacen(from.as_ref(), to.as_ref(), 1)),
        )),
        _ => Ok(Value::Null),
    }
}

pub fn str_replace_all(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match (args.first(), args.get(1), args.get(2)) {
        (Some(Value::Str(s)), Some(Value::Str(from)), Some(Value::Str(to))) => {
            Ok(Value::Str(Arc::from(s.replace(from.as_ref(), to.as_ref()))))
        }
        _ => Ok(Value::Null),
    }
}

pub fn str_split(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match (args.first(), args.get(1)) {
        (Some(Value::Str(s)), Some(Value::Str(sep))) => {
            let parts: Vec<Value> = s
                .split(sep.as_ref())
                .map(|p| Value::Str(Arc::from(p)))
                .collect();
            Ok(new_array(parts))
        }
        (Some(Value::Str(s)), _) => {
            let parts: Vec<Value> = s
                .chars()
                .map(|c| Value::Str(Arc::from(c.to_string())))
                .collect();
            Ok(new_array(parts))
        }
        _ => Ok(Value::empty_array()),
    }
}

pub fn str_char_at(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        if let Some(Value::Int(pos)) = args.get(1) {
            return Ok(s
                .chars()
                .nth(*pos as usize)
                .map(|c| Value::Str(Arc::from(c.to_string())))
                .unwrap_or(Value::Str(Arc::from(""))));
        }
    }
    Ok(Value::Str(Arc::from("")))
}

pub fn str_char_code_at(
    _ctx: &mut dyn tsn_types::Context,
    args: &[Value],
) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        if let Some(Value::Int(pos)) = args.get(1) {
            return Ok(s
                .chars()
                .nth(*pos as usize)
                .map(|c| Value::Int(c as u32 as i64))
                .unwrap_or(Value::Int(-1)));
        }
    }
    Ok(Value::Int(-1))
}

pub fn str_repeat(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match (args.first(), args.get(1)) {
        (Some(Value::Str(s)), Some(Value::Int(n))) => {
            Ok(Value::Str(Arc::from(s.repeat(*n as usize))))
        }
        _ => Ok(Value::Null),
    }
}

pub fn str_pad_start(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        if let Some(Value::Int(target)) = args.get(1) {
            let pad = match args.get(2) {
                Some(Value::Str(p)) => p.as_ref(),
                _ => " ",
            };
            let len = s.chars().count();
            if len >= *target as usize {
                return Ok(Value::Str(s.clone()));
            }
            let mut res = String::new();
            let to_pad = *target as usize - len;
            for _ in 0..to_pad / pad.chars().count() {
                res.push_str(pad);
            }
            res.push_str(&pad[..to_pad % pad.len()]);
            res.push_str(s.as_ref());
            return Ok(Value::Str(Arc::from(res)));
        }
    }
    Ok(Value::Null)
}

pub fn str_pad_end(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        if let Some(Value::Int(target)) = args.get(1) {
            let pad = match args.get(2) {
                Some(Value::Str(p)) => p.as_ref(),
                _ => " ",
            };
            let len = s.chars().count();
            if len >= *target as usize {
                return Ok(Value::Str(s.clone()));
            }
            let mut res = s.to_string();
            let to_pad = *target as usize - len;
            for _ in 0..to_pad / pad.chars().count() {
                res.push_str(pad);
            }
            res.push_str(&pad[..to_pad % pad.len()]);
            return Ok(Value::Str(Arc::from(res)));
        }
    }
    Ok(Value::Null)
}

pub fn str_concat(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    match (args.first(), args.get(1)) {
        (Some(Value::Str(a)), Some(Value::Str(b))) => {
            Ok(Value::Str(Arc::from(format!("{}{}", a, b))))
        }
        _ => Ok(Value::Null),
    }
}

pub fn str_substr(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        let chars: Vec<char> = s.chars().collect();
        let len = chars.len();
        let mut start = match args.get(1) {
            Some(Value::Int(n)) => *n,
            _ => 0,
        };
        if start < 0 {
            start = (len as i64 + start).max(0);
        }
        let sub_len = match args.get(2) {
            Some(Value::Int(n)) => *n as usize,
            _ => len,
        };
        let end = (start as usize + sub_len).min(len);
        return Ok(Value::Str(Arc::from(
            chars[start as usize..end].iter().collect::<String>(),
        )));
    }
    Ok(Value::Null)
}

pub fn str_is_empty(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        return Ok(Value::Bool(s.is_empty()));
    }
    Ok(Value::Bool(true))
}

pub fn str_is_blank(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        return Ok(Value::Bool(s.trim().is_empty()));
    }
    Ok(Value::Bool(true))
}

pub fn str_is_digit(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        return Ok(Value::Bool(
            !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()),
        ));
    }
    Ok(Value::Bool(false))
}

pub fn str_is_letter(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        return Ok(Value::Bool(
            !s.is_empty() && s.chars().all(|c| c.is_alphabetic()),
        ));
    }
    Ok(Value::Bool(false))
}

pub fn str_is_whitespace(
    _ctx: &mut dyn tsn_types::Context,
    args: &[Value],
) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        return Ok(Value::Bool(
            !s.is_empty() && s.chars().all(|c| c.is_whitespace()),
        ));
    }
    Ok(Value::Bool(false))
}

pub fn str_reverse(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        return Ok(Value::Str(Arc::from(s.chars().rev().collect::<String>())));
    }
    Ok(Value::Null)
}

pub fn str_capitalize(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        if s.is_empty() {
            return Ok(Value::Str(s.clone()));
        }
        let mut chars = s.chars();
        let res = chars.next().unwrap().to_uppercase().to_string() + chars.as_str();
        return Ok(Value::Str(Arc::from(res)));
    }
    Ok(Value::Null)
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

pub fn str_to_int(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        return Ok(Value::Int(s.trim().parse::<i64>().unwrap_or(0)));
    }
    Ok(Value::Int(0))
}

pub fn str_to_float(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        return Ok(Value::Float(s.trim().parse::<f64>().unwrap_or(0.0)));
    }
    Ok(Value::Float(0.0))
}

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

pub fn str_at(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        if let Some(Value::Int(pos)) = args.get(1) {
            let len = s.chars().count() as i64;
            let mut idx = *pos;
            if idx < 0 {
                idx += len;
            }
            if idx < 0 || idx >= len {
                return Ok(Value::Null);
            }
            return Ok(Value::Str(Arc::from(
                s.chars().nth(idx as usize).unwrap().to_string(),
            )));
        }
    }
    Ok(Value::Null)
}

pub fn str_code_point_at(
    _ctx: &mut dyn tsn_types::Context,
    args: &[Value],
) -> Result<Value, String> {
    if let Some(Value::Str(s)) = args.first() {
        if let Some(Value::Int(pos)) = args.get(1) {
            return Ok(s
                .chars()
                .nth(*pos as usize)
                .map(|c| Value::Int(c as u32 as i64))
                .unwrap_or(Value::Int(-1)));
        }
    }
    Ok(Value::Int(-1))
}

pub fn int_is_integer(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(val) = args.first() {
        return Ok(Value::Bool(matches!(val, Value::Int(_))));
    }
    Ok(Value::Bool(false))
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

pub fn char_to_str(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Char(c)) = args.first() {
        return Ok(Value::Str(Arc::from(c.to_string())));
    }
    Ok(Value::Str(Arc::from("")))
}

pub fn char_code_at(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(Value::Char(c)) = args.first() {
        return Ok(Value::Int(*c as u32 as i64));
    }
    Ok(Value::Null)
}

pub fn symbol_iterator(
    _ctx: &mut dyn tsn_types::Context,
    _args: &[Value],
) -> Result<Value, String> {
    Ok(Value::Symbol(tsn_types::value::SymbolKind::Iterator))
}

pub fn symbol_async_iterator(
    _ctx: &mut dyn tsn_types::Context,
    _args: &[Value],
) -> Result<Value, String> {
    Ok(Value::Symbol(tsn_types::value::SymbolKind::AsyncIterator))
}
