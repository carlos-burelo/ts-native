use crate::value::Value;
use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum FutureState {
    Pending,
    Resolved(Value),
    Rejected(Value),
}

type SettleCallback = Box<dyn FnOnce(Result<Value, Value>) + Send + 'static>;

struct Inner {
    state: FutureState,
    on_settle: Vec<SettleCallback>,
}

#[derive(Clone)]
pub struct AsyncFuture(Arc<Mutex<Inner>>);

impl PartialEq for AsyncFuture {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for AsyncFuture {}

impl std::hash::Hash for AsyncFuture {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}

impl std::fmt::Debug for AsyncFuture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.peek_state() {
            FutureState::Pending => write!(f, "Future(<pending>)"),
            FutureState::Resolved(v) => write!(f, "Future({})", v),
            FutureState::Rejected(v) => write!(f, "Future(<rejected:{}>)", v),
        }
    }
}

impl AsyncFuture {
    pub fn pending() -> Self {
        AsyncFuture(Arc::new(Mutex::new(Inner {
            state: FutureState::Pending,
            on_settle: Vec::new(),
        })))
    }

    pub fn resolved(v: Value) -> Self {
        AsyncFuture(Arc::new(Mutex::new(Inner {
            state: FutureState::Resolved(v),
            on_settle: Vec::new(),
        })))
    }

    pub fn rejected(v: Value) -> Self {
        AsyncFuture(Arc::new(Mutex::new(Inner {
            state: FutureState::Rejected(v),
            on_settle: Vec::new(),
        })))
    }

    pub fn rejected_msg(msg: impl Into<String>) -> Self {
        let s: String = msg.into();
        Self::rejected(Value::Str(Arc::from(s.as_str())))
    }

    pub fn peek_state(&self) -> FutureState {
        self.0.lock().state.clone()
    }

    pub fn is_pending(&self) -> bool {
        matches!(self.0.lock().state, FutureState::Pending)
    }

    pub fn is_resolved(&self) -> bool {
        matches!(self.0.lock().state, FutureState::Resolved(_))
    }

    pub fn is_rejected(&self) -> bool {
        matches!(self.0.lock().state, FutureState::Rejected(_))
    }

    pub fn settle(&self, result: Result<Value, Value>) {
        let callbacks = {
            let mut g = self.0.lock();
            if !matches!(g.state, FutureState::Pending) {
                return;
            }
            match &result {
                Ok(v) => g.state = FutureState::Resolved(v.clone()),
                Err(v) => g.state = FutureState::Rejected(v.clone()),
            }
            std::mem::take(&mut g.on_settle)
        };
        for cb in callbacks {
            cb(result.clone());
        }
    }

    #[inline]
    pub fn resolve(&self, v: Value) {
        self.settle(Ok(v));
    }

    #[inline]
    pub fn reject(&self, v: Value) {
        self.settle(Err(v));
    }

    #[inline]
    pub fn reject_msg(&self, msg: impl Into<String>) {
        let s: String = msg.into();
        self.reject(Value::Str(Arc::from(s.as_str())));
    }

    pub fn on_settle<F>(&self, cb: F)
    where
        F: FnOnce(Result<Value, Value>) + Send + 'static,
    {
        let already = {
            let mut g = self.0.lock();
            match &g.state {
                FutureState::Pending => {
                    g.on_settle.push(Box::new(cb));
                    return;
                }
                FutureState::Resolved(v) => Ok(v.clone()),
                FutureState::Rejected(v) => Err(v.clone()),
            }
        };
        cb(already);
    }

    #[inline]
    pub fn rejected_value(v: Value) -> Self {
        Self::rejected(v)
    }
}

pub enum Poll {
    Ready(Result<Value, String>),
    Pending,
}

pub fn resolve_future(v: Value) -> Value {
    Value::Future(AsyncFuture::resolved(v))
}

pub fn reject_future(msg: String) -> Value {
    Value::Future(AsyncFuture::rejected_msg(msg))
}

pub fn reject_value_future(v: Value) -> Value {
    Value::Future(AsyncFuture::rejected(v))
}
