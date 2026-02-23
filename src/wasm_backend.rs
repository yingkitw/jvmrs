//! WebAssembly backend - compile JVM bytecode to WASM for browser execution.
//!
//! Enable with `cargo build --features wasm`

#[cfg(feature = "wasm")]
use crate::class_file::{ClassFile, MethodInfo};
#[cfg(feature = "wasm")]
use crate::jit::JitError;
#[cfg(feature = "wasm")]
use wasm_encoder::{CodeSection, Function, FunctionSection, Instruction, Module, TypeSection, ValType};

/// WASM module generator
#[cfg(feature = "wasm")]
pub struct WasmGenerator {
    /// Function bodies (each is a sequence of Instructions)
    funcs: Vec<Vec<Instruction<'static>>>,
}

#[cfg(feature = "wasm")]
impl WasmGenerator {
    pub fn new() -> Self {
        Self { funcs: Vec::new() }
    }

    /// Convert a JVM method to WASM function.
    /// Supports: bipush, iload_0..iload_3, iadd, ireturn.
    pub fn method_to_wasm(
        &mut self,
        class: &ClassFile,
        method: &MethodInfo,
    ) -> Result<(), JitError> {
        let mut body = vec![];

        // Extract bytecode
        let code_attr = method.attributes.iter().find(|a| a.info.len() >= 8);

        if let Some(attr) = code_attr {
            let code_len = ((attr.info[4] as usize) << 24)
                | ((attr.info[5] as usize) << 16)
                | ((attr.info[6] as usize) << 8)
                | (attr.info[7] as usize);
            let bytecode = attr.info.get(8..8 + code_len).unwrap_or(&[]);
            let mut pc = 0usize;
            let mut saw_ireturn = false;

            while pc < bytecode.len() {
                let opcode = bytecode[pc];
                pc += 1;
                match opcode {
                    0x10 => {
                        if pc < bytecode.len() {
                            let byte_val = bytecode[pc] as i8 as i32;
                            pc += 1;
                            body.push(Instruction::I32Const(byte_val));
                        }
                    }
                    0x1a..=0x1d => {
                        body.push(Instruction::LocalGet((opcode - 0x1a) as u32));
                    }
                    0x60 => {
                        body.push(Instruction::I32Add);
                    }
                    0xac => {
                        body.push(Instruction::End);
                        saw_ireturn = true;
                        break;
                    }
                    _ => {}
                }
            }

            if !saw_ireturn {
                body.push(Instruction::I32Const(0));
                body.push(Instruction::End);
            }
        } else {
            body.push(Instruction::I32Const(0));
            body.push(Instruction::End);
        }

        self.funcs.push(body);
        Ok(())
    }

    /// Emit the WASM module as bytes
    pub fn emit(&self) -> Vec<u8> {
        let mut module = Module::new();

        // Type section: (i32,i32,i32,i32) -> i32 for compatibility with iload_0..3
        let mut type_section = TypeSection::new();
        let params = vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32];
        let results = vec![ValType::I32];
        for _ in &self.funcs {
            type_section.function(params.clone(), results.clone());
        }
        module.section(&type_section);

        // Function section
        let mut func_section = FunctionSection::new();
        for i in 0..self.funcs.len() {
            func_section.function(i as u32);
        }
        module.section(&func_section);

        // Code section
        let mut code_section = CodeSection::new();
        for func_body in &self.funcs {
            let mut func = Function::new(vec![]);
            for inst in func_body {
                func.instruction(inst);
            }
            code_section.function(&func);
        }
        module.section(&code_section);

        module.finish()
    }

    /// Emit WASM to a file
    pub fn write_to_file(&self, path: &std::path::Path) -> Result<(), JitError> {
        std::fs::write(path, self.emit()).map_err(|e| {
            crate::jit::JitError::LinkingError(format!("Failed to write WASM: {}", e))
        })
    }
}

#[cfg(feature = "wasm")]
impl Default for WasmGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Stub when wasm feature is disabled
#[cfg(not(feature = "wasm"))]
pub struct WasmGenerator;

#[cfg(not(feature = "wasm"))]
impl WasmGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn method_to_wasm(
        &mut self,
        _class: &crate::class_file::ClassFile,
        _method: &crate::class_file::MethodInfo,
    ) -> Result<(), crate::jit::JitError> {
        Err(crate::jit::JitError::CompilationFailed(
            "WASM feature not enabled. Build with --features wasm.".to_string(),
        ))
    }

    pub fn emit(&self) -> Vec<u8> {
        vec![]
    }
}
