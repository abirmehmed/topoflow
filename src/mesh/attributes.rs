//! Per-element attributes for mesh data
//! 
//! Supports scalar, vector, and custom attributes on vertices, edges, faces

use std::any::Any;
use std::collections::HashMap;

/// Attribute storage type
#[derive(Debug, Clone)]
pub enum AttributeValue {
    Scalar(f32),
    Vector2([f32; 2]),
    Vector3([f32; 3]),
    Vector4([f32; 4]),
    Integer(i32),
    Boolean(bool),
}

/// Attribute container for mesh elements
#[derive(Debug, Default)]
pub struct AttributeStore {
    data: HashMap<String, Vec<AttributeValue>>,
}

impl AttributeStore {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn add_attribute(&mut self, name: &str, values: Vec<AttributeValue>) {
        self.data.insert(name.to_string(), values);
    }

    pub fn get(&self, name: &str, index: usize) -> Option<&AttributeValue> {
        self.data.get(name)?.get(index)
    }

    pub fn set(&mut self, name: &str, index: usize, value: AttributeValue) {
        if let Some(vec) = self.data.get_mut(name) {
            if index < vec.len() {
                vec[index] = value;
            }
        }
    }
}
