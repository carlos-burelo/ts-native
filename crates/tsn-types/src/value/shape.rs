use super::{RuntimeString, Value};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};

static NEXT_SHAPE_ID: AtomicU32 = AtomicU32::new(1);

pub struct Shape {
    pub id: u32,
    pub property_names: HashMap<RuntimeString, usize>,
    transitions: Mutex<HashMap<RuntimeString, Arc<Shape>>>,
}

impl std::fmt::Debug for Shape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

    pub fn len(&self) -> usize { self.values.len() }

    pub fn is_empty(&self) -> bool { self.values.is_empty() }

    pub fn keys(&self) -> std::vec::IntoIter<RuntimeString> {
        let mut pairs: Vec<(RuntimeString, usize)> = self
            .shape
            .property_names
            .iter()
            .map(|(k, &idx)| (k.clone(), idx))
            .collect();
        pairs.sort_unstable_by_key(|(_, idx)| *idx);
        pairs.into_iter().map(|(k, _)| k).collect::<Vec<_>>().into_iter()
    }

    pub fn iter(&self) -> std::vec::IntoIter<(RuntimeString, Value)> {
        let mut pairs: Vec<(RuntimeString, Value, usize)> = self
            .shape
            .property_names
            .iter()
            .map(|(k, &idx)| (k.clone(), self.values[idx].clone(), idx))
            .collect();
        pairs.sort_unstable_by_key(|(_, _, idx)| *idx);
        pairs.into_iter().map(|(k, v, _)| (k, v)).collect::<Vec<_>>().into_iter()
    }
}

impl PartialEq for RuntimeObject {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() { return false; }
        for (k, v) in self.iter() {
            if other.get(&k) != Some(&v) { return false; }
        }
        true
    }
}
impl Eq for RuntimeObject {}
