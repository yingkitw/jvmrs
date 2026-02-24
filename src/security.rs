//! Security instrumentation - runtime vulnerability detection.
//!
//! Provides sanitizer hooks for bounds checking, null dereference detection,
//! and extensible security checks.

/// Security instrumentation configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Enable array bounds checking (abort on out-of-bounds)
    pub bounds_check: bool,
    /// Enable null pointer dereference detection
    pub null_check: bool,
    /// Enable stack overflow detection
    pub stack_overflow_check: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            bounds_check: true,
            null_check: true,
            stack_overflow_check: true,
        }
    }
}

/// Sanitizer - runtime security checks
#[derive(Debug)]
pub struct Sanitizer {
    config: SecurityConfig,
    violation_count: std::cell::Cell<u64>,
}

impl Sanitizer {
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            config,
            violation_count: std::cell::Cell::new(0),
        }
    }

    /// Check array access - returns Err if out of bounds
    pub fn check_array_bounds(&self, index: usize, length: usize) -> Result<(), String> {
        if !self.config.bounds_check {
            return Ok(());
        }
        if index >= length {
            self.violation_count.set(self.violation_count.get() + 1);
            Err(format!("Array index out of bounds: {} >= {}", index, length))
        } else {
            Ok(())
        }
    }

    /// Check for null reference before dereference
    pub fn check_null(&self, reference: Option<u32>) -> Result<(), String> {
        if !self.config.null_check {
            return Ok(());
        }
        match reference {
            None => {
                self.violation_count.set(self.violation_count.get() + 1);
                Err("NullPointerException: attempted dereference of null".to_string())
            }
            Some(0) => {
                self.violation_count.set(self.violation_count.get() + 1);
                Err("NullPointerException: invalid reference 0".to_string())
            }
            Some(_) => Ok(()),
        }
    }

    /// Check stack depth to prevent overflow
    pub fn check_stack_overflow(&self, current_depth: usize, max_depth: usize) -> Result<(), String> {
        if !self.config.stack_overflow_check {
            return Ok(());
        }
        if current_depth >= max_depth {
            self.violation_count.set(self.violation_count.get() + 1);
            Err(format!("Stack overflow: depth {} >= max {}", current_depth, max_depth))
        } else {
            Ok(())
        }
    }

    pub fn violation_count(&self) -> u64 {
        self.violation_count.get()
    }

    pub fn config(&self) -> &SecurityConfig {
        &self.config
    }
}
