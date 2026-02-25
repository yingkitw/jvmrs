//! Built-in runtime operations (println, string concatenation, etc.).

use crate::class_file::ClassFile;
use crate::error::{to_runtime_error_enum, JvmError, RuntimeError};
use crate::memory::{StackFrame, Value};

use super::Interpreter;
use super::InterpreterResult;

impl Interpreter {
    /// Native println implementation (PrintStream.println)
    pub(crate) fn native_println(&mut self, value: Value) -> InterpreterResult {
        match value {
            Value::Int(i) => println!("{}", i),
            Value::Float(f) => println!("{}", f),
            Value::Long(l) => println!("{}", l),
            Value::Double(d) => println!("{}", d),
            Value::Reference(addr) => {
                if let Some(obj) = self.memory.heap.get_object(addr) {
                    if obj.class_name == "java/lang/String" {
                        if let Some(string_data) = &obj.string_data {
                            println!("{}", string_data);
                        } else {
                            println!("[String]");
                        }
                    } else {
                        println!("[{}]", obj.class_name);
                    }
                } else {
                    println!("null");
                }
            }
            Value::ArrayRef(addr) => println!("[Array@{}]", addr),
            Value::Null => println!("null"),
            _ => println!("{:?}", value),
        }
        Ok(())
    }

    /// Handle invokedynamic instruction (e.g. string concatenation)
    pub(crate) fn handle_invokedynamic(
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

        let (_bootstrap_index, name_and_type_index) = match cp_entry {
            crate::class_file::ConstantPoolEntry::ConstantInvokeDynamic {
                bootstrap_method_attr_index,
                name_and_type_index,
            } => (bootstrap_method_attr_index, name_and_type_index),
            _ => {
                return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                    "Expected InvokeDynamic constant".to_string(),
                )))
            }
        };

        let name_and_type_entry = class.constant_pool.get(*name_and_type_index as usize);
        let method_name = match name_and_type_entry {
            Some(crate::class_file::ConstantPoolEntry::ConstantNameAndType {
                name_index, ..
            }) => class.get_string(*name_index).unwrap_or_default(),
            _ => {
                return Err(to_runtime_error_enum(RuntimeError::IllegalArgument(
                    "Invalid NameAndType in invokedynamic".to_string(),
                )))
            }
        };

        if method_name == "makeConcatWithConstants" {
            let mut result = String::new();

            if !frame.stack.is_empty() {
                let value = frame.pop()?;
                match value {
                    Value::Int(i) => result = format!("{}", i),
                    Value::Float(f) => result = format!("{}", f),
                    Value::Double(d) => result = format!("{}", d),
                    Value::Long(l) => result = format!("{}", l),
                    Value::Reference(addr) => {
                        if let Some(string_data) = self.memory.heap.get_string_data(addr) {
                            result = string_data.to_string();
                        } else {
                            result = "[object]".to_string();
                        }
                    }
                    _ => result = "[object]".to_string(),
                }
            }

            let addr = self.memory.heap.allocate_string(result);
            frame.push(Value::Reference(addr))?;
            return Ok(());
        }

        frame.push(Value::Null)?;
        Ok(())
    }
}
