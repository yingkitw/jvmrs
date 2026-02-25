//! JVM runtime stack for method invocation.

use super::frame::StackFrame;

/// JVM runtime stack for method invocation
#[derive(Debug)]
pub struct JVMStack {
    frames: Vec<StackFrame>,
}

impl JVMStack {
    pub fn new() -> Self {
        JVMStack { frames: Vec::new() }
    }

    pub fn push_frame(&mut self, frame: StackFrame) {
        self.frames.push(frame);
    }

    pub fn pop_frame(&mut self) -> Option<StackFrame> {
        self.frames.pop()
    }

    pub fn current_frame(&self) -> Option<&StackFrame> {
        self.frames.last()
    }

    pub fn current_frame_mut(&mut self) -> Option<&mut StackFrame> {
        self.frames.last_mut()
    }

    pub fn depth(&self) -> usize {
        self.frames.len()
    }
}

impl Default for JVMStack {
    fn default() -> Self {
        Self::new()
    }
}
