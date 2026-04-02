use crate::runtime::reactor::{CallbackKind, CallbackTask};
use crate::runtime::scheduler::get_callback_event_queue;
use tsn_types::future::{AsyncFuture, FutureState};
use tsn_types::value::Value;
use tsn_types::Context;
use tsn_types::NativeFn;

pub(super) fn get_property(obj: &Value, key: &str) -> Result<Value, String> {
    fn future_is_pending(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        match args.first() {
            Some(Value::Future(fut)) => Ok(Value::Bool(fut.is_pending())),
            _ => Err("is_pending: receiver is not a Future".to_owned()),
        }
    }
    fn future_is_resolved(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        match args.first() {
            Some(Value::Future(fut)) => Ok(Value::Bool(fut.is_resolved())),
            _ => Err("is_resolved: receiver is not a Future".to_owned()),
        }
    }
    fn future_is_rejected(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        match args.first() {
            Some(Value::Future(fut)) => Ok(Value::Bool(fut.is_rejected())),
            _ => Err("is_rejected: receiver is not a Future".to_owned()),
        }
    }

    fn future_value_or(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        let default = args.get(1).cloned().unwrap_or(Value::Null);
        match args.first() {
            Some(Value::Future(fut)) => match fut.peek_state() {
                FutureState::Resolved(v) => Ok(v),
                _ => Ok(default),
            },
            _ => Err("value_or: receiver is not a Future".to_owned()),
        }
    }
    fn future_rejection_reason(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        match args.first() {
            Some(Value::Future(fut)) => match fut.peek_state() {
                FutureState::Rejected(v) => Ok(v),
                _ => Ok(Value::Null),
            },
            _ => Err("rejection_reason: receiver is not a Future".to_owned()),
        }
    }

    fn future_unwrap(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        match args.first() {
            Some(Value::Future(fut)) => match fut.peek_state() {
                FutureState::Resolved(v) => Ok(v),
                FutureState::Rejected(v) => Err(v.to_string()),
                FutureState::Pending => Err(
                    "Future.unwrap(): future is still pending — use `await` to wait for it"
                        .to_owned(),
                ),
            },
            _ => Err("unwrap: receiver is not a Future".to_owned()),
        }
    }

    fn future_expect(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        let msg = args
            .get(1)
            .map(|v| v.to_string())
            .unwrap_or_else(|| "Future expectation failed".to_owned());
        match args.first() {
            Some(Value::Future(fut)) => match fut.peek_state() {
                FutureState::Resolved(v) => Ok(v),
                FutureState::Rejected(_) | FutureState::Pending => Err(msg),
            },
            _ => Err("expect: receiver is not a Future".to_owned()),
        }
    }

    fn future_then(ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        make_chain(ctx, args, CallbackKind::Then)
    }

    fn future_catch(ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        make_chain(ctx, args, CallbackKind::Catch)
    }

    fn future_finally(ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        make_chain(ctx, args, CallbackKind::Finally)
    }

    fn future_map(ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        future_then(ctx, args)
    }

    fn future_with_timeout(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
        let _timeout_ms: u64 = match args.get(1) {
            Some(Value::Int(n)) => *n as u64,
            Some(Value::Float(f)) => *f as u64,
            _ => return Err("withTimeout: expected milliseconds argument".to_owned()),
        };

        match args.first() {
            Some(Value::Future(_orig)) => {
                let race = AsyncFuture::pending();

                Ok(Value::Future(race))
            }
            _ => Err("withTimeout: receiver must be a Future".to_owned()),
        }
    }

    let method_fn: Option<(NativeFn, &'static str)> = match key {
        "is_pending" => Some((future_is_pending as _, "Future.is_pending")),
        "is_resolved" => Some((future_is_resolved as _, "Future.is_resolved")),
        "is_rejected" => Some((future_is_rejected as _, "Future.is_rejected")),
        "value_or" => Some((future_value_or as _, "Future.value_or")),
        "rejection_reason" => Some((future_rejection_reason as _, "Future.rejection_reason")),
        "unwrap" => Some((future_unwrap as _, "Future.unwrap")),
        "expect" => Some((future_expect as _, "Future.expect")),
        "then" => Some((future_then as _, "Future.then")),
        "catch" => Some((future_catch as _, "Future.catch")),
        "finally" => Some((future_finally as _, "Future.finally")),
        "map" => Some((future_map as _, "Future.map")),
        "withTimeout" => Some((future_with_timeout as _, "Future.withTimeout")),
        _ => None,
    };
    match method_fn {
        Some((f, name)) => Ok(Value::native_bound(obj.clone(), f, name)),
        None => Err(format!("method '{}' not found on Future", key)),
    }
}

fn make_chain(ctx: &mut dyn Context, args: &[Value], kind: CallbackKind) -> Result<Value, String> {
    let fut = match args.first() {
        Some(Value::Future(f)) => f.clone(),
        _ => return Err(format!("{}: receiver is not a Future", kind_name(kind))),
    };

    let cb = args.get(1).cloned().unwrap_or(Value::Null);
    let output = AsyncFuture::pending();

    match fut.peek_state() {
        FutureState::Resolved(v) => {
            let invoke = matches!(kind, CallbackKind::Then | CallbackKind::Finally);
            if invoke {
                let cb_arg = if matches!(kind, CallbackKind::Finally) {
                    Value::Null
                } else {
                    v.clone()
                };
                match ctx.call(cb, &[cb_arg]) {
                    Ok(ret) => {
                        if matches!(kind, CallbackKind::Finally) {
                            output.resolve(v);
                        } else {
                            output.resolve(ret);
                        }
                    }
                    Err(e) => output.reject_msg(e),
                }
            } else {
                output.resolve(v);
            }
            return Ok(Value::Future(output));
        }
        FutureState::Rejected(e) => {
            let invoke = matches!(kind, CallbackKind::Catch | CallbackKind::Finally);
            if invoke {
                let cb_arg = if matches!(kind, CallbackKind::Finally) {
                    Value::Null
                } else {
                    e.clone()
                };
                match ctx.call(cb, &[cb_arg]) {
                    Ok(ret) => {
                        if matches!(kind, CallbackKind::Finally) {
                            output.reject(e);
                        } else {
                            output.resolve(ret);
                        }
                    }
                    Err(err) => output.reject_msg(err),
                }
            } else {
                output.reject(e);
            }
            return Ok(Value::Future(output));
        }
        FutureState::Pending => {}
    }

    let eq = match get_callback_event_queue() {
        Some(eq) => eq,
        None => {
            return Err(format!(
                "Future.{}: cannot chain futures outside of a scheduler context",
                kind_name(kind)
            ))
        }
    };

    let out_clone = output.clone();
    fut.on_settle(move |result| {
        eq.lock()
            .unwrap()
            .push_back(crate::runtime::reactor::ExternalEvent::RunCallback(
                CallbackTask {
                    cb,
                    arg: result,
                    output: out_clone,
                    kind,
                },
            ));
    });

    Ok(Value::Future(output))
}

fn kind_name(kind: CallbackKind) -> &'static str {
    match kind {
        CallbackKind::Then => "then",
        CallbackKind::Catch => "catch",
        CallbackKind::Finally => "finally",
    }
}
