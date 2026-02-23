use crate::class_file::{AttributeInfo, ClassFile, MethodInfo};
use crate::class_loader::ClassLoader;
use crate::debug::{debug_config_from_env, JvmDebugger};
use crate::error::{to_runtime_error_enum, ClassLoadingError, JvmError, RuntimeError};
use crate::jit::{CompilationLevel, JitManager, TieredCompilationConfig};
use crate::memory::{Memory, StackFrame, Value};
use crate::native::{init_builtins, NativeRegistry};
use super::reflection::ReflectionApi;
use log::info;

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
    /// Debugger for logging and tracing
    debugger: JvmDebugger,
    /// Current thread ID (simplified - in a real JVM this would be managed by thread system)
    current_thread_id: u32,
    /// Native method registry
    native_registry: NativeRegistry,
    /// Reflection API instance
    reflection_api: ReflectionApi,
    /// JIT manager for compilation
    jit_manager: Option<JitManager>,
    /// JIT compilation configuration
    jit_config: TieredCompilationConfig,
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
        let debug_config = debug_config_from_env();
        let debugger = JvmDebugger::new(debug_config);
        let mut memory = Memory::with_debugger(debugger.clone());
        let mut native_registry = NativeRegistry::new();
        init_builtins(&mut native_registry);
        let reflection_api = ReflectionApi::new();

        // Try to initialize JIT manager
        let jit_manager = JitManager::new().ok();
        let jit_config = TieredCompilationConfig::default();

        Interpreter {
            class_loader: ClassLoader::new_default(),
            memory,
            string_cache: HashMap::new(),
            exception_handlers: Vec::new(),
            current_exception: None,
            debugger,
            current_thread_id: 1, // Simplified: single-threaded execution
            native_registry,
            reflection_api,
            jit_manager,
            jit_config,
        }
    }

    /// Create a new interpreter with custom classpath
    pub fn with_classpath(classpath: Vec<std::path::PathBuf>) -> Self {
        let debug_config = debug_config_from_env();
        let debugger = JvmDebugger::new(debug_config);
        let memory = Memory::with_debugger(debugger.clone());
        let mut native_registry = NativeRegistry::new();
        init_builtins(&mut native_registry);
        let reflection_api = ReflectionApi::new();

        // Try to initialize JIT manager
        let jit_manager = JitManager::new().ok();
        let jit_config = TieredCompilationConfig::default();

        Interpreter {
            class_loader: ClassLoader::new(classpath),
            memory,
            string_cache: HashMap::new(),
            exception_handlers: Vec::new(),
            current_exception: None,
            debugger,
            current_thread_id: 1, // Simplified: single-threaded execution
            native_registry,
            reflection_api,
            jit_manager,
            jit_config,
        }
    }

    /// Create a new interpreter with JIT enabled and custom config
    pub fn with_jit(jit_config: TieredCompilationConfig) -> Self {
        let debug_config = debug_config_from_env();
        let debugger = JvmDebugger::new(debug_config);
        let mut memory = Memory::with_debugger(debugger.clone());
        let mut native_registry = NativeRegistry::new();
        init_builtins(&mut native_registry);
        let reflection_api = ReflectionApi::new();

        let jit_manager = JitManager::with_config(jit_config.clone()).ok();

        Interpreter {
            class_loader: ClassLoader::new_default(),
            memory,
            string_cache: HashMap::new(),
            exception_handlers: Vec::new(),
            current_exception: None,
            debugger,
            current_thread_id: 1,
            native_registry,
            reflection_api,
            jit_manager,
            jit_config,
        }
    }

    /// Check if JIT is enabled
    pub fn is_jit_enabled(&self) -> bool {
        self.jit_manager.is_some() && self.jit_config.enabled
    }

    /// Enable or disable JIT compilation
    pub fn set_jit_enabled(&mut self, enabled: bool) {
        if enabled && self.jit_manager.is_none() {
            self.jit_manager = JitManager::new().ok();
        } else if !enabled {
            self.jit_manager = None;
        }
        self.jit_config.enabled = enabled;
    }

    /// Get the JIT manager
    pub fn jit_manager(&mut self) -> Option<&mut JitManager> {
        self.jit_manager.as_mut()
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
        // For backward compatibility, we still need to add to class loader
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

    /// Get the reflection API instance
    pub fn get_reflection_api(&self) -> &ReflectionApi {
        &self.reflection_api
    }

    /// Run the main method of a class
    pub fn run_main(&mut self, class_name: &str) -> Result<(), JvmError> {
        // Load the class using class loader
        self.load_class_by_name(class_name)?;

        // Get the loaded class
        let class = self.class_loader.get_class(class_name).ok_or_else(|| {
            JvmError::ClassLoadingError(ClassLoadingError::NoClassDefFound(class_name.to_string()))
        })?;

        // Find the main method: public static void main(String[])
        let main_method = class
            .find_method("main", "([Ljava/lang/String;)V")
            .ok_or_else(|| {
                JvmError::RuntimeError(RuntimeError::MethodNotFound(
                    class_name.to_string(),
                    "main([Ljava/lang/String;)V".to_string(),
                ))
            })?
            .clone();

        // Find the Code attribute
        let code_attr = self
            .find_code_attribute(&class, &main_method)
            .ok_or_else(|| {
                JvmError::RuntimeError(RuntimeError::UnsupportedOperation(
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

        // Execute bytecode instructions in the main frame
        while frame.pc < code.len() {
            let opcode = code[frame.pc];
            frame.pc += 1;

            // Log instruction before execution
            self.debugger.log_instruction(&frame, &class_clone, opcode);

            // Simplified bytecode execution - just handle basic operations for now
            match opcode {
                0xb1 => {
                    // return
                    break;
                }
                // For now, skip other opcodes
                _ => {
                    // In a real implementation, we would handle all opcodes here
                    // For now, just skip based on opcode type
                    if opcode >= 0x01 && opcode <= 0x15 {
                        // Load constant
                        frame.pc += if opcode >= 0x12 { 2 } else { 0 };
                    } else if opcode >= 0x36 && opcode <= 0x3a {
                        // Load local
                        frame.pc += 1;
                    } else if opcode >= 0x60 && opcode <= 0x83 {
                        // Math operations
                        frame.stack.pop();
                        frame.stack.pop();
                        frame.push(Value::Int(0)); // Placeholder
                    } else if opcode >= 0xb2 && opcode <= 0xb8 {
                        // Field/method access
                        frame.pc += 2;
                    }
                    // Ignore others for now
                }
            }
        }

        Ok(())
    }

    /// Collect method arguments from the stack based on descriptor
    fn collect_method_args(
        &self,
        frame: &mut StackFrame,
        descriptor: &str,
    ) -> Result<Vec<Value>, JvmError> {
        // Parse descriptor to get parameter types
        let param_types = self.parse_method_params(descriptor);
        let mut args = Vec::new();

        // Pop arguments from stack in reverse order
        for _ in 0..param_types.len() {
            args.push(frame.pop()?);
        }

        // Reverse to get correct order
        args.reverse();
        Ok(args)
    }

    /// Parse method parameters from descriptor (simplified)
    fn parse_method_params(&self, descriptor: &str) -> Vec<String> {
        let mut params = Vec::new();
        let mut i = descriptor.find('(').unwrap_or(0) + 1;
        let end = descriptor.find(')').unwrap_or(descriptor.len());

        while i < end {
            match descriptor.chars().nth(i) {
                Some('B') => {
                    params.push("byte".to_string());
                    i += 1;
                }
                Some('C') => {
                    params.push("char".to_string());
                    i += 1;
                }
                Some('D') => {
                    params.push("double".to_string());
                    i += 1;
                }
                Some('F') => {
                    params.push("float".to_string());
                    i += 1;
                }
                Some('I') => {
                    params.push("int".to_string());
                    i += 1;
                }
                Some('J') => {
                    params.push("long".to_string());
                    i += 1;
                }
                Some('S') => {
                    params.push("short".to_string());
                    i += 1;
                }
                Some('Z') => {
                    params.push("boolean".to_string());
                    i += 1;
                }
                Some('L') => {
                    // Reference type - find the semicolon
                    let mut j = i + 1;
                    while j < end && descriptor.chars().nth(j) != Some(';') {
                        j += 1;
                    }
                    if j < end {
                        params.push("object".to_string());
                        i = j + 1;
                    } else {
                        break;
                    }
                }
                Some(')') => break,
                _ => i += 1,
            }
        }

        params
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
        // Get method and class names for profiling
        let method_name = class
            .get_string(method.name_index)
            .unwrap_or_else(|| "unknown".to_string());
        let class_name = class.get_class_name().unwrap_or("Unknown".to_string());

        // Record method invocation for hot method detection
        if let Some(jit_manager) = &mut self.jit_manager {
            // Check if we should compile this method
            if let Some(level) = jit_manager.record_and_check_compilation(&class_name, &method_name) {
                info!(
                    "Hot method detected: {}::{} - compiling at level {:?}",
                    class_name, method_name, level
                );

                // Compile the method at the detected tier
                if let Ok(compiled) = jit_manager.get_or_compile_method_at(class, method, Some(level)) {
                    info!(
                        "Method {}::{} compiled in {}ms ({} bytes)",
                        class_name, method_name, compiled.compile_time_ms, compiled.code_size
                    );
                }
            }
        }

        // Check if we have JIT compiled code for this method
        let full_method_name = format!("{}.{}", class_name, method_name);

        if let Some(jit_manager) = &self.jit_manager {
            if let Some(compiled_code) = jit_manager.compiler.get_compiled_function(&full_method_name) {
                info!("Executing JIT compiled method: {}", full_method_name);

                // Create a new frame for the method
                let code_attr = self.find_code_attribute(class, method).ok_or_else(|| {
                    to_runtime_error_enum(RuntimeError::UnsupportedOperation(
                        "Code attribute not found".to_string(),
                    ))
                })?;

                let max_locals = self.read_u16(&code_attr.info, 0) as usize;
                let max_stack = self.read_u16(&code_attr.info, 2) as usize;

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

                // Call the compiled function
                unsafe {
                    let result = (compiled_code.func)(
                        &mut self.memory as *mut Memory,
                        &mut frame as *mut StackFrame,
                    );

                    if result != 0 {
                        return Err(to_runtime_error_enum(RuntimeError::UnsupportedOperation(
                            format!("JIT compiled method returned error code: {}", result)
                        )));
                    }
                }

                // Push return value if any
                if !frame.stack.is_empty() {
                    let return_value = frame.pop()?;
                    caller_frame.push(return_value)?;
                }

                return Ok(());
            }
        }

        // Fallback to interpreter
        // Find the Code attribute
        let code_attr = self.find_code_attribute(class, method).ok_or_else(|| {
            to_runtime_error_enum(RuntimeError::UnsupportedOperation(
                "Code attribute not found".to_string(),
            ))
        })?;

        let max_stack = self.read_u16(&code_attr.info, 0) as usize;
        let max_locals = self.read_u16(&code_attr.info, 2) as usize;
        let code_length = self.read_u32(&code_attr.info, 4) as usize;

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

        // Create a copy of the class to avoid borrowing issues
        let class_clone = class.clone();

        // Execute bytecode instructions in the new frame
        while frame.pc < code.len() {
            let opcode = code[frame.pc];
            frame.pc += 1;

            // Log instruction before execution
            self.debugger.log_instruction(&frame, &class_clone, opcode);

            // Simplified bytecode execution - just handle basic operations for now
            match opcode {
                0xb1 => {
                    // return
                    break;
                }
                0xac => {
                    // ireturn
                    if !frame.stack.is_empty() {
                        let return_value = frame.pop()?;
                        caller_frame.push(return_value)?;
                    }
                    break;
                }
                0xb0 => {
                    // areturn
                    if !frame.stack.is_empty() {
                        let return_value = frame.pop()?;
                        caller_frame.push(return_value)?;
                    }
                    break;
                }
                // For now, skip other opcodes
                _ => {
                    // In a real implementation, we would handle all opcodes here
                    // For now, just skip
                    if opcode >= 0x01 && opcode <= 0x15 {
                        // Load constant
                        frame.pc += if opcode >= 0x12 { 2 } else { 0 };
                    } else if opcode >= 0x36 && opcode <= 0x3a {
                        // Load local
                        frame.pc += 1;
                    } else if opcode >= 0x60 && opcode <= 0x83 {
                        // Math operations
                        frame.stack.pop();
                        frame.stack.pop();
                        frame.push(Value::Int(0)); // Placeholder
                    } else if opcode >= 0xb2 && opcode <= 0xb8 {
                        // Field/method access
                        frame.pc += 2;
                    }
                    // Ignore others for now
                }
            }
        }

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
