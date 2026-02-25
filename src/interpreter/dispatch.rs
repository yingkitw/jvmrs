//! Instruction dispatch - handles individual bytecode opcodes.

use crate::class_file::{ConstantPoolEntry, ClassFile};
use crate::error::{to_runtime_error_enum, JvmError, RuntimeError};
use crate::memory::{StackFrame, Value};
use crate::memory::HeapArray;

use super::Interpreter;
use super::utils;

impl Interpreter {
    /// Resolve ldc constant pool entry to Value (for ldc, ldc_w, ldc2_w)
    fn resolve_ldc(&mut self, class: &ClassFile, index: usize) -> Result<Value, JvmError> {
        let entry = class.constant_pool.get(index).ok_or_else(|| {
            to_runtime_error_enum(RuntimeError::IllegalArgument(format!(
                "Constant pool index {} out of bounds",
                index
            )))
        })?;
        match entry {
            ConstantPoolEntry::ConstantInteger { bytes } => Ok(Value::Int(*bytes)),
            ConstantPoolEntry::ConstantFloat { bytes } => Ok(Value::Float(*bytes)),
            ConstantPoolEntry::ConstantString { string_index } => {
                let s = class.get_string(*string_index).unwrap_or_default();
                let addr = self.memory.heap.allocate_string(s);
                Ok(Value::Reference(addr))
            }
            _ => Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                "ldc: invalid constant type".to_string(),
            ))),
        }
    }

    fn resolve_ldc2_w(&self, class: &ClassFile, index: usize) -> Result<Value, JvmError> {
        let entry = class.constant_pool.get(index).ok_or_else(|| {
            to_runtime_error_enum(RuntimeError::IllegalArgument(format!(
                "Constant pool index {} out of bounds",
                index
            )))
        })?;
        match entry {
            ConstantPoolEntry::ConstantLong { bytes } => Ok(Value::Long(*bytes)),
            ConstantPoolEntry::ConstantDouble { bytes } => Ok(Value::Double(*bytes)),
            _ => Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                "ldc2_w: expected Long or Double".to_string(),
            ))),
        }
    }

    /// Ensure java/lang/System.out is initialized (synthetic PrintStream)
    pub(crate) fn ensure_system_out(&mut self) {
        let fields = self
            .memory
            .static_fields
            .entry("java/lang/System".to_string())
            .or_insert_with(std::collections::HashMap::new);
        if !fields.contains_key("out") {
            let addr = self.memory.heap.allocate("java/io/PrintStream".to_string());
            fields.insert("out".to_string(), Value::Reference(addr));
        }
    }
    /// Dispatch a single bytecode instruction. Returns false to break (return).
    #[inline(always)]
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
            0x12 => {
                // ldc
                if frame.pc < code.len() {
                    let idx = code[frame.pc] as usize;
                    frame.pc += 1;
                    let val = self.resolve_ldc(class, idx)?;
                    frame.push(val)?;
                }
            }
            0x13 => {
                // ldc_w
                if frame.pc + 1 < code.len() {
                    let idx = utils::read_u16(code, frame.pc) as usize;
                    frame.pc += 2;
                    let val = self.resolve_ldc(class, idx)?;
                    frame.push(val)?;
                }
            }
            0x14 => {
                // ldc2_w
                if frame.pc + 1 < code.len() {
                    let idx = utils::read_u16(code, frame.pc) as usize;
                    frame.pc += 2;
                    let val = self.resolve_ldc2_w(class, idx)?;
                    frame.push(val)?;
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
            // aload variants
            0x19 => {
                // aload
                if frame.pc < code.len() {
                    let idx = code[frame.pc] as usize;
                    frame.pc += 1;
                    let v = frame.load_local(idx)?;
                    frame.push(v)?;
                }
            }
            0x2a..=0x2d => {
                let idx = (opcode - 0x2a) as usize;
                let v = frame.load_local(idx)?;
                frame.push(v)?;
            }
            0x3a => {
                // astore
                if frame.pc < code.len() {
                    let idx = code[frame.pc] as usize;
                    frame.pc += 1;
                    let v = frame.pop()?;
                    frame.store_local(idx, v)?;
                }
            }
            0x4b..=0x4e => {
                let idx = (opcode - 0x4b) as usize;
                let v = frame.pop()?;
                frame.store_local(idx, v)?;
            }
            // Stack manipulation
            0x57 => {
                frame.pop()?;
            }
            0x59 => {
                let v = frame.peek()?.clone();
                frame.push(v)?;
            }
            0x5a => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v1.clone())?;
                frame.push(v2)?;
                frame.push(v1)?;
            }
            0x5f => {
                let v1 = frame.pop()?;
                let v2 = frame.pop()?;
                frame.push(v1)?;
                frame.push(v2)?;
            }
            0x84 => {
                // iinc
                if frame.pc + 2 <= code.len() {
                    let idx = code[frame.pc] as usize;
                    let delta = code[frame.pc + 1] as i8 as i32;
                    frame.pc += 2;
                    let v = frame.load_local(idx)?.as_int();
                    frame.store_local(idx, Value::Int(v.wrapping_add(delta)))?;
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
            0xba => {
                if frame.pc + 2 < code.len() {
                    let index = utils::read_u16(code, frame.pc) as usize;
                    frame.pc += 2;
                    let _bootstrap = utils::read_u16(code, frame.pc);
                    frame.pc += 2;
                    self.handle_invokedynamic(class, frame, index)?;
                }
            }
            0xb2 => {
                if frame.pc + 1 < code.len() {
                    let index = utils::read_u16(code, frame.pc) as usize;
                    frame.pc += 2;
                    self.get_static(class, frame, index)?;
                }
            }
            0xb3 => {
                if frame.pc + 1 < code.len() {
                    let index = utils::read_u16(code, frame.pc) as usize;
                    frame.pc += 2;
                    self.put_static(class, frame, index)?;
                }
            }
            0xb4 => {
                if frame.pc + 1 < code.len() {
                    let index = utils::read_u16(code, frame.pc) as usize;
                    frame.pc += 2;
                    self.get_field(class, frame, index)?;
                }
            }
            0xb5 => {
                if frame.pc + 1 < code.len() {
                    let index = utils::read_u16(code, frame.pc) as usize;
                    frame.pc += 2;
                    self.put_field(class, frame, index)?;
                }
            }
            0xb7 => {
                if frame.pc + 1 < code.len() {
                    let index = utils::read_u16(code, frame.pc) as usize;
                    frame.pc += 2;
                    self.invoke_special(class, frame, index)?;
                }
            }
            0xbb => {
                if frame.pc + 1 < code.len() {
                    let index = utils::read_u16(code, frame.pc) as usize;
                    frame.pc += 2;
                    self.do_new(class, frame, index)?;
                }
            }
            0xbc => {
                if frame.pc < code.len() {
                    let atype = code[frame.pc];
                    frame.pc += 1;
                    let len = frame.pop()?.as_int();
                    if len < 0 {
                        return Err(to_runtime_error_enum(RuntimeError::NegativeArraySizeException(len)));
                    }
                    let arr = match atype {
                        4 => HeapArray::BooleanArray(vec![false; len as usize]),
                        5 => HeapArray::CharArray(vec![0; len as usize]),
                        6 => HeapArray::FloatArray(vec![0.0; len as usize]),
                        7 => HeapArray::DoubleArray(vec![0.0; len as usize]),
                        8 => HeapArray::ByteArray(vec![0; len as usize]),
                        9 => HeapArray::ShortArray(vec![0; len as usize]),
                        10 => HeapArray::IntArray(vec![0; len as usize]),
                        11 => HeapArray::LongArray(vec![0; len as usize]),
                        _ => {
                            return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                                format!("Unknown array type: {}", atype),
                            )))
                        }
                    };
                    let addr = self.memory.heap.allocate_array(arr);
                    frame.push(Value::ArrayRef(addr))?;
                }
            }
            0xbd => {
                if frame.pc + 1 < code.len() {
                    let index = utils::read_u16(code, frame.pc) as usize;
                    frame.pc += 2;
                    self.do_anewarray(class, frame, index)?;
                }
            }
            0xbe => {
                let arr_ref = frame.pop()?.as_reference().ok_or_else(|| {
                    to_runtime_error_enum(RuntimeError::NullPointerException)
                })?;
                let len = self.memory.heap.array_length(arr_ref).map_err(|e| {
                    to_runtime_error_enum(RuntimeError::from(e))
                })?;
                frame.push(Value::Int(len as i32))?;
            }
            0x2e => {
                let idx = frame.pop()?.as_int();
                let arr_ref = frame.pop()?.as_reference().ok_or_else(|| {
                    to_runtime_error_enum(RuntimeError::NullPointerException)
                })?;
                let val = self.memory.heap.array_get(arr_ref, idx as usize)
                    .map_err(|e| to_runtime_error_enum(RuntimeError::from(e)))?;
                frame.push(val)?;
            }
            0x4f => {
                let val = frame.pop()?;
                let idx = frame.pop()?.as_int();
                let arr_ref = frame.pop()?.as_reference().ok_or_else(|| {
                    to_runtime_error_enum(RuntimeError::NullPointerException)
                })?;
                self.memory.heap.array_set(arr_ref, idx as usize, val).map_err(|e| {
                    to_runtime_error_enum(RuntimeError::from(e))
                })?;
            }
            0x32 => {
                let idx = frame.pop()?.as_int();
                let arr_ref = frame.pop()?.as_reference().ok_or_else(|| {
                    to_runtime_error_enum(RuntimeError::NullPointerException)
                })?;
                let val = self.memory.heap.array_get(arr_ref, idx as usize)
                    .map_err(|e| to_runtime_error_enum(RuntimeError::from(e)))?;
                frame.push(val)?;
            }
            0x53 => {
                let val = frame.pop()?;
                let idx = frame.pop()?.as_int();
                let arr_ref = frame.pop()?.as_reference().ok_or_else(|| {
                    to_runtime_error_enum(RuntimeError::NullPointerException)
                })?;
                let ref_val = match &val {
                    Value::Null => Value::Reference(0),
                    _ => val,
                };
                self.memory.heap.array_set(arr_ref, idx as usize, ref_val).map_err(|e| {
                    to_runtime_error_enum(RuntimeError::from(e))
                })?;
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
