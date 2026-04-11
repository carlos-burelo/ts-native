use super::{Closure, RuntimeObject, RuntimeString, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

static NEXT_CLASS_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Debug, Clone)]
pub struct ClassObj {
    pub id: u32,
    pub name: String,
    pub is_native: bool,
    pub superclass: Option<Arc<ClassObj>>,
    pub vtable: Vec<Value>,
    pub method_map: HashMap<RuntimeString, usize>,
    pub statics: RuntimeObject,
    pub field_map: HashMap<RuntimeString, usize>,
    pub field_count: usize,
    pub getter_map: HashMap<RuntimeString, Arc<Closure>>,
    pub setter_map: HashMap<RuntimeString, Arc<Closure>>,
    pub static_getter_map: HashMap<RuntimeString, Arc<Closure>>,
    pub static_setter_map: HashMap<RuntimeString, Arc<Closure>>,
}

impl ClassObj {
    pub fn new(name: impl Into<String>) -> Self {
        ClassObj {
            id: NEXT_CLASS_ID.fetch_add(1, Ordering::Relaxed),
            name: name.into(),
            is_native: false,
            superclass: None,
            vtable: Vec::new(),
            method_map: HashMap::new(),
            statics: RuntimeObject::new(),
            field_map: HashMap::new(),
            field_count: 0,
            getter_map: HashMap::new(),
            setter_map: HashMap::new(),
            static_getter_map: HashMap::new(),
            static_setter_map: HashMap::new(),
        }
    }

    pub fn new_native(name: impl Into<String>) -> Self {
        let mut cls = Self::new(name);
        cls.is_native = true;
        cls
    }

    pub fn declare_field(&mut self, name: RuntimeString) -> usize {
        if let Some(&slot) = self.field_map.get(&name) { return slot; }
        let slot = self.field_count;
        self.field_map.insert(name, slot);
        self.field_count += 1;
        slot
    }

    pub fn add_method(&mut self, name: impl Into<Arc<str>>, value: Value) {
        let name: Arc<str> = name.into();
        if let Some(&idx) = self.method_map.get(&name) {
            self.vtable[idx] = value;
        } else {
            let idx = self.vtable.len();
            self.vtable.push(value);
            self.method_map.insert(name, idx);
        }
    }

    pub fn add_getter(&mut self, name: impl Into<Arc<str>>, closure: Arc<Closure>) {
        self.getter_map.insert(name.into(), closure);
    }

    pub fn add_setter(&mut self, name: impl Into<Arc<str>>, closure: Arc<Closure>) {
        self.setter_map.insert(name.into(), closure);
    }

    pub fn add_static_getter(&mut self, name: impl Into<Arc<str>>, closure: Arc<Closure>) {
        self.static_getter_map.insert(name.into(), closure);
    }

    pub fn add_static_setter(&mut self, name: impl Into<Arc<str>>, closure: Arc<Closure>) {
        self.static_setter_map.insert(name.into(), closure);
    }

    pub fn find_getter(&self, name: &str) -> Option<Arc<Closure>> {
        if let Some(g) = self.getter_map.get(name) { return Some(g.clone()); }
        self.superclass.as_ref()?.find_getter(name)
    }

    pub fn find_setter(&self, name: &str) -> Option<Arc<Closure>> {
        if let Some(s) = self.setter_map.get(name) { return Some(s.clone()); }
        self.superclass.as_ref()?.find_setter(name)
    }

    pub fn find_static_getter(&self, name: &str) -> Option<Arc<Closure>> {
        if let Some(g) = self.static_getter_map.get(name) { return Some(g.clone()); }
        self.superclass.as_ref()?.find_static_getter(name)
    }

    pub fn find_static_setter(&self, name: &str) -> Option<Arc<Closure>> {
        if let Some(s) = self.static_setter_map.get(name) { return Some(s.clone()); }
        self.superclass.as_ref()?.find_static_setter(name)
    }

    pub fn find_method(&self, name: &str) -> Option<Value> {
        if let Some(&idx) = self.method_map.get(name) {
            return Some(self.vtable[idx].clone());
        }
        if let Some(super_cls) = &self.superclass {
            return super_cls.find_method(name);
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct BoundMethod {
    pub receiver: Box<Value>,
    pub method: Arc<Closure>,
    pub owner_class: Option<Arc<ClassObj>>,
}

pub fn find_method_with_owner(class: &Arc<ClassObj>, name: &str) -> Option<(Value, Arc<ClassObj>)> {
    if let Some(&idx) = class.method_map.get(name) {
        return Some((class.vtable[idx].clone(), class.clone()));
    }
    if let Some(super_cls) = &class.superclass {
        return find_method_with_owner(super_cls, name);
    }
    None
}
