pub extern crate tsn_types;
pub mod runtime;
pub mod vm;
use tsn_compiler::chunk::FunctionProto;
pub use tsn_types::value::*;
pub use tsn_types::{Context, NativeFn};
pub use vm::Vm;

#[derive(Debug, Clone)]
pub struct StackFrame {
    pub fn_name: String,
    pub line: u32,
}
#[derive(Debug, Clone)]
pub struct RuntimeError {
    pub message: String,
    pub stack: Vec<StackFrame>,
}

impl RuntimeError {
    pub fn new(message: impl Into<String>, stack: Vec<StackFrame>) -> Self {
        RuntimeError {
            message: message.into(),
            stack,
        }
    }
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub fn run(proto: FunctionProto) -> Result<Value, RuntimeError> {
    let mut machine = Vm::new();
    machine.run_proto(proto)
}

pub fn run_debug(proto: FunctionProto, trace: bool, calls: bool) -> Result<Value, RuntimeError> {
    let mut machine = Vm::new();
    machine.trace = trace;
    machine.calls = calls;
    machine.run_proto(proto)
}

/// Pre-run builtin protos into a fresh VM and return a globals snapshot.
/// Each value in the map is Arc-based, so the clone is cheap.
pub fn build_globals_snapshot(
    builtin_protos: &[FunctionProto],
) -> Result<std::collections::HashMap<std::sync::Arc<str>, Value>, RuntimeError> {
    let mut vm = Vm::new();
    for p in builtin_protos {
        vm.run_proto(p.clone())?;
    }
    Ok(vm.snapshot_globals())
}
