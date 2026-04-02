use tsn_types::Value;

pub fn async_spawn(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let callee = args.get(0).cloned().ok_or("spawn: missing function")?;
    let call_args = &args[1..];
    let fut = ctx.spawn(callee, call_args)?;
    Ok(Value::Future(fut))
}

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

pub fn timer_clear(ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let id = match args.get(0) {
        Some(Value::Int(i)) => *i as usize,
        _ => return Err("timer_clear: expected id".into()),
    };
    ctx.clear_timer(id)?;
    Ok(Value::Null)
}
