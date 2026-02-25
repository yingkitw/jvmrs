//! JVM heap for object and array allocation.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::debug::JvmDebugger;
use crate::error::MemoryError;
use crate::security::Sanitizer;

use super::heap_object::{HeapArray, HeapObject};
use super::monitor::Monitor;
use super::Value;

/// JVM heap for object allocation
#[derive(Debug)]
pub struct Heap {
    objects: HashMap<u32, HeapObject>,
    arrays: HashMap<u32, HeapArray>,
    next_addr: u32,
    marked: HashSet<u32>,
    debugger: Option<JvmDebugger>,
    sanitizer: Option<Arc<Sanitizer>>,
}

impl Heap {
    pub fn new() -> Self {
        Heap {
            objects: HashMap::new(),
            arrays: HashMap::new(),
            next_addr: 1,
            marked: HashSet::new(),
            debugger: None,
            sanitizer: None,
        }
    }

    pub fn with_debugger(debugger: JvmDebugger) -> Self {
        Heap {
            objects: HashMap::new(),
            arrays: HashMap::new(),
            next_addr: 1,
            marked: HashSet::new(),
            debugger: Some(debugger),
            sanitizer: None,
        }
    }

    pub fn set_sanitizer(&mut self, sanitizer: Option<Arc<Sanitizer>>) {
        self.sanitizer = sanitizer;
    }

    pub fn allocate(&mut self, class_name: String) -> u32 {
        let addr = self.next_addr;
        self.next_addr += 1;

        if let Some(debugger) = &self.debugger {
            debugger.log_memory_allocation(addr, 0, &class_name);
        }

        self.objects.insert(
            addr,
            HeapObject {
                fields: HashMap::new(),
                class_name,
                marked: false,
                string_data: None,
                monitor: None,
            },
        );
        addr
    }

    pub fn allocate_string(&mut self, data: String) -> u32 {
        let addr = self.next_addr;
        self.next_addr += 1;

        if let Some(debugger) = &self.debugger {
            debugger.log_memory_allocation(addr, data.len(), "java/lang/String");
        }

        self.objects.insert(
            addr,
            HeapObject {
                fields: HashMap::new(),
                class_name: "java/lang/String".to_string(),
                marked: false,
                string_data: Some(data),
                monitor: None,
            },
        );
        addr
    }

    pub fn allocate_array(&mut self, array_type: HeapArray) -> u32 {
        let addr = self.next_addr;
        self.next_addr += 1;
        self.arrays.insert(addr, array_type);
        addr
    }

    pub fn get_object(&self, addr: u32) -> Option<&HeapObject> {
        self.objects.get(&addr)
    }

    pub fn get_object_mut(&mut self, addr: u32) -> Option<&mut HeapObject> {
        self.objects.get_mut(&addr)
    }

    pub fn get_string_data(&self, addr: u32) -> Option<&String> {
        self.objects.get(&addr).and_then(|obj| {
            if obj.class_name == "java/lang/String" {
                obj.string_data.as_ref()
            } else {
                None
            }
        })
    }

    pub fn is_string(&self, addr: u32) -> bool {
        self.objects
            .get(&addr)
            .map(|obj| obj.class_name == "java/lang/String")
            .unwrap_or(false)
    }

    pub fn get_array(&self, addr: u32) -> Option<&HeapArray> {
        self.arrays.get(&addr)
    }

    pub fn get_array_mut(&mut self, addr: u32) -> Option<&mut HeapArray> {
        self.arrays.get_mut(&addr)
    }

    pub fn mark(&mut self, addr: u32) {
        if let Some(obj) = self.objects.get_mut(&addr) {
            obj.marked = true;
            self.marked.insert(addr);
        }
    }

    pub fn unmark_all(&mut self) {
        for obj in self.objects.values_mut() {
            obj.marked = false;
        }
        self.marked.clear();
    }

    pub fn collect_garbage(&mut self, roots: &[u32]) -> Result<usize, MemoryError> {
        self.unmark_all();
        let mut to_mark: Vec<u32> = roots.to_vec();

        while let Some(addr) = to_mark.pop() {
            if self.marked.contains(&addr) {
                continue;
            }

            self.mark(addr);

            if let Some(obj) = self.objects.get(&addr) {
                for value in obj.fields.values() {
                    if let Some(ref_addr) = value.as_reference() {
                        if !self.marked.contains(&ref_addr) {
                            to_mark.push(ref_addr);
                        }
                    }
                }
            }

            if let Some(array) = self.arrays.get(&addr) {
                if let HeapArray::ReferenceArray(refs) = array {
                    for ref_addr in refs {
                        if !self.marked.contains(ref_addr) {
                            to_mark.push(*ref_addr);
                        }
                    }
                }
            }
        }

        let mut freed = 0;
        let objects_to_remove: Vec<_> = self
            .objects
            .iter()
            .filter(|(_, obj)| !obj.marked)
            .map(|(addr, _)| *addr)
            .collect();
        let arrays_to_remove: Vec<_> = self
            .arrays
            .keys()
            .filter(|addr| !self.marked.contains(addr))
            .cloned()
            .collect();

        for addr in objects_to_remove {
            self.objects.remove(&addr);
            freed += 1;
        }
        for addr in arrays_to_remove {
            self.arrays.remove(&addr);
            freed += 1;
        }

        Ok(freed)
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    pub fn array_count(&self) -> usize {
        self.arrays.len()
    }

    pub fn memory_used(&self) -> usize {
        let mut total = 0;
        for obj in self.objects.values() {
            total += obj.class_name.len() + obj.fields.len() * 16;
        }
        for array in self.arrays.values() {
            total += match array {
                HeapArray::IntArray(v) => v.len() * 4,
                HeapArray::FloatArray(v) => v.len() * 4,
                HeapArray::LongArray(v) => v.len() * 8,
                HeapArray::DoubleArray(v) => v.len() * 8,
                HeapArray::ReferenceArray(v) => v.len() * 4,
                HeapArray::ByteArray(v) => v.len(),
                HeapArray::CharArray(v) => v.len() * 2,
                HeapArray::ShortArray(v) => v.len() * 2,
                HeapArray::BooleanArray(v) => v.len(),
            };
        }
        total
    }

    pub fn array_length(&self, addr: u32) -> Result<usize, MemoryError> {
        if let Some(ref s) = self.sanitizer {
            s.check_null(Some(addr))
                .map_err(MemoryError::InvalidArrayOperation)?;
        }
        self.arrays
            .get(&addr)
            .map(|array| match array {
                HeapArray::IntArray(v) => v.len(),
                HeapArray::FloatArray(v) => v.len(),
                HeapArray::LongArray(v) => v.len(),
                HeapArray::DoubleArray(v) => v.len(),
                HeapArray::ReferenceArray(v) => v.len(),
                HeapArray::ByteArray(v) => v.len(),
                HeapArray::CharArray(v) => v.len(),
                HeapArray::ShortArray(v) => v.len(),
                HeapArray::BooleanArray(v) => v.len(),
            })
            .ok_or_else(|| MemoryError::InvalidHeapAddress(addr))
    }

    pub fn array_get(&self, addr: u32, index: usize) -> Result<Value, MemoryError> {
        if let Some(ref s) = self.sanitizer {
            s.check_null(Some(addr))
                .map_err(MemoryError::InvalidArrayOperation)?;
        }
        let array = self
            .arrays
            .get(&addr)
            .ok_or_else(|| MemoryError::InvalidHeapAddress(addr))?;
        let len = match array {
            HeapArray::IntArray(v) => v.len(),
            HeapArray::FloatArray(v) => v.len(),
            HeapArray::LongArray(v) => v.len(),
            HeapArray::DoubleArray(v) => v.len(),
            HeapArray::ReferenceArray(v) => v.len(),
            HeapArray::ByteArray(v) => v.len(),
            HeapArray::CharArray(v) => v.len(),
            HeapArray::ShortArray(v) => v.len(),
            HeapArray::BooleanArray(v) => v.len(),
        };
        if let Some(ref s) = self.sanitizer {
            s.check_array_bounds(index, len)
                .map_err(MemoryError::InvalidArrayOperation)?;
        }
        match array {
            HeapArray::IntArray(v) => v
                .get(index)
                .map(|&val| Value::Int(val))
                .ok_or_else(|| MemoryError::InvalidArrayLength(index)),
            HeapArray::FloatArray(v) => v
                .get(index)
                .map(|&val| Value::Float(val))
                .ok_or_else(|| MemoryError::InvalidArrayLength(index)),
            HeapArray::LongArray(v) => v
                .get(index)
                .map(|&val| Value::Long(val))
                .ok_or_else(|| MemoryError::InvalidArrayLength(index)),
            HeapArray::DoubleArray(v) => v
                .get(index)
                .map(|&val| Value::Double(val))
                .ok_or_else(|| MemoryError::InvalidArrayLength(index)),
            HeapArray::ReferenceArray(v) => v
                .get(index)
                .map(|&val| Value::Reference(val))
                .ok_or_else(|| MemoryError::InvalidArrayLength(index)),
            HeapArray::ByteArray(v) => v
                .get(index)
                .map(|&val| Value::Int(val as i32))
                .ok_or_else(|| MemoryError::InvalidArrayLength(index)),
            HeapArray::CharArray(v) => v
                .get(index)
                .map(|&val| Value::Int(val as i32))
                .ok_or_else(|| MemoryError::InvalidArrayLength(index)),
            HeapArray::ShortArray(v) => v
                .get(index)
                .map(|&val| Value::Int(val as i32))
                .ok_or_else(|| MemoryError::InvalidArrayLength(index)),
            HeapArray::BooleanArray(v) => v
                .get(index)
                .map(|&val| Value::Int(if val { 1 } else { 0 }))
                .ok_or_else(|| MemoryError::InvalidArrayLength(index)),
        }
    }

    pub fn array_set(&mut self, addr: u32, index: usize, value: Value) -> Result<(), MemoryError> {
        if let Some(ref s) = self.sanitizer {
            s.check_null(Some(addr))
                .map_err(MemoryError::InvalidArrayOperation)?;
        }
        let array = self
            .arrays
            .get_mut(&addr)
            .ok_or_else(|| MemoryError::InvalidHeapAddress(addr))?;
        let len = match array {
            HeapArray::IntArray(v) => v.len(),
            HeapArray::FloatArray(v) => v.len(),
            HeapArray::LongArray(v) => v.len(),
            HeapArray::DoubleArray(v) => v.len(),
            HeapArray::ReferenceArray(v) => v.len(),
            HeapArray::ByteArray(v) => v.len(),
            HeapArray::CharArray(v) => v.len(),
            HeapArray::ShortArray(v) => v.len(),
            HeapArray::BooleanArray(v) => v.len(),
        };
        if let Some(ref s) = self.sanitizer {
            s.check_array_bounds(index, len)
                .map_err(MemoryError::InvalidArrayOperation)?;
        }
        match array {
            HeapArray::IntArray(v) => {
                if index >= v.len() {
                    return Err(MemoryError::InvalidArrayLength(index));
                }
                v[index] = value.as_int();
            }
            HeapArray::FloatArray(v) => {
                if index >= v.len() {
                    return Err(MemoryError::InvalidArrayLength(index));
                }
                v[index] = value.as_float();
            }
            HeapArray::LongArray(v) => {
                if index >= v.len() {
                    return Err(MemoryError::InvalidArrayLength(index));
                }
                v[index] = value.as_long();
            }
            HeapArray::DoubleArray(v) => {
                if index >= v.len() {
                    return Err(MemoryError::InvalidArrayLength(index));
                }
                v[index] = value.as_double();
            }
            HeapArray::ReferenceArray(v) => {
                if index >= v.len() {
                    return Err(MemoryError::InvalidArrayLength(index));
                }
                v[index] = value.as_reference().ok_or_else(|| {
                    MemoryError::InvalidArrayType("Expected reference".to_string())
                })?;
            }
            HeapArray::ByteArray(v) => {
                if index >= v.len() {
                    return Err(MemoryError::InvalidArrayLength(index));
                }
                v[index] = (value.as_int() as i8) as u8;
            }
            HeapArray::CharArray(v) => {
                if index >= v.len() {
                    return Err(MemoryError::InvalidArrayLength(index));
                }
                v[index] = value.as_int() as u16;
            }
            HeapArray::ShortArray(v) => {
                if index >= v.len() {
                    return Err(MemoryError::InvalidArrayLength(index));
                }
                v[index] = value.as_int() as i16;
            }
            HeapArray::BooleanArray(v) => {
                if index >= v.len() {
                    return Err(MemoryError::InvalidArrayLength(index));
                }
                v[index] = value.as_int() != 0;
            }
        }

        Ok(())
    }

    pub fn monitor_enter(&mut self, addr: u32, thread_id: u32) -> Result<(), MemoryError> {
        let obj = self
            .get_object_mut(addr)
            .ok_or(MemoryError::InvalidReference(addr))?;

        if obj.monitor.is_none() {
            obj.monitor = Some(Monitor::new());
        }

        let monitor = obj.monitor.as_mut().unwrap();

        if monitor.enter(thread_id) {
            Ok(())
        } else {
            Err(MemoryError::InvalidMonitorState)
        }
    }

    pub fn monitor_exit(&mut self, addr: u32, thread_id: u32) -> Result<(), MemoryError> {
        let obj = self
            .get_object_mut(addr)
            .ok_or(MemoryError::InvalidReference(addr))?;

        if let Some(monitor) = &mut obj.monitor {
            if Monitor::exit(monitor, thread_id) {
                Ok(())
            } else {
                Err(MemoryError::IllegalMonitorState)
            }
        } else {
            Err(MemoryError::IllegalMonitorState)
        }
    }

    pub fn owns_monitor(&self, addr: u32, thread_id: u32) -> bool {
        if let Some(obj) = self.get_object(addr) {
            if let Some(monitor) = &obj.monitor {
                return monitor.is_owned_by(thread_id);
            }
        }
        false
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}
