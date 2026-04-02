use super::frame::CallFrame;
use super::generator::{AsyncGenDriver, SyncGenDriver};
use super::Vm;
use crate::runtime::task::alloc_task_id;
use std::rc::Rc;
use std::sync::Arc;
use tsn_compiler::chunk::FunctionProto;
use tsn_types::chunk::CacheEntry;
use tsn_types::future::AsyncFuture;
use tsn_types::generator::{GenChannel, GeneratorObj};
use tsn_types::value::new_array;
use tsn_types::value::Value;

impl super::Vm {
    pub fn flatten_spread_args(&self, raw_args: Vec<Value>) -> Result<Vec<Value>, String> {
        let mut flat_args = Vec::new();
        for arg in raw_args {
            match arg {
                Value::Spread(inner) => match *inner {
                    Value::Array(arr) => {
                        let values = unsafe { &*arr };
                        flat_args.extend(values.iter().cloned());
                    }
                    other => {
                        return Err(format!(
                            "spread argument must be an array, got {}",
                            other.type_name()
                        ));
                    }
                },
                other => flat_args.push(other),
            }
        }
        Ok(flat_args)
    }

    pub fn call_value(&mut self, callee: Value, arg_count: usize) -> Result<(), String> {
        match callee {
            Value::Closure(c) if c.proto.is_async && c.proto.is_generator => {
                let mut arg_count = arg_count;
                self.bundle_rest_args(&c.proto, &mut arg_count);

                let start = self.stack.len() - arg_count;
                let args: Vec<Value> = self.stack.drain(start..).collect();
                self.stack.pop();

                let task_id = alloc_task_id();
                let gen_channel = GenChannel::new();
                let ic_count = c.proto.cache_count;
                let mut task_vm = Box::new(Vm::new_with_globals(Arc::clone(&self.globals)));
                task_vm.trace = self.trace;
                task_vm.calls = self.calls;
                task_vm.opcode_profile = self.opcode_profile.clone();
                task_vm.stack = args;
                task_vm.gen_channel = Some(Rc::clone(&gen_channel));
                task_vm.frames.push(CallFrame {
                    closure: c,
                    ip: 0,
                    base: 0,
                    current_class: None,
                    ic_slots: vec![CacheEntry::default(); ic_count],
                });

                let driver = AsyncGenDriver::new(gen_channel, task_id);

                self.pending_async_gen_spawns.push((task_id, task_vm));

                let gen = Value::Generator(GeneratorObj(driver));
                self.push(gen);
                Ok(())
            }

            Value::Closure(c) if c.proto.is_async => {
                let mut arg_count = arg_count;
                self.bundle_rest_args(&c.proto, &mut arg_count);

                let start = self.stack.len() - arg_count;
                let args: Vec<Value> = self.stack.drain(start..).collect();
                self.stack.pop();

                let output = AsyncFuture::pending();
                let ic_count = c.proto.cache_count;
                let mut task_vm = Box::new(Vm::new_with_globals(Arc::clone(&self.globals)));
                task_vm.trace = self.trace;
                task_vm.calls = self.calls;
                task_vm.opcode_profile = self.opcode_profile.clone();
                task_vm.stack = args;
                task_vm.frames.push(CallFrame {
                    closure: c,
                    ip: 0,
                    base: 0,
                    current_class: None,
                    ic_slots: vec![CacheEntry::default(); ic_count],
                });
                self.pending_spawns.push((task_vm, output.clone()));
                self.push(Value::Future(output));
                Ok(())
            }

            Value::Closure(c) if c.proto.is_generator => {
                let mut arg_count = arg_count;
                self.bundle_rest_args(&c.proto, &mut arg_count);

                let start = self.stack.len() - arg_count;
                let args: Vec<Value> = self.stack.drain(start..).collect();
                self.stack.pop();

                let ic_count = c.proto.cache_count;
                let mut gen_vm = Box::new(Vm::new_with_globals(Arc::clone(&self.globals)));
                gen_vm.trace = self.trace;
                gen_vm.calls = self.calls;
                gen_vm.opcode_profile = self.opcode_profile.clone();
                gen_vm.stack = args;
                gen_vm.frames.push(CallFrame {
                    closure: c,
                    ip: 0,
                    base: 0,
                    current_class: None,
                    ic_slots: vec![CacheEntry::default(); ic_count],
                });

                let driver = SyncGenDriver::new(gen_vm);
                let gen = Value::Generator(GeneratorObj(driver));
                self.push(gen);
                Ok(())
            }

            Value::Closure(c) => {
                let mut arg_count = arg_count;
                self.bundle_rest_args(&c.proto, &mut arg_count);

                if self.calls {
                    let name = c.proto.name.as_deref().unwrap_or("<anon>");
                    let depth = self.frames.len();
                    eprintln!(
                        "  {}-> {}  ({} arg{})",
                        "  ".repeat(depth),
                        name,
                        arg_count,
                        if arg_count == 1 { "" } else { "s" }
                    );
                }
                let base = self.stack.len() - arg_count;
                let ic_count = c.proto.cache_count;
                self.frames.push(CallFrame {
                    closure: c,
                    ip: 0,
                    base,
                    current_class: None,
                    ic_slots: vec![CacheEntry::default(); ic_count],
                });
                Ok(())
            }
            Value::BoundMethod(bm) => {
                let method_closure = bm.method.clone();
                let owner = bm.owner_class.clone();
                let receiver = *bm.receiver.clone();

                let mut full_arg_count = arg_count + 1;
                let base = self.stack.len() - arg_count;
                self.stack.insert(base, receiver);

                self.bundle_rest_args(&method_closure.proto, &mut full_arg_count);

                if method_closure.proto.is_async && method_closure.proto.is_generator {
                    let start = self.stack.len() - full_arg_count;
                    let args: Vec<Value> = self.stack.drain(start..).collect();
                    self.stack.pop();

                    let task_id = alloc_task_id();
                    let gen_channel = GenChannel::new();
                    let ic_count = method_closure.proto.cache_count;
                    let mut task_vm = Box::new(Vm::new_with_globals(Arc::clone(&self.globals)));
                    task_vm.trace = self.trace;
                    task_vm.calls = self.calls;
                    task_vm.opcode_profile = self.opcode_profile.clone();
                    task_vm.stack = args;
                    task_vm.gen_channel = Some(Rc::clone(&gen_channel));
                    task_vm.frames.push(CallFrame {
                        closure: method_closure,
                        ip: 0,
                        base: 0,
                        current_class: owner,
                        ic_slots: vec![CacheEntry::default(); ic_count],
                    });

                    let driver = AsyncGenDriver::new(gen_channel, task_id);
                    self.pending_async_gen_spawns.push((task_id, task_vm));
                    self.push(Value::Generator(GeneratorObj(driver)));
                    Ok(())
                } else if method_closure.proto.is_async {
                    let start = self.stack.len() - full_arg_count;
                    let args: Vec<Value> = self.stack.drain(start..).collect();
                    self.stack.pop();

                    let output = AsyncFuture::pending();
                    let ic_count = method_closure.proto.cache_count;
                    let mut task_vm = Box::new(Vm::new_with_globals(Arc::clone(&self.globals)));
                    task_vm.trace = self.trace;
                    task_vm.calls = self.calls;
                    task_vm.opcode_profile = self.opcode_profile.clone();
                    task_vm.stack = args;
                    task_vm.frames.push(CallFrame {
                        closure: method_closure,
                        ip: 0,
                        base: 0,
                        current_class: owner,
                        ic_slots: vec![CacheEntry::default(); ic_count],
                    });
                    self.pending_spawns.push((task_vm, output.clone()));
                    self.push(Value::Future(output));
                    Ok(())
                } else if method_closure.proto.is_generator {
                    let start = self.stack.len() - full_arg_count;
                    let args: Vec<Value> = self.stack.drain(start..).collect();
                    self.stack.pop();

                    let ic_count = method_closure.proto.cache_count;
                    let mut gen_vm = Box::new(Vm::new_with_globals(Arc::clone(&self.globals)));
                    gen_vm.trace = self.trace;
                    gen_vm.calls = self.calls;
                    gen_vm.opcode_profile = self.opcode_profile.clone();
                    gen_vm.stack = args;
                    gen_vm.frames.push(CallFrame {
                        closure: method_closure,
                        ip: 0,
                        base: 0,
                        current_class: owner,
                        ic_slots: vec![CacheEntry::default(); ic_count],
                    });

                    let driver = SyncGenDriver::new(gen_vm);
                    self.push(Value::Generator(GeneratorObj(driver)));
                    Ok(())
                } else {
                    let ic_count = method_closure.proto.cache_count;
                    let base = self.stack.len() - full_arg_count;

                    self.frames.push(CallFrame {
                        closure: method_closure,
                        ip: 0,
                        base,
                        current_class: owner,
                        ic_slots: vec![CacheEntry::default(); ic_count],
                    });
                    Ok(())
                }
            }
            Value::NativeFn(b) => {
                let (f, _) = *b;
                let start = self.stack.len() - arg_count;
                let args: Vec<Value> = self.stack.drain(start..).collect();
                self.stack.pop();
                let result = f(self as &mut dyn tsn_types::Context, &args)?;
                self.push(result);
                Ok(())
            }
            Value::NativeBoundMethod(b) => {
                let (receiver, f, _) = *b;
                let start = self.stack.len() - arg_count;
                let call_args: Vec<Value> = self.stack.drain(start..).collect();
                self.stack.pop();
                let mut args = Vec::with_capacity(1 + call_args.len());
                args.push(receiver.clone());
                args.extend(call_args);
                let result = f(self as &mut dyn tsn_types::Context, &args)?;
                self.push(result);
                Ok(())
            }
            Value::Class(cls) => {
                let this = Value::instance(cls.clone());

                let constructor = cls.find_method("constructor");
                if let Some(Value::Closure(ctor)) = constructor {
                    let mut full_arg_count = arg_count + 1;
                    let base = self.stack.len() - arg_count;
                    self.stack.insert(base, this.clone());

                    self.bundle_rest_args(&ctor.proto, &mut full_arg_count);

                    let depth_before = self.frames.len();
                    let ic_count = ctor.proto.cache_count;
                    let base = self.stack.len() - full_arg_count;

                    self.frames.push(CallFrame {
                        closure: ctor,
                        ip: 0,
                        base,
                        current_class: Some(cls.clone()),
                        ic_slots: vec![CacheEntry::default(); ic_count],
                    });
                    let ctor_ret = self.run_until(depth_before)?;
                    match ctor_ret {
                        Value::Object(_) | Value::Map(_) | Value::Set(_) => self.push(ctor_ret),
                        _ => self.push(this),
                    }
                } else if let Some(Value::NativeFn(b)) = constructor {
                    let start = self.stack.len() - arg_count;
                    let call_args: Vec<Value> = self.stack.drain(start..).collect();
                    self.stack.pop();
                    let mut full_args = Vec::with_capacity(1 + call_args.len());
                    full_args.push(this.clone());
                    full_args.extend(call_args);
                    let result = (b.0)(self as &mut dyn tsn_types::Context, &full_args)?;
                    match result {
                        Value::Null => self.push(this),
                        other => self.push(other),
                    }
                } else {
                    for _ in 0..arg_count {
                        self.stack.pop();
                    }
                    self.stack.pop();
                    self.push(this);
                }
                Ok(())
            }
            Value::Object(o) => {
                let maybe_new = unsafe { &*o }.fields.get("new").cloned();
                if let Some(new_fn) = maybe_new {
                    let callee_slot = self.stack.len() - 1 - arg_count;
                    self.stack[callee_slot] = new_fn.clone();
                    self.call_value(new_fn, arg_count)
                } else {
                    Err(format!("cannot call object: no 'new' method found"))
                }
            }
            _ => Err(format!("cannot call value of type {}", callee.type_name())),
        }
    }

    pub fn call_value_with_spread(
        &mut self,
        callee: Value,
        arg_count: usize,
    ) -> Result<(), String> {
        let start = self.stack.len().saturating_sub(arg_count);
        let raw_args: Vec<Value> = self.stack.drain(start..).collect();
        self.stack.pop();
        let flat_args = self.flatten_spread_args(raw_args)?;

        let flat_arg_count = flat_args.len();
        self.push(callee.clone());
        for arg in flat_args {
            self.push(arg);
        }
        self.call_value(callee, flat_arg_count)
    }

    fn bundle_rest_args(&mut self, proto: &FunctionProto, arg_count: &mut usize) {
        let arity = proto.arity;
        if proto.has_rest {
            let rest_idx = arity.saturating_sub(1);
            if *arg_count > rest_idx {
                let num_to_bundle = *arg_count - rest_idx;
                let start = self.stack.len() - num_to_bundle;
                let rest_args: Vec<Value> = self.stack.drain(start..).collect();
                self.push(new_array(rest_args));
                *arg_count = rest_idx + 1;
            } else {
                for _ in *arg_count..rest_idx {
                    self.push(Value::Null);
                }
                self.push(tsn_types::value::new_array(vec![]));
                *arg_count = arity;
            }
        } else if *arg_count < arity {
            for _ in *arg_count..arity {
                self.push(Value::Null);
            }
            *arg_count = arity;
        }
    }
}
