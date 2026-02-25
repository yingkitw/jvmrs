//! Stack frame for method execution.

use crate::error::{to_runtime_error_enum, JvmError, RuntimeError};

use super::Value;

/// A stack frame for method execution
#[derive(Debug, Clone)]
pub struct StackFrame {
    pub locals: Vec<Value>,
    pub stack: Vec<Value>,
    pub pc: usize,
    pub method_name: String,
}

impl StackFrame {
    pub fn new(max_locals: usize, max_stack: usize, method_name: String) -> Self {
        StackFrame {
            locals: vec![Value::Int(0); max_locals],
            stack: Vec::with_capacity(max_stack),
            pc: 0,
            method_name,
        }
    }

    pub fn push(&mut self, value: Value) -> Result<(), JvmError> {
        self.stack.push(value);
        Ok(())
    }

    pub fn pop(&mut self) -> Result<Value, JvmError> {
        self.stack
            .pop()
            .ok_or_else(|| to_runtime_error_enum(RuntimeError::StackUnderflow))
    }

    pub fn peek(&self) -> Result<&Value, JvmError> {
        self.stack
            .last()
            .ok_or_else(|| to_runtime_error_enum(RuntimeError::StackUnderflow))
    }

    pub fn load_local(&self, index: usize) -> Result<Value, JvmError> {
        self.locals
            .get(index)
            .cloned()
            .ok_or_else(|| to_runtime_error_enum(RuntimeError::LocalVariableOutOfBounds(index)))
    }

    pub fn store_local(&mut self, index: usize, value: Value) -> Result<(), JvmError> {
        if index >= self.locals.len() {
            return Err(to_runtime_error_enum(
                RuntimeError::LocalVariableOutOfBounds(index),
            ));
        }
        self.locals[index] = value;
        Ok(())
    }
}
