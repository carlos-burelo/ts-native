use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::future::AsyncFuture;
use crate::value::Value;

pub trait GeneratorDriver: std::fmt::Debug {
    fn next(&self, input: Value) -> Result<Value, String>;
    fn is_done(&self) -> bool;
    fn is_async(&self) -> bool;
}

#[derive(Debug)]
pub struct GenChannel {
    pub output: RefCell<Option<AsyncFuture>>,
    pub done: Cell<bool>,

    pub started: Cell<bool>,
}

impl GenChannel {
    pub fn new() -> Rc<Self> {
        Rc::new(GenChannel {
            output: RefCell::new(None),
            done: Cell::new(false),
            started: Cell::new(false),
        })
    }

    pub fn is_done(&self) -> bool {
        self.done.get()
    }
}

#[derive(Clone, Debug)]
pub struct GeneratorObj(pub Rc<dyn GeneratorDriver>);

impl PartialEq for GeneratorObj {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for GeneratorObj {}

impl std::hash::Hash for GeneratorObj {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (Rc::as_ptr(&self.0) as *const () as usize).hash(state);
    }
}

#[derive(Debug)]
pub struct AsyncQueueInner {
    pub queue: std::collections::VecDeque<Value>,
    pub done: bool,
    pub waiter: Option<AsyncFuture>,
}

#[derive(Clone, Debug)]
pub struct AsyncQueue(pub Arc<Mutex<AsyncQueueInner>>);

impl AsyncQueue {
    pub fn new() -> Self {
        AsyncQueue(Arc::new(Mutex::new(AsyncQueueInner {
            queue: std::collections::VecDeque::new(),
            done: false,
            waiter: None,
        })))
    }

    pub fn push(&self, value: Value) {
        let mut inner = self.0.lock();
        if inner.done {
            return;
        }
        if let Some(waiter) = inner.waiter.take() {
            waiter.resolve(make_iter_result(value, false));
        } else {
            inner.queue.push_back(value);
        }
    }

    pub fn close(&self) {
        let mut inner = self.0.lock();
        inner.done = true;
        if let Some(waiter) = inner.waiter.take() {
            waiter.resolve(make_iter_result(Value::Null, true));
        }
    }

    pub fn next_value(&self) -> Value {
        let mut inner = self.0.lock();
        if let Some(chunk) = inner.queue.pop_front() {
            return Value::Future(AsyncFuture::resolved(make_iter_result(chunk, false)));
        }
        if inner.done {
            return Value::Future(AsyncFuture::resolved(make_iter_result(Value::Null, true)));
        }
        let fut = AsyncFuture::pending();
        inner.waiter = Some(fut.clone());
        Value::Future(fut)
    }
}

fn make_iter_result(value: Value, done: bool) -> Value {
    let ptr = crate::value::alloc_object();
    let obj = unsafe { &mut *ptr };
    obj.fields.insert(Arc::from("value"), value);
    obj.fields.insert(Arc::from("done"), Value::Bool(done));
    Value::Object(ptr)
}
