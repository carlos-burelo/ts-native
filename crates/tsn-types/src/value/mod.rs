mod alloc;
mod class;
mod closure;
mod constructors;
mod future;
mod object;
mod shape;
mod traits;

use crate::future::AsyncFuture;
pub use crate::native::NativeFn;
use std::sync::Arc;

pub use alloc::{
    alloc_array, alloc_map, alloc_object, alloc_set, get_global_vtable, init_thread_heap,
    install_allocator, register_global_vtable, AllocVtable, ArrayRef, MapRef, ObjRef,
    RuntimeString, SetRef,
};
pub type RuntimeArray = Vec<Value>;
pub use class::{find_method_with_owner, BoundMethod, ClassObj};
pub use closure::{Closure, Upvalue, UpvalueInner};
pub use constructors::{new_array, new_object};
pub use future::{reject_future, reject_value_future, resolve_future, FutureState, Poll};
pub use object::ObjData;
pub use shape::{root_shape, RuntimeObject, Shape};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeData {
    pub start: i64,
    pub end: i64,
    pub inclusive: bool,
}

impl std::hash::Hash for RangeData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.start.hash(state);
        self.end.hash(state);
        self.inclusive.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SymbolKind {
    Iterator,
    AsyncIterator,
}

impl SymbolKind {
    pub fn name(&self) -> &'static str {
        match self {
            SymbolKind::Iterator => "Symbol.iterator",
            SymbolKind::AsyncIterator => "Symbol.asyncIterator",
        }
    }
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(RuntimeString),
    BigInt(Box<i128>),
    Decimal(Box<rust_decimal::Decimal>),
    Array(ArrayRef),
    Object(ObjRef),
    Closure(Arc<Closure>),
    Class(Arc<ClassObj>),
    BoundMethod(Arc<BoundMethod>),
    NativeFn(Box<(NativeFn, &'static str)>),
    NativeBoundMethod(Box<(Value, NativeFn, &'static str)>),
    Spread(Box<Value>),
    Future(AsyncFuture),
    Range(Box<RangeData>),
    Map(MapRef),
    Set(SetRef),
    Symbol(SymbolKind),
    Generator(crate::generator::GeneratorObj),
    AsyncQueue(crate::generator::AsyncQueue),
    Char(char),
}

unsafe impl Send for Value {}
unsafe impl Sync for Value {}
