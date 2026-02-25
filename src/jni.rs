//! Java Native Interface (JNI) implementation.
//!
//! Provides JNI-compatible types and RegisterNatives for loading
//! native libraries and dispatching to Rust/FFI implementations.

use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::sync::RwLock;

/// JNI environment - passed to native methods as first argument
#[repr(C)]
pub struct JNIEnv {
    _reserved: *mut c_void,
}

/// Opaque Java object reference
pub type jobject = *mut c_void;

/// Opaque Java class reference
pub type jclass = *mut c_void;

/// Method ID (opaque)
pub type jmethodID = *mut c_void;

/// Field ID (opaque)
pub type jfieldID = *mut c_void;

/// Native method registration entry
#[repr(C)]
pub struct JNINativeMethod {
    pub name: *const c_char,
    pub signature: *const c_char,
    pub fn_ptr: *mut c_void,
}

/// Registry of JNI native method pointers by (class_name, method_name, signature)
fn jni_natives() -> &'static RwLock<HashMap<(String, String, String), usize>> {
    static N: std::sync::OnceLock<RwLock<HashMap<(String, String, String), usize>>> = std::sync::OnceLock::new();
    N.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Register native methods for a class (JNI RegisterNatives equivalent)
pub fn register_natives(
    class_name: &str,
    methods: &[JNINativeMethod],
) -> Result<(), String> {
    let mut map = jni_natives().write().map_err(|e| e.to_string())?;
    for m in methods {
        let name = unsafe {
            CStr::from_ptr(m.name)
                .to_str()
                .map_err(|_| "Invalid method name")?
                .to_string()
        };
        let sig = unsafe {
            CStr::from_ptr(m.signature)
                .to_str()
                .map_err(|_| "Invalid signature")?
                .to_string()
        };
        if !m.fn_ptr.is_null() {
            map.insert((class_name.to_string(), name, sig), m.fn_ptr as usize);
        }
    }
    Ok(())
}

/// Find a registered JNI native method (returns raw fn pointer for FFI dispatch)
pub fn find_native(class_name: &str, name: &str, signature: &str) -> Option<*mut c_void> {
    jni_natives()
        .read()
        .ok()
        .and_then(|m| {
            m.get(&(class_name.to_string(), name.to_string(), signature.to_string()))
                .map(|&p| p as *mut c_void)
        })
}

/// Unregister all natives for a class
pub fn unregister_natives(class_name: &str) -> usize {
    jni_natives()
        .write()
        .ok()
        .map(|mut m| {
            let keys: Vec<_> = m.keys().filter(|(c, _, _)| c == class_name).cloned().collect();
            let count = keys.len();
            for k in keys {
                m.remove(&k);
            }
            count
        })
        .unwrap_or(0)
}
