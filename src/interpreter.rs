use crate::class_file::{AttributeInfo, ClassFile, MethodInfo};
use crate::class_loader::ClassLoader;
use crate::error::{to_runtime_error_enum, ClassLoadingError, JvmError, RuntimeError};
use crate::memory::{HeapArray, Memory, StackFrame, Value};
use std::collections::HashMap;
use std::path::Path;

/// Result type for interpreter operations
pub type InterpreterResult = Result<(), JvmError>;

/// JVM Interpreter
pub struct Interpreter {
    /// Class loader for loading classes
    class_loader: ClassLoader,
    /// Runtime memory
    memory: Memory,
    /// String value cache for heap objects
    string_cache: HashMap<u32, String>,
    /// Exception handler stack
    exception_handlers: Vec<ExceptionHandler>,
    /// Current exception (if any)
    current_exception: Option<RuntimeError>,
}

/// Exception handler information
#[derive(Debug, Clone)]
struct ExceptionHandler {
    /// Start PC of handler range
    start_pc: usize,
    /// End PC of handler range (exclusive)
    end_pc: usize,
    /// Handler PC
    handler_pc: usize,
    /// Exception type (class name) or None for catch-all
    catch_type: Option<String>,
}

impl Interpreter {
    /// Create a new interpreter with default classpath
    pub fn new() -> Self {
        Interpreter {
            class_loader: ClassLoader::new_default(),
            memory: Memory::new(),
            string_cache: HashMap::new(),
            exception_handlers: Vec::new(),
            current_exception: None,
        }
    }

    /// Create a new interpreter with custom classpath
    pub fn with_classpath(classpath: Vec<std::path::PathBuf>) -> Self {
        Interpreter {
            class_loader: ClassLoader::new(classpath),
            memory: Memory::new(),
            string_cache: HashMap::new(),
            exception_handlers: Vec::new(),
            current_exception: None,
        }
    }

    /// Load a class from a file (legacy method, use load_class_by_name for proper classpath resolution)
    pub fn load_class<P: AsRef<Path>>(&mut self, path: P) -> Result<(), JvmError> {
        let class_file = ClassFile::from_file(path).map_err(|e| {
            JvmError::ClassLoadingError(ClassLoadingError::ClassFileNotFound(format!(
                "Failed to load class: {:?}",
                e
            )))
        })?;
        let _class_name = class_file
            .get_class_name()
            .unwrap_or_else(|| "Unknown".to_string());
        // For backward compatibility, we still need to add to class loader cache
        // In a real implementation, we'd add it to the class loader
        Ok(())
    }

    /// Load a class by name using classpath resolution
    pub fn load_class_by_name(&mut self, class_name: &str) -> Result<(), JvmError> {
        self.class_loader.load_class(class_name)?;
        Ok(())
    }

    /// Get a loaded class by name
    pub fn get_class(&self, name: &str) -> Option<&ClassFile> {
        self.class_loader.get_class(name)
    }

    /// Throw an exception
    pub fn throw_exception(&mut self, error: RuntimeError) -> InterpreterResult {
        let error_clone = error.clone();
        self.current_exception = Some(error_clone);
        Err(to_runtime_error_enum(error))
    }

    /// Check if an exception is pending
    pub fn has_exception(&self) -> bool {
        self.current_exception.is_some()
    }

    /// Clear the current exception
    pub fn clear_exception(&mut self) {
        self.current_exception = None;
    }

    /// Get the current exception
    pub fn get_exception(&self) -> Option<&RuntimeError> {
        self.current_exception.as_ref()
    }

    /// Add an exception handler
    pub fn add_exception_handler(
        &mut self,
        start_pc: usize,
        end_pc: usize,
        handler_pc: usize,
        catch_type: Option<String>,
    ) {
        self.exception_handlers.push(ExceptionHandler {
            start_pc,
            end_pc,
            handler_pc,
            catch_type,
        });
    }

    /// Clear all exception handlers
    pub fn clear_exception_handlers(&mut self) {
        self.exception_handlers.clear();
    }

    /// Find an exception handler for the current PC and exception type
    pub fn find_exception_handler(&self, pc: usize, exception_type: &str) -> Option<usize> {
        for handler in &self.exception_handlers {
            if pc >= handler.start_pc && pc < handler.end_pc {
                match &handler.catch_type {
                    Some(catch_type) if catch_type == exception_type => {
                        return Some(handler.handler_pc)
                    }
                    None => return Some(handler.handler_pc), // catch-all handler
                    _ => continue,
                }
            }
        }
        None
    }

    /// Convert RuntimeError to exception class name
    fn error_to_exception_type(error: &RuntimeError) -> String {
        match error {
            RuntimeError::NullPointerException => "java/lang/NullPointerException".to_string(),
            RuntimeError::DivisionByZero => "java/lang/ArithmeticException".to_string(),
            RuntimeError::ArrayIndexOutOfBounds(_, _) => {
                "java/lang/ArrayIndexOutOfBoundsException".to_string()
            }
            RuntimeError::ClassNotFound(_) => "java/lang/ClassNotFoundException".to_string(),
            RuntimeError::ClassCastException(_, _) => "java/lang/ClassCastException".to_string(),
            RuntimeError::ArrayStoreException => "java/lang/ArrayStoreException".to_string(),
            RuntimeError::NegativeArraySizeException(_) => {
                "java/lang/NegativeArraySizeException".to_string()
            }
            RuntimeError::IllegalAccessException(_) => {
                "java/lang/IllegalAccessException".to_string()
            }
            RuntimeError::InstantiationException(_) => {
                "java/lang/InstantiationException".to_string()
            }
            RuntimeError::StringIndexOutOfBounds(_, _) => {
                "java/lang/StringIndexOutOfBoundsException".to_string()
            }
            RuntimeError::IllegalArgument(_) => "java/lang/IllegalArgumentException".to_string(),
            RuntimeError::IllegalState(_) => "java/lang/IllegalStateException".to_string(),
            _ => "java/lang/RuntimeException".to_string(),
        }
    }

    /// Run the main method of a class
    pub fn run_main(&mut self, class_name: &str) -> InterpreterResult {
        // Load the class using class loader
        self.class_loader.load_class(class_name)?;

        // Get the loaded class
        let class = self.class_loader.get_class(class_name).ok_or_else(|| {
            to_runtime_error_enum(RuntimeError::ClassNotFound(class_name.to_string()))
        })?;

        // Find the main method: public static void main(String[])
        let main_method = class
            .find_method("main", "([Ljava/lang/String;)V")
            .ok_or_else(|| {
                to_runtime_error_enum(RuntimeError::MethodNotFound(
                    class_name.to_string(),
                    "main([Ljava/lang/String;)V".to_string(),
                ))
            })?
            .clone();

        // Find the Code attribute
        let code_attr = self
            .find_code_attribute(&class, &main_method)
            .ok_or_else(|| {
                to_runtime_error_enum(RuntimeError::UnsupportedOperation(
                    "Code attribute not found".to_string(),
                ))
            })?
            .clone();

        // Create a stack frame for main
        let max_stack = self.read_u16(&code_attr.info, 0) as usize;
        let max_locals = self.read_u16(&code_attr.info, 2) as usize;
        let code_length = self.read_u32(&code_attr.info, 4) as usize;

        let mut frame = StackFrame::new(max_locals, max_stack, "main".to_string());

        // Parse and execute instructions
        let code = code_attr.info[8..8 + code_length].to_vec();

        // Clone the class to avoid borrowing issues
        let class_clone = class.clone();
        self.execute_instructions(&mut frame, &class_clone, &code)?;

        Ok(())
    }

    /// Execute bytecode instructions
    fn execute_instructions(
        &mut self,
        frame: &mut StackFrame,
        class: &ClassFile,
        code: &[u8],
    ) -> InterpreterResult {
        while frame.pc < code.len() {
            let opcode = code[frame.pc];
            frame.pc += 1;

            match opcode {
                // return: return void from method
                0xb1 => {
                    break; // Break out of the loop instead of returning
                }

                // ireturn: return int from method
                0xac => {
                    break; // Break out of the loop instead of returning
                }

                // freturn: return float from method
                0xae => {
                    break; // Break out of the loop instead of returning
                }

                // lreturn: return long from method
                0xad => {
                    break; // Break out of the loop instead of returning
                }

                // dreturn: return double from method
                0xaf => {
                    break; // Break out of the loop instead of returning
                }

                // areturn: return reference from method
                0xb0 => {
                    break; // Break out of the loop instead of returning
                }

                // nop: do nothing
                0x00 => {}

                // aconst_null: push null onto stack
                0x01 => {
                    frame.push(Value::Null)?;
                }

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

                // ldc_w: push item from constant pool (wide index)
                0x13 => {
                    let index = self.read_u16(code, frame.pc);
                    frame.pc += 2;
                    self.load_constant(frame, class, index)?;
                }

                // ldc2_w: push long or double from constant pool (wide index)
                0x14 => {
                    let index = self.read_u16(code, frame.pc);
                    frame.pc += 2;
                    if index as usize >= class.constant_pool.len() {
                        return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                            format!("Constant pool index {} out of bounds", index),
                        )));
                    }

                    let entry = &class.constant_pool[index as usize];
                    match entry {
                        crate::class_file::ConstantPoolEntry::ConstantLong { bytes } => {
                            frame.push(Value::Long(*bytes))?;
                        }
                        crate::class_file::ConstantPoolEntry::ConstantDouble { bytes } => {
                            frame.push(Value::Double(*bytes))?;
                        }
                        _ => {
                            return Err(to_runtime_error_enum(RuntimeError::UnsupportedOperation(
                                format!("Unsupported constant type for ldc2_w at index {}", index),
                            )));
                        }
                    }
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
                    return Err(to_runtime_error_enum(RuntimeError::Unimplemented(
                        "lload not implemented".to_string(),
                    )));
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
                    return Err(to_runtime_error_enum(RuntimeError::Unimplemented(
                        "dload not implemented".to_string(),
                    )));
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
                    return Err(to_runtime_error_enum(RuntimeError::Unimplemented(
                        "lload_0-3 not implemented".to_string(),
                    )));
                }

                // fload_0 through fload_3: load float from local variable
                0x22..=0x25 => {
                    let index = (opcode - 0x22) as usize;
                    let value = frame.load_local(index)?;
                    frame.push(value)?;
                }

                // dload_0 through dload_3 (not implemented)
                0x26..=0x29 => {
                    return Err(to_runtime_error_enum(RuntimeError::Unimplemented(
                        "dload_0-3 not implemented".to_string(),
                    )));
                }

                // aload_0 through aload_3: load reference from local variable
                0x2a..=0x2d => {
                    let index = (opcode - 0x2a) as usize;
                    let value = frame.load_local(index)?;
                    frame.push(value)?;
                }

                // dup: duplicate top stack value
                0x59 => {
                    let value = frame.peek()?.clone();
                    frame.push(value)?;
                }

                // dup_x1: duplicate top stack value and insert two values down
                0x5a => {
                    let value1 = frame.pop()?;
                    let value2 = frame.pop()?;
                    frame.push(value1.clone())?;
                    frame.push(value2)?;
                    frame.push(value1)?;
                }

                // dup_x2: duplicate top stack value and insert three values down
                0x5b => {
                    let value1 = frame.pop()?;
                    let value2 = frame.pop()?;
                    let value3 = frame.pop()?;
                    frame.push(value1.clone())?;
                    frame.push(value3)?;
                    frame.push(value2)?;
                    frame.push(value1)?;
                }

                // dup2: duplicate top two stack values
                0x5c => {
                    let value2 = frame.pop()?;
                    let value1 = frame.pop()?;
                    frame.push(value1.clone())?;
                    frame.push(value2.clone())?;
                    frame.push(value1)?;
                    frame.push(value2)?;
                }

                // swap: swap top two stack values
                0x5f => {
                    let value2 = frame.pop()?;
                    let value1 = frame.pop()?;
                    frame.push(value2)?;
                    frame.push(value1)?;
                }

                // pop: pop top stack value
                0x57 => {
                    frame.pop()?;
                }

                // pop2: pop top two stack values
                0x58 => {
                    frame.pop()?;
                    frame.pop()?;
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

                // i2b: convert int to byte
                0x91 => {
                    let value = frame.pop()?.as_int();
                    frame.push(Value::Int((value as i8) as i32))?;
                }

                // i2c: convert int to char
                0x92 => {
                    let value = frame.pop()?.as_int();
                    frame.push(Value::Int((value as u16) as i32))?;
                }

                // i2s: convert int to short
                0x93 => {
                    let value = frame.pop()?.as_int();
                    frame.push(Value::Int((value as i16) as i32))?;
                }

                // l2f: convert long to float
                0x89 => {
                    let value = frame.pop()?.as_long();
                    frame.push(Value::Float(value as f32))?;
                }

                // l2d: convert long to double
                0x8a => {
                    let value = frame.pop()?.as_long();
                    frame.push(Value::Double(value as f64))?;
                }

                // d2l: convert double to long
                0x8c => {
                    let value = frame.pop()?.as_double();
                    frame.push(Value::Long(value as i64))?;
                }

                // istore_0 through istore_3: store int into local variable
                0x3b..=0x3e => {
                    let index = (opcode - 0x3b) as usize;
                    let value = frame.pop()?;
                    frame.store_local(index, value)?;
                }

                // lstore_0 through lstore_3 (not implemented)
                0x3f..=0x42 => {
                    return Err(to_runtime_error_enum(RuntimeError::Unimplemented(
                        "lstore_0-3 not implemented".to_string(),
                    )));
                }

                // fstore_0 through fstore_3: store float into local variable
                0x43..=0x46 => {
                    let index = (opcode - 0x43) as usize;
                    let value = frame.pop()?;
                    frame.store_local(index, value)?;
                }

                // dstore_0 through dstore_3 (not implemented)
                0x47..=0x4a => {
                    return Err(to_runtime_error_enum(RuntimeError::Unimplemented(
                        "dstore_0-3 not implemented".to_string(),
                    )));
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
                        return self.throw_exception(RuntimeError::DivisionByZero);
                    }
                    frame.push(Value::Int(value1 / value2))?;
                }

                // fdiv: divide two floats
                0x6e => {
                    let value2 = frame.pop()?.as_float();
                    let value1 = frame.pop()?.as_float();
                    frame.push(Value::Float(value1 / value2))?;
                }

                // irem: remainder of int division
                0x70 => {
                    let value2 = frame.pop()?.as_int();
                    let value1 = frame.pop()?.as_int();
                    if value2 == 0 {
                        return self.throw_exception(RuntimeError::DivisionByZero);
                    }
                    frame.push(Value::Int(value1 % value2))?;
                }

                // frem: remainder of float division
                0x72 => {
                    let value2 = frame.pop()?.as_float();
                    let value1 = frame.pop()?.as_float();
                    frame.push(Value::Float(value1 % value2))?;
                }

                // ineg: negate int
                0x74 => {
                    let value = frame.pop()?.as_int();
                    frame.push(Value::Int(-value))?;
                }

                // fneg: negate float
                0x76 => {
                    let value = frame.pop()?.as_float();
                    frame.push(Value::Float(-value))?;
                }

                // ishl: shift left int
                0x78 => {
                    let shift = frame.pop()?.as_int() & 0x1f; // Only lower 5 bits matter
                    let value = frame.pop()?.as_int();
                    frame.push(Value::Int(value << shift))?;
                }

                // ishr: arithmetic shift right int
                0x7a => {
                    let shift = frame.pop()?.as_int() & 0x1f; // Only lower 5 bits matter
                    let value = frame.pop()?.as_int();
                    frame.push(Value::Int(value >> shift))?;
                }

                // iushr: logical shift right int
                0x7c => {
                    let shift = frame.pop()?.as_int() & 0x1f; // Only lower 5 bits matter
                    let value = frame.pop()?.as_int() as u32;
                    frame.push(Value::Int((value >> shift) as i32))?;
                }

                // iand: bitwise AND int
                0x7e => {
                    let value2 = frame.pop()?.as_int();
                    let value1 = frame.pop()?.as_int();
                    frame.push(Value::Int(value1 & value2))?;
                }

                // ior: bitwise OR int
                0x80 => {
                    let value2 = frame.pop()?.as_int();
                    let value1 = frame.pop()?.as_int();
                    frame.push(Value::Int(value1 | value2))?;
                }

                // ixor: bitwise XOR int
                0x82 => {
                    let value2 = frame.pop()?.as_int();
                    let value1 = frame.pop()?.as_int();
                    frame.push(Value::Int(value1 ^ value2))?;
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

                // iflt: branch if int is less than zero
                0x9b => {
                    let value = frame.pop()?.as_int();
                    let offset = self.read_i16(code, frame.pc);
                    frame.pc += 2;
                    if value < 0 {
                        frame.pc = (frame.pc as i32 + offset as i32 - 2) as usize;
                    }
                }

                // ifge: branch if int is greater than or equal to zero
                0x9c => {
                    let value = frame.pop()?.as_int();
                    let offset = self.read_i16(code, frame.pc);
                    frame.pc += 2;
                    if value >= 0 {
                        frame.pc = (frame.pc as i32 + offset as i32 - 2) as usize;
                    }
                }

                // ifgt: branch if int is greater than zero
                0x9d => {
                    let value = frame.pop()?.as_int();
                    let offset = self.read_i16(code, frame.pc);
                    frame.pc += 2;
                    if value > 0 {
                        frame.pc = (frame.pc as i32 + offset as i32 - 2) as usize;
                    }
                }

                // ifle: branch if int is less than or equal to zero
                0x9e => {
                    let value = frame.pop()?.as_int();
                    let offset = self.read_i16(code, frame.pc);
                    frame.pc += 2;
                    if value <= 0 {
                        frame.pc = (frame.pc as i32 + offset as i32 - 2) as usize;
                    }
                }

                // if_icmpeq: branch if int comparison is equal
                0x9f => {
                    let value2 = frame.pop()?.as_int();
                    let value1 = frame.pop()?.as_int();
                    let offset = self.read_i16(code, frame.pc);
                    frame.pc += 2;
                    if value1 == value2 {
                        frame.pc = (frame.pc as i32 + offset as i32 - 2) as usize;
                    }
                }

                // if_icmpne: branch if int comparison is not equal
                0xa0 => {
                    let value2 = frame.pop()?.as_int();
                    let value1 = frame.pop()?.as_int();
                    let offset = self.read_i16(code, frame.pc);
                    frame.pc += 2;
                    if value1 != value2 {
                        frame.pc = (frame.pc as i32 + offset as i32 - 2) as usize;
                    }
                }

                // if_icmpge: branch if int comparison is greater than or equal
                0xa2 => {
                    let value2 = frame.pop()?.as_int();
                    let value1 = frame.pop()?.as_int();
                    let offset = self.read_i16(code, frame.pc);
                    frame.pc += 2;
                    if value1 >= value2 {
                        frame.pc = (frame.pc as i32 + offset as i32 - 2) as usize;
                    }
                }

                // if_icmpgt: branch if int comparison is greater than
                0xa3 => {
                    let value2 = frame.pop()?.as_int();
                    let value1 = frame.pop()?.as_int();
                    let offset = self.read_i16(code, frame.pc);
                    frame.pc += 2;
                    if value1 > value2 {
                        frame.pc = (frame.pc as i32 + offset as i32 - 2) as usize;
                    }
                }

                // if_icmple: branch if int comparison is less than or equal
                0xa4 => {
                    let value2 = frame.pop()?.as_int();
                    let value1 = frame.pop()?.as_int();
                    let offset = self.read_i16(code, frame.pc);
                    frame.pc += 2;
                    if value1 <= value2 {
                        frame.pc = (frame.pc as i32 + offset as i32 - 2) as usize;
                    }
                }

                // fcmpl: compare two floats, push -1 if NaN
                0x95 => {
                    let value2 = frame.pop()?.as_float();
                    let value1 = frame.pop()?.as_float();
                    let result = if value1.is_nan() || value2.is_nan() {
                        -1
                    } else if value1 > value2 {
                        1
                    } else if value1 < value2 {
                        -1
                    } else {
                        0
                    };
                    frame.push(Value::Int(result))?;
                }

                // fcmpg: compare two floats, push 1 if NaN
                0x96 => {
                    let value2 = frame.pop()?.as_float();
                    let value1 = frame.pop()?.as_float();
                    let result = if value1.is_nan() || value2.is_nan() {
                        1
                    } else if value1 > value2 {
                        1
                    } else if value1 < value2 {
                        -1
                    } else {
                        0
                    };
                    frame.push(Value::Int(result))?;
                }

                // lcmp: compare two longs
                0x94 => {
                    let value2 = frame.pop()?.as_long();
                    let value1 = frame.pop()?.as_long();
                    let result = if value1 > value2 {
                        1
                    } else if value1 < value2 {
                        -1
                    } else {
                        0
                    };
                    frame.push(Value::Int(result))?;
                }

                // goto: branch always
                0xa7 => {
                    let offset = self.read_i16(code, frame.pc);
                    frame.pc += 2;
                    frame.pc = (frame.pc as i32 + offset as i32 - 2) as usize;
                }

                // invokeinterface: invoke interface method
                0xb9 => {
                    let index = self.read_u16(code, frame.pc) as usize;
                    frame.pc += 2;
                    let _count = code[frame.pc];
                    frame.pc += 1;
                    let zero = code[frame.pc];
                    frame.pc += 1;
                    if zero != 0 {
                        return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                            "invokeinterface fourth byte must be zero".to_string(),
                        )));
                    }

                    // Get the interface method reference
                    let cp_entry = class.constant_pool.get(index).ok_or_else(|| {
                        to_runtime_error_enum(RuntimeError::IllegalArgument(format!(
                            "Constant pool index {} out of bounds",
                            index
                        )))
                    })?;

                    let (interface_index, name_and_type_index) = match cp_entry {
                        crate::class_file::ConstantPoolEntry::ConstantInterfaceMethodref {
                            class_index,
                            name_and_type_index,
                        } => (class_index, name_and_type_index),
                        _ => {
                            return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                                "Expected InterfaceMethodRef constant".to_string(),
                            )))
                        }
                    };

                    // Get the interface name
                    let interface_entry = class.constant_pool.get(*interface_index as usize);
                    let interface_name = match interface_entry {
                        Some(crate::class_file::ConstantPoolEntry::ConstantClass {
                            name_index,
                        }) => class.get_string(*name_index).unwrap_or_default(),
                        _ => {
                            return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                                "Invalid interface reference".to_string(),
                            )))
                        }
                    };

                    // Get the method name and descriptor
                    let name_and_type_entry =
                        class.constant_pool.get(*name_and_type_index as usize);
                    let (method_name, descriptor) = match name_and_type_entry {
                        Some(crate::class_file::ConstantPoolEntry::ConstantNameAndType {
                            name_index,
                            descriptor_index,
                        }) => {
                            let name = class.get_string(*name_index).unwrap_or_default();
                            let desc = class.get_string(*descriptor_index).unwrap_or_default();
                            (name, desc)
                        }
                        _ => {
                            return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                                "Invalid NameAndType reference".to_string(),
                            )))
                        }
                    };

                    // Try to resolve the interface method
                    if let Some((resolved_class_name, _)) =
                        self.resolve_interface_method(&interface_name, &method_name, &descriptor)
                    {
                        // Get the class and method without holding the borrow
                        if let Some(class_ref) = self.class_loader.get_class(&resolved_class_name) {
                            let method_name_clone = method_name.clone();
                            let descriptor_clone = descriptor.clone();
                            let class_clone = class_ref.clone();

                            if let Some(method) =
                                class_clone.find_method(&method_name_clone, &descriptor_clone)
                            {
                                return self.execute_method(&class_clone, method, frame);
                            }
                        }
                    }

                    // Interface method not found
                    return Err(to_runtime_error_enum(RuntimeError::MethodNotFound(
                        interface_name,
                        method_name,
                    )));
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
                        return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                            "invokedynamic last two bytes must be zero".to_string(),
                        )));
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

                // newarray: create new array of primitive type
                0xbc => {
                    let atype = code[frame.pc];
                    frame.pc += 1;
                    let count = frame.pop()?.as_int();

                    if count < 0 {
                        return self
                            .throw_exception(RuntimeError::NegativeArraySizeException(count));
                    }

                    let array = match atype {
                        4 => HeapArray::BooleanArray(vec![false; count as usize]),
                        5 => HeapArray::CharArray(vec![0; count as usize]),
                        6 => HeapArray::FloatArray(vec![0.0; count as usize]),
                        7 => HeapArray::DoubleArray(vec![0.0; count as usize]),
                        8 => HeapArray::ByteArray(vec![0; count as usize]),
                        9 => HeapArray::ShortArray(vec![0; count as usize]),
                        10 => HeapArray::IntArray(vec![0; count as usize]),
                        11 => HeapArray::LongArray(vec![0; count as usize]),
                        _ => {
                            return self.throw_exception(RuntimeError::IllegalArgument(format!(
                                "Unknown array type: {}",
                                atype
                            )))
                        }
                    };

                    let addr = self.memory.heap.allocate_array(array);
                    frame.push(Value::ArrayRef(addr))?;
                }

                // anewarray: create new array of references
                0xbd => {
                    let _index = self.read_u16(code, frame.pc);
                    frame.pc += 2;
                    let count = frame.pop()?.as_int();

                    if count < 0 {
                        return self
                            .throw_exception(RuntimeError::NegativeArraySizeException(count));
                    }

                    let array = HeapArray::ReferenceArray(vec![0; count as usize]);
                    let addr = self.memory.heap.allocate_array(array);
                    frame.push(Value::ArrayRef(addr))?;
                }

                // arraylength: get length of array
                0xbe => {
                    let array_ref = frame.pop()?;
                    let addr = array_ref.as_reference().ok_or_else(|| {
                        to_runtime_error_enum(RuntimeError::InvalidTypeConversion(
                            "not a reference".to_string(),
                            "array reference".to_string(),
                        ))
                    })?;

                    let length = self.memory.heap.array_length(addr).map_err(|e| {
                        to_runtime_error_enum(RuntimeError::UnsupportedOperation(format!(
                            "Array length error: {}",
                            e
                        )))
                    })?;

                    frame.push(Value::Int(length as i32))?;
                }

                // iaload: load int from array
                0x2e => {
                    let index = frame.pop()?.as_int() as usize;
                    let array_ref = frame.pop()?;
                    let addr = array_ref.as_reference().ok_or_else(|| {
                        to_runtime_error_enum(RuntimeError::InvalidTypeConversion(
                            "not a reference".to_string(),
                            "array reference".to_string(),
                        ))
                    })?;

                    let value = self.memory.heap.array_get(addr, index).map_err(|e| {
                        to_runtime_error_enum(RuntimeError::UnsupportedOperation(format!(
                            "Array get error: {}",
                            e
                        )))
                    })?;

                    frame.push(value)?;
                }

                // iastore: store int into array
                0x4f => {
                    let value = frame.pop()?;
                    let index = frame.pop()?.as_int() as usize;
                    let array_ref = frame.pop()?;
                    let addr = array_ref.as_reference().ok_or_else(|| {
                        to_runtime_error_enum(RuntimeError::InvalidTypeConversion(
                            "not a reference".to_string(),
                            "array reference".to_string(),
                        ))
                    })?;

                    self.memory
                        .heap
                        .array_set(addr, index, value)
                        .map_err(|e| {
                            to_runtime_error_enum(RuntimeError::UnsupportedOperation(format!(
                                "Array set error: {}",
                                e
                            )))
                        })?;
                }

                // baload: load byte/boolean from array
                0x33 => {
                    let index = frame.pop()?.as_int() as usize;
                    let array_ref = frame.pop()?;
                    let addr = array_ref.as_reference().ok_or_else(|| {
                        to_runtime_error_enum(RuntimeError::InvalidTypeConversion(
                            "not a reference".to_string(),
                            "array reference".to_string(),
                        ))
                    })?;

                    let value = self.memory.heap.array_get(addr, index).map_err(|e| {
                        to_runtime_error_enum(RuntimeError::UnsupportedOperation(format!(
                            "Array get error: {}",
                            e
                        )))
                    })?;

                    frame.push(value)?;
                }

                // bastore: store byte/boolean into array
                0x54 => {
                    let value = frame.pop()?;
                    let index = frame.pop()?.as_int() as usize;
                    let array_ref = frame.pop()?;
                    let addr = array_ref.as_reference().ok_or_else(|| {
                        to_runtime_error_enum(RuntimeError::InvalidTypeConversion(
                            "not a reference".to_string(),
                            "array reference".to_string(),
                        ))
                    })?;

                    self.memory
                        .heap
                        .array_set(addr, index, value)
                        .map_err(|e| {
                            to_runtime_error_enum(RuntimeError::UnsupportedOperation(format!(
                                "Array set error: {}",
                                e
                            )))
                        })?;
                }

                // caload: load char from array
                0x34 => {
                    let index = frame.pop()?.as_int() as usize;
                    let array_ref = frame.pop()?;
                    let addr = array_ref.as_reference().ok_or_else(|| {
                        to_runtime_error_enum(RuntimeError::InvalidTypeConversion(
                            "not a reference".to_string(),
                            "array reference".to_string(),
                        ))
                    })?;

                    let value = self.memory.heap.array_get(addr, index).map_err(|e| {
                        to_runtime_error_enum(RuntimeError::UnsupportedOperation(format!(
                            "Array get error: {}",
                            e
                        )))
                    })?;

                    frame.push(value)?;
                }

                // castore: store char into array
                0x55 => {
                    let value = frame.pop()?;
                    let index = frame.pop()?.as_int() as usize;
                    let array_ref = frame.pop()?;
                    let addr = array_ref.as_reference().ok_or_else(|| {
                        to_runtime_error_enum(RuntimeError::InvalidTypeConversion(
                            "not a reference".to_string(),
                            "array reference".to_string(),
                        ))
                    })?;

                    self.memory
                        .heap
                        .array_set(addr, index, value)
                        .map_err(|e| {
                            to_runtime_error_enum(RuntimeError::UnsupportedOperation(format!(
                                "Array set error: {}",
                                e
                            )))
                        })?;
                }

                // saload: load short from array
                0x35 => {
                    let index = frame.pop()?.as_int() as usize;
                    let array_ref = frame.pop()?;
                    let addr = array_ref.as_reference().ok_or_else(|| {
                        to_runtime_error_enum(RuntimeError::InvalidTypeConversion(
                            "not a reference".to_string(),
                            "array reference".to_string(),
                        ))
                    })?;

                    let value = self.memory.heap.array_get(addr, index).map_err(|e| {
                        to_runtime_error_enum(RuntimeError::UnsupportedOperation(format!(
                            "Array get error: {}",
                            e
                        )))
                    })?;

                    frame.push(value)?;
                }

                // sastore: store short into array
                0x56 => {
                    let value = frame.pop()?;
                    let index = frame.pop()?.as_int() as usize;
                    let array_ref = frame.pop()?;
                    let addr = array_ref.as_reference().ok_or_else(|| {
                        to_runtime_error_enum(RuntimeError::InvalidTypeConversion(
                            "not a reference".to_string(),
                            "array reference".to_string(),
                        ))
                    })?;

                    self.memory
                        .heap
                        .array_set(addr, index, value)
                        .map_err(|e| {
                            to_runtime_error_enum(RuntimeError::UnsupportedOperation(format!(
                                "Array set error: {}",
                                e
                            )))
                        })?;
                }

                // faload: load float from array
                0x30 => {
                    let index = frame.pop()?.as_int() as usize;
                    let array_ref = frame.pop()?;
                    let addr = array_ref.as_reference().ok_or_else(|| {
                        to_runtime_error_enum(RuntimeError::InvalidTypeConversion(
                            "not a reference".to_string(),
                            "array reference".to_string(),
                        ))
                    })?;

                    let value = self.memory.heap.array_get(addr, index).map_err(|e| {
                        to_runtime_error_enum(RuntimeError::UnsupportedOperation(format!(
                            "Array get error: {}",
                            e
                        )))
                    })?;

                    frame.push(value)?;
                }

                // fastore: store float into array
                0x51 => {
                    let value = frame.pop()?;
                    let index = frame.pop()?.as_int() as usize;
                    let array_ref = frame.pop()?;
                    let addr = array_ref.as_reference().ok_or_else(|| {
                        to_runtime_error_enum(RuntimeError::InvalidTypeConversion(
                            "not a reference".to_string(),
                            "array reference".to_string(),
                        ))
                    })?;

                    self.memory
                        .heap
                        .array_set(addr, index, value)
                        .map_err(|e| {
                            to_runtime_error_enum(RuntimeError::UnsupportedOperation(format!(
                                "Array set error: {}",
                                e
                            )))
                        })?;
                }

                // aaload: load reference from array
                0x32 => {
                    let index = frame.pop()?.as_int() as usize;
                    let array_ref = frame.pop()?;
                    let addr = array_ref.as_reference().ok_or_else(|| {
                        to_runtime_error_enum(RuntimeError::InvalidTypeConversion(
                            "not a reference".to_string(),
                            "array reference".to_string(),
                        ))
                    })?;

                    let value = self.memory.heap.array_get(addr, index).map_err(|e| {
                        to_runtime_error_enum(RuntimeError::UnsupportedOperation(format!(
                            "Array get error: {}",
                            e
                        )))
                    })?;

                    frame.push(value)?;
                }

                // aastore: store reference into array
                0x53 => {
                    let value = frame.pop()?;
                    let index = frame.pop()?.as_int() as usize;
                    let array_ref = frame.pop()?;
                    let addr = array_ref.as_reference().ok_or_else(|| {
                        to_runtime_error_enum(RuntimeError::InvalidTypeConversion(
                            "not a reference".to_string(),
                            "array reference".to_string(),
                        ))
                    })?;

                    self.memory
                        .heap
                        .array_set(addr, index, value)
                        .map_err(|e| {
                            to_runtime_error_enum(RuntimeError::UnsupportedOperation(format!(
                                "Array set error: {}",
                                e
                            )))
                        })?;
                }

                // Unknown opcode
                _ => {
                    return Err(to_runtime_error_enum(RuntimeError::InvalidOpcode(opcode)));
                }
            }
        }

        Ok(())
    }

    /// Load a constant from the constant pool onto the stack
    fn load_constant(
        &mut self,
        frame: &mut StackFrame,
        class: &ClassFile,
        index: u16,
    ) -> InterpreterResult {
        if index as usize >= class.constant_pool.len() {
            return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                format!("Constant pool index {} out of bounds", index),
            )));
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
                        let string_value = String::from_utf8(bytes.clone())
                            .unwrap_or_else(|_| "[invalid utf8]".to_string());
                        let addr = self.memory.heap.allocate_string(string_value);
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
                return Err(to_runtime_error_enum(RuntimeError::UnsupportedOperation(
                    format!("Unsupported constant type at index {}", index),
                )));
            }
        }
        Ok(())
    }

    /// Invoke a static method
    fn invoke_static(
        &mut self,
        class: &ClassFile,
        frame: &mut StackFrame,
        index: usize,
    ) -> InterpreterResult {
        let cp_entry = class.constant_pool.get(index).ok_or_else(|| {
            to_runtime_error_enum(RuntimeError::IllegalArgument(format!(
                "Constant pool index {} out of bounds",
                index
            )))
        })?;

        let (class_index, name_and_type_index) = match cp_entry {
            crate::class_file::ConstantPoolEntry::ConstantMethodref {
                class_index,
                name_and_type_index,
            } => (class_index, name_and_type_index),
            _ => {
                return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                    "Expected MethodRef constant".to_string(),
                )))
            }
        };

        // Get the class name
        let class_entry = class.constant_pool.get(*class_index as usize);
        let target_class_name = match class_entry {
            Some(crate::class_file::ConstantPoolEntry::ConstantClass { name_index }) => {
                class.get_string(*name_index).unwrap_or_default()
            }
            _ => {
                return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                    "Invalid class reference".to_string(),
                )))
            }
        };

        // Get the method name and descriptor
        let name_and_type_entry = class.constant_pool.get(*name_and_type_index as usize);
        let (method_name, descriptor) = match name_and_type_entry {
            Some(crate::class_file::ConstantPoolEntry::ConstantNameAndType {
                name_index,
                descriptor_index,
            }) => {
                let name = class.get_string(*name_index).unwrap_or_default();
                let desc = class.get_string(*descriptor_index).unwrap_or_default();
                (name, desc)
            }
            _ => {
                return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                    "Invalid NameAndType reference".to_string(),
                )))
            }
        };

        // Try to resolve the method
        if let Some((resolved_class_name, _)) =
            self.resolve_method(&target_class_name, &method_name, &descriptor)
        {
            // Get the class and method without holding the borrow
            if let Some(class_ref) = self.class_loader.get_class(&resolved_class_name) {
                let method_name_clone = method_name.clone();
                let descriptor_clone = descriptor.clone();
                let class_clone = class_ref.clone();

                if let Some(method) = class_clone.find_method(&method_name_clone, &descriptor_clone)
                {
                    return self.execute_method(&class_clone, method, frame);
                }
            }
        }

        // Method not found
        Err(to_runtime_error_enum(RuntimeError::MethodNotFound(
            target_class_name,
            method_name,
        )))
    }

    /// Resolve a method through class hierarchy
    fn resolve_method(
        &self,
        class_name: &str,
        method_name: &str,
        descriptor: &str,
    ) -> Option<(String, String)> {
        // First, try to find the class
        let class = self.class_loader.get_class(class_name)?;

        // Try to find the method in this class
        if class.find_method(method_name, descriptor).is_some() {
            return Some((class_name.to_string(), method_name.to_string()));
        }

        // If not found, try the super class
        if let Some(super_class_name) = class.get_super_class_name() {
            return self.resolve_method(&super_class_name, method_name, descriptor);
        }

        // Method not found in hierarchy
        None
    }

    /// Resolve an interface method
    fn resolve_interface_method(
        &self,
        interface_name: &str,
        method_name: &str,
        descriptor: &str,
    ) -> Option<(String, String)> {
        // For now, we'll just treat interface methods like regular methods
        // In a real implementation, we'd need to check all implemented interfaces
        self.resolve_method(interface_name, method_name, descriptor)
    }

    /// Invoke a virtual method
    fn invoke_virtual(
        &mut self,
        class: &ClassFile,
        frame: &mut StackFrame,
        index: usize,
    ) -> InterpreterResult {
        let cp_entry = class.constant_pool.get(index).ok_or_else(|| {
            to_runtime_error_enum(RuntimeError::IllegalArgument(format!(
                "Constant pool index {} out of bounds",
                index
            )))
        })?;

        let (class_index, name_and_type_index) = match cp_entry {
            crate::class_file::ConstantPoolEntry::ConstantMethodref {
                class_index,
                name_and_type_index,
            }
            | crate::class_file::ConstantPoolEntry::ConstantInterfaceMethodref {
                class_index,
                name_and_type_index,
            } => (class_index, name_and_type_index),
            _ => {
                return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                    "Expected MethodRef or InterfaceMethodRef constant".to_string(),
                )))
            }
        };

        // Get the class name
        let class_entry = class.constant_pool.get(*class_index as usize);
        let target_class_name = match class_entry {
            Some(crate::class_file::ConstantPoolEntry::ConstantClass { name_index }) => {
                class.get_string(*name_index).unwrap_or_default()
            }
            _ => {
                return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                    "Invalid class reference".to_string(),
                )))
            }
        };

        // Get the method name and descriptor
        let name_and_type_entry = class.constant_pool.get(*name_and_type_index as usize);
        let (method_name, descriptor) = match name_and_type_entry {
            Some(crate::class_file::ConstantPoolEntry::ConstantNameAndType {
                name_index,
                descriptor_index,
            }) => {
                let name = class.get_string(*name_index).unwrap_or_default();
                let desc = class.get_string(*descriptor_index).unwrap_or_default();
                (name, desc)
            }
            _ => {
                return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                    "Invalid NameAndType reference".to_string(),
                )))
            }
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

        // Try to resolve the method through class hierarchy
        if let Some((resolved_class_name, _)) =
            self.resolve_method(&target_class_name, &method_name, &descriptor)
        {
            // Get the class and method without holding the borrow
            if let Some(class_ref) = self.class_loader.get_class(&resolved_class_name) {
                let method_name_clone = method_name.clone();
                let descriptor_clone = descriptor.clone();
                let class_clone = class_ref.clone();

                if let Some(method) = class_clone.find_method(&method_name_clone, &descriptor_clone)
                {
                    return self.execute_method(&class_clone, method, frame);
                }
            }
        }

        // Method not found
        Err(to_runtime_error_enum(RuntimeError::MethodNotFound(
            target_class_name,
            method_name,
        )))
    }

    /// Execute a method
    fn execute_method(
        &mut self,
        class: &ClassFile,
        method: &MethodInfo,
        caller_frame: &mut StackFrame,
    ) -> InterpreterResult {
        // Find the Code attribute
        let code_attr = self.find_code_attribute(class, method).ok_or_else(|| {
            to_runtime_error_enum(RuntimeError::UnsupportedOperation(
                "Code attribute not found".to_string(),
            ))
        })?;

        let max_stack = self.read_u16(&code_attr.info, 0) as usize;
        let max_locals = self.read_u16(&code_attr.info, 2) as usize;
        let code_length = self.read_u32(&code_attr.info, 4) as usize;

        // Get method name
        let method_name = class
            .get_string(method.name_index)
            .unwrap_or_else(|| "unknown".to_string());

        // Create a new frame for the method
        let mut frame = StackFrame::new(max_locals, max_stack, method_name.clone());

        // Parse the method descriptor to get parameter count
        let descriptor = class
            .get_string(method.descriptor_index)
            .unwrap_or_default();
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
    fn find_code_attribute<'a>(
        &self,
        _class: &ClassFile,
        method: &'a MethodInfo,
    ) -> Option<&'a AttributeInfo> {
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
            Value::Long(l) => println!("{}", l),
            Value::Double(d) => println!("{}", d),
            Value::Reference(addr) => {
                // Try to get the string value from the heap
                if let Some(obj) = self.memory.heap.get_object(addr) {
                    // For string objects, try to print the actual value
                    if obj.class_name == "java/lang/String" {
                        if let Some(string_data) = &obj.string_data {
                            println!("{}", string_data);
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
            Value::ArrayRef(addr) => {
                println!("[Array@{}]", addr);
            }
            Value::Null => println!("null"),
            _ => println!("{:?}", value),
        }
        Ok(())
    }

    /// Handle invokedynamic instruction
    fn handle_invokedynamic(
        &mut self,
        class: &ClassFile,
        frame: &mut StackFrame,
        index: usize,
    ) -> InterpreterResult {
        let cp_entry = class.constant_pool.get(index).ok_or_else(|| {
            to_runtime_error_enum(RuntimeError::IllegalArgument(format!(
                "Constant pool index {} out of bounds",
                index
            )))
        })?;

        let (_bootstrap_index, name_and_type_index) = match cp_entry {
            crate::class_file::ConstantPoolEntry::ConstantInvokeDynamic {
                bootstrap_method_attr_index,
                name_and_type_index,
            } => (bootstrap_method_attr_index, name_and_type_index),
            _ => {
                return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                    "Expected InvokeDynamic constant".to_string(),
                )))
            }
        };

        // Get the method name from NameAndType
        let name_and_type_entry = class.constant_pool.get(*name_and_type_index as usize);
        let method_name = match name_and_type_entry {
            Some(crate::class_file::ConstantPoolEntry::ConstantNameAndType {
                name_index, ..
            }) => class.get_string(*name_index).unwrap_or_default(),
            _ => {
                return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                    "Invalid NameAndType in invokedynamic".to_string(),
                )))
            }
        };

        // Handle string concatenation (makeConcatWithConstants)
        if method_name == "makeConcatWithConstants" {
            // For string concatenation, pop values from stack and concatenate them
            // This is a simplified implementation - real JVM uses bootstrap methods
            // We'll pop all arguments and concatenate them

            let mut result = String::new();

            // In a real implementation, we'd know how many arguments to pop
            // For now, pop one argument as a simple example
            if !frame.stack.is_empty() {
                let value = frame.pop()?;
                match value {
                    Value::Int(i) => result = format!("{}", i),
                    Value::Float(f) => result = format!("{}", f),
                    Value::Double(d) => result = format!("{}", d),
                    Value::Long(l) => result = format!("{}", l),
                    Value::Reference(addr) => {
                        if let Some(string_data) = self.memory.heap.get_string_data(addr) {
                            result = string_data.clone();
                        } else {
                            result = "[object]".to_string();
                        }
                    }
                    _ => result = "[object]".to_string(),
                }
            }

            // Create a string object on the heap with the result
            let addr = self.memory.heap.allocate_string(result);
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
        ((data[offset] as u32) << 24)
            | ((data[offset + 1] as u32) << 16)
            | ((data[offset + 2] as u32) << 8)
            | (data[offset + 3] as u32)
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}
