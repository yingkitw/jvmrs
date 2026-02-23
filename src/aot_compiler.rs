//! AOT Compiler - Ahead-of-time compilation to native object files.
//!
//! Uses cranelift-object to compile JVM bytecode to .o files that can be
//! linked into native executables.

use crate::class_file::{ClassFile, MethodInfo};
use crate::jit::JitError;
use cranelift::codegen::ir::{AbiParam, Signature};
use cranelift::codegen::isa::CallConv;
use cranelift::prelude::*;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::path::Path;

/// Build Cranelift IR for a method and define it in the module.
fn build_method_ir(
    method: &MethodInfo,
    module: &mut dyn Module,
    func_name: &str,
    builder_ctx: &mut FunctionBuilderContext,
) -> Result<(), JitError> {
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

    let ptr_type = module.target_config().pointer_type();
    let mut sig = Signature::new(CallConv::SystemV);
    sig.params.push(AbiParam::new(ptr_type));
    sig.params.push(AbiParam::new(ptr_type));
    sig.returns.push(AbiParam::new(types::I32));

    let mut push_sig = Signature::new(CallConv::SystemV);
    push_sig.params.push(AbiParam::new(ptr_type));
    push_sig.params.push(AbiParam::new(types::I32));
    let push_fn = module
        .declare_function("jvmrs_frame_push_int", Linkage::Import, &push_sig)
        .map_err(|e| JitError::CompilationFailed(e.to_string()))?;

    let mut get_sig = Signature::new(CallConv::SystemV);
    get_sig.params.push(AbiParam::new(ptr_type));
    get_sig.params.push(AbiParam::new(types::I32));
    get_sig.returns.push(AbiParam::new(types::I32));
    let get_fn = module
        .declare_function("jvmrs_frame_get_local_int", Linkage::Import, &get_sig)
        .map_err(|e| JitError::CompilationFailed(e.to_string()))?;

    let func_id = module
        .declare_function(func_name, Linkage::Export, &sig)
        .map_err(|e| JitError::CompilationFailed(e.to_string()))?;

    let mut ctx = module.make_context();
    ctx.func.signature = sig.clone();

    let push_fn_ref = module.declare_func_in_func(push_fn, &mut ctx.func);
    let get_fn_ref = module.declare_func_in_func(get_fn, &mut ctx.func);

    let mut builder = FunctionBuilder::new(&mut ctx.func, builder_ctx);

    let entry = builder.create_block();
    builder.append_block_params_for_function_params(entry);
    builder.switch_to_block(entry);
    builder.seal_block(entry);

    let params = builder.block_params(entry);
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
                let call = builder.ins().call(get_fn_ref, &[frame_param, idx_val]);
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

    module
        .define_function(func_id, &mut ctx)
        .map_err(|e| JitError::CompilationFailed(e.to_string()))?;

    Ok(())
}

/// Compile a class to a native object file (.o)
pub fn compile_class_to_object(class: &ClassFile, output_path: &Path) -> Result<(), JitError> {
    let class_name = class
        .get_class_name()
        .unwrap_or_else(|| "Unknown".to_string());
    let safe_name = class_name.replace("/", "_").replace(".", "_");

    let mut flag_builder = cranelift::codegen::settings::builder();
    flag_builder
        .set("use_colocated_libcalls", "false")
        .map_err(|e| JitError::CompilationFailed(e.to_string()))?;
    flag_builder
        .set("is_pic", "true")
        .map_err(|e| JitError::CompilationFailed(e.to_string()))?;

    let isa = cranelift_native::builder()
        .map_err(|e| JitError::CompilationFailed(format!("ISA: {}", e)))?
        .finish(cranelift::codegen::settings::Flags::new(flag_builder))
        .map_err(|e| JitError::CompilationFailed(e.to_string()))?;

    let builder = ObjectBuilder::new(
        isa,
        safe_name.as_str(),
        cranelift_module::default_libcall_names(),
    )
    .map_err(|e| JitError::CompilationFailed(format!("ObjectBuilder: {}", e)))?;
    let mut module = ObjectModule::new(builder);
    let mut builder_ctx = FunctionBuilderContext::new();

    let mut compiled = 0usize;

    for method in &class.methods {
        let method_name = class
            .get_string(method.name_index)
            .unwrap_or_else(|| "unknown".to_string());

        if method.access_flags & 0x0400 != 0 || method.access_flags & 0x0100 != 0 {
            continue;
        }

        if method.attributes.iter().all(|a| a.info.len() < 8) {
            continue;
        }

        let func_name = format!("{}_{}", safe_name, method_name.replace("<", "_").replace(">", "_"));

        if build_method_ir(method, &mut module, &func_name, &mut builder_ctx).is_ok() {
            compiled += 1;
        }
    }

    if compiled == 0 {
        return Err(JitError::CompilationFailed(
            "No compilable methods found".to_string(),
        ));
    }

    let product = module.finish();
    let bytes = product
        .emit()
        .map_err(|e| JitError::LinkingError(format!("Object emission failed: {}", e)))?;

    std::fs::write(output_path, &bytes)
        .map_err(|e| JitError::LinkingError(format!("Failed to write object file: {}", e)))?;

    Ok(())
}
