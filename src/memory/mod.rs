//! JVM memory: heap, stack frames, values.
//!
//! Submodules:
//! - `value` - JVM value representation
//! - `frame` - Stack frame
//! - `monitor` - Object synchronization
//! - `heap_object` - HeapObject, HeapArray
//! - `heap` - Heap allocation
//! - `stack` - JVMStack

mod value;
mod frame;
mod monitor;
mod heap_object;
mod heap;
mod stack;

use std::collections::HashMap;
use std::sync::Arc;

use crate::debug::JvmDebugger;
use crate::security::Sanitizer;

pub use value::Value;
pub use frame::StackFrame;
pub use monitor::Monitor;
pub use heap_object::{HeapArray, HeapObject};
pub use heap::Heap;
pub use stack::JVMStack;

/// JVM runtime memory area
#[derive(Debug)]
pub struct Memory {
    pub stack: JVMStack,
    pub heap: Heap,
    pub static_fields: HashMap<String, HashMap<String, Value>>,
}

impl Memory {
    pub fn new() -> Self {
        Memory {
            stack: JVMStack::new(),
            heap: Heap::new(),
            static_fields: HashMap::new(),
        }
    }

    pub fn with_debugger(debugger: JvmDebugger) -> Self {
        Memory {
            stack: JVMStack::new(),
            heap: Heap::with_debugger(debugger),
            static_fields: HashMap::new(),
        }
    }

    pub fn set_debugger(&mut self, debugger: JvmDebugger) {
        self.heap = Heap::with_debugger(debugger);
    }

    pub fn set_sanitizer(&mut self, sanitizer: Option<Arc<Sanitizer>>) {
        self.heap.set_sanitizer(sanitizer);
    }

    pub fn get_static(&self, class_name: &str, field_name: &str) -> Option<&Value> {
        self.static_fields
            .get(class_name)
            .and_then(|fields| fields.get(field_name))
    }

    pub fn set_static(&mut self, class_name: String, field_name: String, value: Value) {
        self.static_fields
            .entry(class_name)
            .or_insert_with(HashMap::new)
            .insert(field_name, value);
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}
