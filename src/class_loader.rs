//! Class loading implementation for JVMRS

use crate::class_cache::{read_from_cache, write_to_cache};
use crate::class_file::ClassFile;
use crate::error::{to_class_loading_error, ClassLoadingError, JvmError};
use std::collections::HashMap;
use std::path::PathBuf;

/// Class loader that searches for classes in a classpath
pub struct ClassLoader {
    /// Loaded classes cache
    classes: HashMap<String, ClassFile>,
    /// Classpath directories
    classpath: Vec<PathBuf>,
    /// Optional cache directory for fast binary format (.jvmc) - skip classpath search
    cache_dir: Option<PathBuf>,
}

impl ClassLoader {
    /// Create a new class loader with the given classpath
    pub fn new(classpath: Vec<PathBuf>) -> Self {
        ClassLoader {
            classes: HashMap::new(),
            classpath,
            cache_dir: None,
        }
    }

    /// Create a new class loader with default classpath (current directory)
    pub fn new_default() -> Self {
        ClassLoader {
            classes: HashMap::new(),
            classpath: vec![PathBuf::from(".")],
            cache_dir: None,
        }
    }

    /// Enable fast class loading from binary cache directory
    pub fn with_cache_dir(mut self, cache_dir: PathBuf) -> Self {
        self.cache_dir = Some(cache_dir);
        self
    }

    /// Set cache directory for fast class loading
    pub fn set_cache_dir(&mut self, cache_dir: Option<PathBuf>) {
        self.cache_dir = cache_dir;
    }

    /// Load a class by name
    pub fn load_class(&mut self, class_name: &str) -> Result<(), JvmError> {
        // Check if already loaded
        if self.classes.contains_key(class_name) {
            return Ok(());
        }

        // Try fast path: load from binary cache if enabled
        if let Some(ref cache_dir) = self.cache_dir {
            if let Ok(Some(class_file)) = read_from_cache(cache_dir, class_name) {
                let actual_class_name = class_file
                    .get_class_name()
                    .unwrap_or_else(|| class_name.to_string());
                if actual_class_name != class_name {
                    return Err(to_class_loading_error(ClassLoadingError::ClassFormatError(
                        format!("Class name mismatch: expected {}, got {}", class_name, actual_class_name),
                    )));
                }
                if let Some(super_class_name) = class_file.get_super_class_name() {
                    if super_class_name != "java/lang/Object" {
                        self.load_class(&super_class_name)?;
                    }
                }
                self.classes.insert(class_name.to_string(), class_file);
                return Ok(());
            }
        }

        // Convert class name to file path
        let class_file_name = format!("{}.class", class_name.replace('.', "/"));

        // Search in classpath
        for classpath_dir in &self.classpath {
            let full_path = classpath_dir.join(&class_file_name);
            if full_path.exists() {
                let bytes = std::fs::read(&full_path).map_err(|e| {
                    to_class_loading_error(ClassLoadingError::ClassFormatError(format!(
                        "Failed to read {}: {}",
                        full_path.display(),
                        e
                    )))
                })?;
                match ClassFile::parse(&bytes) {
                    Ok(class_file) => {
                        let actual_class_name = class_file
                            .get_class_name()
                            .unwrap_or_else(|| class_name.to_string());

                        // Verify class name matches
                        if actual_class_name != class_name {
                            return Err(to_class_loading_error(
                                ClassLoadingError::ClassFormatError(format!(
                                    "Class name mismatch: expected {}, got {}",
                                    class_name, actual_class_name
                                )),
                            ));
                        }

                        // Load super class if it exists
                        if let Some(super_class_name) = class_file.get_super_class_name() {
                            if super_class_name != "java/lang/Object" {
                                self.load_class(&super_class_name)?;
                            }
                        }

                        // Write to binary cache for future fast loading
                        if let Some(ref cache_dir) = self.cache_dir {
                            let _ = write_to_cache(cache_dir, class_name, &bytes);
                        }

                        // Insert into cache
                        self.classes.insert(class_name.to_string(), class_file);
                        return Ok(());
                    }
                    Err(e) => {
                        return Err(to_class_loading_error(ClassLoadingError::ClassFormatError(
                            format!("Failed to parse class file {}: {:?}", full_path.display(), e),
                        )));
                    }
                }
            }
        }

        // Class not found
        Err(to_class_loading_error(ClassLoadingError::NoClassDefFound(
            class_name.to_string(),
        )))
    }

    /// Get a loaded class by name
    pub fn get_class(&self, name: &str) -> Option<&ClassFile> {
        self.classes.get(name)
    }

    /// Check if a class is already loaded
    pub fn is_class_loaded(&self, name: &str) -> bool {
        self.classes.contains_key(name)
    }

    /// Get all loaded class names
    pub fn get_loaded_classes(&self) -> Vec<&String> {
        self.classes.keys().collect()
    }

    /// Add a directory to the classpath
    pub fn add_classpath(&mut self, path: PathBuf) {
        self.classpath.push(path);
    }

    /// Get the current classpath
    pub fn get_classpath(&self) -> &[PathBuf] {
        &self.classpath
    }

    /// Clear the class cache
    pub fn clear_cache(&mut self) {
        self.classes.clear();
    }
}

impl Default for ClassLoader {
    fn default() -> Self {
        Self::new_default()
    }
}

/// Helper function to parse classpath string (e.g., "dir1:dir2:dir3" on Unix, "dir1;dir2;dir3" on Windows)
pub fn parse_classpath(classpath_str: &str) -> Vec<PathBuf> {
    let separator = if cfg!(windows) { ';' } else { ':' };
    classpath_str
        .split(separator)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .collect()
}

/// Helper function to get system classpath from environment variable
pub fn get_system_classpath() -> Vec<PathBuf> {
    std::env::var("CLASSPATH")
        .map(|cp| parse_classpath(&cp))
        .unwrap_or_else(|_| vec![PathBuf::from(".")])
}
