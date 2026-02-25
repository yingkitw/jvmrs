//! Visualization tools for JVM internals.
//!
//! Heap dump, stack trace, and simple HTML/ASCII visualizations.

use crate::memory::{Memory, StackFrame};
use std::fmt::Write;

/// Dump heap state to ASCII
pub fn heap_dump_ascii(object_count: usize, array_count: usize) -> String {
    format!(
        "=== Heap Dump ===\nObjects: {}\nArrays: {}\n",
        object_count, array_count
    )
}

/// Dump full memory state (heap stats + stack frames)
pub fn memory_dump_ascii(memory: &Memory) -> String {
    let mut out = String::from("=== Memory Dump ===\n\n");
    out.push_str(&heap_dump_ascii(
        memory.heap.object_count(),
        memory.heap.array_count(),
    ));
    out.push_str("\n=== Call Stack ===\n");
    for (i, frame) in memory.stack.frames().iter().enumerate() {
        out.push_str(&format!("\n--- Frame {} ---\n", i));
        out.push_str(&frame_dump_ascii(frame));
    }
    out
}

/// Dump stack frame to ASCII
pub fn frame_dump_ascii(frame: &StackFrame) -> String {
    let mut out = String::from("=== Stack Frame ===\n");
    let _ = write!(out, "Method: {}\n", frame.method_name);
    let _ = write!(out, "PC: {}\n", frame.pc);
    let _ = write!(out, "Locals ({}):\n", frame.locals.len());
    for (i, v) in frame.locals.iter().enumerate() {
        let _ = write!(out, "  [{}] {:?}\n", i, v);
    }
    let _ = write!(out, "Operand stack ({}):\n", frame.stack.len());
    for (i, v) in frame.stack.iter().enumerate() {
        let _ = write!(out, "  [{}] {:?}\n", i, v);
    }
    out
}

/// Export heap/stack to HTML fragment
pub fn export_html_fragment(frames: &[StackFrame]) -> String {
    let mut out = String::from("<div class=\"jvmrs-dump\">\n");
    for (i, f) in frames.iter().enumerate() {
        let _ = write!(out, "<div class=\"frame\"><h4>Frame {}</h4>\n", i);
        let _ = write!(out, "<pre>{}</pre></div>\n", html_escape(&frame_dump_ascii(f)));
    }
    out.push_str("</div>\n");
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
