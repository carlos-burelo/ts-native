use tsn_types::value::{SymbolKind, Value};

use super::generator;
use super::iterator;
use super::property::get_property;
use crate::Vm;

pub fn get_symbol_property(vm: &Vm, obj: &Value, symbol: SymbolKind) -> Result<Value, String> {
    match obj {
        Value::Array(_) => {
            if matches!(symbol, SymbolKind::Iterator) {
                return Ok(Value::native_bound(
                    obj.clone(),
                    iterator::array_symbol_iterator,
                    "[Symbol.iterator]",
                ));
            }
            Err(format!("symbol property {} not found on array", symbol))
        }
        Value::Object(obj_arc) => {
            let guard = unsafe { &**obj_arc };
            if let Some(v) = guard.fields.get(&symbol.to_string()) {
                return Ok(v.clone());
            }
            Err(format!("symbol property {} not found on object", symbol))
        }
        Value::Range(_) => {
            if matches!(symbol, SymbolKind::Iterator) {
                return Ok(Value::native_bound(
                    obj.clone(),
                    iterator::range_symbol_iterator,
                    "[Symbol.iterator]",
                ));
            }
            Err(format!("symbol property {} not found on range", symbol))
        }
        Value::Generator(gen) => generator::get_symbol(obj, gen, symbol),
        Value::AsyncQueue(_) => generator::asyncqueue_get_symbol(obj, symbol),
        _ => get_property(vm, obj, symbol.name()),
    }
}
