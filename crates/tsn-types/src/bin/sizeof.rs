use std::sync::Arc;

pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Arc<str>),
    BigInt(Box<i128>),
    Array(Arc<()>),
    Object(Arc<()>),
    Closure(Arc<()>),
    Class(Arc<()>),
    Instance(Arc<()>),
    BoundMethod(Arc<()>),
    NativeFn(Box<()>),
    NativeBoundMethod(Box<()>),
    Future(Arc<()>),
    TimerFuture(u64),
    Range(Box<RangeData>),
}

pub struct RangeData {
    pub start: i64,
    pub end: i64,
    pub inclusive: bool,
}

fn main() {
    println!("Size: {}", std::mem::size_of::<Value>());
}
