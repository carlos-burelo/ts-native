use super::{ClassObj, RuntimeObject, RuntimeString, Value};
use std::sync::Arc;

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

    pub fn is_instance(&self) -> bool { self.class.is_some() }

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
