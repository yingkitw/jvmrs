//! Arena-based allocators for better cache locality and reduced fragmentation.
//!
//! Arenas provide:
//! - **Cache locality**: Objects allocated together are stored contiguously
//! - **Low fragmentation**: Bump allocation within blocks, block-based growth
//! - **Fast allocation**: O(1) bump pointer or free-list reuse

use crate::debug::JvmDebugger;
use crate::memory::{HeapArray, HeapObject};
use std::collections::HashMap;

/// A single slot in the arena (either occupied or free)
#[derive(Debug, Clone)]
enum ArenaSlot {
    Occupied(HeapObject),
    Free,
}

/// Arena-based allocator for heap objects.
///
/// Uses a contiguous array of slots for cache-friendly iteration during GC.
/// Allocation reuses freed slots (free list) or bumps into new capacity.
#[derive(Debug)]
pub struct ArenaAllocator {
    /// Contiguous storage - iteration order is predictable for cache locality
    slots: Vec<ArenaSlot>,
    /// Indices of freed slots for reuse
    free_list: Vec<u32>,
    /// Next address to assign for new allocations
    next_addr: u32,
    /// Debugger for allocation logging
    debugger: Option<JvmDebugger>,
}

impl ArenaAllocator {
    /// Initial capacity (number of object slots)
    const INITIAL_CAPACITY: usize = 256;

    /// Growth factor when capacity is exceeded
    const GROWTH_FACTOR: f64 = 1.5;

    pub fn new() -> Self {
        Self {
            slots: Vec::with_capacity(Self::INITIAL_CAPACITY),
            free_list: Vec::new(),
            next_addr: 1,
            debugger: None,
        }
    }

    pub fn with_debugger(debugger: JvmDebugger) -> Self {
        Self {
            slots: Vec::with_capacity(Self::INITIAL_CAPACITY),
            free_list: Vec::new(),
            next_addr: 1,
            debugger: Some(debugger),
        }
    }

    /// Allocate a new object, returns the address
    pub fn allocate(&mut self, class_name: String) -> u32 {
        let addr = if let Some(reused) = self.free_list.pop() {
            reused
        } else {
            let a = self.next_addr;
            self.next_addr += 1;
            a
        };

        let idx = self.addr_to_index(addr);
        if idx >= self.slots.len() {
            self.slots.resize(idx + 1, ArenaSlot::Free);
        }

        let obj = HeapObject {
            fields: HashMap::new(),
            class_name: class_name.clone(),
            marked: false,
            string_data: None,
            monitor: None,
        };

        if let Some(debugger) = &self.debugger {
            debugger.log_memory_allocation(addr, 0, &class_name);
        }

        self.slots[idx] = ArenaSlot::Occupied(obj);
        addr
    }

    fn addr_to_index(&self, addr: u32) -> usize {
        (addr - 1) as usize
    }

    /// Get object by address
    pub fn get_object(&self, addr: u32) -> Option<&HeapObject> {
        let idx = self.addr_to_index(addr);
        self.slots.get(idx).and_then(|s| match s {
            ArenaSlot::Occupied(obj) => Some(obj),
            ArenaSlot::Free => None,
        })
    }

    /// Get mutable object by address
    pub fn get_object_mut(&mut self, addr: u32) -> Option<&mut HeapObject> {
        let idx = self.addr_to_index(addr);
        self.slots.get_mut(idx).and_then(|s| match s {
            ArenaSlot::Occupied(obj) => Some(obj),
            ArenaSlot::Free => None,
        })
    }

    /// Free a slot (called during GC sweep)
    pub fn free_slot(&mut self, addr: u32) {
        let idx = self.addr_to_index(addr);
        if idx < self.slots.len() {
            self.slots[idx] = ArenaSlot::Free;
            self.free_list.push(addr);
        }
    }

    /// Iterate over all occupied slots (for GC marking, cache-friendly)
    pub fn iter_objects(&self) -> impl Iterator<Item = (u32, &HeapObject)> {
        self.slots.iter().enumerate().filter_map(|(i, slot)| {
            if let ArenaSlot::Occupied(obj) = slot {
                Some(((i + 1) as u32, obj))
            } else {
                None
            }
        })
    }

    /// Mark object (for GC)
    pub fn mark(&mut self, addr: u32) {
        if let Some(obj) = self.get_object_mut(addr) {
            obj.marked = true;
        }
    }

    /// Number of allocated objects
    pub fn object_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|s| matches!(s, ArenaSlot::Occupied(_)))
            .count()
    }
}

impl Default for ArenaAllocator {
    fn default() -> Self {
        Self::new()
    }
}

/// Arena for array storage - separate from objects for type homogeneity
#[derive(Debug)]
pub struct ArrayArena {
    slots: Vec<Option<HeapArray>>,
    free_list: Vec<u32>,
    next_addr: u32,
}

impl ArrayArena {
    const INITIAL_CAPACITY: usize = 128;

    pub fn new() -> Self {
        Self {
            slots: Vec::with_capacity(Self::INITIAL_CAPACITY),
            free_list: Vec::new(),
            next_addr: 1,
        }
    }

    pub fn allocate(&mut self, array: HeapArray) -> u32 {
        let addr = if let Some(idx) = self.free_list.pop() {
            idx
        } else {
            let addr = self.next_addr;
            self.next_addr += 1;
            addr
        };

        let idx = (addr - 1) as usize;
        if idx >= self.slots.len() {
            self.slots.resize(idx + 1, None);
        }
        self.slots[idx] = Some(array);
        addr
    }

    pub fn get_array(&self, addr: u32) -> Option<&HeapArray> {
        let idx = (addr - 1) as usize;
        self.slots.get(idx).and_then(|s| s.as_ref())
    }

    pub fn get_array_mut(&mut self, addr: u32) -> Option<&mut HeapArray> {
        let idx = (addr - 1) as usize;
        self.slots.get_mut(idx).and_then(|s| s.as_mut())
    }

    pub fn free_slot(&mut self, addr: u32) {
        let idx = (addr - 1) as usize;
        if idx < self.slots.len() {
            self.slots[idx] = None;
            self.free_list.push(addr);
        }
    }

    pub fn iter_arrays(&self) -> impl Iterator<Item = (u32, &HeapArray)> {
        self.slots.iter().enumerate().filter_map(|(i, slot)| {
            slot.as_ref().map(|arr| ((i + 1) as u32, arr))
        })
    }

    pub fn array_count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }
}

impl Default for ArrayArena {
    fn default() -> Self {
        Self::new()
    }
}
