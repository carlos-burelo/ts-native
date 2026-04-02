use std::sync::Arc;
use tsn_types::chunk::CacheEntry;
use tsn_types::value::{ClassObj, Closure};

#[derive(Clone)]
pub(crate) struct TryEntry {
    pub(crate) catch_ip: usize,
    pub(crate) frame_depth: usize,
    pub(crate) stack_depth: usize,
}

pub(crate) struct CallFrame {
    pub(crate) closure: Arc<Closure>,
    pub(crate) ip: usize,
    pub(crate) base: usize,
    pub(crate) current_class: Option<Arc<ClassObj>>,
    pub(crate) ic_slots: Vec<CacheEntry>,
}
