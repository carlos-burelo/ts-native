use std::sync::atomic::{AtomicBool, Ordering};
use tsn_types::value::Value;

static CONSOLE_SILENT: AtomicBool = AtomicBool::new(false);

pub fn set_console_silent(silent: bool) {
    CONSOLE_SILENT.store(silent, Ordering::Relaxed);
}

pub fn console_debug(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    eprintln!(
        "[debug] {}",
        args.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    );
    Ok(Value::Null)
}

pub fn console_log(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
    if CONSOLE_SILENT.load(Ordering::Relaxed) {
        return Ok(Value::Null);
    }
    println!(
        "{}",
        args.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    );
    Ok(Value::Null)
}
