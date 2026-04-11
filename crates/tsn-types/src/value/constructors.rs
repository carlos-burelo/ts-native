use super::{alloc_array, alloc_object, ObjData, Value};

#[inline(always)]
pub fn new_array(v: Vec<Value>) -> Value {
    let ptr = alloc_array();
    unsafe { *ptr = v; }
    Value::Array(ptr)
}

#[inline(always)]
pub fn new_object(obj: ObjData) -> Value {
    let ptr = alloc_object();
    unsafe { *ptr = obj; }
    Value::Object(ptr)
}
