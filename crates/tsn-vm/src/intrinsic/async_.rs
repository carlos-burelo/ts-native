use tsn_op_macros::op;
use tsn_types::{AsyncFuture, Value};

#[op("spawn")]
pub fn async_spawn(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let callee = args.get(0).cloned().ok_or("spawn: missing function")?;
    let call_args = &args[1..];
    let fut = ctx.spawn(callee, call_args)?;
    Ok(Value::Future(fut))
}

#[op("timer_set")]
pub fn timer_set(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let ms = match args.get(0) {
        Some(Value::Int(i)) => *i as u64,
        _ => return Err("timer_set: expected ms".into()),
    };
    let repeat = match args.get(1) {
        Some(Value::Bool(b)) => *b,
        _ => false,
    };
    let callee = args.get(2).cloned().ok_or("timer_set: missing callback")?;
    let call_args = &args[3..];

    let id = ctx.set_timer(ms, repeat, callee, call_args)?;
    Ok(Value::Int(id as i64))
}

#[op("timer_clear")]
pub fn timer_clear(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let id = match args.get(0) {
        Some(Value::Int(i)) => *i as usize,
        _ => return Err("timer_clear: expected id".into()),
    };
    ctx.clear_timer(id)?;
    Ok(Value::Null)
}

#[op("sleep")]
pub fn async_sleep(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let ms = match args.first() {
        Some(Value::Int(i)) => *i as u64,
        Some(Value::Float(f)) => *f as u64,
        _ => return Err("sleep: expected ms".into()),
    };
    std::thread::sleep(std::time::Duration::from_millis(ms));
    Ok(Value::Future(AsyncFuture::resolved(Value::Null)))
}

pub const OPS: &[crate::host_ops::HostOp] =
    &[async_spawn_OP, timer_set_OP, timer_clear_OP, async_sleep_OP];
