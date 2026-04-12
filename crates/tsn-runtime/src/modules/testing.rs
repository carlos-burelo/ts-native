use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tsn_types::NativeFn;
use tsn_types::{
    value::{new_object, ObjData},
    Value,
};

static PASSED: AtomicU64 = AtomicU64::new(0);
static FAILED: AtomicU64 = AtomicU64::new(0);
static SILENT: AtomicBool = AtomicBool::new(false);

pub fn set_testing_silent(silent: bool) {
    SILENT.store(silent, Ordering::Relaxed);
}

pub fn reset_testing_counters() {
    PASSED.store(0, Ordering::Relaxed);
    FAILED.store(0, Ordering::Relaxed);
}

pub fn testing_assert(_ctx: &mut dyn tsn_types::Context, args: &[Value]) -> Result<Value, String> {
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
        if !SILENT.load(Ordering::Relaxed) {
            println!("FAIL: {}", label);
        }
    }
    Ok(Value::Null)
}

pub fn testing_summary(
    _ctx: &mut dyn tsn_types::Context,
    _args: &[Value],
) -> Result<Value, String> {
    let passed = PASSED.load(Ordering::Relaxed);
    let failed = FAILED.load(Ordering::Relaxed);
    if !SILENT.load(Ordering::Relaxed) {
        println!("\n════════════════════════════════════════");
        println!("PASSED: {}", passed);
        println!("FAILED: {}", failed);
        if failed == 0 {
            println!("ALL TESTS PASSED");
        } else {
            println!("SOME TESTS FAILED");
        }
    }
    Ok(Value::Null)
}

pub fn build() -> Value {
    let mut ns = ObjData::new();
    ns.set_field(
        Arc::from("assert"),
        Value::NativeFn(Box::new((testing_assert as NativeFn, "assert"))),
    );
    ns.set_field(
        Arc::from("assertSummary"),
        Value::NativeFn(Box::new((testing_summary as NativeFn, "assertSummary"))),
    );

    let mut exports = ObjData::new();
    exports.set_field(Arc::from("Test"), new_object(ns));
    new_object(exports)
}
