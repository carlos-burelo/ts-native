use std::sync::Arc;
use tsn_types::value::{Closure, Value};

use crate::Vm;

pub fn invoke_getter(vm: &mut Vm, getter: Arc<Closure>, receiver: Value) -> Result<Value, String> {
    let depth_before = vm.frames.len();
    let callee = Value::Closure(getter);
    vm.push(callee.clone());
    vm.push(receiver);
    vm.call_value(callee, 1)?;
    vm.run_until(depth_before)
}

pub fn invoke_setter(
    vm: &mut Vm,
    setter: Arc<Closure>,
    receiver: Value,
    new_val: Value,
) -> Result<(), String> {
    let depth_before = vm.frames.len();
    let callee = Value::Closure(setter);
    vm.push(callee.clone());
    vm.push(receiver);
    vm.push(new_val);
    vm.call_value(callee, 2)?;
    vm.run_until(depth_before)?;
    Ok(())
}

pub fn invoke_static_getter(vm: &mut Vm, getter: Arc<Closure>) -> Result<Value, String> {
    let depth_before = vm.frames.len();
    let callee = Value::Closure(getter);
    vm.push(callee.clone());
    vm.call_value(callee, 0)?;
    vm.run_until(depth_before)
}

pub fn invoke_static_setter(
    vm: &mut Vm,
    setter: Arc<Closure>,
    new_val: Value,
) -> Result<(), String> {
    let depth_before = vm.frames.len();
    let callee = Value::Closure(setter);
    vm.push(callee.clone());
    vm.push(new_val);
    vm.call_value(callee, 1)?;
    vm.run_until(depth_before)?;
    Ok(())
}
