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
            // Constants
            0x02 => frame.push(Value::Int(-1))?,
            0x03 => frame.push(Value::Int(0))?,
            0x04 => frame.push(Value::Int(1))?,
            0x05 => frame.push(Value::Int(2))?,
            0x06 => frame.push(Value::Int(3))?,
            0x07 => frame.push(Value::Int(4))?,
            0x08 => frame.push(Value::Int(5))?,
            0x10 => {
                // bipush
                if frame.pc < code.len() {
                    let byte_val = code[frame.pc] as i8 as i32;
                    frame.pc += 1;
                    frame.push(Value::Int(byte_val))?;
                }
            }
            0x11 => {
                // sipush
                if frame.pc + 1 < code.len() {
                    let short_val = utils::read_i16(code, frame.pc) as i32;
                    frame.pc += 2;
                    frame.push(Value::Int(short_val))?;
                }
            }
            // Loads
            0x1a..=0x1d => {
                let idx = (opcode - 0x1a) as usize;
                let v = frame.load_local(idx)?;
                frame.push(v)?;
            }
            0x15 => {
                // iload
                if frame.pc < code.len() {
                    let idx = code[frame.pc] as usize;
                    frame.pc += 1;
                    let v = frame.load_local(idx)?;
                    frame.push(v)?;
                }
            }
            // Stores
            0x3b..=0x3e => {
                let idx = (opcode - 0x3b) as usize;
                let v = frame.pop()?;
                frame.store_local(idx, v)?;
            }
            0x36 => {
                // istore
                if frame.pc < code.len() {
                    let idx = code[frame.pc] as usize;
                    frame.pc += 1;
                    let v = frame.pop()?;
                    frame.store_local(idx, v)?;
                }
            }
            // Integer arithmetic
            0x60 => {
                let b = frame.pop()?.as_int();
                let a = frame.pop()?.as_int();
                frame.push(Value::Int(a.wrapping_add(b)))?;
            }
            0x64 => {
                let b = frame.pop()?.as_int();
                let a = frame.pop()?.as_int();
                frame.push(Value::Int(a.wrapping_sub(b)))?;
            }
            0x68 => {
                let b = frame.pop()?.as_int();
                let a = frame.pop()?.as_int();
                frame.push(Value::Int(a.wrapping_mul(b)))?;
            }
            0x6c => {
                let b = frame.pop()?.as_int();
                let a = frame.pop()?.as_int();
                let result = if b == 0 {
                    return Err(to_runtime_error_enum(RuntimeError::DivisionByZero));
                } else {
                    a.wrapping_div(b)
                };
                frame.push(Value::Int(result))?;
            }
            0x70 => {
                let b = frame.pop()?.as_int();
                let a = frame.pop()?.as_int();
                let result = if b == 0 {
                    return Err(to_runtime_error_enum(RuntimeError::DivisionByZero));
                } else {
                    a.wrapping_rem(b)
                };
                frame.push(Value::Int(result))?;
            }
            // Control flow
            0x99 => {
                // ifeq
                if frame.pc + 1 < code.len() {
                    let v = frame.pop()?.as_int();
                    let offset = utils::read_i16(code, frame.pc) as i32;
                    frame.pc += 2;
                    if v == 0 {
                        frame.pc = (frame.pc as i32 + offset - 3) as usize;
                    }
                }
            }
            0x9a => {
                // ifne
                if frame.pc + 1 < code.len() {
                    let v = frame.pop()?.as_int();
                    let offset = utils::read_i16(code, frame.pc) as i32;
                    frame.pc += 2;
                    if v != 0 {
                        frame.pc = (frame.pc as i32 + offset - 3) as usize;
                    }
                }
            }
            0x9b => {
                // iflt
                if frame.pc + 1 < code.len() {
                    let v = frame.pop()?.as_int();
                    let offset = utils::read_i16(code, frame.pc) as i32;
                    frame.pc += 2;
                    if v < 0 {
                        frame.pc = (frame.pc as i32 + offset - 3) as usize;
                    }
                }
            }
            0x9c => {
                // ifge
                if frame.pc + 1 < code.len() {
                    let v = frame.pop()?.as_int();
                    let offset = utils::read_i16(code, frame.pc) as i32;
                    frame.pc += 2;
                    if v >= 0 {
                        frame.pc = (frame.pc as i32 + offset - 3) as usize;
                    }
                }
            }
            0x9d => {
                // ifgt
                if frame.pc + 1 < code.len() {
                    let v = frame.pop()?.as_int();
                    let offset = utils::read_i16(code, frame.pc) as i32;
                    frame.pc += 2;
                    if v > 0 {
                        frame.pc = (frame.pc as i32 + offset - 3) as usize;
                    }
                }
            }
            0x9e => {
                // ifle
                if frame.pc + 1 < code.len() {
                    let v = frame.pop()?.as_int();
                    let offset = utils::read_i16(code, frame.pc) as i32;
                    frame.pc += 2;
                    if v <= 0 {
                        frame.pc = (frame.pc as i32 + offset - 3) as usize;
                    }
                }
            }
            0x9f => {
                // if_icmpeq
                if frame.pc + 1 < code.len() {
                    let v2 = frame.pop()?.as_int();
                    let v1 = frame.pop()?.as_int();
                    let offset = utils::read_i16(code, frame.pc) as i32;
                    frame.pc += 2;
                    if v1 == v2 {
                        frame.pc = (frame.pc as i32 + offset - 3) as usize;
                    }
                }
            }
            0xa0 => {
                // if_icmpne
                if frame.pc + 1 < code.len() {
                    let v2 = frame.pop()?.as_int();
                    let v1 = frame.pop()?.as_int();
                    let offset = utils::read_i16(code, frame.pc) as i32;
                    frame.pc += 2;
                    if v1 != v2 {
                        frame.pc = (frame.pc as i32 + offset - 3) as usize;
                    }
                }
            }
            0xa1 => {
                // if_icmplt
                if frame.pc + 1 < code.len() {
                    let v2 = frame.pop()?.as_int();
                    let v1 = frame.pop()?.as_int();
                    let offset = utils::read_i16(code, frame.pc) as i32;
                    frame.pc += 2;
                    if v1 < v2 {
                        frame.pc = (frame.pc as i32 + offset - 3) as usize;
                    }
                }
            }
            0xa2 => {
                // if_icmpge
                if frame.pc + 1 < code.len() {
                    let v2 = frame.pop()?.as_int();
                    let v1 = frame.pop()?.as_int();
                    let offset = utils::read_i16(code, frame.pc) as i32;
                    frame.pc += 2;
                    if v1 >= v2 {
                        frame.pc = (frame.pc as i32 + offset - 3) as usize;
                    }
                }
            }
            0xa3 => {
                // if_icmpgt
                if frame.pc + 1 < code.len() {
                    let v2 = frame.pop()?.as_int();
                    let v1 = frame.pop()?.as_int();
                    let offset = utils::read_i16(code, frame.pc) as i32;
                    frame.pc += 2;
                    if v1 > v2 {
                        frame.pc = (frame.pc as i32 + offset - 3) as usize;
                    }
                }
            }
            0xa4 => {
                // if_icmple
                if frame.pc + 1 < code.len() {
                    let v2 = frame.pop()?.as_int();
                    let v1 = frame.pop()?.as_int();
                    let offset = utils::read_i16(code, frame.pc) as i32;
                    frame.pc += 2;
                    if v1 <= v2 {
                        frame.pc = (frame.pc as i32 + offset - 3) as usize;
                    }
                }
            }
            0xa7 => {
                // goto
                if frame.pc + 1 < code.len() {
                    let offset = utils::read_i16(code, frame.pc) as i32;
                    frame.pc = (frame.pc as i32 - 1 + offset) as usize;
                }
            }
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
                // Skip pc for opcodes that take operands (stub - no-op execution)
                if opcode >= 0x12 && opcode <= 0x14 {
                    frame.pc += if opcode == 0x14 { 2 } else { 1 };
                } else if opcode >= 0x16 && opcode <= 0x19 {
                    frame.pc += 1;
                } else if opcode >= 0x36 && opcode <= 0x3a {
                    frame.pc += 1;
                } else if opcode >= 0x84 && opcode <= 0x84 {
                    frame.pc += 2; // iinc
                } else if opcode >= 0xb2 && opcode <= 0xb5 {
                    frame.pc += 2;
                }
            }
        }
        Ok(true)
    }
}
