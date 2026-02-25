//! Cranelift-based JIT: bytecode-to-native code generation.
//!
//! Translates JVM bytecode (bipush, iload_0..3, iadd, ireturn) to native code
//! using cranelift-jit. The compiled function uses helpers to read/write the
//! stack frame, preserving the existing ABI (memory*, frame*) -> i32.

use crate::class_file::{ClassFile, MethodInfo};
use crate::jit::JitError;
use crate::memory::{StackFrame, Value as JvmValue};
use cranelift::codegen::ir::{AbiParam, Signature};
use cranelift::codegen::isa::CallConv;
use cranelift::prelude::*;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};

/// FFI helpers - called from JIT-compiled code to access the stack frame.
#[unsafe(no_mangle)]
pub extern "C" fn jvmrs_frame_get_local_int(frame_ptr: *mut std::ffi::c_void, index: u32) -> i32 {
    if frame_ptr.is_null() {
        return 0;
    }
    let frame = unsafe { &*(frame_ptr as *const StackFrame) };
    frame
        .locals
        .get(index as usize)
        .map(JvmValue::as_int)
        .unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn jvmrs_frame_push_int(frame_ptr: *mut std::ffi::c_void, value: i32) {
    if frame_ptr.is_null() {
        return;
    }
    let frame = unsafe { &mut *(frame_ptr as *mut StackFrame) };
    let _ = frame.push(JvmValue::Int(value));
}

/// Cranelift JIT backend - compiles bytecode to native code.
pub struct CraneliftJitBackend {
    module: JITModule,
    builder_ctx: FunctionBuilderContext,
}

impl CraneliftJitBackend {
    /// Create a new JIT backend with FFI helpers registered.
    pub fn new() -> Result<Self, JitError> {
        let mut flag_builder = cranelift::codegen::settings::builder();
        flag_builder
            .set("use_colocated_libcalls", "false")
            .map_err(|e| JitError::CompilationFailed(e.to_string()))?;
        flag_builder
            .set("is_pic", "false")
            .map_err(|e| JitError::CompilationFailed(e.to_string()))?;

        let isa = cranelift_native::builder()
            .map_err(|e| JitError::CompilationFailed(format!("ISA: {}", e)))?
            .finish(cranelift::codegen::settings::Flags::new(flag_builder))
            .map_err(|e| JitError::CompilationFailed(e.to_string()))?;

        let mut builder = JITBuilder::with_isa(
            isa,
            cranelift_module::default_libcall_names(),
        );

        builder.symbol(
            "jvmrs_frame_get_local_int",
            jvmrs_frame_get_local_int as *const u8,
        );
        builder.symbol("jvmrs_frame_push_int", jvmrs_frame_push_int as *const u8);

        let module = JITModule::new(builder);

        Ok(Self {
            module,
            builder_ctx: FunctionBuilderContext::new(),
        })
    }

    /// Compile a method to native code. Returns the function pointer and code size.
    /// Supports: bipush (0x10), iload_0..iload_3 (0x1a-0x1d), iadd (0x60), ireturn (0xac).
    pub fn compile(
        &mut self,
        _class: &ClassFile,
        method: &MethodInfo,
        func_name: &str,
    ) -> Result<(*const u8, usize), JitError> {
        let code_attr = method
            .attributes
            .iter()
            .find(|a| a.info.len() >= 8)
            .ok_or_else(|| JitError::InvalidMethod("No Code attribute".to_string()))?;

        let code_len = ((code_attr.info[4] as usize) << 24)
            | ((code_attr.info[5] as usize) << 16)
            | ((code_attr.info[6] as usize) << 8)
            | (code_attr.info[7] as usize);
        let bytecode = code_attr.info.get(8..8 + code_len).unwrap_or(&[]);

        // Signature: (memory: *mut Memory, frame: *mut StackFrame) -> i32
        let ptr_type = self.module.target_config().pointer_type();
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.push(AbiParam::new(ptr_type));
        sig.params.push(AbiParam::new(ptr_type));
        sig.returns.push(AbiParam::new(types::I32));

        // Declare external helpers (void return for push)
        let mut push_sig = Signature::new(CallConv::SystemV);
        push_sig.params.push(AbiParam::new(ptr_type));
        push_sig.params.push(AbiParam::new(types::I32));
        let push_fn = self
            .module
            .declare_function("jvmrs_frame_push_int", Linkage::Import, &push_sig)
            .map_err(|e| JitError::CompilationFailed(e.to_string()))?;

        let mut get_sig = Signature::new(CallConv::SystemV);
        get_sig.params.push(AbiParam::new(ptr_type));
        get_sig.params.push(AbiParam::new(types::I32));
        get_sig.returns.push(AbiParam::new(types::I32));
        let get_fn = self
            .module
            .declare_function("jvmrs_frame_get_local_int", Linkage::Import, &get_sig)
            .map_err(|e| JitError::CompilationFailed(e.to_string()))?;

        let func_id = self
            .module
            .declare_function(func_name, Linkage::Local, &sig)
            .map_err(|e| JitError::CompilationFailed(e.to_string()))?;

        let mut ctx = self.module.make_context();
        ctx.func.signature = sig.clone();

        let push_fn_ref = self.module.declare_func_in_func(push_fn, &mut ctx.func);
        let get_fn_ref = self.module.declare_func_in_func(get_fn, &mut ctx.func);

        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut self.builder_ctx);

        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);
        builder.seal_block(entry);

        let params = builder.block_params(entry);
        let _memory_param = params[0];
        let frame_param = params[1];

        let i32_type = types::I32;
        let mut stack: Vec<cranelift::prelude::Value> = Vec::new();

        let mut pc = 0usize;
        let mut compiled = false;

        while pc < bytecode.len() {
            let opcode = bytecode[pc];
            pc += 1;

            match opcode {
                0x10 => {
                    if pc < bytecode.len() {
                        let byte_val = bytecode[pc] as i8 as i32;
                        pc += 1;
                        stack.push(builder.ins().iconst(i32_type, byte_val as i64));
                    }
                }
                0x1a..=0x1d => {
                    let idx = (opcode - 0x1a) as u32;
                    let idx_val = builder.ins().iconst(i32_type, idx as i64);
                    let call = builder
                        .ins()
                        .call(get_fn_ref, &[frame_param, idx_val]);
                    let result = builder.inst_results(call)[0];
                    stack.push(result);
                }
                0x60 => {
                    if stack.len() >= 2 {
                        let b = stack.pop().unwrap();
                        let a = stack.pop().unwrap();
                        stack.push(builder.ins().iadd(a, b));
                    }
                }
                0x64 => {
                    if stack.len() >= 2 {
                        let b = stack.pop().unwrap();
                        let a = stack.pop().unwrap();
                        stack.push(builder.ins().isub(a, b));
                    }
                }
                0x68 => {
                    if stack.len() >= 2 {
                        let b = stack.pop().unwrap();
                        let a = stack.pop().unwrap();
                        stack.push(builder.ins().imul(a, b));
                    }
                }
                0x6c => {
                    if stack.len() >= 2 {
                        let b = stack.pop().unwrap();
                        let a = stack.pop().unwrap();
                        stack.push(builder.ins().sdiv(a, b));
                    }
                }
                0xac => {
                    let result = stack.pop().unwrap_or_else(|| builder.ins().iconst(i32_type, 0));
                    builder.ins().call(push_fn_ref, &[frame_param, result]);
                    let ret_val = builder.ins().iconst(i32_type, 0);
                    builder.ins().return_(&[ret_val]);
                    compiled = true;
                    break;
                }
                _ => {
                    return Err(JitError::UnsupportedInstruction(format!(
                        "opcode 0x{:02x} at pc {}",
                        opcode, pc
                    )));
                }
            }
        }

        if !compiled {
            let ret_val = builder.ins().iconst(i32_type, 0);
            builder.ins().return_(&[ret_val]);
        }

        builder.finalize();

        self.module
            .define_function(func_id, &mut ctx)
            .map_err(|e| JitError::CompilationFailed(e.to_string()))?;

        self.module.clear_context(&mut ctx);

        self.module
            .finalize_definitions()
            .map_err(|e| JitError::LinkingError(e.to_string()))?;

        let code_ptr = self.module.get_finalized_function(func_id);
        let code_size = 0; // Not easily available from JITModule

        Ok((code_ptr, code_size))
    }
}
