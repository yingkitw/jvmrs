# Architecture - JVMRS

## Source Code Organization

Modules are grouped logically in `src/lib.rs`:

| Group | Modules | Purpose |
|-------|---------|---------|
| **Core** | class_file, class_loader, class_cache, memory, allocator, gc | Class loading, heap, GC |
| **Execution** | interpreter, native, reflection | Bytecode execution, native methods |
| **Compilation** | jit, cranelift_jit, aot_compiler, wasm_backend | JIT, AOT, WASM |
| **Tools** | debug, profiler, trace, deterministic | Debugging, profiling |
| **Optional** | ffi, interop, async_io, simd, truffle, security | Feature-gated extensions |

See `docs/structure.md` for the full directory layout and module dependencies.

---

## Overview
JVMRS is a simplified Java Virtual Machine implementation written in Rust. The architecture follows a modular design with clear separation of concerns between class file parsing, memory management, and instruction interpretation.

## Differentiation from Other JVM Implementations

### Language and Memory Safety
- **Rust-based Implementation**: Unlike HotSpot (C++) or OpenJ9 (C++), JVMRS leverages Rust's ownership system for memory safety without garbage collection in the VM code itself
- **Compile-time Safety Guarantees**: Eliminates entire classes of bugs common in C++ JVM implementations (use-after-free, data races, buffer overflows)
- **No Need for VM-level Memory Safeguards**: The VM code is memory-safe by construction, reducing the attack surface compared to traditional JVMs

### Compilation and Backend Architecture
- **Multi-Backend Compilation**: Native support for multiple compilation targets (x86, WebAssembly, AOT object files) from a single codebase
- **Cranelift-based JIT**: Uses a modern, Rust-native code generator instead of the C2/C1 compilers in HotSpot
- **WebAssembly Native Target**: First-class WASM support for browser and edge deployment scenarios

### Modularity and Feature Gating
- **Fine-Grained Feature Flags**: Components like JIT, FFI, async I/O, and SIMD can be selectively compiled
- **Embedded/No-STD Support**: Can run on resource-constrained platforms where traditional JVMs cannot
- **Polyglot Integration**: Built-in support for cross-language interoperability beyond standard JNI

### Memory Management Innovations
- **Arena-Based Allocators**: Improves cache locality and reduces fragmentation compared to traditional heap management
- **Generational GC with Rust Roots**: Uses Rust's ownership system for efficient root set tracking
- **Parallel GC Sweep**: Utilizes rayon for parallel garbage collection operations

### Tooling and Observability
- **Deterministic Execution Mode**: Enables reproducible execution for testing and debugging
- **Time-Travel Debugging**: Built-in support for historical debugging not available in standard JVMs
- **Integrated Profiling**: Native profiling capabilities without external tools
- **Security Instrumentation**: Built-in security monitoring and analysis capabilities

### Developer Experience
- **Cargo Integration**: Leverages Rust's ecosystem for testing, benchmarking, and dependency management
- **API-First Design**: Clean separation between library and binary components
- **Native FFI Layer**: Designed from the ground up for easy embedding in Rust applications

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Application Layer                       │
├─────────────────────────────────────────────────────────────┤
│  main.rs ──┐                                                │
│            ├──► Interpreter ──┐                             │
│            │                  ├──► Class Loader             │
│            │                  ├──► Instruction Dispatch     │
│            │                  └──► Runtime State            │
│            └──► CLI Interface                               │
├─────────────────────────────────────────────────────────────┤
│                      Core Components                         │
├─────────────────────────────────────────────────────────────┤
│  class_file.rs ──┐                                          │
│                  ├──► Class File Parser                     │
│                  ├──► Constant Pool                         │
│                  ├──► Field/Method Info                     │
│                  └──► Attribute Handling                    │
│                                                             │
│  memory.rs ──────┐                                          │
│                  ├──► Heap Management                       │
│                  ├──► Stack Frames                          │
│                  ├──► Value Representation                  │
│                  └──► Object Model                          │
│                                                             │
│  interpreter.rs ─┐                                          │
│                  ├──► Opcode Implementation                 │
│                  ├──► Method Invocation                     │
│                  ├──► Control Flow                          │
│                  └──► Exception Handling                    │
└─────────────────────────────────────────────────────────────┘
```

## Component Details

### 1. Class File Module (`src/class_file.rs`)
**Purpose**: Parse Java class files according to JVM specification

**Key Structures**:
- `ClassFile`: Main structure representing a loaded class
- `ConstantPool`: Constant pool entries (strings, numbers, references)
- `MethodInfo`: Method metadata and bytecode
- `FieldInfo`: Field metadata
- `AttributeInfo`: Various class file attributes

**Responsibilities**:
- Validate class file magic number and version
- Parse constant pool entries
- Extract method and field information
- Handle class file attributes
- Provide access to bytecode instructions

### 2. Memory Module (`src/memory.rs`)
**Purpose**: Manage JVM memory including heap, stack, and runtime values

**Key Structures**:
- `Heap`: Object storage and allocation
- `StackFrame`: Execution context for methods
- `Value`: Type-safe representation of JVM values
- `Object`: Runtime object representation
- `Array`: Array storage (planned)

**Responsibilities**:
- Allocate and deallocate memory
- Manage method call stack
- Handle value conversions
- Implement object model
- Provide garbage collection

### 2a. Allocator Module (`src/allocator.rs`)
**Purpose**: Arena-based allocation for cache locality and reduced fragmentation

**Key Structures**:
- `ArenaAllocator`: Slot-based arena for heap objects (contiguous storage)
- `ArrayArena`: Arena for array storage

**Benefits**: Cache-friendly iteration during GC, O(1) allocation with free-list reuse

### 2b. GC Module (`src/gc.rs`)
**Purpose**: Advanced garbage collection with generational and ownership-based strategies

**Key Structures**:
- `GenerationalHeap`: Young/old generation heap with promotion
- `ScopedRoot`: RAII-based root set management
- `Generation`: Young vs Old object classification

**Features**:
- Generational GC: Minor GC (young only), Major GC (full heap)
- Parallel sweep using rayon
- Pauseless root tracking via Rust ownership (ScopedRoot)
- Promotion threshold for survivor aging

### 3a. FFI Module (`src/ffi.rs`) - feature `ffi`
**Purpose**: C API for embedding jvmrs in C/C++ applications

**Key Functions**:
- `jvmrs_create`, `jvmrs_create_with_classpath`: Create interpreter
- `jvmrs_load_class`, `jvmrs_run_main`: Execute Java code
- `jvmrs_destroy`: Clean up

### 3b. Interop Module (`src/interop.rs`) - feature `interop`
**Purpose**: Polyglot Java/Rust interop - share objects between runtimes

**Key Structures**:
- `SharedObjectId`, `JavaObjectBridge`: Bridge for Java objects in Rust
- `register_rust_callback`, `invoke_rust_callback`: Rust callbacks from Java native methods

### 3c. Async I/O Module (`src/async_io.rs`) - feature `async`
**Purpose**: Async class loading with tokio

**Key Structures**:
- `AsyncClassLoader` trait, `TokioClassLoader`: Load classes asynchronously

### 3d. SIMD Module (`src/simd.rs`) - feature `simd`
**Purpose**: Vectorized array operations (AVX2/AVX on x86)

**Key Functions**:
- `heap_array_copy_int`, `heap_array_copy_float`: SIMD-accelerated array copy

### 3e. Truffle Module (`src/truffle.rs`) - feature `truffle`
**Purpose**: GraalVM-style API for pluggable language implementations

**Key Structures**:
- `TruffleNode` trait: Executable AST nodes
- `TruffleFrame`: Execution context
- `LanguageFrontend` trait: Language parser/frontend

### 3f. Core Module (`src/core.rs`) - feature `no_std`
**Purpose**: Minimal types for embedded/no_std targets

### 3g. JIT Module (`src/jit.rs`)
**Purpose**: JIT compilation, tiered compilation, AOT

**Key Structures**:
- `JitManager`: Orchestrates JIT compilation and method lookup
- `CraneliftJitCompiler`: Compiles hot methods to native code
- `AotCompiler`: AOT compilation to `.o` files
- `MethodProfile`, `TieredCompilationConfig`: Tiered compilation

### 3h. Cranelift JIT Backend (`src/cranelift_jit.rs`)
**Purpose**: Bytecode-to-native code generation via cranelift-jit

**Key Structures**:
- `CraneliftJitBackend`: JIT module with FFI helpers
- FFI: `jvmrs_frame_get_local_int`, `jvmrs_frame_push_int`

**Supported bytecode**: bipush, iload_0..3, iadd, ireturn

### 3i. AOT Compiler (`src/aot_compiler.rs`)
**Purpose**: Ahead-of-time compilation to native object files

**Key Functions**:
- `compile_class_to_object()`: Emits `.o` via cranelift-object

### 3j. WASM Backend (`src/wasm_backend.rs`) - feature `wasm`
**Purpose**: Emit WebAssembly from JVM bytecode

**Key Structures**:
- `WasmGenerator`: Translates bytecode to WASM instructions

### 4. Interpreter Module (`src/interpreter.rs`)
**Purpose**: Execute Java bytecode instructions

**Key Structures**:
- `Interpreter`: Main execution engine
- `RuntimeState`: Current execution context
- `ClassLoader`: Load and resolve classes
- `MethodArea`: Store loaded classes

**Responsibilities**:
- Dispatch bytecode instructions
- Manage class loading and resolution
- Handle method invocation
- Implement control flow
- Provide runtime services

## Data Flow

1. **Class Loading Phase**:
   ```
   File System → ClassFile Parser → Constant Pool → Method/Field Info → ClassLoader
   ```

2. **Execution Phase**:
   ```
   main() → Interpreter → StackFrame → Instruction Dispatch → Memory Operations → Result
   ```

3. **Memory Management**:
   ```
   Allocation Request → Heap Manager → Object Creation → Reference Tracking → (GC) → Deallocation
   ```

## Key Design Decisions

### 1. Rust-Centric Design
- Leverage Rust's ownership system for memory safety
- Use enums for type-safe value representation
- Implement error handling with `Result` types
- Use traits for extensibility

### 2. Simplified JVM Model
- Start with core JVM features
- Gradually add complexity
- Focus on correctness over performance initially
- Clear separation between specification and implementation

### 3. Modular Architecture
- Each module has well-defined responsibilities
- Minimal dependencies between modules
- Clear interfaces for testing
- Easy to extend or replace components

## Memory Layout

### Heap Organization
```
┌─────────────────┐
│   Young Gen     │  (planned)
│   (Eden/Surv)   │
├─────────────────┤
│   Old Gen       │  (planned)
│   (Tenured)     │
├─────────────────┤
│   Perm Gen      │  (planned)
│   (Metaspace)   │
└─────────────────┘
```

### Stack Frame Layout
```
┌─────────────────┐
│   Local Vars    │  [0..n]
├─────────────────┤
│   Operand Stack │  [0..m]
├─────────────────┤
│   Frame Data    │  (return address, etc.)
└─────────────────┘
```

## Instruction Set Architecture

### Current Support
- Arithmetic operations (iadd, isub, imul, etc.)
- Control flow (goto, if_icmp, etc.)
- Stack manipulation (dup, swap, pop)
- Local variable access (iload, istore)
- Method invocation (invokestatic, invokevirtual)

### Planned Extensions
- Object creation and manipulation
- Array operations
- Exception handling
- ~~Synchronization~~ (monitorenter/monitorexit implemented)
- Type checking

## Performance Considerations

### Current
- Simple interpreter loop
- Direct method dispatch
- Basic memory management

### Future Optimizations
- JIT compilation (Cranelift bytecode-to-native for supported opcodes)
- Inline caching
- Escape analysis
- Memory pooling
- Parallel garbage collection

## Security Model

### Current
- Basic class file validation
- Type checking during execution
- Stack bounds checking

### Planned
- Bytecode verification
- Access control
- Sandboxing
- Resource limits

## Testing Strategy

### Unit Tests
- Each module tested independently
- Mock dependencies where needed
- Test edge cases and error conditions

### Integration Tests
- End-to-end execution of Java programs
- Compatibility with standard Java examples
- Performance regression testing

### Property-Based Tests
- Generate random class files
- Test parser robustness
- Verify execution correctness

## Extension Points

### 1. New Opcode Support
- Add new match arm in interpreter
- Implement required memory operations
- Update documentation

### 2. Memory Management
- Implement different GC algorithms
- Add memory profiling
- Support custom allocators

### 3. Class Loading
- Support custom class loaders
- Add bytecode transformation
- Implement dynamic class generation

## Dependencies

### Core
- `byteorder`: Class file binary reading
- `log`, `env_logger`: Logging
- `rayon`: Parallel GC sweep

### Compilation
- `cranelift`, `cranelift-jit`, `cranelift-module`, `cranelift-object`, `cranelift-codegen`, `cranelift-frontend`, `cranelift-native`: JIT and AOT
- `inkwell` (optional, `llvm`): LLVM IR export
- `wasm-encoder` (optional, `wasm`): WebAssembly emission

### Optional
- `criterion`: Benchmarking

## Development Guidelines

### Code Style
- Follow Rust conventions
- Use meaningful names
- Document public APIs
- Write comprehensive tests

### Error Handling
- Use custom error types
- Provide context in error messages
- Handle all possible error cases
- Log errors appropriately

### Performance
- Profile before optimizing
- Use appropriate data structures
- Minimize allocations in hot paths
- Consider cache locality

## Future Architecture Directions

### 1. Tiered Compilation (Implemented)
- Interpreter for cold code
- Baseline JIT for warm code (after threshold invocations)
- Optimized JIT for hot code
- `JitManager`, `MethodProfile`, `CompilationLevel`, `TieredCompilationConfig` in `src/jit.rs`

### 2. Compilation Backends
- **Cranelift JIT**: `cranelift_jit::CraneliftJitBackend` – bytecode-to-native (bipush, iload_0..3, iadd, ireturn)
- **AOT**: `aot_compiler::compile_class_to_object` – emits `.o` via cranelift-object
- **LLVM IR** (`--features llvm`): Bytecode-to-IR translation
- **WebAssembly** (`--features wasm`): `wasm_backend::WasmGenerator` emits WASM from bytecode

### 3. Multi-threading Support
- Thread-local allocation buffers
- Concurrent garbage collection
- Synchronization primitives

### 4. Cross-Platform
- WebAssembly backend (`cargo build --features wasm`)
- Embedded systems support
- Mobile platform compatibility

### 5. Tooling Integration
- Debugger interface
- Profiling hooks
- Monitoring APIs
- Management console