use std::sync::Arc;
use tsn_types::{value::new_array, Value};

pub fn sys_platform(_args: &[Value]) -> Result<Value, String> {
    Ok(Value::Str(Arc::from(std::env::consts::OS)))
}

pub fn sys_cwd(_args: &[Value]) -> Result<Value, String> {
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(Value::Str(Arc::from(cwd)))
}

pub fn sys_args(_args: &[Value]) -> Result<Value, String> {
    let items: Vec<Value> = std::env::args().map(|a| Value::Str(Arc::from(a))).collect();
    Ok(new_array(items))
}

pub fn sys_exit(args: &[Value]) -> Result<Value, String> {
    let code = match args.get(0) {
        Some(Value::Int(i)) => *i as i32,
        _ => 0,
    };
    std::process::exit(code);
}

pub fn sys_env_get(args: &[Value]) -> Result<Value, String> {
    let key = str_arg(args, 0, "Sys.env")?;
    Ok(std::env::var(key)
        .map(|v| Value::Str(Arc::from(v)))
        .unwrap_or(Value::Null))
}

pub fn sys_env_set(args: &[Value]) -> Result<Value, String> {
    let key = str_arg(args, 0, "Sys.setEnv")?;
    let val = str_arg(args, 1, "Sys.setEnv")?;
    std::env::set_var(key, val);
    Ok(Value::Null)
}

fn str_arg<'a>(args: &'a [Value], idx: usize, name: &str) -> Result<&'a str, String> {
    match args.get(idx) {
        Some(Value::Str(s)) => Ok(s.as_ref()),
        Some(_) => Err(format!("{}: expected string at index {}", name, idx)),
        None => Err(format!("{}: missing argument at index {}", name, idx)),
    }
}
