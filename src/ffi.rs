//! C API for embedding jvmrs as a library in other applications.
//!
//! Enable with `cargo build --features ffi` and link as a C library.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::interpreter::Interpreter;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::ptr;

/// Opaque handle to the JVM interpreter
#[repr(C)]
pub struct JvmrsHandle {
    _private: [u8; 0],
}

/// Create a new JVM interpreter.
/// Returns null on failure.
#[no_mangle]
pub extern "C" fn jvmrs_create() -> *mut JvmrsHandle {
    match std::panic::catch_unwind(|| {
        let interpreter = Box::new(Interpreter::new());
        Box::into_raw(interpreter) as *mut JvmrsHandle
    }) {
        Ok(ptr) => ptr,
        Err(_) => ptr::null_mut(),
    }
}

/// Create a JVM interpreter with custom classpath.
/// classpath: colon-separated (Unix) or semicolon-separated (Windows) paths
/// Returns null on failure.
#[no_mangle]
pub extern "C" fn jvmrs_create_with_classpath(classpath: *const c_char) -> *mut JvmrsHandle {
    if classpath.is_null() {
        return ptr::null_mut();
    }
    match std::panic::catch_unwind(|| {
        let c_str = unsafe { CStr::from_ptr(classpath) };
        let cp_str = c_str.to_string_lossy();
        let paths: Vec<PathBuf> = if cfg!(windows) {
            cp_str.split(';').map(PathBuf::from).collect()
        } else {
            cp_str.split(':').map(PathBuf::from).collect()
        };
        let interpreter = Box::new(Interpreter::with_classpath(paths));
        Box::into_raw(interpreter) as *mut JvmrsHandle
    }) {
        Ok(ptr) => ptr,
        Err(_) => ptr::null_mut(),
    }
}

/// Load a class by name.
/// Returns 0 on success, non-zero error code on failure.
#[no_mangle]
pub extern "C" fn jvmrs_load_class(handle: *mut JvmrsHandle, class_name: *const c_char) -> i32 {
    if handle.is_null() || class_name.is_null() {
        return -1;
    }
    let interpreter = unsafe { &mut *(handle as *mut Interpreter) };
    let name = match unsafe { CStr::from_ptr(class_name).to_str() } {
        Ok(s) => s,
        Err(_) => return -2,
    };
    match interpreter.load_class_by_name(name) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

/// Run the main method of a class.
/// Returns 0 on success, non-zero on failure.
#[no_mangle]
pub extern "C" fn jvmrs_run_main(handle: *mut JvmrsHandle, class_name: *const c_char) -> i32 {
    if handle.is_null() || class_name.is_null() {
        return -1;
    }
    let interpreter = unsafe { &mut *(handle as *mut Interpreter) };
    let name = match unsafe { CStr::from_ptr(class_name).to_str() } {
        Ok(s) => s,
        Err(_) => return -2,
    };
    match interpreter.run_main(name) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

/// Get the last error message (caller must free with jvmrs_free_string).
#[no_mangle]
pub extern "C" fn jvmrs_last_error(handle: *mut JvmrsHandle) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }
    // Simplified: we don't store last error yet - return null
    ptr::null_mut()
}

/// Free a string returned by the API
#[no_mangle]
pub extern "C" fn jvmrs_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe { let _ = CString::from_raw(s); }
    }
}

/// Destroy the interpreter and free resources
#[no_mangle]
pub extern "C" fn jvmrs_destroy(handle: *mut JvmrsHandle) {
    if !handle.is_null() {
        unsafe { let _ = Box::from_raw(handle as *mut Interpreter); }
    }
}
