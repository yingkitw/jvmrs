//! Method invocation (invokevirtual, invokestatic, execute_method).

use crate::class_file::{ClassFile, ConstantPoolEntry, MethodInfo};
use crate::error::{to_runtime_error_enum, JvmError, NativeError, RuntimeError};
use crate::memory::{Memory, StackFrame, Value};
use crate::profiler::ProfileGuard;
use log::info;

use super::descriptor;
use super::utils;
use super::Interpreter;
use super::InterpreterResult;

impl Interpreter {
    pub(crate) fn collect_method_args(
        &self,
        frame: &mut StackFrame,
        descriptor_str: &str,
    ) -> Result<Vec<Value>, JvmError> {
        let param_types = descriptor::parse_method_params(descriptor_str);
        let mut args = Vec::new();
        for _ in 0..param_types.len() {
            args.push(frame.pop()?);
        }
        args.reverse();
        Ok(args)
    }

    pub(crate) fn resolve_method(
        &self,
        class_name: &str,
        method_name: &str,
        descriptor: &str,
    ) -> Option<(String, String)> {
        let class = self.class_loader.get_class(class_name)?;
        if class.find_method(method_name, descriptor).is_some() {
            return Some((class_name.to_string(), method_name.to_string()));
        }
        if let Some(super_class_name) = class.get_super_class_name() {
            return self.resolve_method(&super_class_name, method_name, descriptor);
        }
        None
    }

    pub(crate) fn resolve_interface_method(
        &self,
        interface_name: &str,
        method_name: &str,
        descriptor: &str,
    ) -> Option<(String, String)> {
        self.resolve_method(interface_name, method_name, descriptor)
    }

    pub(crate) fn invoke_native_method(
        &mut self,
        class_name: &str,
        method_name: &str,
        is_instance: bool,
        caller_frame: &mut StackFrame,
    ) -> InterpreterResult {
        if let Some(native) = self.native_registry.find_method(class_name, method_name) {
            let sig = native.signature();
            let mut args = if is_instance {
                vec![caller_frame.pop()?]
            } else {
                Vec::new()
            };
            args.extend(self.collect_method_args(caller_frame, sig)?);
            match native.invoke(&args, &mut self.memory) {
                Ok(result) => {
                    if !sig.ends_with(")V") {
                        caller_frame.push(result)?;
                    }
                    Ok(())
                }
                Err(e) => Err(JvmError::NativeError(e)),
            }
        } else {
            Err(JvmError::NativeError(NativeError::NativeMethodNotFound(
                class_name.to_string(),
                method_name.to_string(),
            )))
        }
    }

    pub(crate) fn invoke_virtual(
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
            ConstantPoolEntry::ConstantMethodref {
                class_index,
                name_and_type_index,
            }
            | ConstantPoolEntry::ConstantInterfaceMethodref {
                class_index,
                name_and_type_index,
            } => (class_index, name_and_type_index),
            _ => {
                return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                    "Expected MethodRef or InterfaceMethodRef constant".to_string(),
                )))
            }
        };

        let target_class_name = match class.constant_pool.get(*class_index as usize) {
            Some(ConstantPoolEntry::ConstantClass { name_index }) => {
                class.get_string(*name_index).unwrap_or_default()
            }
            _ => {
                return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                    "Invalid class reference".to_string(),
                )))
            }
        };

        let (method_name, descriptor) = match class.constant_pool.get(*name_and_type_index as usize)
        {
            Some(ConstantPoolEntry::ConstantNameAndType {
                name_index,
                descriptor_index,
            }) => (
                class.get_string(*name_index).unwrap_or_default(),
                class.get_string(*descriptor_index).unwrap_or_default(),
            ),
            _ => {
                return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                    "Invalid NameAndType reference".to_string(),
                )))
            }
        };

        if target_class_name == "java/io/PrintStream" && method_name == "println" {
            if !frame.stack.is_empty() {
                let value = frame.pop()?;
                let _objectref = frame.pop()?;
                self.native_println(value)?;
            } else {
                let _objectref = frame.pop()?;
                println!();
            }
            return Ok(());
        }

        if let Some((resolved_class_name, _)) =
            self.resolve_method(&target_class_name, &method_name, &descriptor)
        {
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

        Err(to_runtime_error_enum(RuntimeError::MethodNotFound(
            target_class_name,
            method_name,
        )))
    }

    pub(crate) fn invoke_static(
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
            ConstantPoolEntry::ConstantMethodref {
                class_index,
                name_and_type_index,
            } => (class_index, name_and_type_index),
            _ => {
                return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                    "Expected MethodRef for invokestatic".to_string(),
                )))
            }
        };

        let target_class_name = match class.constant_pool.get(*class_index as usize) {
            Some(ConstantPoolEntry::ConstantClass { name_index }) => {
                class.get_string(*name_index).unwrap_or_default()
            }
            _ => {
                return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                    "Invalid class reference".to_string(),
                )))
            }
        };

        let (method_name, descriptor) =
            match class.constant_pool.get(*name_and_type_index as usize) {
                Some(ConstantPoolEntry::ConstantNameAndType {
                    name_index,
                    descriptor_index,
                }) => (
                    class.get_string(*name_index).unwrap_or_default(),
                    class.get_string(*descriptor_index).unwrap_or_default(),
                ),
                _ => {
                    return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                        "Invalid NameAndType reference".to_string(),
                    )))
                }
            };

        self.load_class_by_name(&target_class_name).ok();
        if let Some(class_ref) = self.class_loader.get_class(&target_class_name) {
            if let Some(method) = class_ref.find_method(&method_name, &descriptor) {
                let class_clone = class_ref.clone();
                let method_clone = method.clone();
                return self.execute_method(&class_clone, &method_clone, frame);
            }
        }

        Err(to_runtime_error_enum(RuntimeError::MethodNotFound(
            target_class_name,
            method_name,
        )))
    }

    pub(crate) fn execute_method(
        &mut self,
        class: &ClassFile,
        method: &MethodInfo,
        caller_frame: &mut StackFrame,
    ) -> InterpreterResult {
        let method_name = class
            .get_string(method.name_index)
            .unwrap_or_else(|| "unknown".to_string());
        let class_name = class.get_class_name().unwrap_or("Unknown".to_string());

        if (method.access_flags & 0x0100) != 0 {
            return self.invoke_native_method(
                &class_name,
                &method_name,
                (method.access_flags & 0x0008) == 0,
                caller_frame,
            );
        }

        let profiler_clone = self.profiler.clone();
        let _profile_guard = profiler_clone
            .as_ref()
            .map(|p| ProfileGuard::new(p.as_ref(), &class_name, &method_name));

        if let Some(jit_manager) = &mut self.jit_manager {
            if let Some(level) = jit_manager.record_and_check_compilation(&class_name, &method_name)
            {
                info!(
                    "Hot method detected: {}::{} - compiling at level {:?}",
                    class_name, method_name, level
                );
                if let Ok(compiled) =
                    jit_manager.get_or_compile_method_at(class, method, Some(level))
                {
                    info!(
                        "Method {}::{} compiled in {}ms ({} bytes)",
                        class_name, method_name, compiled.compile_time_ms, compiled.code_size
                    );
                }
            }
        }

        let full_method_name = format!("{}.{}", class_name, method_name);

        if let Some(jit_manager) = &self.jit_manager {
            if let Some(compiled_code) =
                jit_manager.compiler.get_compiled_function(&full_method_name)
            {
                info!("Executing JIT compiled method: {}", full_method_name);

                let code_attr = self.find_code_attribute(class, method).ok_or_else(|| {
                    to_runtime_error_enum(RuntimeError::UnsupportedOperation(
                        "Code attribute not found".to_string(),
                    ))
                })?;

                let max_locals = utils::read_u16(&code_attr.info, 0) as usize;
                let max_stack = utils::read_u16(&code_attr.info, 2) as usize;

                let mut frame = StackFrame::new(max_locals, max_stack, method_name.clone());

                let descriptor = class
                    .get_string(method.descriptor_index)
                    .unwrap_or_default();
                let param_count = descriptor::count_parameters(&descriptor);

                for i in 0..param_count {
                    let value = caller_frame.pop()?;
                    frame.store_local(param_count - 1 - i, value)?;
                }

                unsafe {
                    let result = (compiled_code.func)(
                        &mut self.memory as *mut Memory,
                        &mut frame as *mut StackFrame,
                    );

                    if result != 0 {
                        return Err(to_runtime_error_enum(RuntimeError::UnsupportedOperation(
                            format!("JIT compiled method returned error code: {}", result),
                        )));
                    }
                }

                if !frame.stack.is_empty() {
                    let return_value = frame.pop()?;
                    caller_frame.push(return_value)?;
                }

                return Ok(());
            }
        }

        let code_attr = self.find_code_attribute(class, method).ok_or_else(|| {
            to_runtime_error_enum(RuntimeError::UnsupportedOperation(
                "Code attribute not found".to_string(),
            ))
        })?;

        let max_stack = utils::read_u16(&code_attr.info, 0) as usize;
        let max_locals = utils::read_u16(&code_attr.info, 2) as usize;
        let code_length = utils::read_u32(&code_attr.info, 4) as usize;

        let mut frame = StackFrame::new(max_locals, max_stack, method_name.clone());

        let descriptor = class
            .get_string(method.descriptor_index)
            .unwrap_or_default();
        let param_count = descriptor::count_parameters(&descriptor);

        for i in 0..param_count {
            let value = caller_frame.pop()?;
            frame.store_local(param_count - 1 - i, value)?;
        }

        let code = &code_attr.info[8..8 + code_length];
        let class_clone = class.clone();

        while frame.pc < code.len() {
            let opcode = code[frame.pc];
            let pc_before = frame.pc;
            frame.pc += 1;

            if let Some(ref mut tr) = self.trace_recorder {
                tr.record(
                    pc_before,
                    opcode,
                    &format!("{}.{}", class_name, method_name),
                    frame.stack.len(),
                    frame.locals.len(),
                );
            }

            self.debugger.log_instruction(&frame, &class_clone, opcode);

            match opcode {
                0xb1 => break,
                0xac => {
                    if !frame.stack.is_empty() {
                        let return_value = frame.pop()?;
                        caller_frame.push(return_value)?;
                    }
                    break;
                }
                0xb0 => {
                    if !frame.stack.is_empty() {
                        let return_value = frame.pop()?;
                        caller_frame.push(return_value)?;
                    }
                    break;
                }
                _ => {
                    if !self.dispatch_instruction(&class_clone, code, &mut frame, opcode)? {
                        break;
                    }
                }
            }
        }

        if !frame.stack.is_empty() {
            let return_value = frame.pop()?;
            caller_frame.push(return_value)?;
        }

        Ok(())
    }
}
