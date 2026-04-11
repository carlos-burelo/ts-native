use crate::tsn_types::value::Value;
use tsn_runtime::modules::primitives as rt_prim;

pub(super) fn get_property(obj: &Value, key: &str) -> Result<Value, String> {
    if key == "valueOf" {
        return Ok(Value::native_bound(obj.clone(), float_identity, "valueOf"));
    }
    let (method, name) = match key {
        "toString" => (rt_prim::float_to_str as _, "toString"),
        "toFixed" => (rt_prim::float_to_fixed as _, "toFixed"),
        "abs" => (rt_prim::float_abs as _, "abs"),
        "sign" => (rt_prim::float_sign as _, "sign"),
        "negate" => (rt_prim::float_negate as _, "negate"),
        "min" => (rt_prim::float_min as _, "min"),
        "max" => (rt_prim::float_max as _, "max"),
        "pow" => (rt_prim::float_pow as _, "pow"),
        "isInteger" => (rt_prim::float_is_integer as _, "isInteger"),
        _ => return Err(format!("method '{}' not found on float", key)),
    };
    Ok(Value::native_bound(obj.clone(), method, name))
}

fn float_identity(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    Ok(args.first().cloned().unwrap_or(Value::Null))
}
