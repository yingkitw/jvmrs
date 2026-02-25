use crate::error::NativeError;
use crate::memory::{Memory, Value};
use std::collections::HashMap;
use std::os::raw::c_void;

/// JNI version constants
pub const JNI_VERSION_1_1: i32 = 0x00010001;
pub const JNI_VERSION_1_2: i32 = 0x00010002;
pub const JNI_VERSION_1_4: i32 = 0x00010004;
pub const JNI_VERSION_1_6: i32 = 0x00010006;
pub const JNI_VERSION_1_8: i32 = 0x00010008;

/// JNI Native Method Interface
#[repr(C)]
pub struct JNINativeInterface_ {
    // Reserved for future use
    reserved0: *mut c_void,
    reserved1: *mut c_void,
    reserved2: *mut c_void,
    reserved3: *mut c_void,

    // Method functions (simplified)
    get_version: *mut c_void,
    define_class: *mut c_void,
    find_class: *mut c_void,
    // ... more functions would be added in a complete implementation
}

/// JNI Native Method
pub type JNINativeMethod = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut *mut jvalue);

/// JNI Value union for different primitive types
#[repr(C)]
pub union jvalue {
    pub z: u8,          // boolean
    pub b: i8,          // byte
    pub c: u16,         // char
    pub s: i16,         // short
    pub i: i32,         // int
    pub j: i64,         // long
    pub f: f32,         // float
    pub d: f64,         // double
    pub l: *mut c_void, // object
}

/// Native method trait
pub trait NativeMethod: Send + Sync {
    /// Invoke the native method with the given arguments
    fn invoke(&self, args: &[Value], memory: &mut Memory) -> Result<Value, NativeError>;

    /// Get the signature of this method
    fn signature(&self) -> &str;

    /// Get the name of this method
    fn name(&self) -> &str;
}

/// Simple native method implementation using Rust closures
pub struct SimpleNativeMethod {
    name: String,
    signature: String,
    func: Box<dyn Fn(&[Value], &mut Memory) -> Result<Value, NativeError> + Send + Sync>,
}

impl SimpleNativeMethod {
    /// Create a new simple native method
    pub fn new<F>(name: String, signature: String, func: F) -> Self
    where
        F: Fn(&[Value], &mut Memory) -> Result<Value, NativeError> + Send + Sync + 'static,
    {
        Self {
            name,
            signature,
            func: Box::new(func),
        }
    }
}

impl NativeMethod for SimpleNativeMethod {
    fn invoke(&self, args: &[Value], memory: &mut Memory) -> Result<Value, NativeError> {
        (self.func)(args, memory)
    }

    fn signature(&self) -> &str {
        &self.signature
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Registry for native methods
pub struct NativeRegistry {
    methods: HashMap<String, HashMap<String, Box<dyn NativeMethod>>>,
    // In a real implementation, we would have loaded libraries here
}

impl NativeRegistry {
    /// Create a new native registry
    pub fn new() -> Self {
        Self {
            methods: HashMap::new(),
        }
    }

    /// Register a native method
    pub fn register_method(
        &mut self,
        class_name: &str,
        method_name: &str,
        method: Box<dyn NativeMethod>,
    ) -> Result<(), NativeError> {
        self.methods
            .entry(class_name.to_string())
            .or_insert_with(HashMap::new)
            .insert(method_name.to_string(), method);
        Ok(())
    }

    /// Find a native method
    pub fn find_method(&self, class_name: &str, method_name: &str) -> Option<&dyn NativeMethod> {
        self.methods
            .get(class_name)?
            .get(method_name)
            .map(|m| m.as_ref())
    }

    /// Check if a method is native
    pub fn is_native(&self, class_name: &str, method_name: &str) -> bool {
        self.methods
            .get(class_name)
            .map_or(false, |methods| methods.contains_key(method_name))
    }
}

impl Default for NativeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert JVM Value to JNI jvalue
pub fn value_to_jvalue(value: Value) -> jvalue {
    match value {
        Value::Boolean(b) => jvalue { z: b as u8 },
        Value::Byte(b) => jvalue { b },
        Value::Char(c) => jvalue { c },
        Value::Short(s) => jvalue { s },
        Value::Int(i) => jvalue { i },
        Value::Long(l) => jvalue { j: l },
        Value::Float(f) => jvalue { f },
        Value::Double(d) => jvalue { d },
        Value::Reference(r) => jvalue {
            l: r as *mut c_void,
        },
        Value::ArrayRef(r) => jvalue {
            l: r as *mut c_void,
        },
        Value::Null => jvalue {
            l: std::ptr::null_mut(),
        },
        Value::ReturnAddress(_) => jvalue {
            l: std::ptr::null_mut(),
        },
    }
}

/// Convert JNI jvalue to JVM Value
pub fn jvalue_to_value(jvalue: jvalue) -> Value {
    unsafe {
        // Note: In a real implementation, we'd need type information
        // For now, we'll assume it's an int
        Value::Int(jvalue.i)
    }
}

/// Initialize built-in native methods
pub fn init_builtins(registry: &mut NativeRegistry) {
    // Register System.currentTimeMillis()
    registry
        .register_method(
            "java/lang/System",
            "currentTimeMillis",
            Box::new(SimpleNativeMethod::new(
                "currentTimeMillis".to_string(),
                "()J".to_string(),
                |_args: &[Value], _memory: &mut Memory| {
                    // Return current time in milliseconds
                    use std::time::{SystemTime, UNIX_EPOCH};
                    let duration = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| {
                        NativeError::NativeMethodFailed(
                            "System.currentTimeMillis".to_string(),
                            format!("Failed to get system time: {}", e),
                        )
                    })?;
                    Ok(Value::Long(duration.as_millis() as i64))
                },
            )),
        )
        .ok();

    // Register System.nanoTime()
    registry
        .register_method(
            "java/lang/System",
            "nanoTime",
            Box::new(SimpleNativeMethod::new(
                "nanoTime".to_string(),
                "()J".to_string(),
                |_args: &[Value], _memory: &mut Memory| {
                    // Return current time in nanoseconds
                    use std::time::{SystemTime, UNIX_EPOCH};
                    let duration = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| {
                        NativeError::NativeMethodFailed(
                            "System.nanoTime".to_string(),
                            format!("Failed to get system time: {}", e),
                        )
                    })?;
                    Ok(Value::Long(duration.as_nanos() as i64))
                },
            )),
        )
        .ok();

    // Register Math.sqrt(double)
    registry
        .register_method(
            "java/lang/Math",
            "sqrt",
            Box::new(SimpleNativeMethod::new(
                "sqrt".to_string(),
                "(D)D".to_string(),
                |args: &[Value], _memory: &mut Memory| {
                    if args.len() != 1 {
                        return Err(NativeError::NativeMethodSignatureMismatch(
                            "Math.sqrt".to_string(),
                        ));
                    }

                    match args[0] {
                        Value::Double(d) => {
                            if d >= 0.0 {
                                Ok(Value::Double(d.sqrt()))
                            } else {
                                Ok(Value::Double(f64::NAN))
                            }
                        }
                        _ => Err(NativeError::NativeMethodSignatureMismatch(
                            "Math.sqrt expects a double argument".to_string(),
                        )),
                    }
                },
            )),
        )
        .ok();

    // Register String.hashCode()
    registry
        .register_method(
            "java/lang/String",
            "hashCode",
            Box::new(SimpleNativeMethod::new(
                "hashCode".to_string(),
                "()I".to_string(),
                |args: &[Value], memory: &mut Memory| {
                    if args.len() != 1 {
                        return Err(NativeError::NativeMethodSignatureMismatch(
                            "String.hashCode".to_string(),
                        ));
                    }

                    match &args[0] {
                        Value::Reference(addr) => {
                            if let Some(obj) = memory.heap.get_object(*addr) {
                                if let Some(string_data) = obj.string_data.as_ref() {
                                    // Simple hash code implementation
                                    let mut hash: i32 = 0;
                                    for c in string_data.chars() {
                                        hash = 31 * hash + (c as i32);
                                    }
                                    Ok(Value::Int(hash))
                                } else {
                                    // For non-string objects, use identity hash code
                                    Ok(Value::Int(*addr as i32))
                                }
                            } else {
                                Err(NativeError::NativeMethodFailed(
                                    "String.hashCode".to_string(),
                                    "Invalid object reference".to_string(),
                                ))
                            }
                        }
                        Value::Null => Ok(Value::Int(0)),
                        _ => Err(NativeError::NativeMethodSignatureMismatch(
                            "String.hashCode expects a string object".to_string(),
                        )),
                    }
                },
            )),
        )
        .ok();
}
