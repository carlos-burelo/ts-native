use crate::vm::Vm;
use parking_lot::RwLock;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tsn_types::future::{AsyncFuture, Poll};
use tsn_types::Value;

use super::reactor::{CallbackKind, CallbackTask, EventQueue, ExternalEvent, Reactor, WakeResult};
use super::task::{alloc_task_id, drain_sync_wake_queue, ReadyTask, TaskRegistry};
use crate::vm::VmSuspend;

#[allow(dead_code)]
pub enum TaskStrategy {
    Local,
    Parallel,
}

thread_local! {
    static SCHED_EVENT_QUEUE: RefCell<Option<EventQueue>> = const { RefCell::new(None) };
}

pub(crate) fn get_callback_event_queue() -> Option<EventQueue> {
    SCHED_EVENT_QUEUE.with(|q| q.borrow().clone())
}

pub struct Scheduler {
    registry: TaskRegistry,
    reactor: Reactor,

    event_queue: EventQueue,

    globals: Arc<RwLock<HashMap<Arc<str>, Value>>>,
}

impl Scheduler {
    pub fn new(globals: Arc<RwLock<HashMap<Arc<str>, Value>>>) -> Self {
        let event_queue: EventQueue = Arc::new(Mutex::new(VecDeque::new()));
        Scheduler {
            registry: TaskRegistry::new(),
            reactor: Reactor::spawn(Arc::clone(&event_queue)),
            event_queue,
            globals,
        }
    }

    pub fn spawn_root(&mut self, vm: Box<Vm>, output: AsyncFuture) {
        self.registry.spawn(vm, output);
    }

    pub fn run(mut self) -> Result<Value, String> {
        SCHED_EVENT_QUEUE.with(|q| *q.borrow_mut() = Some(Arc::clone(&self.event_queue)));

        loop {
            let events: Vec<ExternalEvent> = {
                let mut eq = self.event_queue.lock().unwrap();
                eq.drain(..).collect()
            };
            for event in events {
                match event {
                    ExternalEvent::WakeTask(id) => {
                        self.registry.wake(id, Some(Ok(Value::Null)));
                    }
                    ExternalEvent::WakeTaskWithResult(id, WakeResult(resume)) => {
                        self.registry.wake(id, resume);
                    }
                    ExternalEvent::RunCallback(ct) => {
                        run_callback(ct, &mut self.registry, &self.globals);
                    }
                }
            }

            for (id, resume) in drain_sync_wake_queue() {
                self.registry.wake(id, resume);
            }

            if let Some(task) = self.registry.queue.pop_front() {
                run_task(task, &mut self.registry, &self.reactor, &self.event_queue);
            } else if self.registry.all_done() {
                break;
            } else {
                thread::sleep(Duration::from_micros(100));
            }
        }

        Ok(Value::Null)
    }
}

fn run_task(
    mut task: ReadyTask,
    registry: &mut TaskRegistry,
    reactor: &Reactor,
    event_queue: &EventQueue,
) {
    let task_id = task.id;

    if let Some(resume) = task.resume.take() {
        match resume {
            Ok(v) => task.vm.push(v),
            Err(v) => {
                if let Err(msg) = task.vm.dispatch_value(v) {
                    registry.finish();
                    let err_obj = task.vm.create_error_object(msg);
                    task.output.reject(err_obj);
                    return;
                }
            }
        }
    }

    enqueue_spawns(&mut task.vm, registry, event_queue);

    let (poll_result, suspend) = task.vm.poll_vm();

    enqueue_spawns(&mut task.vm, registry, event_queue);

    match poll_result {
        Poll::Ready(result) => {
            registry.finish();
            if let Some(ch) = task.vm.gen_channel.as_ref() {
                ch.done.set(true);
                if let Some(out) = ch.output.borrow_mut().take() {
                    match result {
                        Ok(v) => out.resolve(crate::vm::generator::make_iter_result(v, true)),
                        Err(msg) => {
                            let err_obj = task.vm.create_error_object(msg);
                            out.reject(err_obj);
                        }
                    }
                }
            } else {
                match result {
                    Ok(v) => task.output.resolve(v),
                    Err(msg) => {
                        let err_obj = task.vm.create_error_object(msg);
                        task.output.reject(err_obj);
                    }
                }
            }
        }
        Poll::Pending => match suspend.expect("Poll::Pending without VmSuspend") {
            VmSuspend::Future(fut) => {
                registry.suspend(task_id, task.vm, task.output);
                let eq = Arc::clone(event_queue);
                fut.on_settle(move |result| {
                    eq.lock()
                        .unwrap()
                        .push_back(ExternalEvent::WakeTaskWithResult(
                            task_id,
                            WakeResult(Some(result)),
                        ));
                });
            }
            VmSuspend::Timer(duration) => {
                registry.suspend(task_id, task.vm, task.output);
                reactor.sleep(duration, task_id, Arc::clone(event_queue));
            }
            VmSuspend::Yield(val) => {
                if let Some(ch) = task.vm.gen_channel.as_ref() {
                    if let Some(out) = ch.output.borrow_mut().take() {
                        out.resolve(crate::vm::generator::make_iter_result(val, false));
                    }
                }
                registry.suspend(task_id, task.vm, task.output);
            }
        },
    }
}

fn run_callback(
    ct: CallbackTask,
    registry: &mut TaskRegistry,
    globals: &Arc<RwLock<HashMap<Arc<str>, Value>>>,
) {
    let CallbackTask {
        cb,
        arg,
        output,
        kind,
    } = ct;

    let is_rejection = arg.is_err();

    let invoke = match kind {
        CallbackKind::Then => !is_rejection,
        CallbackKind::Catch => is_rejection,
        CallbackKind::Finally => true,
    };

    if !invoke {
        settle_output(output, arg);
        return;
    }

    let cb_arg = match kind {
        CallbackKind::Finally => Value::Null,

        _ => match &arg {
            Ok(v) => v.clone(),
            Err(e) => e.clone(),
        },
    };

    struct NoCtx;
    impl tsn_types::Context for NoCtx {
        fn spawn(&mut self, _: Value, _: &[Value]) -> Result<AsyncFuture, String> {
            Err("cannot spawn from callback context".into())
        }
        fn call(&mut self, _: Value, _: &[Value]) -> Result<Value, String> {
            Err("cannot call from callback context".into())
        }
        fn set_timer(&mut self, _: u64, _: bool, _: Value, _: &[Value]) -> Result<usize, String> {
            Err("cannot set timer from callback context".into())
        }
        fn clear_timer(&mut self, _: usize) -> Result<(), String> {
            Err("cannot clear timer from callback context".into())
        }
    }

    match cb {
        Value::NativeFn(b) => {
            let (f, _) = *b;
            match f(&mut NoCtx, &[cb_arg]) {
                Ok(v) => finalize_callback(output, arg, kind, v),
                Err(e) => output.reject_msg(e),
            }
        }

        Value::NativeBoundMethod(b) => {
            let (recv, f, _) = *b;
            match f(&mut NoCtx, &[recv, cb_arg]) {
                Ok(v) => finalize_callback(output, arg, kind, v),
                Err(e) => output.reject_msg(e),
            }
        }

        Value::Closure(ref c) => {
            let is_async = c.proto.is_async;
            let mut task_vm = Box::new(Vm::new_with_globals(Arc::clone(globals)));
            task_vm.push(cb.clone());
            task_vm.push(cb_arg);
            if let Err(e) = task_vm.call_value(cb, 1) {
                output.reject_msg(e);
                return;
            }

            let task_output = if kind == CallbackKind::Finally {
                let intermediate = AsyncFuture::pending();
                let out = output;
                intermediate.clone().on_settle(move |r| match r {
                    Ok(_) => settle_output(out, arg),
                    Err(e) => out.reject(e),
                });
                intermediate
            } else {
                output
            };

            if is_async {
                for (sub_vm, sub_out) in task_vm.take_pending_spawns() {
                    let sub_id = alloc_task_id();
                    let to = task_output.clone();
                    sub_out.on_settle(move |r| match r {
                        Ok(v) => to.resolve(v),
                        Err(e) => to.reject(e),
                    });
                    registry.active += 1;
                    registry.queue.push_back(ReadyTask {
                        id: sub_id,
                        vm: sub_vm,
                        output: sub_out,
                        resume: None,
                    });
                }
            } else {
                let id = alloc_task_id();
                registry.active += 1;
                registry.queue.push_back(ReadyTask {
                    id,
                    vm: task_vm,
                    output: task_output,
                    resume: None,
                });
            }
        }

        _ => settle_output(output, arg),
    }
}

fn finalize_callback(
    output: AsyncFuture,
    arg: Result<Value, Value>,
    kind: CallbackKind,
    ret: Value,
) {
    match kind {
        CallbackKind::Finally => settle_output(output, arg),
        _ => output.resolve(ret),
    }
}

fn settle_output(output: AsyncFuture, result: Result<Value, Value>) {
    match result {
        Ok(v) => output.resolve(v),
        Err(e) => output.reject(e),
    }
}

fn enqueue_spawns(vm: &mut Box<Vm>, registry: &mut TaskRegistry, _event_queue: &EventQueue) {
    for (sub_vm, output) in vm.take_pending_spawns() {
        let id = alloc_task_id();
        registry.active += 1;
        registry.queue.push_back(ReadyTask {
            id,
            vm: sub_vm,
            output,
            resume: None,
        });
    }

    for (task_id, task_vm) in vm.take_pending_async_gen_spawns() {
        registry.active += 1;
        let dummy_output = AsyncFuture::pending();
        registry.suspended.insert(
            task_id,
            super::task::SuspendedTask {
                vm: task_vm,
                output: dummy_output,
            },
        );
    }
}
