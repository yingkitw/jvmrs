//! Core types for embedded/minimal dependency targets.
//!
//! When `no_std` feature is enabled, provides portable value types.
//! For full no_std builds, extract to a separate crate with alloc.

#![cfg(feature = "no_std")]

use core::fmt;

/// Minimal value type for embedded/portable use
#[derive(Clone, PartialEq)]
pub enum CoreValue {
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Reference(u32),
    Null,
}

impl CoreValue {
    pub fn as_int(&self) -> i32 {
        match self {
            CoreValue::Int(i) => *i,
            CoreValue::Long(l) => *l as i32,
            CoreValue::Float(f) => *f as i32,
            CoreValue::Double(d) => *d as i32,
            _ => 0,
        }
    }

    pub fn as_long(&self) -> i64 {
        match self {
            CoreValue::Long(l) => *l,
            CoreValue::Int(i) => *i as i64,
            _ => 0,
        }
    }
}

impl fmt::Debug for CoreValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreValue::Int(i) => write!(f, "Int({})", i),
            CoreValue::Long(l) => write!(f, "Long({})", l),
            CoreValue::Float(x) => write!(f, "Float({})", x),
            CoreValue::Double(x) => write!(f, "Double({})", x),
            CoreValue::Reference(r) => write!(f, "Ref({})", r),
            CoreValue::Null => write!(f, "Null"),
        }
    }
}
