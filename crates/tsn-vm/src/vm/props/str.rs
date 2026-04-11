use crate::tsn_types::value::Value;
use std::sync::Arc;
use tsn_runtime::modules::primitives as rt_prim;

pub(super) fn get_property(obj: &Value, s: &Arc<str>, key: &str) -> Result<Value, String> {
    if key == "length" {
        return Ok(Value::Int(if s.is_ascii() {
            s.len() as i64
        } else {
            s.chars().count() as i64
        }));
    }
    if key == "toString" || key == "valueOf" {
        return Ok(Value::native_bound(obj.clone(), str_identity, "toString"));
    }
    if let Ok(n) = key.parse::<usize>() {
        return s
            .chars()
            .nth(n)
            .map(|c| Value::Str(Arc::from(c.to_string())))
            .ok_or_else(|| format!("index {} out of bounds for string", n));
    }

    let (method, name) = match key {
        "toLowerCase" => (rt_prim::str_to_lower as _, "toLowerCase"),
        "toUpperCase" => (rt_prim::str_to_upper as _, "toUpperCase"),
        "trim" => (rt_prim::str_trim as _, "trim"),
        "trimStart" => (rt_prim::str_trim_start as _, "trimStart"),
        "trimEnd" => (rt_prim::str_trim_end as _, "trimEnd"),
        "includes" => (rt_prim::str_includes as _, "includes"),
        "contains" => (rt_prim::str_includes as _, "contains"),
        "startsWith" => (rt_prim::str_starts_with as _, "startsWith"),
        "endsWith" => (rt_prim::str_ends_with as _, "endsWith"),
        "indexOf" => (rt_prim::str_index_of as _, "indexOf"),
        "lastIndexOf" => (rt_prim::str_last_index_of as _, "lastIndexOf"),
        "substring" => (rt_prim::str_substring as _, "substring"),
        "slice" => (rt_prim::str_slice as _, "slice"),
        "at" => (rt_prim::str_at as _, "at"),
        "replace" => (rt_prim::str_replace as _, "replace"),
        "replaceAll" => (rt_prim::str_replace_all as _, "replaceAll"),
        "split" => (rt_prim::str_split as _, "split"),
        "charCode" => (rt_prim::str_char_code as _, "charCode"),
        "charCodeAt" => (rt_prim::str_char_code_at as _, "charCodeAt"),
        "charAt" => (rt_prim::str_char_at as _, "charAt"),
        "codePointAt" => (rt_prim::str_code_point_at as _, "codePointAt"),
        "repeat" => (rt_prim::str_repeat as _, "repeat"),
        "padStart" => (rt_prim::str_pad_start as _, "padStart"),
        "padEnd" => (rt_prim::str_pad_end as _, "padEnd"),
        "concat" => (rt_prim::str_concat as _, "concat"),
        "substr" => (rt_prim::str_substr as _, "substr"),
        "isEmpty" => (rt_prim::str_is_empty as _, "isEmpty"),
        "isBlank" => (rt_prim::str_is_blank as _, "isBlank"),
        "isDigit" => (rt_prim::str_is_digit as _, "isDigit"),
        "isLetter" => (rt_prim::str_is_letter as _, "isLetter"),
        "isWhitespace" => (rt_prim::str_is_whitespace as _, "isWhitespace"),
        "reverse" => (rt_prim::str_reverse as _, "reverse"),
        "capitalize" => (rt_prim::str_capitalize as _, "capitalize"),
        "lines" => (rt_prim::str_lines as _, "lines"),
        "words" => (rt_prim::str_words as _, "words"),
        "toInt" => (rt_prim::str_to_int as _, "toInt"),
        "toFloat" => (rt_prim::str_to_float as _, "toFloat"),
        _ => {
            return Err(format!("method '{}' not found on string", key));
        }
    };

    Ok(Value::native_bound(obj.clone(), method, name))
}

fn str_identity(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(args.first().cloned().unwrap_or(Value::Null))
}
