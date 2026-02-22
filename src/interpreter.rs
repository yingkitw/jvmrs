use crate::class_file::{ClassFile, MethodInfo, AttributeInfo};
use crate::memory::{Memory, StackFrame, Value};
use std::collections::HashMap;
use std::path::Path;

/// Result type for interpreter operations
pub type InterpreterResult = Result<(), String>;

/// JVM Interpreter
pub struct Interpreter {
    /// Loaded classes
    classes: HashMap<String, ClassFile>,
    /// Runtime memory
    memory: Memory,
    /// String value cache for heap objects
    string_cache: HashMap<u32, String>,
}

impl Interpreter {
    /// Create a new interpreter
    pub fn new() -> Self {
        Interpreter {
            classes: HashMap::new(),
            memory: Memory::new(),
            string_cache: HashMap::new(),
        }
    }

    /// Load a class from a file
    pub fn load_class<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        let class_file = ClassFile::from_file(path).map_err(|e| format!("Failed to load class: {:?}", e))?;
        let class_name = class_file.get_class_name().unwrap_or_else(|| "Unknown".to_string());
        self.classes.insert(class_name, class_file);
        Ok(())
    }

    /// Load a class by name (searches for .class file)
    pub fn load_class_by_name(&mut self, class_name: &str) -> Result<(), String> {
        let path = format!("{}.class", class_name.replace('.', "/"));
        self.load_class(path)
    }

    /// Get a loaded class by name
    pub fn get_class(&self, name: &str) -> Option<&ClassFile> {
        self.classes.get(name)
    }

    /// Run the main method of a class
    pub fn run_main(&mut self, class_name: &str) -> InterpreterResult {
        // Remove the class from the HashMap to avoid borrow issues
        let class = self.classes.remove(class_name)
            .ok_or_else(|| format!("Class '{}' not found", class_name))?;

        // Find the main method: public static void main(String[])
        let main_method = class.find_method("main", "([Ljava/lang/String;)V")
            .ok_or_else(|| format!("Main method not found in class '{}'", class_name))?
            .clone();

        // Find the Code attribute
        let code_attr = self.find_code_attribute(&class, &main_method)
            .ok_or_else(|| "Code attribute not found".to_string())?
            .clone();

        // Create a stack frame for main
        let max_stack = self.read_u16(&code_attr.info, 0) as usize;
        let max_locals = self.read_u16(&code_attr.info, 2) as usize;
        let code_length = self.read_u32(&code_attr.info, 4) as usize;

        let mut frame = StackFrame::new(max_locals, max_stack, "main".to_string());

        // Parse and execute instructions
        let code = code_attr.info[8..8 + code_length].to_vec();
        self.execute_instructions(&mut frame, &class, &code)?;

        // Re-insert the class after execution
        self.classes.insert(class_name.to_string(), class);

        Ok(())
    }

    /// Execute bytecode instructions
    fn execute_instructions(&mut self, frame: &mut StackFrame, class: &ClassFile, code: &[u8]) -> InterpreterResult {
        while frame.pc < code.len() {
            let opcode = code[frame.pc];
            frame.pc += 1;

            match opcode {
                // return: return void from method
                0xb1 => {
                    break;  // Break out of the loop instead of returning
                }

                // ireturn: return int from method
                0xac => {
                    break;  // Break out of the loop instead of returning
                }

                // freturn: return float from method
                0xae => {
                    break;  // Break out of the loop instead of returning
                }

                // nop: do nothing
                0x00 => {}

                // aconst_null: push null onto stack
                0x01 => { frame.push(Value::Null)?; }

                // iconst_m1 through iconst_5: push int constant -1 to 5
                0x02..=0x08 => {
                    let value = (opcode as i32) - 3;
                    frame.push(Value::Int(value))?;
                }

                // bipush: push byte as integer
                0x10 => {
                    let byte = code[frame.pc] as i8;
                    frame.pc += 1;
                    frame.push(Value::Int(byte as i32))?;
                }

                // sipush: push short as integer
                0x11 => {
                    let byte1 = code[frame.pc] as i8;
                    let byte2 = code[frame.pc + 1];
                    frame.pc += 2;
                    let value = ((byte1 as i32) << 8) | (byte2 as i32);
                    frame.push(Value::Int(value))?;
                }

                // ldc: push item from constant pool
                0x12 => {
                    let index = code[frame.pc] as u16;
                    frame.pc += 1;
                    self.load_constant(frame, class, index)?;
                }

                // iload: load int from local variable at index (unsigned byte)
                0x15 => {
                    let index = code[frame.pc] as usize;
                    frame.pc += 1;
                    let value = frame.load_local(index)?;
                    frame.push(value)?;
                }

                // lload: load long (not implemented, just skip)
                0x16 => {
                    let _index = code[frame.pc];
                    frame.pc += 1;
                    return Err("lload not implemented".to_string());
                }

                // fload: load float from local variable at index (unsigned byte)
                0x17 => {
                    let index = code[frame.pc] as usize;
                    frame.pc += 1;
                    let value = frame.load_local(index)?;
                    frame.push(value)?;
                }

                // dload: load double (not implemented, just skip)
                0x18 => {
                    let _index = code[frame.pc];
                    frame.pc += 1;
                    return Err("dload not implemented".to_string());
                }

                // aload: load reference from local variable at index (unsigned byte)
                0x19 => {
                    let index = code[frame.pc] as usize;
                    frame.pc += 1;
                    let value = frame.load_local(index)?;
                    frame.push(value)?;
                }

                // iload_0 through iload_3: load int from local variable
                0x1a..=0x1d => {
                    let index = (opcode - 0x1a) as usize;
                    let value = frame.load_local(index)?;
                    frame.push(value)?;
                }

                // lload_0 through lload_3 (not implemented)
                0x1e..=0x21 => {
                    return Err("lload_0-3 not implemented".to_string());
                }

                // fload_0 through fload_3: load float from local variable
                0x22..=0x25 => {
                    let index = (opcode - 0x22) as usize;
                    let value = frame.load_local(index)?;
                    frame.push(value)?;
                }

                // dload_0 through dload_3 (not implemented)
                0x26..=0x29 => {
                    return Err("dload_0-3 not implemented".to_string());
                }

                // aload_0 through aload_3: load reference from local variable
                0x2a..=0x2d => {
                    let index = (opcode - 0x2a) as usize;
                    let value = frame.load_local(index)?;
                    frame.push(value)?;
                }

                // i2l: convert int to long
                0x85 => {
                    let value = frame.pop()?.as_int();
                    frame.push(Value::Long(value as i64))?;
                }

                // i2f: convert int to float
                0x86 => {
                    let value = frame.pop()?.as_int();
                    frame.push(Value::Float(value as f32))?;
                }

                // i2d: convert int to double
                0x87 => {
                    let value = frame.pop()?.as_int();
                    frame.push(Value::Double(value as f64))?;
                }

                // l2i: convert long to int
                0x88 => {
                    let value = frame.pop()?.as_long();
                    frame.push(Value::Int(value as i32))?;
                }

                // f2i: convert float to int
                0x8b => {
                    let value = frame.pop()?.as_float();
                    frame.push(Value::Int(value as i32))?;
                }

                // f2d: convert float to double
                0x8d => {
                    let value = frame.pop()?.as_float();
                    frame.push(Value::Double(value as f64))?;
                }

                // d2i: convert double to int
                0x8e => {
                    let value = frame.pop()?.as_double();
                    frame.push(Value::Int(value as i32))?;
                }

                // d2f: convert double to float
                0x8f => {
                    let value = frame.pop()?.as_double();
                    frame.push(Value::Float(value as f32))?;
                }

                // istore_0 through istore_3: store int into local variable
                0x3b..=0x3e => {
                    let index = (opcode - 0x3b) as usize;
                    let value = frame.pop()?;
                    frame.store_local(index, value)?;
                }

                // lstore_0 through lstore_3 (not implemented)
                0x3f..=0x42 => {
                    return Err("lstore_0-3 not implemented".to_string());
                }

                // fstore_0 through fstore_3: store float into local variable
                0x43..=0x46 => {
                    let index = (opcode - 0x43) as usize;
                    let value = frame.pop()?;
                    frame.store_local(index, value)?;
                }

                // dstore_0 through dstore_3 (not implemented)
                0x47..=0x4a => {
                    return Err("dstore_0-3 not implemented".to_string());
                }

                // astore_0 through astore_3: store reference into local variable
                0x4b..=0x4e => {
                    let index = (opcode - 0x4b) as usize;
                    let value = frame.pop()?;
                    frame.store_local(index, value)?;
                }

                // iadd: add two integers
                0x60 => {
                    let value2 = frame.pop()?.as_int();
                    let value1 = frame.pop()?.as_int();
                    frame.push(Value::Int(value1.wrapping_add(value2)))?;
                }

                // fadd: add two floats
                0x62 => {
                    let value2 = frame.pop()?.as_float();
                    let value1 = frame.pop()?.as_float();
                    frame.push(Value::Float(value1 + value2))?;
                }

                // isub: subtract two integers
                0x64 => {
                    let value2 = frame.pop()?.as_int();
                    let value1 = frame.pop()?.as_int();
                    frame.push(Value::Int(value1.wrapping_sub(value2)))?;
                }

                // fsub: subtract two floats
                0x66 => {
                    let value2 = frame.pop()?.as_float();
                    let value1 = frame.pop()?.as_float();
                    frame.push(Value::Float(value1 - value2))?;
                }

                // imul: multiply two integers
                0x68 => {
                    let value2 = frame.pop()?.as_int();
                    let value1 = frame.pop()?.as_int();
                    frame.push(Value::Int(value1.wrapping_mul(value2)))?;
                }

                // fmul: multiply two floats
                0x6a => {
                    let value2 = frame.pop()?.as_float();
                    let value1 = frame.pop()?.as_float();
                    frame.push(Value::Float(value1 * value2))?;
                }

                // idiv: divide two integers
                0x6c => {
                    let value2 = frame.pop()?.as_int();
                    let value1 = frame.pop()?.as_int();
                    if value2 == 0 {
                        return Err("Division by zero".to_string());
                    }
                    frame.push(Value::Int(value1 / value2))?;
                }

                // fdiv: divide two floats
                0x6e => {
                    let value2 = frame.pop()?.as_float();
                    let value1 = frame.pop()?.as_float();
                    frame.push(Value::Float(value1 / value2))?;
                }

                // iinc: increment local variable by constant
                0x84 => {
                    let index = code[frame.pc] as usize;
                    let const_val = code[frame.pc + 1] as i8;
                    frame.pc += 2;
                    let current = frame.load_local(index)?.as_int();
                    frame.store_local(index, Value::Int(current.wrapping_add(const_val as i32)))?;
                }

                // ifeq: branch if int is zero
                0x99 => {
                    let value = frame.pop()?.as_int();
                    let offset = self.read_i16(code, frame.pc);
                    frame.pc += 2;
                    if value == 0 {
                        frame.pc = (frame.pc as i32 + offset as i32 - 2) as usize;
                    }
                }

                // ifne: branch if int is not zero
                0x9a => {
                    let value = frame.pop()?.as_int();
                    let offset = self.read_i16(code, frame.pc);
                    frame.pc += 2;
                    if value != 0 {
                        frame.pc = (frame.pc as i32 + offset as i32 - 2) as usize;
                    }
                }

                // if_icmplt: branch if int comparison is less than
                0xa1 => {
                    let value2 = frame.pop()?.as_int();
                    let value1 = frame.pop()?.as_int();
                    let offset = self.read_i16(code, frame.pc);
                    frame.pc += 2;
                    if value1 < value2 {
                        frame.pc = (frame.pc as i32 + offset as i32 - 2) as usize;
                    }
                }

                // goto: branch always
                0xa7 => {
                    let offset = self.read_i16(code, frame.pc);
                    frame.pc += 2;
                    frame.pc = (frame.pc as i32 + offset as i32 - 2) as usize;
                }

                // invokeinterface: invoke interface method
                0xb9 => {
                    let _index = self.read_u16(code, frame.pc) as usize;
                    frame.pc += 2;
                    let _count = code[frame.pc];
                    frame.pc += 1;
                    let _zero = code[frame.pc];
                    frame.pc += 1;
                    if _zero != 0 {
                        return Err("invokeinterface fourth byte must be zero".to_string());
                    }
                    // For now, just pop arguments and ignore
                    // TODO: Implement proper interface method invocation
                    return Ok(());
                }

                // invokedynamic: invoke dynamically (for lambda and other dynamic features)
                0xba => {
                    let index = self.read_u16(code, frame.pc) as usize;
                    frame.pc += 2;
                    let _zero1 = code[frame.pc];
                    frame.pc += 1;
                    let _zero2 = code[frame.pc];
                    frame.pc += 1;
                    if _zero1 != 0 || _zero2 != 0 {
                        return Err("invokedynamic last two bytes must be zero".to_string());
                    }
                    // Handle invokedynamic for string concatenation
                    self.handle_invokedynamic(class, frame, index)?;
                }

                // invokespecial: invoke instance method (constructor, super, private)
                0xb7 => {
                    let index = self.read_u16(code, frame.pc) as usize;
                    frame.pc += 2;
                    // For now, just pop arguments and ignore
                    self.invoke_virtual(class, frame, index)?;
                }

                // invokevirtual: invoke instance method
                0xb6 => {
                    let index = self.read_u16(code, frame.pc) as usize;
                    frame.pc += 2;
                    self.invoke_virtual(class, frame, index)?;
                }

                // invokestatic: invoke a static method
                0xb8 => {
                    let index = self.read_u16(code, frame.pc) as usize;
                    frame.pc += 2;
                    self.invoke_static(class, frame, index)?;
                }

                // invokespecial: invoke instance method (constructor, super, private)
                0xb7 => {
                    let index = self.read_u16(code, frame.pc) as usize;
                    frame.pc += 2;
                    // For now, just pop arguments and ignore
                    self.invoke_virtual(class, frame, index)?;
                }

                // invokevirtual: invoke instance method
                0xb6 => {
                    let index = self.read_u16(code, frame.pc) as usize;
                    frame.pc += 2;
                    self.invoke_virtual(class, frame, index)?;
                }

                // getstatic: get static field from class
                0xb2 => {
                    let _ = self.read_u16(code, frame.pc);
                    frame.pc += 2;
                    // For now, just push a placeholder for java/lang/System.out
                    frame.push(Value::Null)?;
                }

                // putfield: set field in object
                0xb5 => {
                    let _ = self.read_u16(code, frame.pc);
                    frame.pc += 2;
                    let _value = frame.pop()?;
                    let _objectref = frame.pop()?;
                    // Ignore for now
                }

                // Unknown opcode
                _ => {
                    return Err(format!("Unknown opcode: 0x{:02x} at PC {}", opcode, frame.pc - 1));
                }
            }
        }

        Ok(())
    }

    /// Load a constant from the constant pool onto the stack
    fn load_constant(&mut self, frame: &mut StackFrame, class: &ClassFile, index: u16) -> InterpreterResult {
        if index as usize >= class.constant_pool.len() {
            return Err(format!("Constant pool index {} out of bounds", index));
        }

        let entry = &class.constant_pool[index as usize];
        match entry {
            crate::class_file::ConstantPoolEntry::ConstantInteger { bytes } => {
                frame.push(Value::Int(*bytes))?;
            }
            crate::class_file::ConstantPoolEntry::ConstantFloat { bytes } => {
                frame.push(Value::Float(*bytes))?;
            }
            crate::class_file::ConstantPoolEntry::ConstantString { string_index } => {
                // Create a string reference
                if let Some(utf8) = class.constant_pool.get(*string_index as usize) {
                    if let crate::class_file::ConstantPoolEntry::ConstantUtf8 { bytes } = utf8 {
                        // Convert bytes to string
                        let string_value = String::from_utf8(bytes.clone()).unwrap_or_else(|_| "[invalid utf8]".to_string());
                        let addr = self.memory.heap.allocate("java/lang/String".to_string());
                        // Cache the string value for printing
                        self.string_cache.insert(addr, string_value);
                        frame.push(Value::Reference(addr))?;
                    }
                }
            }
            crate::class_file::ConstantPoolEntry::ConstantClass { .. } => {
                // Create a Class object reference
                let addr = self.memory.heap.allocate("java/lang/Class".to_string());
                frame.push(Value::Reference(addr))?;
            }
            _ => {
                return Err(format!("Unsupported constant type at index {}", index));
            }
        }
        Ok(())
    }

    /// Invoke a static method
    fn invoke_static(&mut self, class: &ClassFile, frame: &mut StackFrame, index: usize) -> InterpreterResult {
        let cp_entry = class.constant_pool.get(index)
            .ok_or_else(|| format!("Constant pool index {} out of bounds", index))?;

        let (class_index, name_and_type_index) = match cp_entry {
            crate::class_file::ConstantPoolEntry::ConstantMethodref { class_index, name_and_type_index } => {
                (class_index, name_and_type_index)
            }
            _ => return Err("Expected MethodRef constant".to_string()),
        };

        // Get the class name
        let class_entry = class.constant_pool.get(*class_index as usize);
        let target_class_name = match class_entry {
            Some(crate::class_file::ConstantPoolEntry::ConstantClass { name_index }) => {
                class.get_string(*name_index).unwrap_or_default()
            }
            _ => return Err("Invalid class reference".to_string()),
        };

        // Get the method name and descriptor
        let name_and_type_entry = class.constant_pool.get(*name_and_type_index as usize);
        let (method_name, descriptor) = match name_and_type_entry {
            Some(crate::class_file::ConstantPoolEntry::ConstantNameAndType { name_index, descriptor_index }) => {
                let name = class.get_string(*name_index).unwrap_or_default();
                let desc = class.get_string(*descriptor_index).unwrap_or_default();
                (name, desc)
            }
            _ => return Err("Invalid NameAndType reference".to_string()),
        };

        // Handle same-class method invocation
        if target_class_name == class.get_class_name().unwrap_or_default() {
            // Find and execute the method
            if let Some(method) = class.find_method(&method_name, &descriptor) {
                return self.execute_method(class, method, frame);
            }
        }

        // For other methods, just skip
        Ok(())
    }

    /// Invoke a virtual method
    fn invoke_virtual(&mut self, class: &ClassFile, frame: &mut StackFrame, index: usize) -> InterpreterResult {
        let cp_entry = class.constant_pool.get(index)
            .ok_or_else(|| format!("Constant pool index {} out of bounds", index))?;

        let (class_index, name_and_type_index) = match cp_entry {
            crate::class_file::ConstantPoolEntry::ConstantMethodref { class_index, name_and_type_index } |
            crate::class_file::ConstantPoolEntry::ConstantInterfaceMethodref { class_index, name_and_type_index } => {
                (class_index, name_and_type_index)
            }
            _ => return Err("Expected MethodRef or InterfaceMethodRef constant".to_string()),
        };

        // Get the class name
        let class_entry = class.constant_pool.get(*class_index as usize);
        let target_class_name = match class_entry {
            Some(crate::class_file::ConstantPoolEntry::ConstantClass { name_index }) => {
                class.get_string(*name_index).unwrap_or_default()
            }
            _ => return Err("Invalid class reference".to_string()),
        };

        // Get the method name and descriptor
        let name_and_type_entry = class.constant_pool.get(*name_and_type_index as usize);
        let (method_name, descriptor) = match name_and_type_entry {
            Some(crate::class_file::ConstantPoolEntry::ConstantNameAndType { name_index, descriptor_index }) => {
                let name = class.get_string(*name_index).unwrap_or_default();
                let desc = class.get_string(*descriptor_index).unwrap_or_default();
                (name, desc)
            }
            _ => return Err("Invalid NameAndType reference".to_string()),
        };

        // Handle PrintStream.println
        if target_class_name == "java/io/PrintStream" && method_name == "println" {
            // For println, pop parameters first, then objectref
            // The object reference is at position n-1 (below parameters)
            if !frame.stack.is_empty() {
                let value = frame.pop()?;
                let _objectref = frame.pop()?;
                self.native_println(value)?;
            } else {
                // No-arg println
                let _objectref = frame.pop()?;
                println!();
            }
            return Ok(());
        }

        // Handle same-class method invocation
        if target_class_name == class.get_class_name().unwrap_or_default() {
            // Find and execute the method
            if let Some(method) = class.find_method(&method_name, &descriptor) {
                return self.execute_method(class, method, frame);
            }
        }

        // For other methods, just skip
        Ok(())
    }

    /// Execute a method
    fn execute_method(&mut self, class: &ClassFile, method: &MethodInfo, caller_frame: &mut StackFrame) -> InterpreterResult {
        // Find the Code attribute
        let code_attr = self.find_code_attribute(class, method)
            .ok_or_else(|| "Code attribute not found".to_string())?;

        let max_stack = self.read_u16(&code_attr.info, 0) as usize;
        let max_locals = self.read_u16(&code_attr.info, 2) as usize;
        let code_length = self.read_u32(&code_attr.info, 4) as usize;

        // Get method name
        let method_name = class.get_string(method.name_index).unwrap_or_else(|| "unknown".to_string());

        // Create a new frame for the method
        let mut frame = StackFrame::new(max_locals, max_stack, method_name.clone());

        // Parse the method descriptor to get parameter count
        let descriptor = class.get_string(method.descriptor_index).unwrap_or_default();
        let param_count = self.count_parameters(&descriptor);

        // Pop arguments from caller frame and push to new frame's locals
        for i in 0..param_count {
            let value = caller_frame.pop()?;
            frame.store_local(param_count - 1 - i, value)?;
        }

        // Execute the method bytecode
        let code = &code_attr.info[8..8 + code_length];
        self.execute_instructions(&mut frame, class, code)?;

        // Push return value (if any) to caller frame
        // For now, assume void methods or int methods that leave value on stack
        if !frame.stack.is_empty() {
            let return_value = frame.pop()?;
            caller_frame.push(return_value)?;
        }

        Ok(())
    }

    /// Count the number of parameters in a method descriptor
    fn count_parameters(&self, descriptor: &str) -> usize {
        // Descriptor format: "(params)return"
        // e.g., "(II)I" = two int params, returns int
        // e.g., "(FF)F" = two float params, returns float
        if let Some(params_end) = descriptor.find(')') {
            let params = &descriptor[1..params_end];
            let mut count = 0;
            let mut chars = params.chars();
            while let Some(c) = chars.next() {
                match c {
                    'I' | 'F' | 'B' | 'C' | 'S' | 'Z' => count += 1,
                    'J' | 'D' => count += 2, // long and double take 2 slots
                    'L' => {
                        count += 1;
                        // Skip until semicolon
                        while let Some(c) = chars.next() {
                            if c == ';' {
                                break;
                            }
                        }
                    }
                    '[' => {
                        // Array type - keep processing
                        while let Some(c) = chars.next() {
                            if c != '[' {
                                // Put back the non-[ character
                                // Actually, we can't put back, so handle differently
                                if c == 'L' {
                                    count += 1;
                                    // Skip until semicolon
                                    while let Some(c) = chars.next() {
                                        if c == ';' {
                                            break;
                                        }
                                    }
                                    break;
                                }
                                count += 1;
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }
            count
        } else {
            0
        }
    }

    /// Find the Code attribute in a method
    fn find_code_attribute<'a>(&self, _class: &ClassFile, method: &'a MethodInfo) -> Option<&'a AttributeInfo> {
        method.attributes.iter().find(|attr| {
            // We could check the name against class constant pool here
            // For now, just find an attribute that could be Code
            attr.info.len() >= 8 // Code attribute has at least 8 bytes (max_stack, max_locals, code_length)
        })
    }

    /// Native println implementation
    fn native_println(&mut self, value: Value) -> InterpreterResult {
        match value {
            Value::Int(i) => println!("{}", i),
            Value::Float(f) => println!("{}", f),
            Value::Reference(addr) => {
                // Try to get the string value from the heap
                if let Some(obj) = self.memory.heap.get_object(addr) {
                    // For string objects, try to print the actual value
                    if obj.class_name == "java/lang/String" {
                        // Check if we have a cached string value
                        if let Some(string_cache) = self.string_cache.get(&addr) {
                            println!("{}", string_cache);
                        } else {
                            println!("[String]");
                        }
                    } else {
                        println!("[{}]", obj.class_name);
                    }
                } else {
                    println!("null");
                }
            }
            Value::Null => println!("null"),
            _ => println!("{:?}", value),
        }
        Ok(())
    }

    /// Handle invokedynamic instruction
    fn handle_invokedynamic(&mut self, class: &ClassFile, frame: &mut StackFrame, index: usize) -> InterpreterResult {
        let cp_entry = class.constant_pool.get(index)
            .ok_or_else(|| format!("Constant pool index {} out of bounds", index))?;

        let (bootstrap_index, name_and_type_index) = match cp_entry {
            crate::class_file::ConstantPoolEntry::ConstantInvokeDynamic { bootstrap_method_attr_index, name_and_type_index } => {
                (bootstrap_method_attr_index, name_and_type_index)
            }
            _ => return Err("Expected InvokeDynamic constant".to_string()),
        };

        // Get the method name from NameAndType
        let name_and_type_entry = class.constant_pool.get(*name_and_type_index as usize);
        let method_name = match name_and_type_entry {
            Some(crate::class_file::ConstantPoolEntry::ConstantNameAndType { name_index, .. }) => {
                class.get_string(*name_index).unwrap_or_default()
            }
            _ => return Err("Invalid NameAndType in invokedynamic".to_string()),
        };

        // Handle string concatenation (makeConcatWithConstants)
        if method_name == "makeConcatWithConstants" {
            // For string concatenation, pop values from stack and concatenate them
            // This is a simplified implementation - real JVM uses bootstrap methods

            // Check if there's a value on the stack to concatenate
            let result = if !frame.stack.is_empty() {
                let value = frame.pop()?;
                match value {
                    Value::Int(i) => format!("{}", i),
                    Value::Float(f) => format!("{}", f),
                    Value::Double(d) => format!("{}", d),
                    Value::Long(l) => format!("{}", l),
                    _ => "[object]".to_string(),
                }
            } else {
                String::new()
            };

            // Create a string object on the heap with the result
            let addr = self.memory.heap.allocate("java/lang/String".to_string());
            // Cache the string value for printing
            self.string_cache.insert(addr, result);

            frame.push(Value::Reference(addr))?;
            return Ok(());
        }

        // For other invokedynamic calls, push a placeholder
        frame.push(Value::Null)?;
        Ok(())
    }

    /// Read a u16 from a byte slice at offset
    fn read_u16(&self, data: &[u8], offset: usize) -> u16 {
        ((data[offset] as u16) << 8) | (data[offset + 1] as u16)
    }

    /// Read a i16 from a byte slice at offset
    fn read_i16(&self, data: &[u8], offset: usize) -> i16 {
        ((data[offset] as i16) << 8) | (data[offset + 1] as i16)
    }

    /// Read a u32 from a byte slice at offset
    fn read_u32(&self, data: &[u8], offset: usize) -> u32 {
        ((data[offset] as u32) << 24) |
        ((data[offset + 1] as u32) << 16) |
        ((data[offset + 2] as u32) << 8) |
        (data[offset + 3] as u32)
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}
