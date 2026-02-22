# JVMRS - A Simple JVM Implementation in Rust

This is a basic implementation of the Java Virtual Machine (JVM) written in Rust. It demonstrates the core concepts of JVM execution including class file parsing, constant pool handling, instruction interpretation, and memory management.

## Features

- Class file parser supporting Java bytecode
- Constant pool parsing for various entry types
- Basic instruction interpreter for common opcodes
- Stack-based execution engine
- Simple memory management with heap and stack frames
- Support for primitive operations (integer and float arithmetic)
- Basic method invocation

## Project Structure

- `src/class_file.rs` - Class file parser and related data structures
- `src/memory.rs` - Memory management (heap, stack frames, values)
- `src/interpreter.rs` - Instruction interpreter and class loader
- `src/main.rs` - Main entry point
- `examples/` - Example Java source files

## Running the JVM

1. Compile the Java examples:

```bash
javac examples/HelloWorld.java
javac examples/Calculator.java
```

2. Run with our JVM:

```bash
cargo run HelloWorld
cargo run Calculator
```

## Current Limitations

This is a simplified implementation that only supports a subset of JVM features:

- Limited instruction set (not all JVM opcodes are implemented)
- No garbage collection
- Minimal error handling
- No threading support
- No native method interface
- Simplified class loading

## Example Java Programs

The examples directory contains simple Java programs that can be compiled and run on this JVM:

- `HelloWorld.java` - A basic "Hello, World!" program
- `Calculator.java` - Demonstrates arithmetic operations

## Building

```bash
cargo build
```

## Testing

```bash
cargo test
```
