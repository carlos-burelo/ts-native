use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tsn_types::{value::{new_object, ObjData}, Value};
use tsn_types::NativeFn;

static SILENT: AtomicBool = AtomicBool::new(false);

pub fn set_console_silent(silent: bool) {
    SILENT.store(silent, Ordering::Relaxed);
}

pub fn console_log(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if SILENT.load(Ordering::Relaxed) {
        return Ok(Value::Null);
    }
    println!("{}", args.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" "));
    Ok(Value::Null)
}

pub fn console_debug(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    eprintln!("[debug] {}", args.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" "));
    Ok(Value::Null)
}

pub fn console_warn(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    eprintln!("[warn] {}", args.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" "));
    Ok(Value::Null)
}

pub fn console_error(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    eprintln!("[error] {}", args.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" "));
    Ok(Value::Null)
}

pub fn build() -> Value {
    let mut ns = ObjData::new();
    ns.set_field(Arc::from("log"),   Value::NativeFn(Box::new((console_log   as NativeFn, "log"))));
    ns.set_field(Arc::from("debug"), Value::NativeFn(Box::new((console_debug as NativeFn, "debug"))));
    ns.set_field(Arc::from("warn"),  Value::NativeFn(Box::new((console_warn  as NativeFn, "warn"))));
    ns.set_field(Arc::from("error"), Value::NativeFn(Box::new((console_error as NativeFn, "error"))));
    ns.set_field(Arc::from("info"),  Value::NativeFn(Box::new((console_log   as NativeFn, "info"))));

    let mut exports = ObjData::new();
    exports.set_field(Arc::from("console"), new_object(ns));
    new_object(exports)
}
