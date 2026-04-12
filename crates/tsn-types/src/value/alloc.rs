use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

pub type RuntimeString = Arc<str>;
pub type ObjRef = *mut super::ObjData;
pub type ArrayRef = *mut Vec<super::Value>;
pub type MapRef = *mut HashMap<super::Value, super::Value>;
pub type SetRef = *mut HashSet<super::Value>;

pub struct AllocVtable {
    pub alloc_object: fn() -> ObjRef,
    pub alloc_array: fn() -> ArrayRef,
    pub alloc_map: fn() -> MapRef,
    pub alloc_set: fn() -> SetRef,
}

static GLOBAL_VTABLE: OnceLock<&'static AllocVtable> = OnceLock::new();

pub fn register_global_vtable(v: &'static AllocVtable) {
    let _ = GLOBAL_VTABLE.set(v);
}

pub fn init_thread_heap() {
    if let Some(v) = GLOBAL_VTABLE.get() {
        install_allocator(v);
    }
}

pub fn get_global_vtable() -> Option<&'static AllocVtable> {
    GLOBAL_VTABLE.get().copied()
}

fn _stub_panic(what: &str) -> ! {
    panic!(
        "TSN heap not initialized — heap::init_heap() must be called before allocating a {}",
        what
    )
}
fn _stub_obj() -> ObjRef {
    _stub_panic("Object")
}
fn _stub_arr() -> ArrayRef {
    _stub_panic("Array")
}
fn _stub_map() -> MapRef {
    _stub_panic("Map")
}
fn _stub_set() -> SetRef {
    _stub_panic("Set")
}

thread_local! {
    static TL_VTABLE: Cell<*const AllocVtable> = Cell::new(std::ptr::null());
}

pub fn install_allocator(v: &'static AllocVtable) {
    TL_VTABLE.with(|c| c.set(v as *const AllocVtable));
}

#[inline(always)]
fn get_vtable() -> &'static AllocVtable {
    let ptr = TL_VTABLE.with(|c| c.get());
    if ptr.is_null() {
        static STUBS: AllocVtable = AllocVtable {
            alloc_object: _stub_obj,
            alloc_array: _stub_arr,
            alloc_map: _stub_map,
            alloc_set: _stub_set,
        };
        return &STUBS;
    }
    unsafe { &*ptr }
}

#[inline(always)]
pub fn alloc_object() -> ObjRef {
    (get_vtable().alloc_object)()
}

#[inline(always)]
pub fn alloc_array() -> ArrayRef {
    (get_vtable().alloc_array)()
}

#[inline(always)]
pub fn alloc_map() -> MapRef {
    (get_vtable().alloc_map)()
}

#[inline(always)]
pub fn alloc_set() -> SetRef {
    (get_vtable().alloc_set)()
}
