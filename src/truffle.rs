//! GraalVM-style Truffle API for language implementation.
//!
//! Provides abstraction for pluggable execution strategies and language frontends.

use crate::memory::Value;

/// Truffle-style Node - represents an executable AST node
/// Can be specialized/optimized based on observed types
pub trait TruffleNode: Send + Sync {
    /// Execute this node, returning the result value
    fn execute(&self, frame: &mut TruffleFrame) -> Result<Value, String>;

    /// Check if this node can be specialized (e.g. for single-type optimization)
    fn is_specializable(&self) -> bool {
        false
    }
}

/// Execution frame - holds local variables and stack
#[derive(Debug, Default)]
pub struct TruffleFrame {
    pub locals: Vec<Value>,
    pub stack: Vec<Value>,
}

impl TruffleFrame {
    pub fn new(max_locals: usize, max_stack: usize) -> Self {
        Self {
            locals: vec![Value::Int(0); max_locals],
            stack: Vec::with_capacity(max_stack),
        }
    }

    pub fn push(&mut self, v: Value) {
        self.stack.push(v);
    }

    pub fn pop(&mut self) -> Option<Value> {
        self.stack.pop()
    }

    pub fn get_local(&self, i: usize) -> Option<Value> {
        self.locals.get(i).cloned()
    }

    pub fn set_local(&mut self, i: usize, v: Value) {
        if i < self.locals.len() {
            self.locals[i] = v;
        }
    }
}

/// Language frontend - describes a language that can be executed on the Truffle-style runtime
pub trait LanguageFrontend: Send + Sync {
    /// Name of the language
    fn name(&self) -> &str;

    /// Parse source code into executable nodes
    fn parse(&self, source: &str) -> Result<Box<dyn TruffleNode>, String>;

    /// MIME type of source (e.g. "application/java")
    fn mime_type(&self) -> &str {
        "text/plain"
    }
}
