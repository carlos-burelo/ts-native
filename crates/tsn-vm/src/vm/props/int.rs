use crate::tsn_types::value::Value;
use tsn_runtime::modules::primitives as rt_prim;

pub(super) fn get_property(obj: &Value, key: &str) -> Result<Value, String> {
    if key == "rawValue" {
        return Ok(obj.clone());
    }
    if key == "valueOf" {
        return Ok(Value::native_bound(obj.clone(), int_identity, "valueOf"));
    }
    let (method, name) = match key {
        "toString" => (rt_prim::int_to_str as _, "toString"),
        "toLocaleString" => (rt_prim::int_to_str as _, "toLocaleString"),
        "toFixed" => (rt_prim::int_to_fixed as _, "toFixed"),
        "abs" => (rt_prim::int_abs as _, "abs"),
        "sign" => (rt_prim::int_sign as _, "sign"),
        "negate" => (rt_prim::int_negate as _, "negate"),
        "bitwiseNot" => (rt_prim::int_bitwise_not as _, "bitwiseNot"),
        "min" => (rt_prim::int_min as _, "min"),
        "max" => (rt_prim::int_max as _, "max"),
        "clamp" => (rt_prim::int_clamp as _, "clamp"),
        "toHex" => (rt_prim::int_to_hex as _, "toHex"),
        "toBinary" => (rt_prim::int_to_binary as _, "toBinary"),
        "toOctal" => (rt_prim::int_to_octal as _, "toOctal"),
        "toFloat" => (rt_prim::int_to_float as _, "toFloat"),
        "pow" => (rt_prim::int_pow as _, "pow"),
        _ => return Err(format!("property '{}' not found on primitive int", key)),
    };
    Ok(Value::native_bound(obj.clone(), method, name))
}

fn int_identity(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(args.first().cloned().unwrap_or(Value::Null))
}
