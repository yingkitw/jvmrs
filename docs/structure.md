# JVMRS Source Code Structure

## Directory Layout

```
jvmrs/
├── src/
│   ├── lib.rs              # Library root, module declarations
│   ├── main.rs             # CLI entry point
│   ├── error.rs            # Error types (JvmError, ParseError, etc.)
│   │
│   ├── # Core / Runtime (class loading, memory)
│   ├── class_file.rs       # Class file parser, ConstantPool, MethodInfo
│   ├── class_loader.rs     # Classpath resolution, load_class
│   ├── class_cache.rs      # Binary cache format (.jvmc)
│   ├── memory/             # Heap, StackFrame, Value, Monitor (modular)
│   │   ├── mod.rs          # Memory, re-exports
│   │   ├── value.rs        # Value enum
│   │   ├── frame.rs        # StackFrame
│   │   ├── monitor.rs      # Monitor (sync)
│   │   ├── heap_object.rs  # HeapObject, HeapArray
│   │   ├── heap.rs         # Heap allocation
│   │   └── stack.rs        # JVMStack
│   ├── allocator.rs        # Arena-based allocation
│   ├── gc.rs               # Generational GC, ScopedRoot
│   │
│   ├── # Execution
│   ├── interpreter/        # Bytecode dispatch, method invocation (modular)
│   │   ├── mod.rs          # Interpreter, run_main, public API
│   │   ├── descriptor.rs   # Method descriptor parsing
│   │   ├── utils.rs        # Bytecode read helpers
│   │   ├── dispatch.rs     # Instruction dispatch
│   │   ├── invocation.rs   # invoke_virtual, invoke_static, execute_method
│   │   └── builtins.rs     # native_println, handle_invokedynamic
│   ├── native.rs           # Native method registry, builtins
│   ├── reflection.rs       # ClassReflection, class_to_reflection
│   │
│   ├── # Compilation
│   ├── jit.rs              # JitManager, tiered compilation
│   ├── cranelift_jit.rs    # Cranelift backend
│   ├── aot_compiler.rs     # AOT to .o files
│   ├── wasm_backend.rs     # WASM emission (feature: wasm)
│   │
│   ├── # Developer tools
│   ├── debug.rs            # JvmDebugger, DebugConfig
│   ├── profiler.rs         # Profiler, ProfileGuard
│   ├── trace.rs            # TraceRecorder (time-travel)
│   ├── deterministic.rs    # DeterministicConfig
│   │
│   ├── # Optional features
│   ├── security.rs         # Sanitizer, bounds checking
│   ├── aop.rs              # Aspect-oriented hooks
│   ├── cloud.rs            # Cloud-related stubs
│   ├── hot_reload.rs       # Hot reload hooks
│   ├── ffi.rs              # C API (feature: ffi)
│   ├── interop.rs          # Java/Rust bridge (feature: interop)
│   ├── async_io.rs         # Async loading (feature: async)
│   ├── simd.rs             # SIMD arrays (feature: simd)
│   ├── truffle.rs          # Language API (feature: truffle)
│   ├── core.rs             # no_std types (feature: no_std)
│   │
│   └── tests.rs            # Unit tests
│
├── benches/
│   ├── jvm_benchmarks.rs
│   └── instruction_benchmarks.rs
│
├── examples/               # Java example sources
│   ├── HelloWorld.java
│   ├── Calculator.java
│   └── SimpleMath.java
│
├── docs/
│   ├── structure.md        # This file
│   └── index.html          # Documentation landing
│
├── Cargo.toml
├── TODO.md
├── ARCHITECTURE.md
└── spec.md
```

## Module Dependencies (Simplified)

```
main.rs
  └── interpreter, jit, profiler, trace, debug, class_loader, class_file

interpreter (interpreter/)
  └── class_file, class_loader, memory, native, jit
  └── debug, profiler, trace, deterministic, security, reflection

class_loader
  └── class_file, class_cache, error

memory (memory/)
  └── error, debug, security
  └── Submodules: value, frame, monitor, heap_object, heap, stack

jit
  └── class_file, memory, cranelift_jit, native

cranelift_jit
  └── class_file, jit, memory
```

## Adding New Modules

1. Add `pub mod new_module;` in `lib.rs` under the appropriate section
2. Add `mod new_module;` in `main.rs` if the binary needs it
3. Update this document
