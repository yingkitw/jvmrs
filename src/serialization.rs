//! Serialization/deserialization support (java.io.Serializable).
//!
//! Stub implementation for ObjectInputStream/ObjectOutputStream compatibility.

use crate::memory::{HeapObject, Value};

/// Check if a class is serializable (has java/io/Serializable interface)
pub fn is_serializable(_class_name: &str) -> bool {
    // Would check implemented interfaces
    false
}

/// Serialize an object to bytes (stub)
pub fn serialize_object(_obj: &HeapObject) -> Result<Vec<u8>, String> {
    Err("Serialization not yet implemented".to_string())
}

/// Deserialize bytes to object (stub)
pub fn deserialize_object(
    _data: &[u8],
    _class_name: &str,
) -> Result<HeapObject, String> {
    Err("Deserialization not yet implemented".to_string())
}

/// Serialize a Value
pub fn serialize_value(_v: &Value) -> Result<Vec<u8>, String> {
    Err("Value serialization not yet implemented".to_string())
}
