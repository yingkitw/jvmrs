//! Heap object and array types.

use std::collections::HashMap;

use super::monitor::Monitor;
use super::Value;

/// Represents an object on the heap
#[derive(Debug, Clone)]
pub struct HeapObject {
    pub fields: HashMap<String, Value>,
    pub class_name: String,
    pub marked: bool,
    pub string_data: Option<String>,
    pub monitor: Option<Monitor>,
}

/// Represents an array on the heap
#[derive(Debug, Clone)]
pub enum HeapArray {
    IntArray(Vec<i32>),
    FloatArray(Vec<f32>),
    LongArray(Vec<i64>),
    DoubleArray(Vec<f64>),
    ReferenceArray(Vec<u32>),
    ByteArray(Vec<u8>),
    CharArray(Vec<u16>),
    ShortArray(Vec<i16>),
    BooleanArray(Vec<bool>),
}
