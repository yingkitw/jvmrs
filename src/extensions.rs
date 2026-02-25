//! Rust crate integration - plugin system for Java extensions.
//!
//! Allows Rust crates to register native method implementations
//! without JNI. Integrates with the interop layer.
//!
//! # Example
//! ```ignore
//! struct MyExt;
//! impl JavaExtension for MyExt {
//!     fn id(&self) -> &str { "com.example.myext" }
//!     fn version(&self) -> &str { "0.1.0" }
//!     fn on_load(&self, registry: &mut ExtensionRegistry) {
//!         registry.register_native("com.example.Utils.add", Box::new(|args| {
//!             let a = args.get(0).map(|v| v.as_int()).unwrap_or(0);
//!             let b = args.get(1).map(|v| v.as_int()).unwrap_or(0);
//!             Ok(Value::Int(a + b))
//!         }));
//!     }
//! }
//! ExtensionRegistry::global().load(&MyExt);
//! ```

use crate::memory::Value;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

/// Extension that can be loaded from a Rust crate
pub trait JavaExtension: Send + Sync {
    /// Unique extension id (e.g. "com.example.myext")
    fn id(&self) -> &str;

    /// Version string for compatibility checks
    fn version(&self) -> &str;

    /// Register native methods, callbacks, etc.
    fn on_load(&self, registry: &mut ExtensionRegistry);

    /// Optional: extension-specific metadata
    fn metadata(&self) -> Option<Box<dyn Any + Send>> {
        None
    }
}

type NativeCallback = Arc<dyn Fn(&[Value]) -> Result<Value, String> + Send + Sync>;

/// Registry for extensions to register their capabilities
pub struct ExtensionRegistry {
    native_methods: RwLock<HashMap<String, NativeCallback>>,
    extensions_loaded: RwLock<Vec<String>>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self {
            native_methods: RwLock::new(HashMap::new()),
            extensions_loaded: RwLock::new(Vec::new()),
        }
    }

    /// Register a native method implementation (key: "class.name.method")
    pub fn register_native(&self, key: &str, f: Box<dyn Fn(&[Value]) -> Result<Value, String> + Send + Sync>) {
        self.native_methods
            .write()
            .unwrap()
            .insert(key.to_string(), Arc::from(f));
    }

    /// Check if a native is registered
    pub fn has_native(&self, key: &str) -> bool {
        self.native_methods.read().unwrap().contains_key(key)
    }

    /// Invoke a registered native
    pub fn invoke_native(&self, key: &str, args: &[Value]) -> Result<Value, String> {
        let f = self
            .native_methods
            .read()
            .unwrap()
            .get(key)
            .cloned()
            .ok_or_else(|| format!("Extension native not found: {}", key))?;
        f(args)
    }

    /// Load an extension
    pub fn load(&self, ext: &dyn JavaExtension) {
        let mut registry = ExtensionRegistry {
            native_methods: RwLock::new(HashMap::new()),
            extensions_loaded: RwLock::new(Vec::new()),
        };
        ext.on_load(&mut registry);

        let natives: Vec<_> = registry.native_methods.write().unwrap().drain().collect();

        // Merge into self and wire to interop (no JNI) when available
        let mut native = self.native_methods.write().unwrap();
        for (k, v) in natives {
            #[cfg(feature = "interop")]
            {
                let v_clone = v.clone();
                crate::interop::register_rust_callback(&k, Box::new(move |args| v_clone(args)));
            }
            native.insert(k, v);
        }
        self.extensions_loaded.write().unwrap().push(ext.id().to_string());
    }

    /// List loaded extension ids
    pub fn loaded_extensions(&self) -> Vec<String> {
        self.extensions_loaded.read().unwrap().clone()
    }
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

static GLOBAL_REGISTRY: OnceLock<RwLock<ExtensionRegistry>> = OnceLock::new();

impl ExtensionRegistry {
    /// Global extension registry (used by Interpreter when interop is enabled)
    pub fn global() -> &'static RwLock<ExtensionRegistry> {
        GLOBAL_REGISTRY.get_or_init(|| RwLock::new(ExtensionRegistry::new()))
    }
}
