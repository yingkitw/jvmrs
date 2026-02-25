//! jvmrs-bindgen - Compile-time Java-to-Rust binding generation
//!
//! Generate Rust bindings from Java class files so you can call Java
//! methods from Rust without manual JNI or interop registration.
//!
//! # Status
//!
//! This crate is a scaffold. Full implementation would:
//! - Parse .class files at build time
//! - Emit Rust functions that invoke JVMRS interpreter
//! - Map Java types to Rust types
//!
//! # Example (future API)
//!
//! ```ignore
//! // In build.rs, JVMRS_BINDGEN_CLASSES=com.example.Utils
//! jvmrs_bindgen::generate!();
//!
//! // In lib.rs or main.rs
//! use jvmrs_bindgen::com_example_Utils;
//! let result = com_example_Utils::add(2, 3);
//! ```

/// Placeholder: generate bindings (no-op for now)
pub fn generate() {
    // Build script handles generation when JVMRS_BINDGEN_CLASSES is set
}
