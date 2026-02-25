//! Garbage collection: generational GC with parallel collection
//! and ownership-based root tracking for low-pause behavior.

use crate::debug::JvmDebugger;
use crate::error::MemoryError;
use crate::memory::{HeapArray, HeapObject, Value};
use rayon::prelude::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

/// Generation for objects: young (frequently collected) vs old (long-lived)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Generation {
    /// Young generation - eden + survivor spaces
    Young,
    /// Old generation - promoted survivors
    Old,
}

/// Tracks which generation an object belongs to
#[derive(Debug, Clone)]
struct GenObject {
    obj: HeapObject,
    generation: Generation,
    /// Number of minor GCs survived (promote to old when threshold reached)
    age: u8,
}

/// Generational heap with young/old generations and parallel collection.
///
/// - **Young gen**: New objects allocated here. Minor GC collects frequently.
/// - **Old gen**: Survivors promoted after surviving N minor GCs.
/// - **Parallel sweep**: Sweep phase runs in parallel using rayon.
#[derive(Debug)]
pub struct GenerationalHeap {
    /// Young generation: objects that haven't been promoted
    young_objects: HashMap<u32, GenObject>,
    young_arrays: HashMap<u32, HeapArray>,
    /// Old generation: promoted objects
    old_objects: HashMap<u32, GenObject>,
    old_arrays: HashMap<u32, HeapArray>,
    /// Next address (shared across generations)
    next_addr: u32,
    /// Marked set for GC
    marked: HashSet<u32>,
    /// Age threshold for promotion (survive this many minor GCs)
    promotion_threshold: u8,
    /// Debugger
    debugger: Option<JvmDebugger>,
    /// Total minor GC count
    minor_gc_count: u64,
    /// Total major GC count
    major_gc_count: u64,
}

impl GenerationalHeap {
    pub const DEFAULT_PROMOTION_THRESHOLD: u8 = 8;

    pub fn new() -> Self {
        Self {
            young_objects: HashMap::new(),
            young_arrays: HashMap::new(),
            old_objects: HashMap::new(),
            old_arrays: HashMap::new(),
            next_addr: 1,
            marked: HashSet::new(),
            promotion_threshold: Self::DEFAULT_PROMOTION_THRESHOLD,
            debugger: None,
            minor_gc_count: 0,
            major_gc_count: 0,
        }
    }

    pub fn with_debugger(debugger: JvmDebugger) -> Self {
        Self {
            young_objects: HashMap::new(),
            young_arrays: HashMap::new(),
            old_objects: HashMap::new(),
            old_arrays: HashMap::new(),
            next_addr: 1,
            marked: HashSet::new(),
            promotion_threshold: Self::DEFAULT_PROMOTION_THRESHOLD,
            debugger: Some(debugger),
            minor_gc_count: 0,
            major_gc_count: 0,
        }
    }

    fn alloc_addr(&mut self) -> u32 {
        let addr = self.next_addr;
        self.next_addr += 1;
        addr
    }

    pub fn allocate(&mut self, class_name: String) -> u32 {
        let addr = self.alloc_addr();
        if let Some(debugger) = &self.debugger {
            debugger.log_memory_allocation(addr, 0, &class_name);
        }
        self.young_objects.insert(
            addr,
            GenObject {
                obj: HeapObject {
                    fields: HashMap::new(),
                    class_name,
                    marked: false,
                    string_data: None,
                    monitor: None,
                },
                generation: Generation::Young,
                age: 0,
            },
        );
        addr
    }

    pub fn allocate_string(&mut self, data: String) -> u32 {
        let addr = self.alloc_addr();
        if let Some(debugger) = &self.debugger {
            debugger.log_memory_allocation(addr, data.len(), "java/lang/String");
        }
        self.young_objects.insert(
            addr,
            GenObject {
                obj: HeapObject {
                    fields: HashMap::new(),
                    class_name: "java/lang/String".to_string(),
                    marked: false,
                    string_data: Some(data),
                    monitor: None,
                },
                generation: Generation::Young,
                age: 0,
            },
        );
        addr
    }

    pub fn allocate_array(&mut self, array_type: HeapArray) -> u32 {
        let addr = self.alloc_addr();
        self.young_arrays.insert(addr, array_type);
        addr
    }

    fn get_gen_object(&self, addr: u32) -> Option<&GenObject> {
        self.young_objects
            .get(&addr)
            .or_else(|| self.old_objects.get(&addr))
    }

    fn get_gen_object_mut(&mut self, addr: u32) -> Option<&mut GenObject> {
        if self.young_objects.contains_key(&addr) {
            self.young_objects.get_mut(&addr)
        } else {
            self.old_objects.get_mut(&addr)
        }
    }

    pub fn get_object(&self, addr: u32) -> Option<&HeapObject> {
        self.get_gen_object(addr).map(|g| &g.obj)
    }

    pub fn get_object_mut(&mut self, addr: u32) -> Option<&mut HeapObject> {
        self.get_gen_object_mut(addr).map(|g| &mut g.obj)
    }

    pub fn get_string_data(&self, addr: u32) -> Option<&String> {
        self.get_object(addr)
            .and_then(|obj| obj.string_data.as_ref())
    }

    pub fn is_string(&self, addr: u32) -> bool {
        self.get_object(addr)
            .map(|obj| obj.class_name == "java/lang/String")
            .unwrap_or(false)
    }

    pub fn get_array(&self, addr: u32) -> Option<&HeapArray> {
        self.young_arrays
            .get(&addr)
            .or_else(|| self.old_arrays.get(&addr))
    }

    pub fn get_array_mut(&mut self, addr: u32) -> Option<&mut HeapArray> {
        if self.young_arrays.contains_key(&addr) {
            self.young_arrays.get_mut(&addr)
        } else {
            self.old_arrays.get_mut(&addr)
        }
    }

    fn mark_from_roots(&mut self, addr: u32) {
        if self.marked.contains(&addr) {
            return;
        }
        self.marked.insert(addr);

        let refs = self.collect_references(addr);
        for ref_addr in refs {
            self.mark_from_roots(ref_addr);
        }
    }

    /// Collect references from an object (for parallel marking)
    fn collect_references(&self, addr: u32) -> Vec<u32> {
        let mut refs = Vec::new();
        if let Some(gen_obj) = self.get_gen_object(addr) {
            for value in gen_obj.obj.fields.values() {
                if let Some(ref_addr) = Value::as_reference(value) {
                    refs.push(ref_addr);
                }
            }
        }
        if let Some(array) = self.get_array(addr) {
            if let HeapArray::ReferenceArray(refs_arr) = array {
                refs.extend(refs_arr.iter().copied());
            }
        }
        refs
    }

    /// Minor GC: collect young generation only, promote survivors
    pub fn minor_gc(&mut self, roots: &[u32]) -> Result<usize, MemoryError> {
        self.marked.clear();

        // Mark phase: trace from roots
        for &root in roots {
            self.mark_from_roots(root);
        }

        // Sequential sweep for young gen
        let mut survivors = Vec::new();
        let mut to_free_objects = Vec::new();
        let mut to_free_arrays = Vec::new();

        for addr in self.young_objects.keys().copied().collect::<Vec<_>>() {
            if self.marked.contains(&addr) {
                survivors.push(addr);
            } else {
                to_free_objects.push(addr);
            }
        }
        for addr in self.young_arrays.keys().copied().collect::<Vec<_>>() {
            if !self.marked.contains(&addr) {
                to_free_arrays.push(addr);
            }
        }

        let freed = to_free_objects.len() + to_free_arrays.len();

        // Promote survivors: age+1, move to old if threshold reached
        for addr in survivors {
            if let Some(mut gen_obj) = self.young_objects.remove(&addr) {
                gen_obj.age += 1;
                if gen_obj.age >= self.promotion_threshold {
                    gen_obj.generation = Generation::Old;
                    gen_obj.obj.marked = false;
                    self.old_objects.insert(addr, gen_obj);
                } else {
                    gen_obj.obj.marked = false;
                    self.young_objects.insert(addr, gen_obj);
                }
            }
        }

        for addr in to_free_objects {
            self.young_objects.remove(&addr);
        }
        for addr in to_free_arrays {
            self.young_arrays.remove(&addr);
        }

        self.minor_gc_count += 1;
        Ok(freed)
    }

    /// Major GC: collect entire heap with parallel sweep
    pub fn major_gc(&mut self, roots: &[u32]) -> Result<usize, MemoryError> {
        self.marked.clear();

        for &root in roots {
            self.mark_from_roots(root);
        }

        // Parallel sweep: build list of addresses to remove in parallel
        let object_addrs: Vec<u32> = self
            .young_objects
            .keys()
            .chain(self.old_objects.keys())
            .copied()
            .collect();
        let array_addrs: Vec<u32> = self
            .young_arrays
            .keys()
            .chain(self.old_arrays.keys())
            .copied()
            .collect();

        let to_remove: Vec<u32> = object_addrs
            .par_iter()
            .chain(array_addrs.par_iter())
            .filter(|addr| !self.marked.contains(addr))
            .copied()
            .collect();

        let mut freed = 0;
        for addr in to_remove {
            if self.young_objects.remove(&addr).is_some()
                || self.old_objects.remove(&addr).is_some()
            {
                freed += 1;
            }
            if self.young_arrays.remove(&addr).is_some()
                || self.old_arrays.remove(&addr).is_some()
            {
                freed += 1;
            }
        }

        self.major_gc_count += 1;
        Ok(freed)
    }

    pub fn collect_garbage(&mut self, roots: &[u32]) -> Result<usize, MemoryError> {
        self.minor_gc(roots)
    }

    /// Mark an object as reachable (for external root registration)
    pub fn mark_object(&mut self, addr: u32) {
        self.marked.insert(addr);
        if let Some(gen_obj) = self.get_gen_object_mut(addr) {
            gen_obj.obj.marked = true;
        }
    }

    pub fn unmark_all(&mut self) {
        for obj in self
            .young_objects
            .values_mut()
            .chain(self.old_objects.values_mut())
        {
            obj.obj.marked = false;
        }
        self.marked.clear();
    }

    pub fn object_count(&self) -> usize {
        self.young_objects.len() + self.old_objects.len()
    }

    pub fn array_count(&self) -> usize {
        self.young_arrays.len() + self.old_arrays.len()
    }

    pub fn memory_used(&self) -> usize {
        let mut total = 0;
        for g in self
            .young_objects
            .values()
            .chain(self.old_objects.values())
        {
            total += g.obj.class_name.len() + g.obj.fields.len() * 16;
        }
        for array in self.young_arrays.values().chain(self.old_arrays.values()) {
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
        self.get_array(addr)
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
        let array = self
            .get_array(addr)
            .ok_or_else(|| MemoryError::InvalidHeapAddress(addr))?;

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
        let array = self
            .get_array_mut(addr)
            .ok_or_else(|| MemoryError::InvalidHeapAddress(addr))?;

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
            obj.monitor = Some(crate::memory::Monitor::new());
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
            if monitor.exit(thread_id) {
                Ok(())
            } else {
                Err(MemoryError::IllegalMonitorState)
            }
        } else {
            Err(MemoryError::IllegalMonitorState)
        }
    }

    pub fn owns_monitor(&self, addr: u32, thread_id: u32) -> bool {
        self.get_object(addr)
            .and_then(|obj| obj.monitor.as_ref())
            .map(|m| m.is_owned_by(thread_id))
            .unwrap_or(false)
    }
}

impl Default for GenerationalHeap {
    fn default() -> Self {
        Self::new()
    }
}

thread_local! {
    static ROOT_SET: RefCell<HashSet<u32>> = RefCell::new(HashSet::new());
}

/// Scoped root - when dropped, removes the reference from the root set.
/// This enables "pauseless" GC in the sense that roots are automatically managed
/// via Rust's ownership, preventing leaks from forgotten root removal.
pub struct ScopedRoot {
    addr: u32,
    removed: bool,
}

impl ScopedRoot {
    pub fn new(addr: u32) -> Self {
        ROOT_SET.with(|roots| {
            roots.borrow_mut().insert(addr);
        });
        Self { addr, removed: false }
    }

    pub fn addr(&self) -> u32 {
        self.addr
    }
}

impl Drop for ScopedRoot {
    fn drop(&mut self) {
        if !self.removed {
            ROOT_SET.with(|roots| {
                roots.borrow_mut().remove(&self.addr);
            });
            self.removed = true;
        }
    }
}

/// Get the current root set (for GC)
pub fn get_roots() -> Vec<u32> {
    ROOT_SET.with(|roots| roots.borrow().iter().copied().collect())
}

/// Add a root (use ScopedRoot for automatic removal)
pub fn add_root(addr: u32) {
    ROOT_SET.with(|roots| {
        roots.borrow_mut().insert(addr);
    });
}

/// Remove a root
pub fn remove_root(addr: u32) {
    ROOT_SET.with(|roots| {
        roots.borrow_mut().remove(&addr);
    });
}
