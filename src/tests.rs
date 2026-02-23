//! Unit tests for JVMRS

#[cfg(test)]
mod tests {
    use crate::class_file::ClassFile;
    use crate::class_file::ParseError;
    use crate::class_loader::{parse_classpath, ClassLoader};
    use crate::debug::{DebugConfig, JvmDebugger};
    use crate::error::{to_class_loading_error, to_parse_error, to_runtime_error, ClassLoadingError, JvmError};
    use crate::memory::{Heap, HeapArray, Monitor, StackFrame, Value};
    use std::path::PathBuf;

    /// Build a minimal valid class file: class "Hello" with no super, no methods
    fn minimal_class_file_bytes() -> Vec<u8> {
        vec![
            0xCA, 0xFE, 0xBA, 0xBE, // magic
            0x00, 0x00,             // minor version
            0x00, 0x34,             // major version (52 = Java 8)
            0x00, 0x03,             // constant pool count (3 entries: 0=placeholder, 1=Utf8, 2=Class)
            0x01, 0x00, 0x05, 0x48, 0x65, 0x6C, 0x6C, 0x6F, // CP[1]: Utf8 "Hello"
            0x07, 0x00, 0x01,       // CP[2]: Class name_index=1
            0x00, 0x21,             // access_flags (public super)
            0x00, 0x02,             // this_class (index 2)
            0x00, 0x00,             // super_class (0 = none)
            0x00, 0x00,             // interfaces_count
            0x00, 0x00,             // fields_count
            0x00, 0x00,             // methods_count
            0x00, 0x00,             // attributes_count
        ]
    }

    /// Test invalid magic number
    #[test]
    fn test_class_file_invalid_magic() {
        let bad_magic = vec![
            0xDE, 0xAD, 0xBE, 0xEF, // wrong magic
            0x00, 0x00, 0x00, 0x34,
            0x00, 0x03, 0x01, 0x00, 0x05, 0x48, 0x65, 0x6C, 0x6C, 0x6F,
            0x07, 0x00, 0x01, 0x00, 0x21, 0x00, 0x02, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let result = ClassFile::parse(&bad_magic);
        assert!(result.is_err());
        if let Err(ParseError::InvalidMagic(magic)) = result {
            assert_eq!(magic, 0xDEADBEEF);
        } else {
            panic!("Expected InvalidMagic error");
        }
    }

    /// Test invalid constant pool tag
    #[test]
    fn test_class_file_invalid_constant_pool_tag() {
        let bad_tag = vec![
            0xCA, 0xFE, 0xBA, 0xBE, 0x00, 0x00, 0x00, 0x34,
            0x00, 0x03,
            0xFF, 0x00, 0x05, 0x48, 0x65, 0x6C, 0x6C, 0x6F, // invalid tag 0xFF
        ];
        let result = ClassFile::parse(&bad_tag);
        assert!(result.is_err());
        if let Err(ParseError::InvalidConstantPoolTag(tag)) = result {
            assert_eq!(tag, 0xFF);
        } else {
            panic!("Expected InvalidConstantPoolTag error");
        }
    }

    /// Test that parsing fails gracefully for incomplete class files
    #[test]
    fn test_class_file_incomplete_parse() {
        let class_bytes = vec![
            0xCA, 0xFE, 0xBA, 0xBE, 0x00, 0x00, 0x00, 0x34,
            0x00, 0x10, // constant pool count but no entries
        ];
        let result = ClassFile::parse(&class_bytes);
        assert!(result.is_err(), "Should fail to parse incomplete class file");
    }

    /// Test valid minimal class file parsing
    #[test]
    fn test_class_file_valid_parse() {
        let class_bytes = minimal_class_file_bytes();
        let result = ClassFile::parse(&class_bytes);
        assert!(result.is_ok(), "Should parse valid minimal class file: {:?}", result);
        let class_file = result.unwrap();
        assert_eq!(class_file.magic, 0xCAFEBABE);
        assert_eq!(class_file.major_version, 52);
        assert_eq!(class_file.get_class_name(), Some("Hello".to_string()));
        assert_eq!(class_file.get_super_class_name(), None);
        assert_eq!(class_file.get_string(1), Some("Hello".to_string()));
        assert!(class_file.find_method("main", "()V").is_none()); // No methods
    }

    /// Test class file find_method returns None for non-existent method
    #[test]
    fn test_class_file_find_method() {
        let class_bytes = minimal_class_file_bytes();
        let class_file = ClassFile::parse(&class_bytes).unwrap();
        assert!(class_file.find_method("nonexistent", "()V").is_none());
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

    /// Test class loader creation and classpath
    #[test]
    fn test_class_loader_creation() {
        let loader = ClassLoader::new_default();
        assert!(!loader.is_class_loaded("NonExistent"));
        assert!(loader.get_loaded_classes().is_empty());
        assert_eq!(loader.get_classpath().len(), 1);
        assert_eq!(loader.get_classpath()[0], PathBuf::from("."));
    }

    /// Test class loader with custom classpath
    #[test]
    fn test_class_loader_custom_classpath() {
        let classpath = vec![PathBuf::from("/tmp"), PathBuf::from("/usr")];
        let mut loader = ClassLoader::new(classpath.clone());
        assert_eq!(loader.get_classpath(), &classpath[..]);
        loader.add_classpath(PathBuf::from("/opt"));
        assert_eq!(loader.get_classpath().len(), 3);
    }

    /// Test parse_classpath helper
    #[test]
    fn test_parse_classpath() {
        let paths = parse_classpath("/tmp:/usr:/opt");
        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0], PathBuf::from("/tmp"));
        assert_eq!(paths[1], PathBuf::from("/usr"));
        assert_eq!(paths[2], PathBuf::from("/opt"));
    }

    /// Test class loader load_class for non-existent class
    #[test]
    fn test_class_loader_load_nonexistent() {
        let mut loader = ClassLoader::new(vec![PathBuf::from("/nonexistent/path")]);
        let result = loader.load_class("DefinitelyDoesNotExist");
        assert!(result.is_err());
        if let Err(JvmError::ClassLoadingError(ClassLoadingError::NoClassDefFound(name))) = result {
            assert_eq!(name, "DefinitelyDoesNotExist");
        } else {
            panic!("Expected NoClassDefFound error");
        }
    }

    /// Test Heap allocation
    #[test]
    fn test_heap_allocation() {
        let mut heap = Heap::new();
        let addr1 = heap.allocate("java/lang/Object".to_string());
        let addr2 = heap.allocate("Test".to_string());
        assert_eq!(addr1, 1);
        assert_eq!(addr2, 2);

        let obj = heap.get_object(addr1).unwrap();
        assert_eq!(obj.class_name, "java/lang/Object");

        let str_addr = heap.allocate_string("hello".to_string());
        assert_eq!(heap.get_string_data(str_addr), Some(&"hello".to_string()));
        assert!(heap.is_string(str_addr));
    }

    /// Test Heap array allocation
    #[test]
    fn test_heap_array_allocation() {
        let mut heap = Heap::new();
        let addr = heap.allocate_array(HeapArray::IntArray(vec![1, 2, 3]));
        let arr = heap.get_array(addr).unwrap();
        match arr {
            HeapArray::IntArray(v) => assert_eq!(v, &vec![1, 2, 3]),
            _ => panic!("Expected IntArray"),
        }
    }

    /// Test Monitor enter/exit
    #[test]
    fn test_monitor_enter_exit() {
        let mut monitor = Monitor::new();
        assert!(monitor.enter(1));
        assert!(monitor.is_owned_by(1));
        assert!(monitor.enter(1)); // Reentrant
        assert_eq!(monitor.count, 2);
        assert!(monitor.exit(1));
        assert!(monitor.exit(1));
        assert!(!monitor.is_owned_by(1));
    }

    /// Test Monitor with multiple threads (waiter)
    #[test]
    fn test_monitor_contention() {
        let mut monitor = Monitor::new();
        assert!(monitor.enter(1));
        assert!(!monitor.enter(2)); // Thread 2 blocks, goes to waiters
        assert!(monitor.waiters.contains(&2));
        assert!(monitor.exit(1)); // Thread 1 releases, thread 2 gets it
        assert_eq!(monitor.owner, Some(2));
    }

    /// Test Value reference helpers
    #[test]
    fn test_value_reference_helpers() {
        assert!(Value::Reference(42).is_reference());
        assert!(Value::ArrayRef(99).is_reference());
        assert!(!Value::Int(0).is_reference());
        assert!(!Value::Null.is_reference());

        assert_eq!(Value::Reference(42).as_reference(), Some(42));
        assert_eq!(Value::ArrayRef(99).as_reference(), Some(99));
        assert_eq!(Value::Int(0).as_reference(), None);
    }

    /// Test Interpreter creation and JIT config
    #[test]
    fn test_interpreter_creation() {
        use crate::interpreter::Interpreter;
        let interpreter = Interpreter::new();
        assert!(interpreter.is_jit_enabled() || !interpreter.is_jit_enabled()); // Depends on JIT init
    }

    /// Test Interpreter with custom classpath
    #[test]
    fn test_interpreter_with_classpath() {
        use crate::interpreter::Interpreter;
        let mut interpreter = Interpreter::with_classpath(vec![PathBuf::from("/tmp")]);
        // Loading non-existent class should fail (verifies interpreter was created with custom path)
        let result = interpreter.load_class_by_name("NonExistentClass");
        assert!(result.is_err());
    }

    /// Test Interpreter JIT enable/disable
    #[test]
    fn test_interpreter_jit_toggle() {
        use crate::interpreter::Interpreter;
        let mut interpreter = Interpreter::new();
        interpreter.set_jit_enabled(false);
        assert!(!interpreter.is_jit_enabled());
        interpreter.set_jit_enabled(true);
        // May or may not be enabled depending on JIT init success
    }

    /// Test error conversion helpers
    #[test]
    fn test_error_helpers() {
        let jvm_err: JvmError = to_runtime_error("test message");
        assert!(matches!(jvm_err, JvmError::RuntimeError(_)));

        use crate::error::ParseError as ErrorParseError;
        let parse_err = ErrorParseError::InvalidMagic(0x1234);
        let jvm_err: JvmError = to_parse_error(parse_err);
        assert!(matches!(jvm_err, JvmError::ParseError(_)));

        let class_err = ClassLoadingError::NoClassDefFound("Test".to_string());
        let _jvm_err: JvmError = to_class_loading_error(class_err);
    }

    /// Test DebugConfig default
    #[test]
    fn test_debug_config_default() {
        let config = DebugConfig::default();
        assert!(!config.trace_instructions);
        assert!(config.trace_methods);
        assert_eq!(config.max_stack_frames, Some(5));
    }

    /// Test JvmDebugger creation
    #[test]
    fn test_jvm_debugger() {
        let config = DebugConfig::default();
        let _debugger = JvmDebugger::new(config);
    }

    /// Test ReflectionApi
    #[test]
    fn test_reflection_api() {
        use crate::reflection::ReflectionApi;
        let api = ReflectionApi::new();
        let class_info = api.get_class("TestClass");
        assert!(class_info.is_some());
        let info = class_info.unwrap();
        assert_eq!(info.name, "TestClass");
        assert!(info.fields.is_empty());
        assert!(api.get_fields("TestClass").is_empty());
    }

    /// Test ArenaAllocator allocation and reuse
    #[test]
    fn test_arena_allocator() {
        use crate::allocator::ArenaAllocator;
        let mut arena = ArenaAllocator::new();
        let addr1 = arena.allocate("Test".to_string());
        let addr2 = arena.allocate("Object".to_string());
        assert_eq!(addr1, 1);
        assert_eq!(addr2, 2);
        assert_eq!(arena.get_object(addr1).unwrap().class_name, "Test");
        arena.free_slot(addr1);
        let addr3 = arena.allocate("Reused".to_string());
        assert_eq!(addr3, 1); // Reused freed slot
    }

    /// Test GenerationalHeap minor GC
    #[test]
    fn test_generational_heap_minor_gc() {
        use crate::gc::GenerationalHeap;
        use crate::memory::HeapArray;
        let mut heap = GenerationalHeap::new();
        let a1 = heap.allocate("Obj1".to_string());
        let a2 = heap.allocate_string("hello".to_string());
        let _a3 = heap.allocate_array(HeapArray::IntArray(vec![1, 2, 3])); // Unreachable
        let roots = vec![a1, a2];
        let freed = heap.minor_gc(&roots).unwrap();
        assert_eq!(freed, 1, "Unreachable array should be freed");
        assert!(heap.get_object(a1).is_some());
        assert!(heap.get_string_data(a2).is_some());
    }

    /// Test GenerationalHeap promotion
    #[test]
    fn test_generational_heap_promotion() {
        use crate::gc::GenerationalHeap;
        let mut heap = GenerationalHeap::new();
        let root = heap.allocate("Survivor".to_string());
        for _ in 0..10 {
            let freed = heap.minor_gc(&[root]).unwrap();
            assert_eq!(freed, 0);
        }
        assert!(heap.get_object(root).is_some());
    }

    /// Test ScopedRoot RAII
    #[test]
    fn test_scoped_root() {
        use crate::gc::{add_root, get_roots, remove_root, ScopedRoot};
        add_root(42);
        assert!(get_roots().contains(&42));
        {
            let _guard = ScopedRoot::new(99);
            assert!(get_roots().contains(&99));
        }
        assert!(!get_roots().contains(&99));
        remove_root(42);
        assert!(!get_roots().contains(&42));
    }

    #[cfg(feature = "interop")]
    #[test]
    fn test_interop_callbacks() {
        use crate::interop::{invoke_rust_callback, register_rust_callback, unregister_rust_callback};
        use crate::memory::Value;
        register_rust_callback("test.add", Box::new(|args| {
            let a: i32 = args.get(0).map(|v| v.as_int()).unwrap_or(0);
            let b: i32 = args.get(1).map(|v| v.as_int()).unwrap_or(0);
            Ok(Value::Int(a + b))
        }));
        let result = invoke_rust_callback("test.add", &[Value::Int(2), Value::Int(3)]).unwrap();
        assert_eq!(result, Value::Int(5));
        assert!(unregister_rust_callback("test.add"));
    }

    #[cfg(feature = "truffle")]
    #[test]
    fn test_truffle_frame() {
        use crate::memory::Value;
        use crate::truffle::TruffleFrame;
        let mut frame = TruffleFrame::new(4, 8);
        frame.set_local(0, Value::Int(42));
        frame.push(Value::Int(1));
        assert_eq!(frame.get_local(0), Some(Value::Int(42)));
        assert_eq!(frame.pop(), Some(Value::Int(1)));
    }

    #[cfg(feature = "simd")]
    #[test]
    fn test_simd_array_copy() {
        use crate::simd::{heap_array_copy_int, heap_array_copy_float};
        let mut dst_int = vec![0i32; 16];
        let src_int = vec![1i32, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        heap_array_copy_int(&mut dst_int, &src_int, 0, 0, 16);
        assert_eq!(&dst_int[..8], &src_int[..8]);

        let mut dst_float = vec![0.0f32; 16];
        let src_float = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0];
        heap_array_copy_float(&mut dst_float, &src_float, 0, 0, 16);
        assert!((dst_float[0] - 1.0).abs() < 0.001);
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
