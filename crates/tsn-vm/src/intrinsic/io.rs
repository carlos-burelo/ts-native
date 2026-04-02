use std::io::Write;
use std::sync::Arc;
use tsn_types::Value;

pub fn io_write(args: &[Value]) -> Result<Value, String> {
    for arg in args {
        print!("{}", arg);
    }
    Ok(Value::Null)
}

pub fn io_writeln(args: &[Value]) -> Result<Value, String> {
    for arg in args {
        print!("{}", arg);
    }
    println!();
    Ok(Value::Null)
}

pub fn io_flush(_args: &[Value]) -> Result<Value, String> {
    std::io::stdout()
        .flush()
        .map_err(|e| format!("io.flush: {}", e))?;
    Ok(Value::Null)
}

pub fn io_read_line(args: &[Value]) -> Result<Value, String> {
    if let Some(prompt) = args.get(0) {
        if !matches!(prompt, Value::Null) {
            print!("{}", prompt);
            let _ = std::io::stdout().flush();
        }
    }

    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|e| format!("io.readLine: {}", e))?;

    // Trim newline
    if line.ends_with('\n') {
        line.pop();
    }
    if line.ends_with('\r') {
        line.pop();
    }

    Ok(Value::Str(Arc::from(line)))
}

pub fn io_read_all(_args: &[Value]) -> Result<Value, String> {
    use std::io::Read;
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| format!("io.readAll: {}", e))?;
    Ok(Value::Str(Arc::from(buf)))
}
