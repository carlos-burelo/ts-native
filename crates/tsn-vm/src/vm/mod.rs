mod call;
mod exec;
mod frame;
mod math;

mod props;

use crate::runtime::task::TaskId;

use frame::{CallFrame, TryEntry};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tsn_compiler::chunk::{FunctionProto, Literal, PoolEntry};
pub use tsn_types::{AsyncFuture, Closure, Context, NativeFn, ObjData, Poll, Upvalue, Value};
pub(crate) mod generator;

pub enum VmSuspend {
    Future(AsyncFuture),

    Timer(Duration),

    Yield(Value),
}

pub struct OpcodeProfile {
    counters: Vec<AtomicU64>,
}

impl OpcodeProfile {
    pub fn new() -> Self {
        let len = tsn_core::OpCode::OpAssertNotNull as usize + 1;
        let counters = (0..len).map(|_| AtomicU64::new(0)).collect();
        Self { counters }
    }

    pub fn record(&self, op: tsn_core::OpCode) {
        self.counters[op as usize].fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> Vec<u64> {
        self.counters
            .iter()
            .map(|c| c.load(Ordering::Relaxed))
            .collect()
    }
}

pub struct Vm {
    pub(super) stack: Vec<Value>,
    pub(super) frames: Vec<CallFrame>,
    pub(super) globals: Arc<RwLock<HashMap<Arc<str>, Value>>>,
    pub(super) try_handlers: Vec<TryEntry>,
    pub(super) modules: HashMap<String, Value>,
    pub(super) module_exports: tsn_types::RuntimeObject,
    pub(super) precompiled_protos: HashMap<String, Arc<FunctionProto>>,
    pub(super) vm_suspend: Option<VmSuspend>,
    pub(crate) pending_spawns: Vec<(Box<Vm>, AsyncFuture)>,
    pub(crate) pending_async_gen_spawns: Vec<(TaskId, Box<Vm>)>,
    pub(crate) gen_channel: Option<Rc<tsn_types::generator::GenChannel>>,
    pub(super) open_upvalues: Vec<Arc<tsn_types::Upvalue>>,
    pub(super) opcode_profile: Option<Arc<OpcodeProfile>>,
    pub calls: bool,
    pub trace: bool,
}

impl Vm {
    pub fn new() -> Self {
        crate::runtime::init_heap();
        let mut globals_map = HashMap::new();
        tsn_runtime::register_globals(&mut globals_map);
        let globals = Arc::new(RwLock::new(globals_map));
        let modules = HashMap::new();
        let vm = Vm {
            stack: Vec::with_capacity(256),
            frames: Vec::with_capacity(64),
            globals,
            try_handlers: Vec::new(),
            modules,
            module_exports: tsn_types::RuntimeObject::new(),
            precompiled_protos: HashMap::new(),
            trace: false,
            calls: false,
            vm_suspend: None,
            pending_spawns: Vec::new(),
            pending_async_gen_spawns: Vec::new(),
            gen_channel: None,
            open_upvalues: Vec::new(),
            opcode_profile: None,
        };

        vm
    }

    pub fn snapshot_globals(&self) -> HashMap<Arc<str>, Value> {
        self.globals.read().clone()
    }

    pub fn from_globals_snapshot(snapshot: HashMap<Arc<str>, Value>) -> Self {
        crate::runtime::init_heap();
        Vm {
            stack: Vec::with_capacity(256),
            frames: Vec::with_capacity(64),
            modules: HashMap::new(),
            globals: Arc::new(RwLock::new(snapshot)),
            try_handlers: Vec::new(),
            module_exports: tsn_types::RuntimeObject::new(),
            precompiled_protos: HashMap::new(),
            trace: false,
            calls: false,
            vm_suspend: None,
            pending_spawns: Vec::new(),
            pending_async_gen_spawns: Vec::new(),
            gen_channel: None,
            open_upvalues: Vec::new(),
            opcode_profile: None,
        }
    }

    pub fn new_with_globals(globals: Arc<RwLock<HashMap<Arc<str>, Value>>>) -> Self {
        Vm {
            stack: Vec::with_capacity(256),
            frames: Vec::with_capacity(64),
            modules: HashMap::new(),
            globals,
            try_handlers: Vec::new(),
            module_exports: tsn_types::RuntimeObject::new(),
            precompiled_protos: HashMap::new(),
            trace: false,
            calls: false,
            vm_suspend: None,
            pending_spawns: Vec::new(),
            pending_async_gen_spawns: Vec::new(),
            gen_channel: None,
            open_upvalues: Vec::new(),
            opcode_profile: None,
        }
    }

    pub fn enable_opcode_profile(&mut self) {
        self.opcode_profile = Some(Arc::new(OpcodeProfile::new()));
    }

    pub fn opcode_profile_snapshot(&self) -> Option<Vec<u64>> {
        self.opcode_profile.as_ref().map(|p| p.snapshot())
    }

    pub fn set_precompiled_protos(
        &mut self,
        protos: std::collections::HashMap<String, Arc<FunctionProto>>,
    ) {
        self.precompiled_protos = protos;
    }

    pub(super) fn push(&mut self, v: Value) {
        self.stack.push(v);
    }

    pub(super) fn pop(&mut self) -> Result<Value, String> {
        self.stack.pop().ok_or_else(|| "stack underflow".to_owned())
    }

    pub(super) fn pop2(&mut self) -> Result<(Value, Value), String> {
        let b = self.pop()?;
        let a = self.pop()?;
        Ok((a, b))
    }

    pub fn dispatch_value(&mut self, val: Value) -> Result<(), String> {
        loop {
            let handler = match self.try_handlers.pop() {
                Some(h) => h,
                None => return Err(val.to_string()),
            };
            if handler.frame_depth > self.frames.len() {
                continue;
            }
            while self.frames.len() > handler.frame_depth {
                let frame = self
                    .frames
                    .pop()
                    .expect("frames underflow in error dispatch");
                while self.stack.len() > frame.base {
                    self.stack.pop();
                }
            }
            self.stack.truncate(handler.stack_depth);
            self.push(val);
            self.frame_mut().ip = handler.catch_ip;
            return Ok(());
        }
    }

    pub(super) fn frame(&self) -> &CallFrame {
        self.frames.last().expect("no active call frame")
    }

    pub(super) fn frame_mut(&mut self) -> &mut CallFrame {
        self.frames.last_mut().expect("no active call frame")
    }

    pub(super) fn read_u16(&mut self) -> u16 {
        let frame = self.frames.last_mut().expect("no active call frame");
        let v = frame.closure.proto.chunk.code[frame.ip];
        frame.ip += 1;
        v
    }

    pub(super) fn read_const(&self, idx: u16) -> Value {
        match self.frame().closure.proto.chunk.constants.get(idx as usize) {
            Some(PoolEntry::Literal(lit)) => match lit {
                Literal::Null => Value::Null,
                Literal::Bool(b) => Value::Bool(*b),
                Literal::Int(n) => Value::Int(*n),
                Literal::Float(f) => Value::Float(*f),
                Literal::Str(s) => Value::Str(s.clone()),
                Literal::BigInt(n) => Value::BigInt(Box::new(*n)),
                Literal::Decimal(d) => Value::Decimal(Box::new(*d)),
                Literal::Symbol(s) => Value::Symbol(s.clone()),
            },
            Some(PoolEntry::Function(p)) => Value::Closure(Arc::new(Closure {
                proto: Arc::new(p.clone()),
                upvalues: vec![],
            })),
            None => Value::Null,
        }
    }

    pub(super) fn get_str_const(&self, idx: u16) -> Arc<str> {
        match self.frame().closure.proto.chunk.constants.get(idx as usize) {
            Some(PoolEntry::Literal(Literal::Str(s))) => s.clone(),
            _ => Arc::from(format!("<const#{}>", idx)),
        }
    }

    pub fn register_module(&mut self, path: impl Into<String>, value: Value) {
        self.modules.insert(path.into(), value);
    }

    pub fn take_module_exports(&mut self) -> Value {
        let fields = std::mem::replace(&mut self.module_exports, tsn_types::RuntimeObject::new());
        let mut obj = ObjData::new();
        obj.fields = fields;
        tsn_types::value::new_object(obj)
    }

    pub fn take_pending_spawns(&mut self) -> Vec<(Box<Vm>, AsyncFuture)> {
        std::mem::take(&mut self.pending_spawns)
    }

    pub fn take_pending_async_gen_spawns(&mut self) -> Vec<(TaskId, Box<Vm>)> {
        std::mem::take(&mut self.pending_async_gen_spawns)
    }

    pub fn poll_vm(&mut self) -> (Poll, Option<VmSuspend>) {
        self.vm_suspend = None;
        let result = self.run_loop();
        let suspend = self.vm_suspend.take();

        match suspend {
            Some(s) => (Poll::Pending, Some(s)),
            None => (Poll::Ready(result), None),
        }
    }

    pub fn create_error_object(&self, message: String) -> Value {
        let mut obj = ObjData::new();
        obj.fields.insert(
            Arc::from("message"),
            Value::Str(Arc::from(message.as_str())),
        );

        let stack = self.capture_stack_trace();
        let mut stack_arr = Vec::with_capacity(stack.len());
        for frame in stack {
            let mut f_obj = ObjData::new();
            f_obj.fields.insert(
                Arc::from("fn"),
                Value::Str(Arc::from(frame.fn_name.as_str())),
            );
            f_obj
                .fields
                .insert(Arc::from("line"), Value::Int(frame.line as i64));
            stack_arr.push(tsn_types::value::new_object(f_obj));
        }
        obj.fields
            .insert(Arc::from("stack"), tsn_types::value::new_array(stack_arr));
        obj.fields
            .insert(Arc::from("__is_tsn_error__"), Value::Bool(true));

        tsn_types::value::new_object(obj)
    }

    fn capture_stack_trace(&self) -> Vec<crate::StackFrame> {
        self.frames
            .iter()
            .rev()
            .map(|frame| {
                let ip = frame.ip.saturating_sub(1);
                let line = frame
                    .closure
                    .proto
                    .chunk
                    .lines
                    .get(ip)
                    .copied()
                    .unwrap_or(0);
                let fn_name = frame
                    .closure
                    .proto
                    .name
                    .clone()
                    .unwrap_or_else(|| "<anonymous>".to_owned());
                crate::StackFrame { fn_name, line }
            })
            .collect()
    }

    pub fn run_proto(&mut self, proto: FunctionProto) -> Result<Value, crate::RuntimeError> {
        use crate::runtime::Scheduler;
        use tsn_types::future::{AsyncFuture, FutureState};

        let closure = Arc::new(Closure {
            proto: Arc::new(proto),
            upvalues: vec![],
        });
        let ic_count = closure.proto.cache_count;

        let mut main_vm = Box::new(Vm::new_with_globals(Arc::clone(&self.globals)));
        main_vm.trace = self.trace;
        main_vm.calls = self.calls;
        main_vm.opcode_profile = self.opcode_profile.clone();
        main_vm.precompiled_protos = self.precompiled_protos.clone();
        main_vm.frames.push(CallFrame {
            closure,
            ip: 0,
            base: 0,
            current_class: None,
            ic_slots: vec![tsn_types::chunk::CacheEntry::default(); ic_count],
        });

        main_vm.modules = std::mem::take(&mut self.modules);

        let output = AsyncFuture::pending();
        let output_clone = output.clone();

        let mut sched = Scheduler::new(Arc::clone(&self.globals));
        sched.spawn_root(main_vm, output_clone);
        let _ = sched.run();

        self.modules = HashMap::new();

        match output.peek_state() {
            FutureState::Resolved(v) => Ok(v),
            FutureState::Rejected(v) => {
                let mut stack = Vec::new();
                let mut message = v.to_string();

                if let Value::Object(obj_ptr) = &v {
                    let obj = unsafe { &**obj_ptr };
                    if let Some(Value::Bool(true)) = obj.fields.get("__is_tsn_error__") {
                        if let Some(msg_val) = obj.fields.get("message") {
                            message = msg_val.to_string();
                        }

                        if let Some(Value::Array(stack_arr_ptr)) = obj.fields.get("stack") {
                            let stack_arr = unsafe { &**stack_arr_ptr };
                            for f_val in stack_arr.iter() {
                                if let Value::Object(f_obj_ptr) = f_val {
                                    let f_obj = unsafe { &**f_obj_ptr };
                                    let fn_name = f_obj
                                        .fields
                                        .get("fn")
                                        .map(|v| v.to_string())
                                        .unwrap_or_default();
                                    let line = f_obj
                                        .fields
                                        .get("line")
                                        .and_then(|v| {
                                            if let Value::Int(i) = v {
                                                Some(*i as u32)
                                            } else {
                                                None
                                            }
                                        })
                                        .unwrap_or(0);
                                    stack.push(crate::StackFrame { fn_name, line });
                                }
                            }
                        }
                    }
                }

                if stack.is_empty() {
                    stack = self.capture_stack_trace();
                }
                Err(crate::RuntimeError::new(message, stack))
            }
            FutureState::Pending => Err(crate::RuntimeError::new(
                "main task never completed".to_owned(),
                vec![],
            )),
        }
    }

    pub fn run_loop(&mut self) -> Result<Value, String> {
        self.run_until(0)
    }

    pub fn call(&mut self, callee: Value, args: &[Value]) -> Result<Value, String> {
        self.push(callee.clone());
        for arg in args {
            self.push(arg.clone());
        }
        let depth_before = self.frames.len();
        self.call_value(callee, args.len())?;
        self.run_until(depth_before)
    }

    pub(super) fn close_upvalues_on_stack(&mut self, last_idx: usize) {
        let mut i = 0;
        while i < self.open_upvalues.len() {
            let upvalue = &self.open_upvalues[i];
            let close = {
                let inner = upvalue.inner.lock();
                if let Some(loc) = inner.location {
                    loc >= last_idx
                } else {
                    false
                }
            };

            if close {
                let upvalue = self.open_upvalues.remove(i);
                let mut inner = upvalue.inner.lock();
                if let Some(loc) = inner.location.take() {
                    if loc < self.stack.len() {
                        inner.value = self.stack[loc].clone();
                    }
                }
            } else {
                i += 1;
            }
        }
    }
}

const _: fn() = || {
    let _: Option<Arc<Upvalue>> = None;
};

impl tsn_types::Context for Vm {
    fn spawn(&mut self, callee: Value, args: &[Value]) -> Result<AsyncFuture, String> {
        let output = AsyncFuture::pending();
        let mut task_vm = Box::new(Vm::new_with_globals(Arc::clone(&self.globals)));
        task_vm.trace = self.trace;
        task_vm.calls = self.calls;
        task_vm.opcode_profile = self.opcode_profile.clone();
        task_vm.stack = args.to_vec();

        match callee {
            Value::Closure(c) => {
                let ic_count = c.proto.cache_count;
                task_vm.frames.push(CallFrame {
                    closure: c,
                    ip: 0,
                    base: 0,
                    current_class: None,
                    ic_slots: vec![tsn_types::chunk::CacheEntry::default(); ic_count],
                });
            }
            _ => return Err(format!("cannot spawn non-closure: {}", callee.type_name())),
        }

        self.pending_spawns.push((task_vm, output.clone()));
        Ok(output)
    }

    fn call(&mut self, callee: Value, args: &[Value]) -> Result<Value, String> {
        Vm::call(self, callee, args)
    }

    fn set_timer(
        &mut self,
        _ms: u64,
        _repeat: bool,
        _callee: Value,
        _args: &[Value],
    ) -> Result<usize, String> {
        static TIMER_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
        let id = TIMER_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        Ok(id)
    }

    fn clear_timer(&mut self, _id: usize) -> Result<(), String> {
        Ok(())
    }
}
