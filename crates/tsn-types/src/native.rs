use crate::{value::Value, AsyncFuture};

pub trait Context {
    fn spawn(&mut self, callee: Value, args: &[Value]) -> Result<AsyncFuture, String>;
    fn call(&mut self, callee: Value, args: &[Value]) -> Result<Value, String>;
    fn set_timer(
        &mut self,
        ms: u64,
        repeat: bool,
        callee: Value,
        args: &[Value],
    ) -> Result<usize, String>;
    fn clear_timer(&mut self, id: usize) -> Result<(), String>;
}

pub type NativeFn = fn(&mut dyn Context, &[Value]) -> Result<Value, String>;
