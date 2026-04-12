use std::sync::Arc;
use tsn_types::NativeFn;
use tsn_types::{
    value::{new_array, new_object, ObjData},
    Value,
};

pub fn sys_platform(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    Ok(Value::Str(Arc::from(std::env::consts::OS)))
}

pub fn sys_cwd(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(Value::Str(Arc::from(cwd)))
}

pub fn sys_args(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    let items: Vec<Value> = std::env::args().map(|a| Value::Str(Arc::from(a))).collect();
    Ok(new_array(items))
}

pub fn sys_exit(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let code = match args.first() {
        Some(Value::Int(i)) => *i as i32,
        _ => 0,
    };
    std::process::exit(code);
}

pub fn sys_env(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let key = str_arg(args, 0, "Sys.env")?;
    Ok(std::env::var(key)
        .map(|v| Value::Str(Arc::from(v)))
        .unwrap_or(Value::Null))
}

pub fn sys_set_env(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    let key = str_arg(args, 0, "Sys.setEnv")?;
    let val = str_arg(args, 1, "Sys.setEnv")?;
    std::env::set_var(key, val);
    Ok(Value::Null)
}

pub fn str_arg<'a>(args: &'a [Value], idx: usize, name: &str) -> Result<&'a str, String> {
    match args.get(idx) {
        Some(Value::Str(s)) => Ok(s.as_ref()),
        Some(_) => Err(format!("{}: expected string at index {}", name, idx)),
        None => Err(format!("{}: missing argument at index {}", name, idx)),
    }
}

pub fn build() -> Value {
    let mut ns = ObjData::new();
    ns.set_field(
        Arc::from("platform"),
        Value::NativeFn(Box::new((sys_platform as NativeFn, "platform"))),
    );
    ns.set_field(
        Arc::from("cwd"),
        Value::NativeFn(Box::new((sys_cwd as NativeFn, "cwd"))),
    );
    ns.set_field(
        Arc::from("args"),
        Value::NativeFn(Box::new((sys_args as NativeFn, "args"))),
    );
    ns.set_field(
        Arc::from("exit"),
        Value::NativeFn(Box::new((sys_exit as NativeFn, "exit"))),
    );
    ns.set_field(
        Arc::from("env"),
        Value::NativeFn(Box::new((sys_env as NativeFn, "env"))),
    );
    ns.set_field(
        Arc::from("setEnv"),
        Value::NativeFn(Box::new((sys_set_env as NativeFn, "setEnv"))),
    );

    let mut exports = ObjData::new();
    exports.set_field(Arc::from("Sys"), new_object(ns));
    new_object(exports)
}
