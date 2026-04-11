pub use crate::future::{FutureState, Poll};

use crate::future::AsyncFuture;
use super::Value;

pub fn resolve_future(v: Value) -> Value { Value::Future(AsyncFuture::resolved(v)) }

pub fn reject_future(msg: String) -> Value { Value::Future(AsyncFuture::rejected_msg(msg)) }

pub fn reject_value_future(v: Value) -> Value { Value::Future(AsyncFuture::rejected(v)) }
