//! Polyglot interop - allow Java and Rust code to share objects directly.
//!
//! Provides FFI bridge for:
//! - Registering Rust callbacks invokable from Java (via native methods)
//! - Invoking Java methods from Rust
//! - Sharing object references between runtimes

use crate::memory::Value;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// Handle for an object shared between Java and Rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SharedObjectId(pub u32);

/// Callback type: Rust function that can be invoked from Java bytecode
pub type RustCallback = Box<
    dyn Fn(&[Value]) -> Result<Value, String> + Send + Sync,
>;

fn rust_callbacks() -> &'static RwLock<HashMap<String, RustCallback>> {
    static RUST_CALLBACKS: OnceLock<RwLock<HashMap<String, RustCallback>>> = OnceLock::new();
    RUST_CALLBACKS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Register a Rust callback under a qualified name (e.g. "com.example.MyClass.myMethod")
pub fn register_rust_callback(name: &str, callback: RustCallback) {
    rust_callbacks().write().unwrap().insert(name.to_string(), callback);
}

/// Unregister a callback by name
pub fn unregister_rust_callback(name: &str) -> bool {
    rust_callbacks().write().unwrap().remove(name).is_some()
}

/// Check if a callback is registered (avoids stack manipulation when not found)
pub fn has_rust_callback(name: &str) -> bool {
    rust_callbacks().read().unwrap().contains_key(name)
}

/// Invoke a registered Rust callback (called from native method dispatcher)
pub fn invoke_rust_callback(name: &str, args: &[Value]) -> Result<Value, String> {
    let callbacks = rust_callbacks().read().unwrap();
    callbacks
        .get(name)
        .ok_or_else(|| format!("Rust callback not found: {}", name))?
        (args)
}

/// Bridge for passing Java objects to Rust - stores metadata
#[derive(Debug, Clone)]
pub struct JavaObjectBridge {
    /// Heap reference (u32 address)
    pub heap_ref: u32,
    /// Class name for type info
    pub class_name: String,
}

fn java_object_bridge() -> &'static RwLock<HashMap<SharedObjectId, JavaObjectBridge>> {
    static JAVA_OBJECT_BRIDGE: OnceLock<RwLock<HashMap<SharedObjectId, JavaObjectBridge>>> =
        OnceLock::new();
    JAVA_OBJECT_BRIDGE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Expose a Java object to Rust, returns SharedObjectId
pub fn expose_java_object(heap_ref: u32, class_name: String) -> SharedObjectId {
    let id = SharedObjectId(heap_ref);
    java_object_bridge().write().unwrap().insert(
        id,
        JavaObjectBridge { heap_ref, class_name },
    );
    id
}

/// Get Java object metadata
pub fn get_java_object(id: SharedObjectId) -> Option<JavaObjectBridge> {
    java_object_bridge().read().unwrap().get(&id).cloned()
}

/// Remove from bridge when no longer needed
pub fn release_java_object(id: SharedObjectId) {
    java_object_bridge().write().unwrap().remove(&id);
}
