use std::collections::HashMap;

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
}

impl Value {
    /// Convert value to default int (for boolean, byte, char, short)
    pub fn as_int(&self) -> i32 {
        match self {
            Value::Int(i) => *i,
            Value::Float(f) => *f as i32,
            _ => 0,
        }
    }

    /// Convert value to float
    pub fn as_float(&self) -> f32 {
        match self {
            Value::Int(i) => *i as f32,
            Value::Float(f) => *f,
            _ => 0.0,
        }
    }

    /// Convert value to long
    pub fn as_long(&self) -> i64 {
        match self {
            Value::Long(l) => *l,
            Value::Int(i) => *i as i64,
            _ => 0,
        }
    }

    /// Convert value to double
    pub fn as_double(&self) -> f64 {
        match self {
            Value::Double(d) => *d,
            Value::Float(f) => *f as f64,
            _ => 0.0,
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
    pub fn push(&mut self, value: Value) -> Result<(), String> {
        self.stack.push(value);
        Ok(())
    }

    /// Pop a value from the operand stack
    pub fn pop(&mut self) -> Result<Value, String> {
        self.stack.pop().ok_or_else(|| "Stack underflow".to_string())
    }

    /// Peek at the top value on the operand stack
    pub fn peek(&self) -> Result<&Value, String> {
        self.stack.last().ok_or_else(|| "Stack is empty".to_string())
    }

    /// Load a local variable at index
    pub fn load_local(&self, index: usize) -> Result<Value, String> {
        self.locals
            .get(index)
            .cloned()
            .ok_or_else(|| format!("Local variable index {} out of bounds", index))
    }

    /// Store a value to a local variable at index
    pub fn store_local(&mut self, index: usize, value: Value) -> Result<(), String> {
        if index >= self.locals.len() {
            return Err(format!("Local variable index {} out of bounds", index));
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
    /// Next available address
    next_addr: u32,
}

/// Represents an object on the heap
#[derive(Debug, Clone)]
pub struct HeapObject {
    pub fields: HashMap<String, Value>,
    pub class_name: String,
}

impl Heap {
    /// Create a new empty heap
    pub fn new() -> Self {
        Heap {
            objects: HashMap::new(),
            next_addr: 1,
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
            },
        );
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
