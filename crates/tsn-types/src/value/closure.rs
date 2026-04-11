use crate::chunk::FunctionProto;
use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Upvalue {
    pub inner: Arc<Mutex<UpvalueInner>>,
}

#[derive(Debug, Clone)]
pub struct UpvalueInner {
    pub value: Value,
    pub location: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Closure {
    pub proto: Arc<FunctionProto>,
    pub upvalues: Vec<Upvalue>,
}

use super::Value;
