//! Instruction dispatch - handles individual bytecode opcodes.

use crate::class_file::ClassFile;
use crate::error::{to_runtime_error_enum, JvmError, RuntimeError};
use crate::memory::{StackFrame, Value};

use super::Interpreter;
use super::utils;

impl Interpreter {
    /// Dispatch a single bytecode instruction. Returns false to break (return).
    pub(crate) fn dispatch_instruction(
        &mut self,
        class: &ClassFile,
        code: &[u8],
        frame: &mut StackFrame,
        opcode: u8,
    ) -> Result<bool, JvmError> {
        match opcode {
            0xb1 => return Ok(false), // return
            0xb6 => {
                if frame.pc + 1 < code.len() {
                    let index = utils::read_u16(code, frame.pc) as usize;
                    frame.pc += 2;
                    self.invoke_virtual(class, frame, index)?;
                }
            }
            0xb8 => {
                if frame.pc + 1 < code.len() {
                    let index = utils::read_u16(code, frame.pc) as usize;
                    frame.pc += 2;
                    self.invoke_static(class, frame, index)?;
                }
            }
            0xc2 => {
                let obj_ref = frame.pop()?.as_reference().ok_or_else(|| {
                    to_runtime_error_enum(RuntimeError::NullPointerException)
                })?;
                self.memory
                    .heap
                    .monitor_enter(obj_ref, self.current_thread_id)
                    .map_err(|e| to_runtime_error_enum(RuntimeError::from(e)))?;
            }
            0xc3 => {
                let obj_ref = frame.pop()?.as_reference().ok_or_else(|| {
                    to_runtime_error_enum(RuntimeError::NullPointerException)
                })?;
                self.memory
                    .heap
                    .monitor_exit(obj_ref, self.current_thread_id)
                    .map_err(|e| to_runtime_error_enum(RuntimeError::from(e)))?;
            }
            _ => {
                if opcode >= 0x01 && opcode <= 0x15 {
                    frame.pc += if opcode >= 0x12 { 2 } else { 0 };
                } else if opcode >= 0x36 && opcode <= 0x3a {
                    frame.pc += 1;
                } else if opcode >= 0x60 && opcode <= 0x83 {
                    let _ = frame.stack.pop();
                    let _ = frame.stack.pop();
                    let _ = frame.push(Value::Int(0));
                } else if opcode >= 0xb2 && opcode <= 0xb5 {
                    frame.pc += 2;
                }
            }
        }
        Ok(true)
    }
}
