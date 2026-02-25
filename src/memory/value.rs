//! JVM value representation.

/// Represents a value in the JVM
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Boolean(bool),
    Byte(i8),
    Char(u16),
    Short(i16),
    Int(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    Reference(u32),
    ReturnAddress(u32),
    Null,
    ArrayRef(u32),
}

impl Value {
    pub fn as_int(&self) -> i32 {
        match self {
            Value::Int(i) => *i,
            Value::Float(f) => *f as i32,
            Value::Long(l) => *l as i32,
            Value::Double(d) => *d as i32,
            _ => 0,
        }
    }

    pub fn as_float(&self) -> f32 {
        match self {
            Value::Int(i) => *i as f32,
            Value::Float(f) => *f,
            Value::Long(l) => *l as f32,
            Value::Double(d) => *d as f32,
            _ => 0.0,
        }
    }

    pub fn as_long(&self) -> i64 {
        match self {
            Value::Long(l) => *l,
            Value::Int(i) => *i as i64,
            Value::Float(f) => *f as i64,
            Value::Double(d) => *d as i64,
            _ => 0,
        }
    }

    pub fn as_double(&self) -> f64 {
        match self {
            Value::Double(d) => *d,
            Value::Float(f) => *f as f64,
            Value::Int(i) => *i as f64,
            Value::Long(l) => *l as f64,
            _ => 0.0,
        }
    }

    pub fn is_reference(&self) -> bool {
        matches!(self, Value::Reference(_) | Value::ArrayRef(_))
    }

    pub fn as_reference(&self) -> Option<u32> {
        match self {
            Value::Reference(addr) | Value::ArrayRef(addr) => Some(*addr),
            _ => None,
        }
    }
}
