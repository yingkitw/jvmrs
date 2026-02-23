use crate::error::{to_runtime_error_enum, JvmError, MemoryError, RuntimeError};
use std::collections::{HashMap, HashSet};

/// Represents a value in the JVM
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    Reference(u32), // Object/array reference
    ReturnAddress(u32),
    Null,
    // Array types
    ArrayRef(u32), // Array reference (separate from object reference)
}

impl Value {
    /// Convert value to default int (for boolean, byte, char, short)
    pub fn as_int(&self) -> i32 {
        match self {
            Value::Int(i) => *i,
            Value::Float(f) => *f as i32,
            Value::Long(l) => *l as i32,
            Value::Double(d) => *d as i32,
            _ => 0,
        }
    }

    /// Convert value to float
    pub fn as_float(&self) -> f32 {
        match self {
            Value::Int(i) => *i as f32,
            Value::Float(f) => *f,
            Value::Long(l) => *l as f32,
            Value::Double(d) => *d as f32,
            _ => 0.0,
        }
    }

    /// Convert value to long
    pub fn as_long(&self) -> i64 {
        match self {
            Value::Long(l) => *l,
            Value::Int(i) => *i as i64,
            Value::Float(f) => *f as i64,
            Value::Double(d) => *d as i64,
            _ => 0,
        }
    }

    /// Convert value to double
    pub fn as_double(&self) -> f64 {
        match self {
            Value::Double(d) => *d,
            Value::Float(f) => *f as f64,
            Value::Int(i) => *i as f64,
            Value::Long(l) => *l as f64,
            _ => 0.0,
        }
    }

    /// Check if value is a reference (object or array)
    pub fn is_reference(&self) -> bool {
        matches!(self, Value::Reference(_) | Value::ArrayRef(_))
    }

    /// Get reference address if this is a reference
    pub fn as_reference(&self) -> Option<u32> {
        match self {
            Value::Reference(addr) | Value::ArrayRef(addr) => Some(*addr),
            _ => None,
        }
    }
}

/// A stack frame for method execution
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// Local variables
    pub locals: Vec<Value>,
    /// Operand stack
    pub stack: Vec<Value>,
    /// Current program counter
    pub pc: usize,
    /// Method name (for debugging)
    pub method_name: String,
}

impl StackFrame {
    /// Create a new stack frame
    pub fn new(max_locals: usize, max_stack: usize, method_name: String) -> Self {
        StackFrame {
            locals: vec![Value::Int(0); max_locals],
            stack: Vec::with_capacity(max_stack),
            pc: 0,
            method_name,
        }
    }

    /// Push a value onto the operand stack
    pub fn push(&mut self, value: Value) -> Result<(), JvmError> {
        self.stack.push(value);
        Ok(())
    }

    /// Pop a value from the operand stack
    pub fn pop(&mut self) -> Result<Value, JvmError> {
        self.stack
            .pop()
            .ok_or_else(|| to_runtime_error_enum(RuntimeError::StackUnderflow))
    }

    /// Peek at the top value on the operand stack
    pub fn peek(&self) -> Result<&Value, JvmError> {
        self.stack
            .last()
            .ok_or_else(|| to_runtime_error_enum(RuntimeError::StackUnderflow))
    }

    /// Load a local variable at index
    pub fn load_local(&self, index: usize) -> Result<Value, JvmError> {
        self.locals
            .get(index)
            .cloned()
            .ok_or_else(|| to_runtime_error_enum(RuntimeError::LocalVariableOutOfBounds(index)))
    }

    /// Store a value to a local variable at index
    pub fn store_local(&mut self, index: usize, value: Value) -> Result<(), JvmError> {
        if index >= self.locals.len() {
            return Err(to_runtime_error_enum(
                RuntimeError::LocalVariableOutOfBounds(index),
            ));
        }
        self.locals[index] = value;
        Ok(())
    }
}

/// JVM heap for object allocation
#[derive(Debug)]
pub struct Heap {
    /// Simple heap using a map from address to object
    objects: HashMap<u32, HeapObject>,
    /// Arrays stored separately
    arrays: HashMap<u32, HeapArray>,
    /// Next available address
    next_addr: u32,
    /// Marked objects for garbage collection
    marked: HashSet<u32>,
}

/// Represents an object on the heap
#[derive(Debug, Clone)]
pub struct HeapObject {
    pub fields: HashMap<String, Value>,
    pub class_name: String,
    /// Whether this object is marked during GC
    pub marked: bool,
    /// String data for java/lang/String objects
    pub string_data: Option<String>,
}

/// Represents an array on the heap
#[derive(Debug, Clone)]
pub enum HeapArray {
    IntArray(Vec<i32>),
    FloatArray(Vec<f32>),
    LongArray(Vec<i64>),
    DoubleArray(Vec<f64>),
    ReferenceArray(Vec<u32>), // Array of object references
    ByteArray(Vec<u8>),
    CharArray(Vec<u16>),
    ShortArray(Vec<i16>),
    BooleanArray(Vec<bool>),
}

impl Heap {
    /// Create a new empty heap
    pub fn new() -> Self {
        Heap {
            objects: HashMap::new(),
            arrays: HashMap::new(),
            next_addr: 1,
            marked: HashSet::new(),
        }
    }

    /// Allocate a new object
    pub fn allocate(&mut self, class_name: String) -> u32 {
        let addr = self.next_addr;
        self.next_addr += 1;
        self.objects.insert(
            addr,
            HeapObject {
                fields: HashMap::new(),
                class_name,
                marked: false,
                string_data: None,
            },
        );
        addr
    }

    /// Allocate a new string object
    pub fn allocate_string(&mut self, data: String) -> u32 {
        let addr = self.next_addr;
        self.next_addr += 1;
        self.objects.insert(
            addr,
            HeapObject {
                fields: HashMap::new(),
                class_name: "java/lang/String".to_string(),
                marked: false,
                string_data: Some(data),
            },
        );
        addr
    }

    /// Allocate a new array
    pub fn allocate_array(&mut self, array_type: HeapArray) -> u32 {
        let addr = self.next_addr;
        self.next_addr += 1;
        self.arrays.insert(addr, array_type);
        addr
    }

    /// Get an object by address
    pub fn get_object(&self, addr: u32) -> Option<&HeapObject> {
        self.objects.get(&addr)
    }

    /// Get a mutable object by address
    pub fn get_object_mut(&mut self, addr: u32) -> Option<&mut HeapObject> {
        self.objects.get_mut(&addr)
    }

    /// Get string data from a string object
    pub fn get_string_data(&self, addr: u32) -> Option<&String> {
        self.objects.get(&addr).and_then(|obj| {
            if obj.class_name == "java/lang/String" {
                obj.string_data.as_ref()
            } else {
                None
            }
        })
    }

    /// Check if an object is a string
    pub fn is_string(&self, addr: u32) -> bool {
        self.objects
            .get(&addr)
            .map(|obj| obj.class_name == "java/lang/String")
            .unwrap_or(false)
    }

    /// Get an array by address
    pub fn get_array(&self, addr: u32) -> Option<&HeapArray> {
        self.arrays.get(&addr)
    }

    /// Get a mutable array by address
    pub fn get_array_mut(&mut self, addr: u32) -> Option<&mut HeapArray> {
        self.arrays.get_mut(&addr)
    }

    /// Mark an object as reachable during garbage collection
    pub fn mark(&mut self, addr: u32) {
        if let Some(obj) = self.objects.get_mut(&addr) {
            obj.marked = true;
            self.marked.insert(addr);
        }
    }

    /// Unmark all objects (prepare for next GC cycle)
    pub fn unmark_all(&mut self) {
        for obj in self.objects.values_mut() {
            obj.marked = false;
        }
        self.marked.clear();
    }

    /// Perform garbage collection (mark-and-sweep)
    pub fn collect_garbage(&mut self, roots: &[u32]) -> Result<usize, MemoryError> {
        // Mark phase: mark all reachable objects
        self.unmark_all();
        let mut to_mark: Vec<u32> = roots.to_vec();

        while let Some(addr) = to_mark.pop() {
            if self.marked.contains(&addr) {
                continue;
            }

            self.mark(addr);

            // Mark references from this object
            if let Some(obj) = self.objects.get(&addr) {
                for value in obj.fields.values() {
                    if let Some(ref_addr) = value.as_reference() {
                        if !self.marked.contains(&ref_addr) {
                            to_mark.push(ref_addr);
                        }
                    }
                }
            }

            // Mark references from arrays
            if let Some(array) = self.arrays.get(&addr) {
                match array {
                    HeapArray::ReferenceArray(refs) => {
                        for ref_addr in refs {
                            if !self.marked.contains(ref_addr) {
                                to_mark.push(*ref_addr);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Sweep phase: remove unmarked objects and arrays
        let mut freed = 0;
        let mut objects_to_remove = Vec::new();
        let mut arrays_to_remove = Vec::new();

        for (addr, obj) in &self.objects {
            if !obj.marked {
                objects_to_remove.push(*addr);
            }
        }

        // Arrays are always considered reachable if referenced
        // For simplicity, we'll remove arrays that aren't in objects
        // In a real implementation, we'd track array references too
        for (addr, _) in &self.arrays {
            if !self.marked.contains(addr) {
                arrays_to_remove.push(*addr);
            }
        }

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

    /// Get the number of objects in the heap
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    /// Get the number of arrays in the heap
    pub fn array_count(&self) -> usize {
        self.arrays.len()
    }

    /// Get the total memory used (estimated)
    pub fn memory_used(&self) -> usize {
        let mut total = 0;
        for obj in self.objects.values() {
            // Estimate: class name + fields
            total += obj.class_name.len() + obj.fields.len() * 16; // Rough estimate
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

    /// Get array length
    pub fn array_length(&self, addr: u32) -> Result<usize, MemoryError> {
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

    /// Get array element
    pub fn array_get(&self, addr: u32, index: usize) -> Result<Value, MemoryError> {
        let array = self
            .arrays
            .get(&addr)
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

    /// Set array element
    pub fn array_set(&mut self, addr: u32, index: usize, value: Value) -> Result<(), MemoryError> {
        let array = self
            .arrays
            .get_mut(&addr)
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
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

/// JVM runtime stack for method invocation
#[derive(Debug)]
pub struct JVMStack {
    /// Stack frames
    frames: Vec<StackFrame>,
}

impl JVMStack {
    /// Create a new empty JVM stack
    pub fn new() -> Self {
        JVMStack { frames: Vec::new() }
    }

    /// Push a new frame onto the stack
    pub fn push_frame(&mut self, frame: StackFrame) {
        self.frames.push(frame);
    }

    /// Pop the current frame from the stack
    pub fn pop_frame(&mut self) -> Option<StackFrame> {
        self.frames.pop()
    }

    /// Get the current (top) frame
    pub fn current_frame(&self) -> Option<&StackFrame> {
        self.frames.last()
    }

    /// Get a mutable reference to the current frame
    pub fn current_frame_mut(&mut self) -> Option<&mut StackFrame> {
        self.frames.last_mut()
    }

    /// Get the number of frames
    pub fn depth(&self) -> usize {
        self.frames.len()
    }
}

impl Default for JVMStack {
    fn default() -> Self {
        Self::new()
    }
}

/// JVM runtime memory area
#[derive(Debug)]
pub struct Memory {
    /// Method invocation stack
    pub stack: JVMStack,
    /// Heap for objects
    pub heap: Heap,
    /// Static fields (class_name -> field_name -> value)
    pub static_fields: HashMap<String, HashMap<String, Value>>,
}

impl Memory {
    /// Create a new memory area
    pub fn new() -> Self {
        Memory {
            stack: JVMStack::new(),
            heap: Heap::new(),
            static_fields: HashMap::new(),
        }
    }

    /// Get a static field value
    pub fn get_static(&self, class_name: &str, field_name: &str) -> Option<&Value> {
        self.static_fields
            .get(class_name)
            .and_then(|fields| fields.get(field_name))
    }

    /// Set a static field value
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
