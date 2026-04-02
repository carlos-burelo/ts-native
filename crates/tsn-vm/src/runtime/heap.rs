use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use tsn_types::value::{
    install_allocator, AllocVtable, ArrayRef, MapRef, ObjData, ObjRef, SetRef, Value,
};

pub struct Heap {
    pub objects: Vec<ObjRef>,
    pub arrays: Vec<ArrayRef>,
    pub maps: Vec<MapRef>,
    pub sets: Vec<SetRef>,
}

impl Heap {
    pub fn new() -> Self {
        Heap {
            objects: Vec::new(),
            arrays: Vec::new(),
            maps: Vec::new(),
            sets: Vec::new(),
        }
    }

    pub fn alloc_object(&mut self) -> ObjRef {
        let ptr = Box::into_raw(Box::new(ObjData::new()));
        self.objects.push(ptr);
        ptr
    }

    pub fn alloc_array(&mut self) -> ArrayRef {
        let ptr = Box::into_raw(Box::new(Vec::<Value>::new()));
        self.arrays.push(ptr);
        ptr
    }

    pub fn alloc_map(&mut self) -> MapRef {
        let ptr = Box::into_raw(Box::new(HashMap::<Value, Value>::new()));
        self.maps.push(ptr);
        ptr
    }

    pub fn alloc_set(&mut self) -> SetRef {
        let ptr = Box::into_raw(Box::new(HashSet::<Value>::new()));
        self.sets.push(ptr);
        ptr
    }

    pub fn stats(&self) -> (usize, usize, usize, usize) {
        (
            self.objects.len(),
            self.arrays.len(),
            self.maps.len(),
            self.sets.len(),
        )
    }
}

impl Drop for Heap {
    fn drop(&mut self) {
        for p in self.sets.drain(..).rev() {
            unsafe {
                drop(Box::from_raw(p));
            }
        }
        for p in self.maps.drain(..).rev() {
            unsafe {
                drop(Box::from_raw(p));
            }
        }
        for p in self.arrays.drain(..).rev() {
            unsafe {
                drop(Box::from_raw(p));
            }
        }

        for p in self.objects.drain(..).rev() {
            unsafe {
                drop(Box::from_raw(p));
            }
        }
    }
}

thread_local! {
    static HEAP: RefCell<Heap> = RefCell::new(Heap::new());
}

fn do_alloc_object() -> ObjRef {
    HEAP.with(|h| h.borrow_mut().alloc_object())
}
fn do_alloc_array() -> ArrayRef {
    HEAP.with(|h| h.borrow_mut().alloc_array())
}
fn do_alloc_map() -> MapRef {
    HEAP.with(|h| h.borrow_mut().alloc_map())
}
fn do_alloc_set() -> SetRef {
    HEAP.with(|h| h.borrow_mut().alloc_set())
}

static VTABLE: AllocVtable = AllocVtable {
    alloc_object: do_alloc_object,
    alloc_array: do_alloc_array,
    alloc_map: do_alloc_map,
    alloc_set: do_alloc_set,
};

pub fn init_heap() {
    tsn_types::value::register_global_vtable(&VTABLE);
    install_allocator(&VTABLE);
}
