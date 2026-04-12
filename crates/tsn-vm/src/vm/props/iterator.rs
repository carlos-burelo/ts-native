use std::sync::Arc;
use tsn_types::value::{new_object, ObjData, Value};

pub fn range_symbol_iterator(
    _ctx: &mut dyn tsn_types::Context,
    args: &[Value],
) -> Result<Value, String> {
    let (cur, end_excl) = match args.first() {
        Some(Value::Range(r)) => (r.start, if r.inclusive { r.end + 1 } else { r.end }),
        _ => return Err("range_symbol_iterator: invalid receiver".into()),
    };
    let mut iter_obj = ObjData::new();
    iter_obj.fields.insert(Arc::from("__cur"), Value::Int(cur));
    iter_obj
        .fields
        .insert(Arc::from("__end"), Value::Int(end_excl));
    let iter_val = new_object(iter_obj);
    let next_method = Value::native_bound(iter_val.clone(), range_iter_next, "next");
    if let Value::Object(o) = &iter_val {
        unsafe { &mut **o }
            .fields
            .insert(Arc::from("next"), next_method);
    }
    Ok(iter_val)
}

fn range_iter_next(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let iter_ptr = match args.first() {
        Some(Value::Object(o)) => *o,
        _ => return Err("range_iter_next: invalid receiver".into()),
    };
    let iter_obj = unsafe { &mut *iter_ptr };
    let cur = match iter_obj.fields.get("__cur") {
        Some(Value::Int(i)) => *i,
        _ => return Err("range_iter_next: __cur not an int".into()),
    };
    let end = match iter_obj.fields.get("__end") {
        Some(Value::Int(i)) => *i,
        _ => return Err("range_iter_next: __end not an int".into()),
    };
    if cur >= end {
        let mut done = ObjData::new();
        done.fields.insert(Arc::from("value"), Value::Null);
        done.fields.insert(Arc::from("done"), Value::Bool(true));
        return Ok(new_object(done));
    }
    iter_obj
        .fields
        .insert(Arc::from("__cur"), Value::Int(cur + 1));
    let mut result = ObjData::new();
    result.fields.insert(Arc::from("value"), Value::Int(cur));
    result.fields.insert(Arc::from("done"), Value::Bool(false));
    Ok(new_object(result))
}

pub fn array_symbol_iterator(
    _ctx: &mut dyn tsn_types::Context,
    args: &[Value],
) -> Result<Value, String> {
    let arr = args.first().cloned().unwrap_or(Value::Null);
    let mut iter_obj = ObjData::new();
    iter_obj.fields.insert(Arc::from("__arr"), arr);
    iter_obj.fields.insert(Arc::from("__idx"), Value::Int(0));
    let iter_val = new_object(iter_obj);
    let next_method = Value::native_bound(iter_val.clone(), array_iter_next, "next");
    if let Value::Object(o) = &iter_val {
        unsafe { &mut **o }
            .fields
            .insert(Arc::from("next"), next_method);
    }
    Ok(iter_val)
}

fn array_iter_next(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let iter_ptr = match args.first() {
        Some(Value::Object(o)) => *o,
        _ => return Err("array_iter_next: invalid receiver".into()),
    };
    let iter_obj = unsafe { &mut *iter_ptr };
    let arr_val = iter_obj.fields.get("__arr").cloned().unwrap_or(Value::Null);
    let idx = match iter_obj.fields.get("__idx") {
        Some(Value::Int(i)) => *i,
        _ => 0,
    };
    let arr_len = match &arr_val {
        Value::Array(a) => unsafe { &**a }.len(),
        _ => return Err("array_iter_next: __arr not an array".into()),
    };
    if idx as usize >= arr_len {
        let mut done = ObjData::new();
        done.fields.insert(Arc::from("value"), Value::Null);
        done.fields.insert(Arc::from("done"), Value::Bool(true));
        return Ok(new_object(done));
    }
    let item = match &arr_val {
        Value::Array(a) => {
            let slice = unsafe { &**a };
            slice[idx as usize].clone()
        }
        _ => unreachable!(),
    };
    iter_obj
        .fields
        .insert(Arc::from("__idx"), Value::Int(idx + 1));
    let mut result = ObjData::new();
    result.fields.insert(Arc::from("value"), item);
    result.fields.insert(Arc::from("done"), Value::Bool(false));
    Ok(new_object(result))
}
