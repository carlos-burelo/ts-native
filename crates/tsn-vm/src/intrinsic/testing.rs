use std::sync::atomic::{AtomicU64, Ordering};
use tsn_types::Value;

static PASSED: AtomicU64 = AtomicU64::new(0);
static FAILED: AtomicU64 = AtomicU64::new(0);

pub fn assert_test(args: &[Value]) -> Result<Value, String> {
    let label = match args.first() {
        Some(Value::Str(s)) => s.to_string(),
        _ => "?".to_string(),
    };
    let cond = match args.get(1) {
        Some(Value::Bool(b)) => *b,
        _ => false,
    };
    if cond {
        PASSED.fetch_add(1, Ordering::Relaxed);
    } else {
        FAILED.fetch_add(1, Ordering::Relaxed);
        println!("FAIL: {}", label);
    }
    Ok(Value::Null)
}

pub fn assert_summary(_args: &[Value]) -> Result<Value, String> {
    let passed = PASSED.load(Ordering::Relaxed);
    let failed = FAILED.load(Ordering::Relaxed);
    println!("\n════════════════════════════════════════");
    println!("PASSED: {}", passed);
    println!("FAILED: {}", failed);
    if failed == 0 {
        println!("ALL TESTS PASSED — production ready!");
    } else {
        println!("SOME TESTS FAILED — see output above");
    }
    Ok(Value::Null)
}
