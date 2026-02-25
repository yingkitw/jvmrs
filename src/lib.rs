//! JVMRS - JVM implementation in Rust
//!
//! ## Module Organization
//!
//! ### Core / Runtime
//! - `class_file` - Class file parsing
//! - `class_loader` - Class loading from classpath
//! - `class_cache` - Fast binary cache (.jvmc)
//! - `memory` - Heap, stack frames, values
//! - `allocator` - Arena-based allocation
//! - `gc` - Garbage collection
//!
//! ### Execution
//! - `interpreter` - Bytecode execution engine
//! - `native` - Native method dispatch
//! - `reflection` - Runtime introspection
//!
//! ### Compilation
//! - `jit` - JIT manager, tiered compilation
//! - `cranelift_jit` - Cranelift bytecode-to-native
//! - `aot_compiler` - AOT to object files
//! - `wasm_backend` - WebAssembly emission (feature: wasm)
//!
//! ### Developer Tools
//! - `debug` - Logging, trace config
//! - `profiler` - Flame graphs, hotspots
//! - `trace` - Time-travel debugging
//! - `deterministic` - Reproducible execution
//!
//! ### Optional Features
//! - `ffi` - C API (feature: ffi)
//! - `interop` - Java/Rust interop (feature: interop)
//! - `async_io` - Async class loading (feature: async)
//! - `simd` - Vectorized arrays (feature: simd)
//! - `truffle` - Language frontend API (feature: truffle)
//! - `security`, `aop`, `cloud`, `hot_reload` - Additional capabilities

pub mod error;

// Core: class loading and memory
pub mod class_file;
pub mod class_cache;
pub mod class_loader;
pub mod memory;
pub mod allocator;
pub mod gc;

// Execution
pub mod interpreter;
pub mod native;
pub mod reflection;

// Compilation
pub mod jit;
pub mod cranelift_jit;
pub mod aot_compiler;

// Developer tools
pub mod debug;
pub mod profiler;
pub mod trace;
pub mod deterministic;

// Optional / experimental
pub mod security;
pub mod aop;
pub mod cloud;
pub mod hot_reload;

#[cfg(feature = "no_std")]
pub mod core;

#[cfg(feature = "ffi")]
pub mod ffi;

#[cfg(feature = "interop")]
pub mod interop;

#[cfg(feature = "async")]
pub mod async_io;

#[cfg(feature = "simd")]
pub mod simd;

#[cfg(feature = "wasm")]
pub mod wasm_backend;

#[cfg(feature = "truffle")]
pub mod truffle;
