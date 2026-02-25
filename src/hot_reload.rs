//! Hot code reloading - replace methods at runtime without restart.
//!
//! For Java/Rust hybrid development: edit Java, recompile, reload without
//! restarting the process. Integrates with JIT when cranelift supports hotswap.

use crate::class_file::{ClassFile, MethodInfo};
use std::path::Path;
use std::time::SystemTime;

/// Hot reload manager - tracks replaceable methods and class file mtimes
pub struct HotReloadManager {
    enabled: bool,
    /// (class_path, last_modified)
    watched_classes: std::collections::HashMap<String, Option<SystemTime>>,
}

impl HotReloadManager {
    pub fn new() -> Self {
        Self {
            enabled: false,
            watched_classes: std::collections::HashMap::new(),
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Register a method for potential hot reload
    pub fn register_reloadable(&mut self, class: &str, _method: &str) {
        if self.enabled {
            self.watched_classes
                .entry(class.to_string())
                .or_insert(None);
        }
    }

    /// Check if a class file has changed (by mtime)
    pub fn class_file_changed(&self, path: &Path) -> bool {
        if !self.enabled {
            return false;
        }
        let Ok(meta) = std::fs::metadata(path) else {
            return false;
        };
        let Ok(modified) = meta.modified() else {
            return false;
        };
        let class_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        self.watched_classes
            .get(&class_name)
            .and_then(|prev| *prev)
            .map(|p| modified > p)
            .unwrap_or(true)
    }

    /// Record class load time (call after loading)
    pub fn record_class_loaded(&mut self, class_name: &str, path: &Path) {
        if let Ok(meta) = std::fs::metadata(path) {
            if let Ok(modified) = meta.modified() {
                self.watched_classes
                    .insert(class_name.to_string(), Some(modified));
            }
        }
    }

    /// Replace method implementation with new bytecode
    pub fn replace_method(
        &mut self,
        _class: &ClassFile,
        _method: &MethodInfo,
    ) -> Result<(), String> {
        if !self.enabled {
            return Err("Hot reload disabled".to_string());
        }
        Err("Hot reload requires JIT hotswap support - see cranelift_jit hotswap".to_string())
    }
}

impl Default for HotReloadManager {
    fn default() -> Self {
        Self::new()
    }
}
