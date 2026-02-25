//! JVM bytecode interpreter.
//!
//! Split into submodules:
//! - `descriptor` - Method descriptor parsing
//! - `utils` - Bytecode reading helpers
//! - `invocation` - Method invocation (invokevirtual, invokestatic, execute_method)
//! - `dispatch` - Instruction dispatch
//! - `builtins` - Native builtins (println, invokedynamic)

mod descriptor;
mod utils;

mod invocation;
mod dispatch;
mod builtins;

use crate::class_file::{AttributeInfo, ClassFile, MethodInfo};
use crate::class_loader::ClassLoader;
use crate::debug::{debug_config_from_env, JvmDebugger};
use crate::error::{ClassLoadingError, JvmError, RuntimeError};
use crate::jit::{JitManager, TieredCompilationConfig};
use crate::memory::{Memory, StackFrame, Value};
use crate::native::{init_builtins, NativeRegistry};
use crate::profiler::{ProfileGuard, Profiler};
use crate::deterministic::DeterministicConfig;
use crate::security::Sanitizer;
use crate::trace::TraceRecorder;
use crate::reflection::ReflectionApi;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Result type for interpreter operations
pub type InterpreterResult = Result<(), JvmError>;

/// JVM Interpreter
pub struct Interpreter {
    pub(crate) class_loader: ClassLoader,
    pub(crate) memory: Memory,
    pub(crate) string_cache: HashMap<u32, String>,
    pub(crate) exception_handlers: Vec<ExceptionHandler>,
    pub(crate) current_exception: Option<RuntimeError>,
    pub(crate) debugger: JvmDebugger,
    pub(crate) current_thread_id: u32,
    pub(crate) native_registry: NativeRegistry,
    pub(crate) reflection_api: ReflectionApi,
    pub(crate) jit_manager: Option<JitManager>,
    pub(crate) jit_config: TieredCompilationConfig,
    pub(crate) profiler: Option<Arc<Profiler>>,
    pub(crate) trace_recorder: Option<TraceRecorder>,
    pub(crate) sanitizer: Option<Arc<Sanitizer>>,
    pub(crate) deterministic_config: Option<DeterministicConfig>,
}

/// Exception handler information
#[derive(Debug, Clone)]
pub(crate) struct ExceptionHandler {
    pub start_pc: usize,
    pub end_pc: usize,
    pub handler_pc: usize,
    pub catch_type: Option<String>,
}

impl Interpreter {
    /// Create a new interpreter with default classpath
    pub fn new() -> Self {
        let debug_config = debug_config_from_env();
        let debugger = JvmDebugger::new(debug_config);
        let memory = Memory::with_debugger(debugger.clone());
        let mut native_registry = NativeRegistry::new();
        init_builtins(&mut native_registry);
        let reflection_api = ReflectionApi::new();

        let jit_manager = JitManager::new().ok();
        let jit_config = TieredCompilationConfig::default();

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
            profiler: None,
            trace_recorder: None,
            sanitizer: None,
            deterministic_config: None,
        }
    }

    /// Create a new interpreter with custom classpath
    pub fn with_classpath(classpath: Vec<PathBuf>) -> Self {
        let debug_config = debug_config_from_env();
        let debugger = JvmDebugger::new(debug_config);
        let memory = Memory::with_debugger(debugger.clone());
        let mut native_registry = NativeRegistry::new();
        init_builtins(&mut native_registry);
        let reflection_api = ReflectionApi::new();

        let jit_manager = JitManager::new().ok();
        let jit_config = TieredCompilationConfig::default();

        Interpreter {
            class_loader: ClassLoader::new(classpath),
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
            profiler: None,
            trace_recorder: None,
            sanitizer: None,
            deterministic_config: None,
        }
    }

    /// Create a new interpreter with JIT enabled and custom config
    pub fn with_jit(jit_config: TieredCompilationConfig) -> Self {
        let debug_config = debug_config_from_env();
        let debugger = JvmDebugger::new(debug_config);
        let memory = Memory::with_debugger(debugger.clone());
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
            profiler: None,
            trace_recorder: None,
            sanitizer: None,
            deterministic_config: None,
        }
    }

    /// Enable deterministic execution
    pub fn set_deterministic(&mut self, config: Option<DeterministicConfig>) {
        self.deterministic_config = config;
    }

    /// Enable profiling
    pub fn set_profiler(&mut self, profiler: Option<Arc<Profiler>>) {
        self.profiler = profiler;
    }

    /// Get profiler reference
    pub fn profiler(&self) -> Option<&Arc<Profiler>> {
        self.profiler.as_ref()
    }

    /// Enable trace recording
    pub fn set_trace_recorder(&mut self, mut recorder: Option<TraceRecorder>) {
        if let Some(ref mut r) = recorder {
            r.set_enabled(true);
        }
        self.trace_recorder = recorder;
    }

    /// Enable security sanitizer
    pub fn set_sanitizer(&mut self, sanitizer: Option<Arc<Sanitizer>>) {
        self.memory.set_sanitizer(sanitizer.clone());
        self.sanitizer = sanitizer;
    }

    /// Set class cache directory
    pub fn set_class_cache_dir(&mut self, path: Option<PathBuf>) {
        self.class_loader.set_cache_dir(path);
    }

    /// Get trace recorder reference
    pub fn trace_recorder(&self) -> Option<&TraceRecorder> {
        self.trace_recorder.as_ref()
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

    /// Load a class from a file (legacy)
    pub fn load_class<P: AsRef<Path>>(&mut self, path: P) -> Result<(), JvmError> {
        let _ = ClassFile::from_file(path).map_err(|e| {
            JvmError::ClassLoadingError(ClassLoadingError::ClassFileNotFound(format!(
                "Failed to load class: {:?}",
                e
            )))
        })?;
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

    /// Get reflection information for a loaded class
    pub fn get_class_reflection(&self, class_name: &str) -> Option<crate::reflection::ClassReflection> {
        let class = self.class_loader.get_class(class_name)?;
        Some(crate::reflection::class_to_reflection(class))
    }

    /// Run the main method of a class
    pub fn run_main(&mut self, class_name: &str) -> Result<(), JvmError> {
        self.load_class_by_name(class_name)?;

        let class = self.class_loader.get_class(class_name).ok_or_else(|| {
            JvmError::ClassLoadingError(ClassLoadingError::NoClassDefFound(class_name.to_string()))
        })?;

        let main_method = class
            .find_method("main", "([Ljava/lang/String;)V")
            .ok_or_else(|| {
                JvmError::RuntimeError(RuntimeError::MethodNotFound(
                    class_name.to_string(),
                    "main([Ljava/lang/String;)V".to_string(),
                ))
            })?
            .clone();

        let code_attr = self
            .find_code_attribute(class, &main_method)
            .ok_or_else(|| {
                JvmError::RuntimeError(RuntimeError::UnsupportedOperation(
                    "Code attribute not found".to_string(),
                ))
            })?
            .clone();

        let max_stack = utils::read_u16(&code_attr.info, 0) as usize;
        let max_locals = utils::read_u16(&code_attr.info, 2) as usize;
        let code_length = utils::read_u32(&code_attr.info, 4) as usize;

        let mut frame = StackFrame::new(max_locals, max_stack, "main".to_string());
        let code = code_attr.info[8..8 + code_length].to_vec();
        let class_clone = class.clone();

        while frame.pc < code.len() {
            let opcode = code[frame.pc];
            frame.pc += 1;

            self.debugger.log_instruction(&frame, &class_clone, opcode);

            if !self.dispatch_instruction(&class_clone, &code, &mut frame, opcode)? {
                break;
            }
        }

        Ok(())
    }

    /// Find the Code attribute in a method
    pub(crate) fn find_code_attribute<'a>(
        &self,
        _class: &ClassFile,
        method: &'a MethodInfo,
    ) -> Option<&'a AttributeInfo> {
        method.attributes.iter().find(|attr| attr.info.len() >= 8)
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}
