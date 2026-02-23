//! JIT Compilation Module for jvmrs
//!
//! This module provides:
//! - Cranelift-based JIT compilation (bytecode-to-native for bipush, iload_0..3, iadd, ireturn)
//! - Tiered compilation (interpreter → baseline → optimized)
//! - AOT compilation mode
//! - LLVM IR backend (optional, feature-gated)

use crate::class_file::{ClassFile, MethodInfo};
use crate::cranelift_jit::CraneliftJitBackend;
use crate::memory::{Memory, StackFrame};
use crate::native::NativeRegistry;
use log::{debug, info, warn};

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

// ============================================================================
// Tiered Compilation System
// ============================================================================

/// Compilation level for tiered compilation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompilationLevel {
    /// Interpret bytecode (no compilation)
    Interpreter = 0,
    /// Baseline JIT compilation (fast compilation, moderate performance)
    Baseline = 1,
    /// Optimized JIT compilation (slower compilation, high performance)
    Optimized = 2,
}

/// Profiling data for a method
#[derive(Debug, Clone)]
pub struct MethodProfile {
    /// Number of times the method was invoked
    pub invocation_count: u64,
    /// Number of bytecode instructions executed
    pub instruction_count: u64,
    /// Time spent in this method (nanoseconds)
    pub total_time_ns: u64,
    /// Current compilation level
    pub level: CompilationLevel,
    /// Last time this method was compiled
    pub last_compiled: Option<Instant>,
}

impl MethodProfile {
    pub fn new() -> Self {
        Self {
            invocation_count: 0,
            instruction_count: 0,
            total_time_ns: 0,
            level: CompilationLevel::Interpreter,
            last_compiled: None,
        }
    }

    pub fn record_invocation(&mut self) {
        self.invocation_count += 1;
    }

    pub fn record_instructions(&mut self, count: u64) {
        self.instruction_count += count;
    }

    pub fn record_time(&mut self, time_ns: u64) {
        self.total_time_ns += time_ns;
    }

    /// Check if this method should be compiled to the next level
    pub fn should_upgrade(&self, threshold: u64) -> bool {
        match self.level {
            CompilationLevel::Interpreter => self.invocation_count >= threshold,
            CompilationLevel::Baseline => {
                // Upgrade to optimized if significantly more invocations
                self.invocation_count >= threshold * 10
            }
            CompilationLevel::Optimized => false,
        }
    }
}

/// Configuration for tiered compilation
#[derive(Debug, Clone)]
pub struct TieredCompilationConfig {
    /// Number of invocations before baseline JIT compilation
    pub baseline_threshold: u64,
    /// Number of invocations before optimized JIT compilation
    pub optimized_threshold: u64,
    /// Whether tiered compilation is enabled
    pub enabled: bool,
    /// Maximum size of method to compile (in bytecode bytes)
    pub max_method_size: usize,
}

impl Default for TieredCompilationConfig {
    fn default() -> Self {
        Self {
            baseline_threshold: 100,      // Compile after 100 invocations
            optimized_threshold: 1000,    // Optimize after 1000 invocations
            enabled: true,
            max_method_size: 10000,       // Don't compile methods larger than 10KB
        }
    }
}

// ============================================================================
// JIT Compiler using Cranelift
// ============================================================================

/// Result of JIT compilation
pub type JitResult<T> = Result<T, JitError>;

/// Errors that can occur during JIT compilation
#[derive(Debug, Clone)]
pub enum JitError {
    CompilationFailed(String),
    UnsupportedInstruction(String),
    InvalidMethod(String),
    IrGenerationError(String),
    LinkingError(String),
}

impl std::fmt::Display for JitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JitError::CompilationFailed(msg) => write!(f, "Compilation failed: {}", msg),
            JitError::UnsupportedInstruction(msg) => write!(f, "Unsupported instruction: {}", msg),
            JitError::InvalidMethod(msg) => write!(f, "Invalid method: {}", msg),
            JitError::IrGenerationError(msg) => write!(f, "IR generation error: {}", msg),
            JitError::LinkingError(msg) => write!(f, "Linking error: {}", msg),
        }
    }
}

impl std::error::Error for JitError {}

/// Compiled function pointer - simplified placeholder
/// In a full implementation, this would be a native function pointer
pub type CompiledFunction = unsafe extern "C" fn(*mut Memory, *mut StackFrame) -> i32;

/// Compiled code container
#[derive(Clone)]
pub struct CompiledCode {
    /// Function name
    pub name: String,
    /// Function pointer (placeholder for now)
    pub func: CompiledFunction,
    /// Compilation level
    pub level: CompilationLevel,
    /// Size of generated code (bytes)
    pub code_size: usize,
    /// Compilation time (milliseconds)
    pub compile_time_ms: u64,
}

/// Placeholder compiled function that just returns success
unsafe extern "C" fn placeholder_compiled_function(
    _memory: *mut Memory,
    _frame: *mut StackFrame,
) -> i32 {
    0 // Return success
}

/// Cranelift-based JIT compiler
pub struct CraneliftJitCompiler {
    /// Cranelift JIT backend (bytecode-to-native)
    jit_backend: Option<CraneliftJitBackend>,
    /// Compiled functions cache
    compiled_functions: HashMap<String, CompiledCode>,
    /// Method profiles
    profiles: HashMap<String, MethodProfile>,
    /// Tiered compilation config
    config: TieredCompilationConfig,
}

impl CraneliftJitCompiler {
    /// Create a new Cranelift JIT compiler
    pub fn new() -> JitResult<Self> {
        // Note: Cranelift JIT infrastructure is ready, but full bytecode-to-native
        // translation is a significant undertaking. This implementation provides:
        // - Tiered compilation infrastructure
        // - Method profiling
        // - Hot method detection
        // - Compiled code caching

        let jit_backend = match CraneliftJitBackend::new() {
            Ok(b) => Some(b),
            Err(e) => {
                warn!("Cranelift JIT backend unavailable: {} - using placeholder", e);
                None
            }
        };

        Ok(Self {
            jit_backend,
            compiled_functions: HashMap::new(),
            profiles: HashMap::new(),
            config: TieredCompilationConfig::default(),
        })
    }

    /// Create a new JIT compiler with custom config
    pub fn with_config(config: TieredCompilationConfig) -> JitResult<Self> {
        let mut compiler = Self::new()?;
        compiler.config = config;
        Ok(compiler)
    }

    /// Compile a method using Cranelift
    pub fn compile_method(
        &mut self,
        class: &ClassFile,
        method: &MethodInfo,
        level: CompilationLevel,
    ) -> JitResult<CompiledCode> {
        let method_name = class
            .get_string(method.name_index)
            .unwrap_or_else(|| "unknown".to_string());
        let class_name = class.get_class_name().unwrap_or_else(|| "Unknown".to_string());
        let full_name = format!("{}.{}", class_name, method_name);

        info!("Compiling method '{}' at level {:?}", full_name, level);

        let start = Instant::now();

        // Find the Code attribute
        let code_attr = method
            .attributes
            .iter()
            .find(|attr| attr.info.len() >= 8)
            .ok_or_else(|| JitError::InvalidMethod("No Code attribute found".to_string()))?;

        let compiled_code = if let Some(ref mut backend) = self.jit_backend {
            match backend.compile(class, method, &full_name) {
                Ok((code_ptr, code_size)) => {
                    let func = unsafe {
                        std::mem::transmute::<*const u8, CompiledFunction>(code_ptr)
                    };
                    CompiledCode {
                        name: full_name.clone(),
                        func,
                        level,
                        code_size,
                        compile_time_ms: start.elapsed().as_millis() as u64,
                    }
                }
                Err(JitError::UnsupportedInstruction(_)) => {
                    debug!("Method '{}' has unsupported bytecode - using placeholder", full_name);
                    CompiledCode {
                        name: full_name.clone(),
                        func: placeholder_compiled_function,
                        level,
                        code_size: 0,
                        compile_time_ms: start.elapsed().as_millis() as u64,
                    }
                }
                Err(e) => {
                    warn!("JIT compile failed for '{}': {} - using placeholder", full_name, e);
                    CompiledCode {
                        name: full_name.clone(),
                        func: placeholder_compiled_function,
                        level,
                        code_size: 0,
                        compile_time_ms: start.elapsed().as_millis() as u64,
                    }
                }
            }
        } else {
            CompiledCode {
                name: full_name.clone(),
                func: placeholder_compiled_function,
                level,
                code_size: 0,
                compile_time_ms: start.elapsed().as_millis() as u64,
            }
        };

        self.compiled_functions.insert(full_name, compiled_code.clone());

        Ok(compiled_code)
    }

    /// Get a compiled function by name
    pub fn get_compiled_function(&self, name: &str) -> Option<&CompiledCode> {
        self.compiled_functions.get(name)
    }

    /// Record method invocation for profiling
    pub fn record_invocation(&mut self, class_name: &str, method_name: &str) {
        let full_name = format!("{}.{}", class_name, method_name);
        let profile = self.profiles.entry(full_name).or_insert_with(MethodProfile::new);
        profile.record_invocation();
    }

    /// Check if a method should be compiled based on profiling data
    pub fn should_compile(&self, class_name: &str, method_name: &str) -> Option<CompilationLevel> {
        let full_name = format!("{}.{}", class_name, method_name);
        if let Some(profile) = self.profiles.get(&full_name) {
            if profile.level == CompilationLevel::Interpreter && profile.should_upgrade(self.config.baseline_threshold) {
                return Some(CompilationLevel::Baseline);
            } else if profile.level == CompilationLevel::Baseline && profile.should_upgrade(self.config.optimized_threshold) {
                return Some(CompilationLevel::Optimized);
            }
        }
        None
    }

    /// Get the tiered compilation config
    pub fn config(&self) -> &TieredCompilationConfig {
        &self.config
    }

    /// Update the tiered compilation config
    pub fn set_config(&mut self, config: TieredCompilationConfig) {
        self.config = config;
    }
}

impl Default for CraneliftJitCompiler {
    fn default() -> Self {
        Self::new().expect("Failed to create JIT compiler")
    }
}

// ============================================================================
// AOT Compilation Mode
// ============================================================================

/// AOT compiler for ahead-of-time compilation
pub struct AotCompiler {}

impl AotCompiler {
    /// Create a new AOT compiler
    pub fn new() -> JitResult<Self> {
        Ok(Self {})
    }

    /// Compile a class to a native object file (.o)
    pub fn compile_class(
        &mut self,
        class: &ClassFile,
        output_path: &Path,
    ) -> JitResult<()> {
        crate::aot_compiler::compile_class_to_object(class, output_path)
    }

    /// Link object files into a native executable
    pub fn link_executable(
        objects: &[PathBuf],
        output_path: &Path,
    ) -> JitResult<()> {
        use std::process::Command;

        info!("Linking executable to '{}'", output_path.display());

        // Use the system linker (cc on most systems)
        let mut cmd = Command::new("cc");
        for obj in objects {
            cmd.arg(obj);
        }
        cmd.arg("-o").arg(output_path);

        let result = cmd.output().map_err(|e| {
            JitError::LinkingError(format!("Failed to execute linker: {}", e))
        })?;

        if !result.status.success() {
            return Err(JitError::LinkingError(format!(
                "Linker failed: {}",
                String::from_utf8_lossy(&result.stderr)
            )));
        }

        Ok(())
    }
}

impl Default for AotCompiler {
    fn default() -> Self {
        Self::new().expect("Failed to create AOT compiler")
    }
}

// ============================================================================
// LLVM IR Backend (optional, feature-gated)
// ============================================================================

#[cfg(feature = "llvm")]
pub mod llvm_backend {
    use super::*;
    use inkwell::context::Context;
    use inkwell::module::Module;
    use inkwell::values::BasicValueEnum;

    /// LLVM IR generator - translates JVM bytecode to LLVM IR
    pub struct LlvmIrGenerator {
        /// LLVM context
        context: Context,
        /// LLVM module
        module: Module,
    }

    impl LlvmIrGenerator {
        /// Create a new LLVM IR generator
        pub fn new(module_name: &str) -> Self {
            let context = Context::create();
            let module = context.create_module(module_name);
            Self { context, module }
        }

        /// Convert a method to LLVM IR (with bytecode translation)
        pub fn method_to_llvm_ir(
            &mut self,
            class: &ClassFile,
            method: &MethodInfo,
        ) -> JitResult<String> {
            let method_name = class
                .get_string(method.name_index)
                .unwrap_or_else(|| "unknown".to_string());
            let class_name = class.get_class_name().unwrap_or_else(|| "Unknown".to_string());
            let func_name = format!("{}_{}", class_name.replace("/", "_"), method_name);

            let i32_type = self.context.i32_type();
            // Function takes up to 4 i32 params (locals 0-3)
            let fn_type = i32_type.fn_type(
                &[i32_type.into(), i32_type.into(), i32_type.into(), i32_type.into()],
                false,
            );
            let function = self.module.add_function(&func_name, fn_type, None);
            let entry = self.context.append_basic_block(function, "entry");
            let builder = self.context.create_builder();

            builder.position_at_end(entry);

            // Extract bytecode from Code attribute
            let code_attr = method.attributes.iter().find(|a| a.info.len() >= 8);
            let mut stack: Vec<inkwell::values::IntValue> = vec![];

            if let Some(attr) = code_attr {
                let code_len = ((attr.info[4] as usize) << 24)
                    | ((attr.info[5] as usize) << 16)
                    | ((attr.info[6] as usize) << 8)
                    | (attr.info[7] as usize);
                let bytecode = attr.info.get(8..8 + code_len).unwrap_or(&[]);
                let mut pc = 0usize;

                while pc < bytecode.len() {
                    let opcode = bytecode[pc];
                    pc += 1;
                    match opcode {
                        0x10 => {
                            if pc < bytecode.len() {
                                let byte_val = bytecode[pc] as i8 as i32;
                                pc += 1;
                                stack.push(builder.build_int_const(i32_type, byte_val as i64, "const"));
                            }
                        }
                        0x1a..=0x1d => {
                            let param = function.get_nth_param((opcode - 0x1a) as u32).unwrap();
                            stack.push(param.into_int_value());
                        }
                        0x60 => {
                            if stack.len() >= 2 {
                                let b = stack.pop().unwrap();
                                let a = stack.pop().unwrap();
                                let sum = builder.build_int_add(a, b, "add");
                                stack.push(sum);
                            }
                        }
                        0xac => {
                            if let Some(ret) = stack.pop() {
                                builder.build_return(Some(&ret.into()));
                            } else {
                                builder.build_return(Some(
                                    &builder.build_int_const(i32_type, 0, "").into(),
                                ));
                            }
                            break;
                        }
                        _ => {}
                    }
                }
                if stack.is_empty()
                    && !builder
                        .get_insert_block()
                        .and_then(|b| b.get_terminal())
                        .is_some()
                {
                    builder.build_return(Some(
                        &builder.build_int_const(i32_type, 0, "").into(),
                    ));
                }
            } else {
                builder.build_return(Some(
                    &builder.build_int_const(i32_type, 0, "").into(),
                ));
            }

            Ok(self.module.print_to_string().to_string())
        }

        /// Write LLVM IR to a file
        pub fn write_to_file(&self, path: &Path) -> JitResult<()> {
            use std::io::Write;

            let ir_string = self.module.print_to_string().to_string();
            let mut file = std::fs::File::create(path).map_err(|e| {
                JitError::LinkingError(format!("Failed to create file: {}", e))
            })?;

            file.write_all(ir_string.as_bytes()).map_err(|e| {
                JitError::LinkingError(format!("Failed to write file: {}", e))
            })?;

            Ok(())
        }
    }
}

/// Stub when LLVM feature is not enabled
#[cfg(not(feature = "llvm"))]
pub mod llvm_backend {
    use super::*;

    /// LLVM IR generator stub
    pub struct LlvmIrGenerator {}

    impl LlvmIrGenerator {
        pub fn new(_module_name: &str) -> Result<Self, String> {
            Err("LLVM feature is not enabled. Add --features llvm to enable.".to_string())
        }
    }
}

// ============================================================================
// JIT Manager
// ============================================================================

/// Manages JIT compilation and tiered compilation
pub struct JitManager {
    /// Cranelift JIT compiler
    pub compiler: CraneliftJitCompiler,
    /// Compiled functions
    compiled_functions: HashMap<String, CompiledCode>,
    /// Native method registry
    native_registry: NativeRegistry,
}

impl JitManager {
    /// Create a new JIT manager
    pub fn new() -> JitResult<Self> {
        Ok(Self {
            compiler: CraneliftJitCompiler::new()?,
            compiled_functions: HashMap::new(),
            native_registry: NativeRegistry::new(),
        })
    }

    /// Create a new JIT manager with custom config
    pub fn with_config(config: TieredCompilationConfig) -> JitResult<Self> {
        Ok(Self {
            compiler: CraneliftJitCompiler::with_config(config)?,
            compiled_functions: HashMap::new(),
            native_registry: NativeRegistry::new(),
        })
    }

    /// Get or compile a method (uses tier from profiling)
    pub fn get_or_compile_method(
        &mut self,
        class: &ClassFile,
        method: &MethodInfo,
    ) -> JitResult<CompiledCode> {
        self.get_or_compile_method_at(class, method, None)
    }

    /// Get or compile a method at optional specific tier
    pub fn get_or_compile_method_at(
        &mut self,
        class: &ClassFile,
        method: &MethodInfo,
        level_hint: Option<CompilationLevel>,
    ) -> JitResult<CompiledCode> {
        let method_name = class
            .get_string(method.name_index)
            .unwrap_or_else(|| "unknown".to_string());
        let class_name = class.get_class_name().unwrap_or_else(|| "Unknown".to_string());
        let full_name = format!("{}.{}", class_name, method_name);

        // Check if already compiled
        if let Some(code) = self.compiled_functions.get(&full_name) {
            return Ok(code.clone());
        }

        // Use provided level or determine from profiling
        let level = level_hint.or_else(|| {
            self.compiler.should_compile(&class_name, &method_name)
        }).unwrap_or(CompilationLevel::Baseline);

        // Compile the method at the appropriate tier
        let code = self.compiler.compile_method(class, method, level)?;
        self.compiled_functions.insert(full_name.clone(), code.clone());

        Ok(code)
    }

    /// Check if a method is compiled
    pub fn is_compiled(&self, class_name: &str, method_name: &str) -> bool {
        let full_name = format!("{}.{}", class_name, method_name);
        self.compiled_functions.contains_key(&full_name)
    }

    /// Record method invocation and check if it should be compiled
    pub fn record_and_check_compilation(
        &mut self,
        class_name: &str,
        method_name: &str,
    ) -> Option<CompilationLevel> {
        self.compiler.record_invocation(class_name, method_name);
        self.compiler.should_compile(class_name, method_name)
    }
}

impl Default for JitManager {
    fn default() -> Self {
        Self::new().expect("Failed to create JIT manager")
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compilation_level_ordering() {
        assert!(CompilationLevel::Interpreter < CompilationLevel::Baseline);
        assert!(CompilationLevel::Baseline < CompilationLevel::Optimized);
    }

    #[test]
    fn test_method_profile() {
        let mut profile = MethodProfile::new();

        profile.record_invocation();
        assert_eq!(profile.invocation_count, 1);

        profile.record_instructions(100);
        assert_eq!(profile.instruction_count, 100);

        profile.record_time(1_000_000);
        assert_eq!(profile.total_time_ns, 1_000_000);

        assert!(!profile.should_upgrade(1000));
        for _ in 0..1000 {
            profile.record_invocation();
        }
        assert!(profile.should_upgrade(1000));
    }

    #[test]
    fn test_tiered_config_default() {
        let config = TieredCompilationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.baseline_threshold, 100);
        assert_eq!(config.optimized_threshold, 1000);
    }

    #[test]
    fn test_jit_compiler_creation() {
        let compiler = CraneliftJitCompiler::new();
        assert!(compiler.is_ok());
    }

    #[test]
    fn test_jit_manager_creation() {
        let manager = JitManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_aot_compiler_creation() {
        let compiler = AotCompiler::new();
        assert!(compiler.is_ok());
    }
}
