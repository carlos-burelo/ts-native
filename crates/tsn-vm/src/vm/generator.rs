use super::{Vm, VmSuspend};
use crate::runtime::task::{schedule_wake_sync, TaskId};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tsn_types::future::AsyncFuture;
use tsn_types::generator::{AsyncQueue, GenChannel, GeneratorDriver};
use tsn_types::Value;

pub(crate) fn make_iter_result(value: Value, done: bool) -> Value {
    let mut obj = tsn_types::value::ObjData::new();
    obj.fields.insert(Arc::from("value"), value);
    obj.fields.insert(Arc::from("done"), Value::Bool(done));
    tsn_types::value::new_object(obj)
}

struct SyncGenInner {
    vm: Box<Vm>,
    started: bool,
    done: bool,
}

impl std::fmt::Debug for SyncGenInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SyncGenInner(done={})", self.done)
    }
}

#[derive(Debug)]
pub struct SyncGenDriver {
    inner: RefCell<SyncGenInner>,
}

impl SyncGenDriver {
    pub fn new(vm: Box<Vm>) -> Rc<Self> {
        Rc::new(SyncGenDriver {
            inner: RefCell::new(SyncGenInner {
                vm,
                started: false,
                done: false,
            }),
        })
    }
}

impl GeneratorDriver for SyncGenDriver {
    fn next(&self, input: Value) -> Result<Value, String> {
        let mut inner = self.inner.borrow_mut();

        if inner.done {
            return Ok(make_iter_result(Value::Null, true));
        }

        if inner.started {
            inner.vm.push(input);
        }
        inner.started = true;

        let result = inner.vm.run_loop();

        match inner.vm.vm_suspend.take() {
            Some(VmSuspend::Yield(val)) => Ok(make_iter_result(val, false)),
            Some(VmSuspend::Future(_)) | Some(VmSuspend::Timer(_)) => {
                inner.done = true;
                Err("cannot use `await` inside a sync generator (`function*`)".to_string())
            }
            None => {
                inner.done = true;
                let ret = match result {
                    Ok(v) => v,
                    Err(e) => return Err(e),
                };
                Ok(make_iter_result(ret, true))
            }
        }
    }

    fn is_done(&self) -> bool {
        self.inner.borrow().done
    }

    fn is_async(&self) -> bool {
        false
    }
}

pub struct AsyncGenDriver {
    pub gen_channel: Rc<GenChannel>,
    pub task_id: TaskId,
}

impl std::fmt::Debug for AsyncGenDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AsyncGenDriver(task_id={})", self.task_id)
    }
}

impl AsyncGenDriver {
    pub fn new(gen_channel: Rc<GenChannel>, task_id: TaskId) -> Rc<Self> {
        Rc::new(AsyncGenDriver {
            gen_channel,
            task_id,
        })
    }
}

impl GeneratorDriver for AsyncGenDriver {
    fn next(&self, input: Value) -> Result<Value, String> {
        if self.gen_channel.is_done() {
            return Ok(Value::Future(AsyncFuture::resolved(make_iter_result(
                Value::Null,
                true,
            ))));
        }

        let output = AsyncFuture::pending();
        *self.gen_channel.output.borrow_mut() = Some(output.clone());

        let already_started = self.gen_channel.started.replace(true);
        let resume = if already_started {
            Some(Ok(input))
        } else {
            None
        };
        schedule_wake_sync(self.task_id, resume);

        Ok(Value::Future(output))
    }

    fn is_done(&self) -> bool {
        self.gen_channel.is_done()
    }

    fn is_async(&self) -> bool {
        true
    }
}

impl Drop for AsyncGenDriver {
    fn drop(&mut self) {
        if !self.gen_channel.is_done() {
            self.gen_channel.done.set(true);
            *self.gen_channel.output.borrow_mut() = None;
            schedule_wake_sync(
                self.task_id,
                Some(Err(Value::Str(Arc::from("generator dropped")))),
            );
        }
    }
}

#[derive(Debug, Clone)]
pub struct AsyncQueueDriver(pub AsyncQueue);

impl GeneratorDriver for AsyncQueueDriver {
    fn next(&self, _input: Value) -> Result<Value, String> {
        Ok(self.0.next_value())
    }

    fn is_done(&self) -> bool {
        let inner = self.0 .0.lock();
        inner.done && inner.queue.is_empty()
    }

    fn is_async(&self) -> bool {
        true
    }
}
