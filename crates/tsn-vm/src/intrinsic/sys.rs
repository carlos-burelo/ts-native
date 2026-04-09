use std::sync::Arc;
use tsn_op_macros::op;
use tsn_types::{value::new_array, Value};

#[op("platform")]
pub fn sys_platform(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Str(Arc::from(std::env::consts::OS)))
}

#[op("cwd")]
pub fn sys_cwd(_args: &[Value]) -> Result<Value, String> {
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(Value::Str(Arc::from(cwd)))
}

#[op("args")]
pub fn sys_args(_args: &[Value]) -> Result<Value, String> {
    let items: Vec<Value> = std::env::args().map(|a| Value::Str(Arc::from(a))).collect();
    Ok(new_array(items))
}

#[op("exit")]
pub fn sys_exit(args: &[Value]) -> Result<Value, String> {
    let code = match args.get(0) {
        Some(Value::Int(i)) => *i as i32,
        _ => 0,
    };
    std::process::exit(code);
}

#[op("env")]
pub fn sys_env_get(args: &[Value]) -> Result<Value, String> {
    let key = str_arg(args, 0, "Sys.env")?;
    Ok(std::env::var(key)
        .map(|v| Value::Str(Arc::from(v)))
        .unwrap_or(Value::Null))
}

#[op("setEnv")]
pub fn sys_env_set(args: &[Value]) -> Result<Value, String> {
    let key = str_arg(args, 0, "Sys.setEnv")?;
    let val = str_arg(args, 1, "Sys.setEnv")?;
    std::env::set_var(key, val);
    Ok(Value::Null)
}

pub const OPS: &[crate::host_ops::HostOp] = &[
    sys_platform_OP,
    sys_cwd_OP,
    sys_args_OP,
    sys_exit_OP,
    sys_env_get_OP,
    sys_env_set_OP,
];

fn str_arg<'a>(args: &'a [Value], idx: usize, name: &str) -> Result<&'a str, String> {
    match args.get(idx) {
        Some(Value::Str(s)) => Ok(s.as_ref()),
        Some(_) => Err(format!("{}: expected string at index {}", name, idx)),
        None => Err(format!("{}: missing argument at index {}", name, idx)),
    }
}
