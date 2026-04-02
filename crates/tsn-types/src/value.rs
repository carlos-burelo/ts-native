use crate::chunk::FunctionProto;
use crate::future::AsyncFuture;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::{Arc, OnceLock};

static GLOBAL_VTABLE: OnceLock<&'static AllocVtable> = OnceLock::new();

pub fn register_global_vtable(v: &'static AllocVtable) {
    let _ = GLOBAL_VTABLE.set(v);
}

pub fn init_thread_heap() {
    if let Some(v) = GLOBAL_VTABLE.get() {
        install_allocator(v);
    }
}
pub type RuntimeString = Arc<str>;
pub type RuntimeArray = Vec<Value>;
pub type ObjRef = *mut ObjData;
pub type ArrayRef = *mut Vec<Value>;
pub type MapRef = *mut HashMap<Value, Value>;
pub type SetRef = *mut HashSet<Value>;
pub struct AllocVtable {
    pub alloc_object: fn() -> ObjRef,
    pub alloc_array: fn() -> ArrayRef,
    pub alloc_map: fn() -> MapRef,
    pub alloc_set: fn() -> SetRef,
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

use parking_lot::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_SHAPE_ID: AtomicU32 = AtomicU32::new(1);
static NEXT_CLASS_ID: AtomicU32 = AtomicU32::new(1);

pub struct Shape {
    pub id: u32,
    pub property_names: HashMap<RuntimeString, usize>,
    transitions: Mutex<HashMap<RuntimeString, Arc<Shape>>>,
}

impl fmt::Debug for Shape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Shape")
            .field("id", &self.id)
            .field("property_names", &self.property_names)
            .finish()
    }
}

impl Shape {
    fn create(property_names: HashMap<RuntimeString, usize>) -> Arc<Self> {
        Arc::new(Shape {
            id: NEXT_SHAPE_ID.fetch_add(1, Ordering::Relaxed),
            property_names,
            transitions: Mutex::new(HashMap::new()),
        })
    }

    fn create_root() -> Arc<Self> {
        Shape::create(HashMap::new())
    }

    pub fn transition(&self, key: RuntimeString) -> Arc<Shape> {
        let mut trans = self.transitions.lock();
        if let Some(child) = trans.get(&key) {
            return Arc::clone(child);
        }
        let mut new_props = self.property_names.clone();
        let slot = new_props.len();
        new_props.insert(Arc::clone(&key), slot);
        let child = Shape::create(new_props);
        trans.insert(key, Arc::clone(&child));
        child
    }
}

static ROOT_SHAPE: OnceLock<Arc<Shape>> = OnceLock::new();

pub fn root_shape() -> Arc<Shape> {
    Arc::clone(ROOT_SHAPE.get_or_init(Shape::create_root))
}

#[derive(Debug, Clone)]
pub struct RuntimeObject {
    pub shape: Arc<Shape>,

    pub values: Vec<Value>,
}

impl RuntimeObject {
    pub fn new() -> Self {
        Self {
            shape: root_shape(),
            values: Vec::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        let idx = self.shape.property_names.get(name).copied()?;
        self.values.get(idx)
    }

    pub fn insert(&mut self, name: RuntimeString, value: Value) {
        if let Some(&idx) = self.shape.property_names.get(&name) {
            self.values[idx] = value;
        } else {
            self.shape = self.shape.transition(Arc::clone(&name));
            self.values.push(value);
        }
    }

    pub fn remove(&mut self, name: &str) -> Option<Value> {
        let removed_slot = *self.shape.property_names.get(name)?;
        let removed_val = self.values[removed_slot].clone();

        let mut remaining: Vec<(RuntimeString, Value)> = self
            .shape
            .property_names
            .iter()
            .filter(|(k, _)| k.as_ref() != name)
            .map(|(k, &slot)| (Arc::clone(k), self.values[slot].clone()))
            .collect();
        remaining.sort_by_key(|(k, _)| self.shape.property_names[k]);

        let mut new_shape = root_shape();
        let mut new_values = Vec::with_capacity(remaining.len());
        for (k, v) in remaining {
            new_shape = new_shape.transition(Arc::clone(&k));
            new_values.push(v);
        }
        self.shape = new_shape;
        self.values = new_values;
        Some(removed_val)
    }

    pub fn contains_key(&self, name: &str) -> bool {
        self.shape.property_names.contains_key(name)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn keys(&self) -> std::vec::IntoIter<RuntimeString> {
        let mut pairs: Vec<(RuntimeString, usize)> = self
            .shape
            .property_names
            .iter()
            .map(|(k, &idx)| (k.clone(), idx))
            .collect();
        pairs.sort_unstable_by_key(|(_, idx)| *idx);
        pairs
            .into_iter()
            .map(|(k, _)| k)
            .collect::<Vec<_>>()
            .into_iter()
    }

    pub fn iter(&self) -> std::vec::IntoIter<(RuntimeString, Value)> {
        let mut pairs: Vec<(RuntimeString, Value, usize)> = self
            .shape
            .property_names
            .iter()
            .map(|(k, &idx)| (k.clone(), self.values[idx].clone(), idx))
            .collect();
        pairs.sort_unstable_by_key(|(_, _, idx)| *idx);
        pairs
            .into_iter()
            .map(|(k, v, _)| (k, v))
            .collect::<Vec<_>>()
            .into_iter()
    }
}

impl PartialEq for RuntimeObject {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        for (k, v) in self.iter() {
            if other.get(&k) != Some(&v) {
                return false;
            }
        }
        true
    }
}
impl Eq for RuntimeObject {}

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
        if let Some(&slot) = self.field_map.get(&name) {
            return slot;
        }
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
        if let Some(g) = self.getter_map.get(name) {
            return Some(g.clone());
        }
        self.superclass.as_ref()?.find_getter(name)
    }

    pub fn find_setter(&self, name: &str) -> Option<Arc<Closure>> {
        if let Some(s) = self.setter_map.get(name) {
            return Some(s.clone());
        }
        self.superclass.as_ref()?.find_setter(name)
    }

    pub fn find_static_getter(&self, name: &str) -> Option<Arc<Closure>> {
        if let Some(g) = self.static_getter_map.get(name) {
            return Some(g.clone());
        }
        self.superclass.as_ref()?.find_static_getter(name)
    }

    pub fn find_static_setter(&self, name: &str) -> Option<Arc<Closure>> {
        if let Some(s) = self.static_setter_map.get(name) {
            return Some(s.clone());
        }
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
pub struct ObjData {
    pub class: Option<Arc<ClassObj>>,
    pub slots: Vec<Value>,
    pub fields: RuntimeObject,
}

impl ObjData {
    pub fn new() -> Self {
        ObjData {
            class: None,
            slots: Vec::new(),
            fields: RuntimeObject::new(),
        }
    }

    pub fn new_instance(class: Arc<ClassObj>) -> Self {
        let field_count = class.field_count;
        ObjData {
            class: Some(class),
            slots: vec![Value::Null; field_count],
            fields: RuntimeObject::new(),
        }
    }

    pub fn is_instance(&self) -> bool {
        self.class.is_some()
    }

    pub fn class_name(&self) -> String {
        match &self.class {
            Some(c) => c.name.clone(),
            None => "object".to_owned(),
        }
    }

    pub fn get_field(&self, key: &str) -> Option<Value> {
        if let Some(cls) = &self.class {
            if let Some(&slot) = cls.field_map.get(key) {
                return self.slots.get(slot).cloned();
            }
        }
        self.fields.get(key).cloned()
    }

    pub fn set_field(&mut self, key: RuntimeString, value: Value) {
        if let Some(cls) = &self.class {
            let slot = cls.field_map.get(&key).copied();
            if let Some(slot) = slot {
                if slot < self.slots.len() {
                    self.slots[slot] = value;
                    return;
                }
            }
        }
        self.fields.insert(key, value);
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

pub use crate::future::FutureState;

use crate::native::NativeFn;

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

impl fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Value::Null => {}
            Value::Bool(b) => b.hash(state),
            Value::Int(n) => n.hash(state),
            Value::Float(f) => f.to_bits().hash(state),
            Value::Str(s) => s.hash(state),
            Value::BigInt(n) => n.hash(state),
            Value::Decimal(d) => d.hash(state),
            Value::Array(a) => (*a as usize).hash(state),
            Value::Object(o) => (*o as usize).hash(state),
            Value::Closure(c) => Arc::as_ptr(c).hash(state),
            Value::Class(c) => Arc::as_ptr(c).hash(state),
            Value::BoundMethod(m) => Arc::as_ptr(m).hash(state),
            Value::NativeFn(b) => (b.0 as usize).hash(state),
            Value::NativeBoundMethod(b) => {
                b.0.hash(state);
                (b.1 as usize).hash(state);
            }
            Value::Spread(v) => v.hash(state),
            Value::Future(f) => f.hash(state),
            Value::Range(r) => r.hash(state),
            Value::Map(m) => (*m as usize).hash(state),
            Value::Set(s) => (*s as usize).hash(state),
            Value::Symbol(s) => s.hash(state),
            Value::Generator(g) => g.hash(state),
            Value::AsyncQueue(q) => Arc::as_ptr(&q.0).hash(state),
            Value::Char(c) => c.hash(state),
        }
    }
}

impl Value {
    #[inline(always)]
    pub fn native(func: NativeFn, name: &'static str) -> Self {
        Value::NativeFn(Box::new((func, name)))
    }

    #[inline(always)]
    pub fn native_bound(receiver: Value, func: NativeFn, name: &'static str) -> Self {
        Value::NativeBoundMethod(Box::new((receiver, func, name)))
    }

    pub fn instance(class: Arc<ClassObj>) -> Self {
        let ptr = alloc_object();

        unsafe {
            *ptr = ObjData::new_instance(class);
        }
        Value::Object(ptr)
    }

    pub fn plain_object() -> Self {
        Value::Object(alloc_object())
    }

    #[inline(always)]
    pub fn empty_array() -> Self {
        Value::Array(alloc_array())
    }

    pub fn is_truthy(&self) -> Result<bool, String> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err(format!(
                "expected bool for condition, got {}",
                self.type_name()
            )),
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "str",
            Value::BigInt(_) => "bigint",
            Value::Decimal(_) => "decimal",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
            Value::Closure(_) => "fn",
            Value::Class(_) => "class",
            Value::BoundMethod(_) => "fn",
            Value::NativeFn(_) => "fn",
            Value::NativeBoundMethod(_) => "fn",
            Value::Spread(v) => v.type_name(),
            Value::Future(_) => "Future",
            Value::Range(_) => "range",
            Value::Map(_) => "Map",
            Value::Set(_) => "Set",
            Value::Symbol(_) => "symbol",
            Value::Generator(_) => "generator",
            Value::AsyncQueue(_) => "asyncqueue",
            Value::Char(_) => "char",
        }
    }

    pub fn num_add(&self, rhs: &Value) -> Result<Value, String> {
        match (self, rhs) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
            (Value::Decimal(a), Value::Decimal(b)) => Ok(Value::Decimal(Box::new(**a + **b))),
            (Value::Decimal(a), Value::Int(b)) => Ok(Value::Decimal(Box::new(
                **a + rust_decimal::Decimal::from(*b),
            ))),
            (Value::Int(a), Value::Decimal(b)) => Ok(Value::Decimal(Box::new(
                rust_decimal::Decimal::from(*a) + **b,
            ))),
            (Value::Str(a), Value::Str(b)) => {
                let mut s = String::with_capacity(a.len() + b.len());
                s.push_str(a);
                s.push_str(b);
                Ok(Value::Str(Arc::from(s)))
            }
            (Value::Str(a), other) => {
                let mut s = a.to_string();
                s.push_str(&other.to_string());
                Ok(Value::Str(Arc::from(s)))
            }
            (other, Value::Str(b)) => {
                let mut s = other.to_string();
                s.push_str(b);
                Ok(Value::Str(Arc::from(s)))
            }
            _ => Err(format!(
                "cannot add {} + {}",
                self.type_name(),
                rhs.type_name()
            )),
        }
    }
}

#[inline(always)]
pub fn new_array(v: Vec<Value>) -> Value {
    let ptr = alloc_array();
    unsafe {
        *ptr = v;
    }
    Value::Array(ptr)
}

#[inline(always)]
pub fn new_object(obj: ObjData) -> Value {
    let ptr = alloc_object();
    unsafe {
        *ptr = obj;
    }
    Value::Object(ptr)
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) => (*a as f64) == *b,
            (Value::Float(a), Value::Int(b)) => *a == (*b as f64),
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::BigInt(a), Value::BigInt(b)) => a == b,
            (Value::Decimal(a), Value::Decimal(b)) => a == b,
            (Value::Range(a), Value::Range(b)) => a == b,
            (Value::Future(a), Value::Future(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => *a == *b,
            (Value::Set(a), Value::Set(b)) => *a == *b,
            (Value::Array(a), Value::Array(b)) => *a == *b,
            (Value::Object(a), Value::Object(b)) => *a == *b,
            (Value::Closure(a), Value::Closure(b)) => Arc::ptr_eq(a, b),
            (Value::Class(a), Value::Class(b)) => Arc::ptr_eq(a, b),
            (Value::Symbol(a), Value::Symbol(b)) => a == b,
            (Value::Generator(a), Value::Generator(b)) => a == b,
            (Value::AsyncQueue(a), Value::AsyncQueue(b)) => Arc::ptr_eq(&a.0, &b.0),
            (Value::Char(a), Value::Char(b)) => a == b,
            _ => false,
        }
    }
}
impl Eq for Value {}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Null, Value::Null) => Some(std::cmp::Ordering::Equal),
            (Value::Bool(a), Value::Bool(b)) => a.partial_cmp(b),
            (Value::Int(a), Value::Int(b)) => a.partial_cmp(b),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b),
            (Value::Int(a), Value::Float(b)) => (*a as f64).partial_cmp(b),
            (Value::Float(a), Value::Int(b)) => a.partial_cmp(&(*b as f64)),
            (Value::Str(a), Value::Str(b)) => a.partial_cmp(b),
            (Value::Char(a), Value::Char(b)) => a.partial_cmp(b),
            (Value::BigInt(a), Value::BigInt(b)) => a.partial_cmp(b),
            (Value::Decimal(a), Value::Decimal(b)) => a.partial_cmp(b),
            _ => {
                if self == other {
                    Some(std::cmp::Ordering::Equal)
                } else {
                    None
                }
            }
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(d) => {
                if d.fract() == 0.0 && d.abs() < 9_007_199_254_740_992.0 {
                    write!(f, "{}", *d as i64)
                } else {
                    write!(f, "{}", d)
                }
            }
            Value::Str(s) => write!(f, "{}", s),
            Value::BigInt(n) => write!(f, "{}n", n),
            Value::Decimal(d) => write!(f, "{}", d),
            Value::Array(ptr) => {
                let v = unsafe { &**ptr };
                write!(f, "[")?;
                for (i, val) in v.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", val)?;
                }
                write!(f, "]")
            }
            Value::Object(ptr) => {
                let obj = unsafe { &**ptr };
                if let Some(class) = &obj.class {
                    write!(f, "[object {}]", class.name)
                } else {
                    write!(f, "{{ ")?;
                    for (i, (k, v)) in obj.fields.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}: {}", k, v)?;
                    }
                    write!(f, " }}")
                }
            }
            Value::Closure(c) => {
                let name = c.proto.name.as_deref().unwrap_or("<fn>");
                write!(f, "[Function: {}]", name)
            }
            Value::Class(c) => write!(f, "[class {}]", c.name),
            Value::BoundMethod(m) => {
                let name = m.method.proto.name.as_deref().unwrap_or("<method>");
                write!(f, "[BoundMethod: {}]", name)
            }
            Value::NativeFn(b) => write!(f, "[NativeFn: {}]", b.1),
            Value::NativeBoundMethod(b) => write!(f, "[Function: {}]", b.2),
            Value::Spread(v) => write!(f, "{}", v),
            Value::Future(fut) => match fut.peek_state() {
                FutureState::Pending => write!(f, "Future {{ <pending> }}"),
                FutureState::Resolved(v) => write!(f, "Future {{ {} }}", v),
                FutureState::Rejected(v) => write!(f, "Future {{ <rejected: {}> }}", v),
            },
            Value::Range(r) => {
                if r.inclusive {
                    write!(f, "{}..={}", r.start, r.end)
                } else {
                    write!(f, "{}..{}", r.start, r.end)
                }
            }
            Value::Map(_) => write!(f, "[Map]"),
            Value::Set(_) => write!(f, "[Set]"),
            Value::Symbol(s) => write!(f, "{}", s),
            Value::Generator(_) => write!(f, "[Generator]"),
            Value::AsyncQueue(_) => write!(f, "[AsyncQueue]"),
            Value::Char(c) => write!(f, "'{}'", c),
        }
    }
}

pub fn resolve_future(v: Value) -> Value {
    crate::future::resolve_future(v)
}

pub fn reject_future(msg: String) -> Value {
    crate::future::reject_future(msg)
}

pub fn reject_value_future(v: Value) -> Value {
    crate::future::reject_value_future(v)
}
