//! Annotation parsing from class file attributes.
//!
//! Parses RuntimeVisibleAnnotations, RuntimeInvisibleAnnotations,
//! and Signature (for generics) attributes.

use crate::class_file::{AttributeInfo, ClassFile, FieldInfo, MethodInfo};
use std::collections::HashMap;

/// Parsed annotation (simplified - type + element pairs)
#[derive(Debug, Clone)]
pub struct Annotation {
    pub type_name: String,
    pub elements: HashMap<String, ElementValue>,
}

/// Annotation element value
#[derive(Debug, Clone)]
pub enum ElementValue {
    ConstInt(i32),
    ConstLong(i64),
    ConstFloat(f32),
    ConstDouble(f64),
    ConstString(String),
    Enum(String, String),
    Class(String),
    Annotation(Annotation),
    Array(Vec<ElementValue>),
}

/// Find attribute by UTF8 name (lookup via constant pool)
fn find_attribute<'a>(class: &'a ClassFile, attrs: &'a [AttributeInfo], name: &str) -> Option<&'a AttributeInfo> {
    attrs.iter().find(|a| {
        class.get_string(a.attribute_name_index).as_deref() == Some(name)
    })
}

/// Parse annotations from RuntimeVisibleAnnotations or RuntimeInvisibleAnnotations attribute
pub fn parse_annotations(class: &ClassFile, attr: &AttributeInfo) -> Vec<Annotation> {
    let data = &attr.info;
    if data.len() < 2 {
        return vec![];
    }
    let num = ((data[0] as usize) << 8) | (data[1] as usize);
    let mut result = Vec::with_capacity(num);
    let mut pos = 2usize;
    for _ in 0..num {
        if pos + 2 > data.len() {
            break;
        }
        let type_index = ((data[pos] as u16) << 8) | (data[pos + 1] as u16);
        pos += 2;
        let type_name = class
            .get_string(type_index)
            .map(|s| s.replace('/', "."))
            .unwrap_or_default();
        // Skip element_value_pairs (num_pairs u16 + pairs)
        if pos + 2 > data.len() {
            break;
        }
        let num_pairs = ((data[pos] as usize) << 8) | (data[pos + 1] as usize);
        pos += 2;
        let mut elements = HashMap::new();
        for _ in 0..num_pairs {
            if pos + 4 > data.len() {
                break;
            }
            let name_index = ((data[pos] as u16) << 8) | (data[pos + 1] as u16);
            pos += 2;
            let name = class.get_string(name_index).unwrap_or_default();
            let ev = parse_element_value(class, data, &mut pos);
            elements.insert(name, ev);
        }
        result.push(Annotation { type_name, elements });
    }
    result
}

fn parse_element_value(class: &ClassFile, data: &[u8], pos: &mut usize) -> ElementValue {
    if *pos >= data.len() {
        return ElementValue::ConstInt(0);
    }
    let tag = data[*pos];
    *pos += 1;
    let read_u16 = |p: &mut usize| -> u16 {
        if *p + 2 <= data.len() {
            let v = ((data[*p] as u16) << 8) | (data[*p + 1] as u16);
            *p += 2;
            v
        } else {
            0
        }
    };
    match tag {
        b'B' | b'C' | b'I' | b'S' | b'Z' => {
            let _idx = read_u16(pos);
            ElementValue::ConstInt(0)
        }
        b's' => {
            let idx = read_u16(pos);
            ElementValue::ConstString(class.get_string(idx).unwrap_or_default())
        }
        b'[' => {
            let num = read_u16(pos) as usize;
            let mut arr = Vec::with_capacity(num);
            for _ in 0..num {
                arr.push(parse_element_value(class, data, pos));
            }
            ElementValue::Array(arr)
        }
        _ => {
            let _ = read_u16(pos);
            ElementValue::ConstInt(0)
        }
    }
}

/// Get annotations for a class, method, or field
pub fn get_annotations(class: &ClassFile, attrs: &[AttributeInfo]) -> Vec<Annotation> {
    let attr = find_attribute(class, attrs, "RuntimeVisibleAnnotations")
        .or_else(|| find_attribute(class, attrs, "RuntimeInvisibleAnnotations"));
    attr.map(|a| parse_annotations(class, a)).unwrap_or_default()
}

/// Get Signature attribute (for generics / type erasure)
pub fn get_signature(class: &ClassFile, attrs: &[AttributeInfo]) -> Option<String> {
    let attr = find_attribute(class, attrs, "Signature")?;
    if attr.info.len() >= 2 {
        let idx = ((attr.info[0] as u16) << 8) | (attr.info[1] as u16);
        class.get_string(idx)
    } else {
        None
    }
}

impl ClassFile {
    /// Get class-level annotations
    pub fn get_class_annotations(&self) -> Vec<Annotation> {
        get_annotations(self, &self.attributes)
    }

    /// Get class signature (generics)
    pub fn get_class_signature(&self) -> Option<String> {
        get_signature(self, &self.attributes)
    }
}

impl MethodInfo {
    /// Get method-level annotations
    pub fn get_annotations(&self, class: &ClassFile) -> Vec<Annotation> {
        get_annotations(class, &self.attributes)
    }

    /// Get method signature (generics)
    pub fn get_signature_attr(&self, class: &ClassFile) -> Option<String> {
        get_signature(class, &self.attributes)
    }
}

impl FieldInfo {
    /// Get field-level annotations
    pub fn get_annotations(&self, class: &ClassFile) -> Vec<Annotation> {
        get_annotations(class, &self.attributes)
    }
}
