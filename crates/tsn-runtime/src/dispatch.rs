mod common;
mod crypto_http_net;
mod entry;
mod map_set;
mod math;
mod primitives;

use entry::DispatchEntry;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tsn_core::intrinsic::IntrinsicId;
use tsn_types::{Context, Value};

static GROUPS: &[&[DispatchEntry]] = &[
    common::OPS,
    map_set::OPS,
    crypto_http_net::OPS,
    math::OPS,
    primitives::OPS,
];

static TABLE: OnceLock<Vec<Option<DispatchEntry>>> = OnceLock::new();

fn iter_entries() -> impl Iterator<Item = DispatchEntry> {
    GROUPS.iter().flat_map(|group| group.iter().copied())
}

fn build_table() -> Vec<Option<DispatchEntry>> {
    let max_id = IntrinsicId::FloatIsInteger as usize;
    let mut table = vec![None; max_id + 1];
    for entry in iter_entries() {
        let idx = entry.id as usize;
        if idx < table.len() {
            table[idx] = Some(entry);
        }
    }
    table
}

/// Dispatches a native runtime operation by canonical intrinsic ID.
pub fn dispatch_intrinsic(id: u16, ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
    let table = TABLE.get_or_init(build_table);
    if let Some(Some(entry)) = table.get(id as usize) {
        return (entry.func)(ctx, args);
    }
    Err(format!("Unknown intrinsic ID: {}", id))
}

/// Registers all native operations in the VM globals under `name`.
pub fn register_globals(globals: &mut HashMap<Arc<str>, Value>) {
    for entry in iter_entries() {
        globals.insert(
            Arc::from(entry.name),
            Value::NativeFn(Box::new((entry.func, entry.name))),
        );
    }

    globals.insert(
        Arc::from("print"),
        Value::NativeFn(Box::new((crate::modules::globals::global_print, "print"))),
    );
    globals.insert(
        Arc::from("debug"),
        Value::NativeFn(Box::new((crate::modules::globals::global_debug, "debug"))),
    );
    globals.insert(
        Arc::from("input"),
        Value::NativeFn(Box::new((crate::modules::globals::global_input, "input"))),
    );
    globals.insert(
        Arc::from("assert"),
        Value::NativeFn(Box::new((crate::modules::globals::global_assert, "assert"))),
    );
    globals.insert(
        Arc::from("assertSummary"),
        Value::NativeFn(Box::new((
            crate::modules::globals::global_assert_summary,
            "assertSummary",
        ))),
    );
    globals.insert(
        Arc::from("Map"),
        Value::NativeFn(Box::new((crate::modules::map::map_new, "Map"))),
    );
    globals.insert(
        Arc::from("Set"),
        Value::NativeFn(Box::new((crate::modules::set::set_new, "Set"))),
    );
    globals.insert(Arc::from("str"), crate::modules::globals::str_type_global());
    globals.insert(Arc::from("int"), crate::modules::globals::int_type_global());
    globals.insert(
        Arc::from("float"),
        crate::modules::globals::float_type_global(),
    );
    globals.insert(
        Arc::from("Array"),
        crate::modules::globals::array_type_global(),
    );
    let (error_cls, type_error_cls, range_error_cls) =
        crate::modules::globals::error_classes_globals();
    globals.insert(Arc::from("Error"), error_cls);
    globals.insert(Arc::from("TypeError"), type_error_cls);
    globals.insert(Arc::from("RangeError"), range_error_cls);
    globals.insert(Arc::from("NaN"), Value::Float(f64::NAN));
    globals.insert(Arc::from("Infinity"), Value::Float(f64::INFINITY));
    globals.insert(
        Arc::from("Symbol"),
        crate::modules::globals::symbol_global(),
    );
}
