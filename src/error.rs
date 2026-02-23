//! Custom error types for JVMRS

use std::fmt;

/// Main error type for JVM operations
#[derive(Debug)]
pub enum JvmError {
    /// Class file parsing errors
    ParseError(ParseError),
    /// Runtime execution errors
    RuntimeError(RuntimeError),
    /// Memory management errors
    MemoryError(MemoryError),
    /// Class loading errors
    ClassLoadingError(ClassLoadingError),
    /// Native method errors
    NativeError(NativeError),
}

impl fmt::Display for JvmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JvmError::ParseError(e) => write!(f, "Parse error: {}", e),
            JvmError::RuntimeError(e) => write!(f, "Runtime error: {}", e),
            JvmError::MemoryError(e) => write!(f, "Memory error: {}", e),
            JvmError::ClassLoadingError(e) => write!(f, "Class loading error: {}", e),
            JvmError::NativeError(e) => write!(f, "Native error: {}", e),
        }
    }
}

impl std::error::Error for JvmError {}

impl From<String> for JvmError {
    fn from(err: String) -> Self {
        JvmError::RuntimeError(RuntimeError::Unimplemented(err))
    }
}

impl From<&str> for JvmError {
    fn from(err: &str) -> Self {
        JvmError::RuntimeError(RuntimeError::Unimplemented(err.to_string()))
    }
}

/// Class file parsing errors
#[derive(Debug)]
pub enum ParseError {
    /// Invalid magic number (not 0xCAFEBABE)
    InvalidMagic(u32),
    /// Unsupported class file version
    UnsupportedVersion(u16, u16),
    /// Invalid constant pool tag
    InvalidConstantPoolTag(u8),
    /// Invalid attribute length
    InvalidAttributeLength,
    /// Invalid UTF-8 string in constant pool
    InvalidUtf8String,
    /// Invalid method descriptor
    InvalidMethodDescriptor(String),
    /// Invalid field descriptor
    InvalidFieldDescriptor(String),
    /// Invalid opcode
    InvalidOpcode(u8),
    /// Invalid bytecode
    InvalidBytecode(String),
    /// IO error
    IoError(std::io::Error),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::InvalidMagic(magic) => write!(
                f,
                "Invalid magic number: 0x{:08X} (expected 0xCAFEBABE)",
                magic
            ),
            ParseError::UnsupportedVersion(major, minor) => {
                write!(f, "Unsupported class file version: {}.{}", major, minor)
            }
            ParseError::InvalidConstantPoolTag(tag) => {
                write!(f, "Invalid constant pool tag: {}", tag)
            }
            ParseError::InvalidAttributeLength => write!(f, "Invalid attribute length"),
            ParseError::InvalidUtf8String => write!(f, "Invalid UTF-8 string in constant pool"),
            ParseError::InvalidMethodDescriptor(desc) => {
                write!(f, "Invalid method descriptor: {}", desc)
            }
            ParseError::InvalidFieldDescriptor(desc) => {
                write!(f, "Invalid field descriptor: {}", desc)
            }
            ParseError::InvalidOpcode(opcode) => write!(f, "Invalid opcode: 0x{:02X}", opcode),
            ParseError::InvalidBytecode(msg) => write!(f, "Invalid bytecode: {}", msg),
            ParseError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl From<std::io::Error> for ParseError {
    fn from(err: std::io::Error) -> Self {
        ParseError::IoError(err)
    }
}

/// Runtime execution errors
#[derive(Debug, Clone)]
pub enum RuntimeError {
    /// Stack underflow (pop from empty stack)
    StackUnderflow,
    /// Stack overflow (push to full stack)
    StackOverflow,
    /// Local variable index out of bounds
    LocalVariableOutOfBounds(usize),
    /// Array index out of bounds
    ArrayIndexOutOfBounds(usize, usize),
    /// Null pointer exception
    NullPointerException,
    /// Division by zero
    DivisionByZero,
    /// Class not found
    ClassNotFound(String),
    /// Method not found
    MethodNotFound(String, String),
    /// Field not found
    FieldNotFound(String, String),
    /// Invalid type conversion
    InvalidTypeConversion(String, String),
    /// Unsupported operation
    UnsupportedOperation(String),
    /// Arithmetic overflow
    ArithmeticOverflow,
    /// Invalid object reference
    InvalidReference(u32),
    /// Invalid array type
    InvalidArrayType(String),
    /// Invalid array length
    InvalidArrayLength(usize),
    /// Invalid monitor state
    InvalidMonitorState,
    /// Illegal monitor state
    IllegalMonitorState,
    /// Illegal argument
    IllegalArgument(String),
    /// Illegal state
    IllegalState(String),
    /// Unimplemented feature
    Unimplemented(String),
    /// Invalid opcode
    InvalidOpcode(u8),
    /// Exception thrown
    ExceptionThrown(String),
    /// Class cast exception
    ClassCastException(String, String),
    /// Array store exception
    ArrayStoreException,
    /// Negative array size exception
    NegativeArraySizeException(i32),
    /// Illegal access exception
    IllegalAccessException(String),
    /// Instantiation exception
    InstantiationException(String),
    /// String index out of bounds
    StringIndexOutOfBounds(usize, usize),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::StackUnderflow => write!(f, "Stack underflow"),
            RuntimeError::StackOverflow => write!(f, "Stack overflow"),
            RuntimeError::LocalVariableOutOfBounds(index) => {
                write!(f, "Local variable index {} out of bounds", index)
            }
            RuntimeError::ArrayIndexOutOfBounds(index, length) => write!(
                f,
                "Array index {} out of bounds (length: {})",
                index, length
            ),
            RuntimeError::NullPointerException => write!(f, "Null pointer exception"),
            RuntimeError::DivisionByZero => write!(f, "Division by zero"),
            RuntimeError::ClassNotFound(name) => write!(f, "Class not found: {}", name),
            RuntimeError::MethodNotFound(class, method) => {
                write!(f, "Method {} not found in class {}", method, class)
            }
            RuntimeError::FieldNotFound(class, field) => {
                write!(f, "Field {} not found in class {}", field, class)
            }
            RuntimeError::InvalidTypeConversion(from, to) => {
                write!(f, "Invalid type conversion from {} to {}", from, to)
            }
            RuntimeError::UnsupportedOperation(op) => write!(f, "Unsupported operation: {}", op),
            RuntimeError::ArithmeticOverflow => write!(f, "Arithmetic overflow"),
            RuntimeError::InvalidReference(addr) => write!(f, "Invalid object reference: {}", addr),
            RuntimeError::InvalidArrayType(ty) => write!(f, "Invalid array type: {}", ty),
            RuntimeError::InvalidArrayLength(len) => write!(f, "Invalid array length: {}", len),
            RuntimeError::InvalidMonitorState => write!(f, "Invalid monitor state"),
            RuntimeError::IllegalMonitorState => write!(f, "Illegal monitor state"),
            RuntimeError::IllegalArgument(msg) => write!(f, "Illegal argument: {}", msg),
            RuntimeError::IllegalState(msg) => write!(f, "Illegal state: {}", msg),
            RuntimeError::Unimplemented(feature) => write!(f, "Unimplemented feature: {}", feature),
            RuntimeError::InvalidOpcode(opcode) => write!(f, "Invalid opcode: 0x{:02X}", opcode),
            RuntimeError::ExceptionThrown(msg) => write!(f, "Exception thrown: {}", msg),
            RuntimeError::ClassCastException(from, to) => write!(
                f,
                "Class cast exception: cannot cast from {} to {}",
                from, to
            ),
            RuntimeError::ArrayStoreException => write!(f, "Array store exception"),
            RuntimeError::NegativeArraySizeException(size) => {
                write!(f, "Negative array size exception: {}", size)
            }
            RuntimeError::IllegalAccessException(msg) => {
                write!(f, "Illegal access exception: {}", msg)
            }
            RuntimeError::InstantiationException(msg) => {
                write!(f, "Instantiation exception: {}", msg)
            }
            RuntimeError::StringIndexOutOfBounds(index, length) => write!(
                f,
                "String index {} out of bounds (length: {})",
                index, length
            ),
        }
    }
}

/// Memory management errors
#[derive(Debug)]
pub enum MemoryError {
    /// Out of memory
    OutOfMemory,
    /// Invalid heap address
    InvalidHeapAddress(u32),
    /// Heap corruption detected
    HeapCorruption,
    /// Garbage collection failed
    GcError(String),
    /// Memory limit exceeded
    MemoryLimitExceeded(usize),
    /// Invalid object header
    InvalidObjectHeader,
    /// Invalid array header
    InvalidArrayHeader,
    /// Memory allocation failed
    AllocationFailed(String),
    /// Invalid array length
    InvalidArrayLength(usize),
    /// Invalid array type
    InvalidArrayType(String),
    /// Array index out of bounds
    ArrayIndexOutOfBounds(usize, usize), // index, length
    /// Invalid array operation
    InvalidArrayOperation(String),
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::OutOfMemory => write!(f, "Out of memory"),
            MemoryError::InvalidHeapAddress(addr) => write!(f, "Invalid heap address: {}", addr),
            MemoryError::HeapCorruption => write!(f, "Heap corruption detected"),
            MemoryError::GcError(msg) => write!(f, "Garbage collection error: {}", msg),
            MemoryError::MemoryLimitExceeded(limit) => {
                write!(f, "Memory limit exceeded: {} bytes", limit)
            }
            MemoryError::InvalidObjectHeader => write!(f, "Invalid object header"),
            MemoryError::InvalidArrayHeader => write!(f, "Invalid array header"),
            MemoryError::AllocationFailed(msg) => write!(f, "Memory allocation failed: {}", msg),
            MemoryError::InvalidArrayLength(len) => write!(f, "Invalid array length: {}", len),
            MemoryError::InvalidArrayType(ty) => write!(f, "Invalid array type: {}", ty),
            MemoryError::ArrayIndexOutOfBounds(index, length) => write!(
                f,
                "Array index {} out of bounds (length: {})",
                index, length
            ),
            MemoryError::InvalidArrayOperation(msg) => {
                write!(f, "Invalid array operation: {}", msg)
            }
        }
    }
}

/// Class loading errors
#[derive(Debug)]
pub enum ClassLoadingError {
    /// Class file not found
    ClassFileNotFound(String),
    /// Class format error
    ClassFormatError(String),
    /// Class circularity error
    ClassCircularityError(String),
    /// No class definition found
    NoClassDefFound(String),
    /// Unsupported class version
    UnsupportedClassVersion(String, u16, u16),
    /// Class verification failed
    VerificationFailed(String),
    /// Linkage error
    LinkageError(String),
    /// Illegal access error
    IllegalAccessError(String),
    /// Instantiation error
    InstantiationError(String),
    /// Class loader constraint violation
    ClassLoaderConstraintViolation(String),
}

impl fmt::Display for ClassLoadingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClassLoadingError::ClassFileNotFound(name) => {
                write!(f, "Class file not found: {}", name)
            }
            ClassLoadingError::ClassFormatError(msg) => write!(f, "Class format error: {}", msg),
            ClassLoadingError::ClassCircularityError(name) => {
                write!(f, "Class circularity error: {}", name)
            }
            ClassLoadingError::NoClassDefFound(name) => {
                write!(f, "No class definition found: {}", name)
            }
            ClassLoadingError::UnsupportedClassVersion(name, major, minor) => write!(
                f,
                "Unsupported class version for {}: {}.{}",
                name, major, minor
            ),
            ClassLoadingError::VerificationFailed(msg) => {
                write!(f, "Class verification failed: {}", msg)
            }
            ClassLoadingError::LinkageError(msg) => write!(f, "Linkage error: {}", msg),
            ClassLoadingError::IllegalAccessError(msg) => {
                write!(f, "Illegal access error: {}", msg)
            }
            ClassLoadingError::InstantiationError(msg) => write!(f, "Instantiation error: {}", msg),
            ClassLoadingError::ClassLoaderConstraintViolation(msg) => {
                write!(f, "Class loader constraint violation: {}", msg)
            }
        }
    }
}

/// Native method errors
#[derive(Debug)]
pub enum NativeError {
    /// Native method not found
    NativeMethodNotFound(String, String),
    /// Native method failed
    NativeMethodFailed(String, String),
    /// Native library not found
    NativeLibraryNotFound(String),
    /// Native library load failed
    NativeLibraryLoadFailed(String),
    /// Unsatisfied link error
    UnsatisfiedLinkError(String),
    /// Native method signature mismatch
    NativeMethodSignatureMismatch(String),
}

impl fmt::Display for NativeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NativeError::NativeMethodNotFound(class, method) => {
                write!(f, "Native method {}.{} not found", class, method)
            }
            NativeError::NativeMethodFailed(class, method) => {
                write!(f, "Native method {}.{} failed", class, method)
            }
            NativeError::NativeLibraryNotFound(lib) => {
                write!(f, "Native library not found: {}", lib)
            }
            NativeError::NativeLibraryLoadFailed(lib) => {
                write!(f, "Native library load failed: {}", lib)
            }
            NativeError::UnsatisfiedLinkError(msg) => write!(f, "Unsatisfied link error: {}", msg),
            NativeError::NativeMethodSignatureMismatch(msg) => {
                write!(f, "Native method signature mismatch: {}", msg)
            }
        }
    }
}

/// Result type for JVM operations
pub type JvmResult<T> = Result<T, JvmError>;

/// Convenience result type for interpreter operations
pub type InterpreterResult = Result<(), JvmError>;

/// Convenience result type for class file operations
pub type ClassFileResult<T> = Result<T, JvmError>;

/// Convenience result type for memory operations
pub type MemoryResult<T> = Result<T, JvmError>;

/// Helper to convert string errors to JvmError
pub fn to_runtime_error<T: ToString>(msg: T) -> JvmError {
    JvmError::RuntimeError(RuntimeError::Unimplemented(msg.to_string()))
}

/// Helper to convert parse errors
pub fn to_parse_error<T: Into<ParseError>>(err: T) -> JvmError {
    JvmError::ParseError(err.into())
}

/// Helper to convert runtime errors
pub fn to_runtime_error_enum<T: Into<RuntimeError>>(err: T) -> JvmError {
    JvmError::RuntimeError(err.into())
}

/// Helper to convert memory errors
pub fn to_memory_error<T: Into<MemoryError>>(err: T) -> JvmError {
    JvmError::MemoryError(err.into())
}

/// Helper to convert class loading errors
pub fn to_class_loading_error<T: Into<ClassLoadingError>>(err: T) -> JvmError {
    JvmError::ClassLoadingError(err.into())
}
