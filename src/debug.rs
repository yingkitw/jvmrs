use crate::class_file::ClassFile;
use crate::memory::{StackFrame, Value};
use log::{debug, error, info, trace, warn};

/// Configuration for debugging and logging
#[derive(Debug, Clone)]
pub struct DebugConfig {
    /// Enable instruction-level tracing
    pub trace_instructions: bool,
    /// Enable memory operation logging
    pub trace_memory: bool,
    /// Enable class loading logging
    pub trace_classes: bool,
    /// Enable method entry/exit logging
    pub trace_methods: bool,
    /// Maximum number of stack frames to log (None for all)
    pub max_stack_frames: Option<usize>,
    /// Maximum number of operand stack entries to log (None for all)
    pub max_stack_entries: Option<usize>,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            trace_instructions: false,
            trace_memory: false,
            trace_classes: true,
            trace_methods: true,
            max_stack_frames: Some(5),
            max_stack_entries: Some(10),
        }
    }
}

/// Debugger for JVM operations
pub struct JvmDebugger {
    config: DebugConfig,
    instruction_count: usize,
}

impl JvmDebugger {
    /// Create a new debugger with the given configuration
    pub fn new(config: DebugConfig) -> Self {
        Self {
            config,
            instruction_count: 0,
        }
    }

    /// Log the start of method execution
    pub fn log_method_entry(&self, class_name: &str, method_name: &str, method_descriptor: &str) {
        if self.config.trace_methods {
            info!(
                "Entering method: {}.{}{}",
                class_name, method_name, method_descriptor
            );
        }
    }

    /// Log the end of method execution
    pub fn log_method_exit(
        &self,
        class_name: &str,
        method_name: &str,
        method_descriptor: &str,
        result: Option<&Value>,
    ) {
        if self.config.trace_methods {
            match result {
                Some(value) => info!(
                    "Exiting method: {}.{}{} with return value: {:?}",
                    class_name, method_name, method_descriptor, value
                ),
                None => info!(
                    "Exiting method: {}.{}{} (void)",
                    class_name, method_name, method_descriptor
                ),
            }
        }
    }

    /// Log an instruction before execution
    pub fn log_instruction(&mut self, frame: &StackFrame, class: &ClassFile, opcode: u8) {
        if self.config.trace_instructions {
            self.instruction_count += 1;
            trace!(
                "Instruction #{}: 0x{:02x} ({})",
                self.instruction_count,
                opcode,
                self.get_opcode_name(opcode)
            );

            // Log current frame state
            trace!(
                "  Frame: {} (PC: {}, Local vars: {}, Stack depth: {})",
                frame.method_name,
                frame.pc,
                frame.local_vars.len(),
                frame.operand_stack.len()
            );

            // Log operand stack if configured
            if self.config.trace_memory {
                self.log_operand_stack(frame);
                self.log_local_variables(frame);
            }
        }
    }

    /// Log operand stack state
    fn log_operand_stack(&self, frame: &StackFrame) {
        let max_entries = self
            .config
            .max_stack_entries
            .unwrap_or(frame.operand_stack.len());
        let to_show = std::cmp::min(max_entries, frame.operand_stack.len());

        trace!(
            "    Operand Stack (showing {}/{}):",
            to_show,
            frame.operand_stack.len()
        );
        for (i, value) in frame.operand_stack.iter().rev().take(to_show).enumerate() {
            trace!("      [{}]: {:?}", i, value);
        }
    }

    /// Log local variables state
    fn log_local_variables(&self, frame: &StackFrame) {
        trace!("    Local Variables ({}):", frame.local_vars.len());
        for (i, value) in frame.local_vars.iter().enumerate() {
            trace!("      [{}]: {:?}", i, value);
        }
    }

    /// Log class loading
    pub fn log_class_loaded(&self, class_name: &str, source: &str) {
        if self.config.trace_classes {
            info!("Loaded class '{}' from {}", class_name, source);
        }
    }

    /// Log memory allocation
    pub fn log_memory_allocation(&self, address: u32, size: usize, object_type: &str) {
        if self.config.trace_memory {
            debug!(
                "Allocated {} bytes at address 0x{:08x} for {}",
                size, address, object_type
            );
        }
    }

    /// Log memory access
    pub fn log_memory_access(&self, address: u32, operation: &str, value: Option<&Value>) {
        if self.config.trace_memory {
            match value {
                Some(v) => trace!("Memory {}: address 0x{:08x} = {:?}", operation, address, v),
                None => trace!("Memory {}: address 0x{:08x}", operation, address),
            }
        }
    }

    /// Log exception handling
    pub fn log_exception(&self, exception: &str, handler_pc: Option<usize>) {
        warn!("Exception thrown: {}", exception);
        if let Some(pc) = handler_pc {
            info!("Handling exception at PC {}", pc);
        }
    }

    /// Log garbage collection
    pub fn log_gc(&self, freed_objects: usize, freed_bytes: usize) {
        info!(
            "GC completed: freed {} objects ({} bytes)",
            freed_objects, freed_bytes
        );
    }

    /// Get opcode name for logging
    fn get_opcode_name(&self, opcode: u8) -> &'static str {
        match opcode {
            0x00 => "nop",
            0x01 => "aconst_null",
            0x02 => "iconst_m1",
            0x03 => "iconst_0",
            0x04 => "iconst_1",
            0x05 => "iconst_2",
            0x06 => "iconst_3",
            0x07 => "iconst_4",
            0x08 => "iconst_5",
            0x09 => "lconst_0",
            0x0a => "lconst_1",
            0x0b => "fconst_0",
            0x0c => "fconst_1",
            0x0d => "fconst_2",
            0x0e => "dconst_0",
            0x0f => "dconst_1",
            0x10 => "bipush",
            0x11 => "sipush",
            0x12 => "ldc",
            0x13 => "ldc_w",
            0x14 => "ldc2_w",
            0x15 => "iload",
            0x16 => "lload",
            0x17 => "fload",
            0x18 => "dload",
            0x19 => "aload",
            0x1a => "iload_0",
            0x1b => "iload_1",
            0x1c => "iload_2",
            0x1d => "iload_3",
            0x1e => "lload_0",
            0x1f => "lload_1",
            0x20 => "lload_2",
            0x21 => "lload_3",
            0x22 => "fload_0",
            0x23 => "fload_1",
            0x24 => "fload_2",
            0x25 => "fload_3",
            0x26 => "dload_0",
            0x27 => "dload_1",
            0x28 => "dload_2",
            0x29 => "dload_3",
            0x2a => "aload_0",
            0x2b => "aload_1",
            0x2c => "aload_2",
            0x2d => "aload_3",
            0x2e => "iaload",
            0x2f => "laload",
            0x30 => "faload",
            0x31 => "daload",
            0x32 => "aaload",
            0x33 => "baload",
            0x34 => "caload",
            0x35 => "saload",
            0x36 => "istore",
            0x37 => "lstore",
            0x38 => "fstore",
            0x39 => "dstore",
            0x3a => "astore",
            0x3b => "istore_0",
            0x3c => "istore_1",
            0x3d => "istore_2",
            0x3e => "istore_3",
            0x3f => "lstore_0",
            0x40 => "lstore_1",
            0x41 => "lstore_2",
            0x42 => "lstore_3",
            0x43 => "fstore_0",
            0x44 => "fstore_1",
            0x45 => "fstore_2",
            0x46 => "fstore_3",
            0x47 => "dstore_0",
            0x48 => "dstore_1",
            0x49 => "dstore_2",
            0x4a => "dstore_3",
            0x4b => "astore_0",
            0x4c => "astore_1",
            0x4d => "astore_2",
            0x4e => "astore_3",
            0x4f => "iastore",
            0x50 => "lastore",
            0x51 => "fastore",
            0x52 => "dastore",
            0x53 => "aastore",
            0x54 => "bastore",
            0x55 => "castore",
            0x56 => "sastore",
            0x57 => "pop",
            0x58 => "pop2",
            0x59 => "dup",
            0x5a => "dup_x1",
            0x5b => "dup_x2",
            0x5c => "dup2",
            0x5d => "dup2_x1",
            0x5e => "dup2_x2",
            0x5f => "swap",
            0x60 => "iadd",
            0x61 => "ladd",
            0x62 => "fadd",
            0x63 => "dadd",
            0x64 => "isub",
            0x65 => "lsub",
            0x66 => "fsub",
            0x67 => "dsub",
            0x68 => "imul",
            0x69 => "lmul",
            0x6a => "fmul",
            0x6b => "dmul",
            0x6c => "idiv",
            0x6d => "ldiv",
            0x6e => "fdiv",
            0x6f => "ddiv",
            0x70 => "irem",
            0x71 => "lrem",
            0x72 => "frem",
            0x73 => "drem",
            0x74 => "ineg",
            0x75 => "lneg",
            0x76 => "fneg",
            0x77 => "dneg",
            0x78 => "ishl",
            0x79 => "lshl",
            0x7a => "ishr",
            0x7b => "lshr",
            0x7c => "iushr",
            0x7d => "lushr",
            0x7e => "iand",
            0x7f => "land",
            0x80 => "ior",
            0x81 => "lor",
            0x82 => "ixor",
            0x83 => "lxor",
            0x84 => "iinc",
            0x85 => "i2l",
            0x86 => "i2f",
            0x87 => "i2d",
            0x88 => "l2i",
            0x89 => "l2f",
            0x8a => "l2d",
            0x8b => "f2i",
            0x8c => "f2l",
            0x8d => "f2d",
            0x8e => "d2i",
            0x8f => "d2l",
            0x90 => "d2f",
            0x91 => "i2b",
            0x92 => "i2c",
            0x93 => "i2s",
            0x94 => "lcmp",
            0x95 => "fcmpl",
            0x96 => "fcmpg",
            0x97 => "dcmpl",
            0x98 => "dcmpg",
            0x99 => "ifeq",
            0x9a => "ifne",
            0x9b => "iflt",
            0x9c => "ifge",
            0x9d => "ifgt",
            0x9e => "ifle",
            0x9f => "if_icmpeq",
            0xa0 => "if_icmpne",
            0xa1 => "if_icmplt",
            0xa2 => "if_icmpge",
            0xa3 => "if_icmpgt",
            0xa4 => "if_icmple",
            0xa5 => "if_acmpeq",
            0xa6 => "if_acmpne",
            0xa7 => "goto",
            0xa8 => "jsr",
            0xa9 => "ret",
            0xaa => "tableswitch",
            0xab => "lookupswitch",
            0xac => "ireturn",
            0xad => "lreturn",
            0xae => "freturn",
            0xaf => "dreturn",
            0xb0 => "areturn",
            0xb1 => "return",
            0xb2 => "getstatic",
            0xb3 => "putstatic",
            0xb4 => "getfield",
            0xb5 => "putfield",
            0xb6 => "invokevirtual",
            0xb7 => "invokespecial",
            0xb8 => "invokestatic",
            0xb9 => "invokeinterface",
            0xba => "invokedynamic",
            0xbb => "new",
            0xbc => "newarray",
            0xbd => "anewarray",
            0xbe => "arraylength",
            0xbf => "athrow",
            0xc0 => "checkcast",
            0xc1 => "instanceof",
            0xc2 => "monitorenter",
            0xc3 => "monitorexit",
            0xc4 => "wide",
            0xc5 => "multianewarray",
            0xc6 => "ifnull",
            0xc7 => "ifnonnull",
            0xc8 => "goto_w",
            0xc9 => "jsr_w",
            _ => "unknown",
        }
    }

    /// Get instruction count
    pub fn instruction_count(&self) -> usize {
        self.instruction_count
    }

    /// Reset instruction count
    pub fn reset_instruction_count(&mut self) {
        self.instruction_count = 0;
    }
}

/// Initialize logging with the given level
pub fn init_logging(level: log::LevelFilter) {
    env_logger::Builder::from_default_env()
        .filter_level(level)
        .init();
}

/// Create a debug config from environment variables
pub fn debug_config_from_env() -> DebugConfig {
    DebugConfig {
        trace_instructions: std::env::var("JVMRS_TRACE_INSTRUCTIONS").is_ok(),
        trace_memory: std::env::var("JVMRS_TRACE_MEMORY").is_ok(),
        trace_classes: !std::env::var("JVMRS_TRACE_CLASSES").is_ok_and(|v| v == "false"),
        trace_methods: !std::env::var("JVMRS_TRACE_METHODS").is_ok_and(|v| v == "false"),
        max_stack_frames: std::env::var("JVMRS_MAX_STACK_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok()),
        max_stack_entries: std::env::var("JVMRS_MAX_STACK_ENTRIES")
            .ok()
            .and_then(|s| s.parse().ok()),
    }
}
