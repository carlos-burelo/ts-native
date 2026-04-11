use std::sync::Arc;
use tsn_types::{value::{new_object, ObjData}, AsyncFuture, Value};
use tsn_types::NativeFn;

pub fn async_spawn(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let callee = args.first().cloned().ok_or("spawn: missing function")?;
    let fut = ctx.spawn(callee, &args[1..])?;
    Ok(Value::Future(fut))
}

pub fn async_sleep(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let ms = match args.first() {
        Some(Value::Int(i)) => *i as u64,
        Some(Value::Float(f)) => *f as u64,
        _ => return Err("sleep: expected ms".into()),
    };
    std::thread::sleep(std::time::Duration::from_millis(ms));
    Ok(Value::Future(AsyncFuture::resolved(Value::Null)))
}

pub fn timer_set(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let ms = match args.first() {
        Some(Value::Int(i)) => *i as u64,
        _ => return Err("setInterval: expected ms".into()),
    };
    let repeat = match args.get(1) {
        Some(Value::Bool(b)) => *b,
        _ => false,
    };
    let callee = args.get(2).cloned().ok_or("setInterval: missing callback")?;
    let id = ctx.set_timer(ms, repeat, callee, &args[3..])?;
    Ok(Value::Int(id as i64))
}

pub fn timer_clear(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::Int(i)) => *i as usize,
        _ => return Err("clearInterval: expected id".into()),
    };
    ctx.clear_timer(id)?;
    Ok(Value::Null)
}

pub fn build() -> Value {
    let mut exports = ObjData::new();
    exports.set_field(Arc::from("spawn"),        Value::NativeFn(Box::new((async_spawn as NativeFn, "spawn"))));
    exports.set_field(Arc::from("sleep"),        Value::NativeFn(Box::new((async_sleep as NativeFn, "sleep"))));
    exports.set_field(Arc::from("setInterval"),  Value::NativeFn(Box::new((timer_set   as NativeFn, "setInterval"))));
    exports.set_field(Arc::from("clearInterval"),Value::NativeFn(Box::new((timer_clear as NativeFn, "clearInterval"))));
    new_object(exports)
}
