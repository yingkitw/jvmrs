# Architecture - JVMRS

## Overview
JVMRS is a simplified Java Virtual Machine implementation written in Rust. The architecture follows a modular design with clear separation of concerns between class file parsing, memory management, and instruction interpretation.

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
- Provide garbage collection (planned)

### 3. Interpreter Module (`src/interpreter.rs`)
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
- Synchronization
- Type checking

## Performance Considerations

### Current
- Simple interpreter loop
- Direct method dispatch
- Basic memory management

### Future Optimizations
- JIT compilation
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
- `byteorder`: For reading class file binary data

### Planned
- `log`: For structured logging
- `serde`: For serialization/deserialization
- `rayon`: For parallel execution
- `criterion`: For benchmarking

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

### 1. Tiered Compilation
- Interpreter for cold code
- Simple JIT for warm code
- Optimizing JIT for hot code

### 2. Multi-threading Support
- Thread-local allocation buffers
- Concurrent garbage collection
- Synchronization primitives

### 3. Cross-Platform
- WebAssembly compilation
- Embedded systems support
- Mobile platform compatibility

### 4. Tooling Integration
- Debugger interface
- Profiling hooks
- Monitoring APIs
- Management console