use std::sync::Arc;
use tsn_types::{value::{new_object, ObjData}, Value};
use tsn_types::NativeFn;

pub fn range_from(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let start = int_arg(args, 0)?;
    let end = int_arg(args, 1)?;
    Ok(Value::Range(Box::new(tsn_types::value::RangeData { start, end, inclusive: false })))
}

pub fn range_from_inclusive(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let start = int_arg(args, 0)?;
    let end = int_arg(args, 1)?;
    Ok(Value::Range(Box::new(tsn_types::value::RangeData { start, end, inclusive: true })))
}

pub fn int_arg(args: &[Value], idx: usize) -> Result<i64, String> {
    match args.get(idx) {
        Some(Value::Int(i)) => Ok(*i),
        _ => Err(format!("Range: expected int at index {}", idx)),
    }
}

pub fn build() -> Value {
    let mut range_ns = ObjData::new();
    range_ns.set_field(Arc::from("from"),          Value::NativeFn(Box::new((range_from          as NativeFn, "from"))));
    range_ns.set_field(Arc::from("fromInclusive"), Value::NativeFn(Box::new((range_from_inclusive as NativeFn, "fromInclusive"))));

    let mut exports = ObjData::new();
    exports.set_field(Arc::from("Range"), new_object(range_ns));
    new_object(exports)
}
