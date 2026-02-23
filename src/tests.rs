//! Unit tests for JVMRS

#[cfg(test)]
mod tests {
    use crate::class_file::ClassFile;
    use crate::error::JvmError;
    use crate::memory::{StackFrame, Value};

    /// Test basic class file parsing
    #[test]
    fn test_class_file_parsing() {
        // Create a simple class file byte array
        let class_bytes = vec![
            0xCA, 0xFE, 0xBA, 0xBE, // magic
            0x00, 0x00, // minor version
            0x00, 0x34, // major version (52 = Java 8)
            0x00,
            0x10, // constant pool count (16 entries)
                  // Constant pool entries would go here
                  // This is a minimal test - actual parsing would require full class file
        ];

        // Test that parsing fails gracefully for incomplete class files
        let result = ClassFile::parse(&class_bytes);
        assert!(
            result.is_err(),
            "Should fail to parse incomplete class file"
        );
    }

    /// Test stack frame operations
    #[test]
    fn test_stack_frame() -> Result<(), JvmError> {
        let mut frame = StackFrame::new(10, 20, "test".to_string());

        // Test push and pop
        frame.push(Value::Int(42))?;
        let value = frame.pop()?;
        assert_eq!(value, Value::Int(42));

        // Test local variable storage
        frame.store_local(0, Value::Int(100))?;
        let local_value = frame.load_local(0)?;
        assert_eq!(local_value, Value::Int(100));

        // Test stack underflow
        assert!(frame.pop().is_err(), "Should fail on stack underflow");

        // Test local variable bounds
        assert!(
            frame.load_local(100).is_err(),
            "Should fail on out of bounds access"
        );
        assert!(
            frame.store_local(100, Value::Int(0)).is_err(),
            "Should fail on out of bounds store"
        );

        Ok(())
    }

    /// Test value conversions
    #[test]
    fn test_value_conversions() {
        let int_value = Value::Int(42);
        let float_value = Value::Float(3.14);
        let long_value = Value::Long(1000);
        let double_value = Value::Double(2.718);

        // Test as_int conversions
        assert_eq!(int_value.as_int(), 42);
        assert_eq!(float_value.as_int(), 3); // truncation
        assert_eq!(long_value.as_int(), 1000);

        // Test as_float conversions
        assert_eq!(int_value.as_float(), 42.0);
        assert!((float_value.as_float() - 3.14).abs() < 0.0001);
        assert!((double_value.as_float() - 2.718).abs() < 0.0001);

        // Test as_long conversions
        assert_eq!(int_value.as_long(), 42);
        assert_eq!(long_value.as_long(), 1000);

        // Test as_double conversions
        assert_eq!(int_value.as_double(), 42.0);
        assert!((float_value.as_double() - 3.14).abs() < 0.0001);
        assert_eq!(double_value.as_double(), 2.718);
    }

    /// Test value equality
    #[test]
    fn test_value_equality() {
        assert_eq!(Value::Int(42), Value::Int(42));
        assert_ne!(Value::Int(42), Value::Int(43));
        assert_eq!(Value::Float(3.14), Value::Float(3.14));
        assert_ne!(Value::Float(3.14), Value::Float(3.15));
        assert_eq!(Value::Null, Value::Null);
        assert_ne!(Value::Null, Value::Int(0));
    }

    /// Test error types
    #[test]
    fn test_error_display() {
        use crate::error::{ClassLoadingError, MemoryError, NativeError, ParseError, RuntimeError};

        // Test RuntimeError display
        let stack_underflow = RuntimeError::StackUnderflow;
        assert_eq!(format!("{}", stack_underflow), "Stack underflow");

        let division_by_zero = RuntimeError::DivisionByZero;
        assert_eq!(format!("{}", division_by_zero), "Division by zero");

        let class_not_found = RuntimeError::ClassNotFound("Test".to_string());
        assert_eq!(format!("{}", class_not_found), "Class not found: Test");

        let invalid_opcode = RuntimeError::InvalidOpcode(0xFF);
        assert_eq!(format!("{}", invalid_opcode), "Invalid opcode: 0xFF");

        // Test ParseError display
        let invalid_magic = ParseError::InvalidMagic(0xDEADBEEF);
        assert_eq!(
            format!("{}", invalid_magic),
            "Invalid magic number: 0xDEADBEEF (expected 0xCAFEBABE)"
        );

        // Test MemoryError display
        let out_of_memory = MemoryError::OutOfMemory;
        assert_eq!(format!("{}", out_of_memory), "Out of memory");

        // Test ClassLoadingError display
        let class_file_not_found = ClassLoadingError::ClassFileNotFound("Test.class".to_string());
        assert_eq!(
            format!("{}", class_file_not_found),
            "Class file not found: Test.class"
        );

        // Test NativeError display
        let native_method_not_found = NativeError::NativeMethodNotFound(
            "java.lang.System".to_string(),
            "currentTimeMillis".to_string(),
        );
        assert_eq!(
            format!("{}", native_method_not_found),
            "Native method java.lang.System.currentTimeMillis not found"
        );
    }
}
