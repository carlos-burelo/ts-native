use crate::tsn_types::generator::GeneratorObj;
use crate::tsn_types::value::{SymbolKind, Value};
use crate::tsn_types::Context;
use crate::vm::generator::AsyncQueueDriver;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) fn get_property(obj: &Value, _gen: &GeneratorObj, key: &str) -> Result<Value, String> {
    match key {
        "next" => Ok(Value::native_bound(obj.clone(), gen_next, "next")),
        "return" => Ok(Value::native_bound(obj.clone(), gen_return, "return")),
        _ => Err(format!("generator has no property '{}'", key)),
    }
}

pub(crate) fn get_symbol(
    obj: &Value,
    _gen: &GeneratorObj,
    sym: SymbolKind,
) -> Result<Value, String> {
    match sym {
        SymbolKind::Iterator | SymbolKind::AsyncIterator => Ok(Value::native_bound(
            obj.clone(),
            gen_get_self,
            "[Symbol.iterator]",
        )),
    }
}

fn gen_next(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
    let gen = match args.first() {
        Some(Value::Generator(g)) => g,
        _ => return Err("gen.next: receiver is not a generator".to_string()),
    };
    let input = args.get(1).cloned().unwrap_or(Value::Null);
    gen.0.next(input)
}

fn gen_return(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
    let value = args.get(1).cloned().unwrap_or(Value::Null);
    let mut obj = tsn_types::value::ObjData::new();
    obj.fields.insert(Arc::from("value"), value);
    obj.fields.insert(Arc::from("done"), Value::Bool(true));
    Ok(tsn_types::value::new_object(obj))
}

fn gen_get_self(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
    Ok(args.first().cloned().unwrap_or(Value::Null))
}

pub(crate) fn asyncqueue_get_symbol(obj: &Value, sym: SymbolKind) -> Result<Value, String> {
    match sym {
        SymbolKind::AsyncIterator => {
            if let Value::AsyncQueue(q) = obj {
                let driver = Rc::new(AsyncQueueDriver(q.clone()))
                    as Rc<dyn tsn_types::generator::GeneratorDriver>;
                let gen = Value::Generator(tsn_types::generator::GeneratorObj(driver));

                let gen_clone = gen.clone();
                Ok(Value::native_bound(
                    gen_clone,
                    asyncqueue_self_iter,
                    "[Symbol.asyncIterator]",
                ))
            } else {
                Err("asyncqueue_get_symbol: not an AsyncQueue".to_string())
            }
        }
        SymbolKind::Iterator => Err("AsyncQueue is not a sync iterable".to_string()),
    }
}

fn asyncqueue_self_iter(_ctx: &mut dyn Context, args: &[Value]) -> Result<Value, String> {
    Ok(args.first().cloned().unwrap_or(Value::Null))
}
