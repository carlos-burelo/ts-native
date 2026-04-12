use std::io::Write as IoWrite;
use std::sync::Arc;
use tsn_types::NativeFn;
use tsn_types::{
    value::{new_object, ObjData},
    Value,
};

pub fn io_write(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    for arg in args {
        print!("{}", arg);
    }
    Ok(Value::Null)
}

pub fn io_writeln(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    for arg in args {
        print!("{}", arg);
    }
    println!();
    Ok(Value::Null)
}

pub fn io_flush(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    std::io::stdout()
        .flush()
        .map_err(|e| format!("io.flush: {}", e))?;
    Ok(Value::Null)
}

pub fn io_read_line(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if let Some(prompt) = args.first() {
        if !matches!(prompt, Value::Null) {
            print!("{}", prompt);
            let _ = std::io::stdout().flush();
        }
    }
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|e| format!("io.readLine: {}", e))?;
    if line.ends_with('\n') {
        line.pop();
    }
    if line.ends_with('\r') {
        line.pop();
    }
    Ok(Value::Str(Arc::from(line)))
}

pub fn io_read_all(_ctx: &mut dyn tsn_types::Context, _args: &[Value]) -> Result<Value, String> {
    use std::io::Read;
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| format!("io.readAll: {}", e))?;
    Ok(Value::Str(Arc::from(buf)))
}

pub fn build() -> Value {
    let mut exports = ObjData::new();
    exports.set_field(
        Arc::from("write"),
        Value::NativeFn(Box::new((io_write as NativeFn, "write"))),
    );
    exports.set_field(
        Arc::from("writeln"),
        Value::NativeFn(Box::new((io_writeln as NativeFn, "writeln"))),
    );
    exports.set_field(
        Arc::from("print"),
        Value::NativeFn(Box::new((io_writeln as NativeFn, "print"))),
    );
    exports.set_field(
        Arc::from("flush"),
        Value::NativeFn(Box::new((io_flush as NativeFn, "flush"))),
    );
    exports.set_field(
        Arc::from("readLine"),
        Value::NativeFn(Box::new((io_read_line as NativeFn, "readLine"))),
    );
    exports.set_field(
        Arc::from("readAll"),
        Value::NativeFn(Box::new((io_read_all as NativeFn, "readAll"))),
    );
    new_object(exports)
}
