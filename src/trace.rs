//! Execution trace recorder - foundation for time-travel debugging.
//!
//! Records opcode execution, frame state, and control flow for replay and analysis.

use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Recorded opcode execution step
#[derive(Debug, Clone)]
pub struct TraceStep {
    pub pc: usize,
    pub opcode: u8,
    pub method: String,
    pub stack_depth: usize,
    pub locals_count: usize,
}

/// Execution trace recorder
pub struct TraceRecorder {
    steps: VecDeque<TraceStep>,
    max_steps: usize,
    enabled: bool,
}

impl TraceRecorder {
    pub fn new() -> Self {
        Self {
            steps: VecDeque::new(),
            max_steps: 1_000_000,
            enabled: false,
        }
    }

    pub fn with_capacity(max_steps: usize) -> Self {
        Self {
            steps: VecDeque::new(),
            max_steps,
            enabled: false,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Record a single opcode execution
    pub fn record(&mut self, pc: usize, opcode: u8, method: &str, stack_depth: usize, locals_count: usize) {
        if !self.enabled {
            return;
        }
        if self.steps.len() >= self.max_steps {
            self.steps.pop_front();
        }
        self.steps.push_back(TraceStep {
            pc,
            opcode,
            method: method.to_string(),
            stack_depth,
            locals_count,
        });
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Export trace to text format for analysis
    pub fn export_text(&self) -> String {
        self.steps
            .iter()
            .enumerate()
            .map(|(i, s)| format!("{}: {} pc={} op=0x{:02x} stack={} locals={}", i, s.method, s.pc, s.opcode, s.stack_depth, s.locals_count))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Write trace to file
    pub fn write_to_file(&self, path: &Path) -> std::io::Result<()> {
        let mut f = File::create(path)?;
        write!(f, "{}", self.export_text())
    }

    /// Clear recorded trace
    pub fn clear(&mut self) {
        self.steps.clear();
    }
}

impl Default for TraceRecorder {
    fn default() -> Self {
        Self::new()
    }
}
