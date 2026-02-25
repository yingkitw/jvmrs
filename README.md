# JVMRS

[![Crates.io](https://img.shields.io/crates/v/jvmrs.svg)](https://crates.io/crates/jvmrs)
[![Documentation](https://docs.rs/jvmrs/badge.svg)](https://docs.rs/jvmrs)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

A Java Virtual Machine implementation in Rust, featuring Cranelift JIT, AOT to native object files, generational GC, and multiple compilation backends.

## Why JVMRS?

Compared to HotSpot, OpenJ9, and GraalVM, jvmrs differentiates with **Rust-native design** and unique capabilities:

| Capability | HotSpot / OpenJ9 / GraalVM | JVMRS |
|------------|----------------------------|-------|
| **Language** | C++ / Java | **Rust** – memory safety, zero-cost abstractions |
| **JIT backend** | C2 / Graal / Eclipse OMR | **Cranelift** – Rust-native, permissive license |
| **AOT output** | GraalVM native-image (binary) | **Object files (.o)** – link with any C toolchain |
| **WebAssembly** | Limited / experimental | **WASM emission** – run Java in browsers |
| **Java ↔ Rust interop** | JNI only | **Direct polyglot** – shared objects, no JNI |
| **Embedded / no_std** | Not supported | **no_std targets** – microcontrollers, bare-metal |
| **SIMD** | Auto-vectorization | **Explicit SIMD** – Rust `core::arch` |
| **Embedding** | Heavy footprint | **C API** – embed as a library in any app |
| **Truffle-style API** | GraalVM proprietary | **Open implementation** – language-agnostic runtime |

**Use jvmrs when you need**: embeddable JVM, Java→WASM, Rust/Java interop, AOT to `.o` files, or a Rust-based JVM for research and tooling.

---

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
jvmrs = "0.1"
```

For optional features (LLVM IR, WebAssembly, etc.):

```toml
[dependencies]
jvmrs = { version = "0.1", features = ["wasm"] }
```

## Quick Start

```bash
# Clone and run
git clone https://github.com/jvmrs/jvmrs
cd jvmrs

# Compile example Java
javac examples/HelloWorld.java

# Run
cargo run HelloWorld
```

---

## Usage

### Run a class

```bash
# Run main class (resolves via classpath)
cargo run HelloWorld
cargo run Calculator
cargo run SimpleMath
```

### CLI options

| Option | Description |
|--------|-------------|
| `--aot <output>` | AOT compile class to `.o` file instead of executing |
| `--no-jit` | Disable JIT; interpreter-only |
| `--jit-threshold <n>` | Invocations before JIT compile (default: 100) |
| `--llvm` | Emit LLVM IR to stdout (requires `--features llvm`) |
| `--help`, `-h` | Show help |

### Environment variables

| Variable | Description |
|----------|-------------|
| `JVMRS_DEBUG` | Enable debug logging |
| `JVMRS_TRACE` | Enable trace logging |

### Examples

```bash
# Run with default JIT
cargo run Calculator

# Run without JIT
cargo run -- --no-jit Calculator

# AOT compile to object file
cargo run -- --aot output.o HelloWorld

# Emit LLVM IR (requires: cargo build --features llvm)
cargo run --features llvm -- --llvm Calculator > calc.ll

# Custom JIT threshold
cargo run -- --jit-threshold 50 SimpleMath
```

---

## Examples

Example Java programs in `examples/`:

### HelloWorld.java

```java
public class HelloWorld {
    public static void main(String[] args) {
        System.out.println("Hello, World!");
        int a = 5, b = 10, c = a + b;
        System.out.println("5 + 10 = " + c);
    }
}
```

### Calculator.java

```java
public class Calculator {
    public static int add(int a, int b) { return a + b; }
    public static int subtract(int a, int b) { return a - b; }
    public static int multiply(int a, int b) { return a * b; }
    public static float divide(float a, float b) { return a / b; }
    
    public static void main(String[] args) {
        int x = 20, y = 8;
        System.out.println("x + y = " + add(x, y));
        System.out.println("x - y = " + subtract(x, y));
        System.out.println("x * y = " + multiply(x, y));
        System.out.println("x / y = " + divide(x, y));
    }
}
```

### SimpleMath.java

```java
public class SimpleMath {
    public static int add(int a, int b) { return a + b; }
    public static void main(String[] args) {
        int z = add(5, 10);
        System.out.println("5 + 10 = " + z);
    }
}
```

### Running the examples

```bash
javac examples/HelloWorld.java examples/Calculator.java examples/SimpleMath.java
cargo run HelloWorld    # Hello, World! / 5 + 10 = 15
cargo run Calculator   # Arithmetic demo
cargo run SimpleMath   # 5 + 10 = 15
```

---

## Features

### Core
- Class file parser, constant pool, stack-based interpreter
- Generational GC with parallel sweep
- Arrays, strings, inheritance, interfaces

### Compilation
- **Cranelift JIT** – bytecode-to-native for hot methods
- **Tiered compilation** – interpreter → baseline → optimized
- **AOT** – compile to `.o` via cranelift-object
- **LLVM IR** (`--features llvm`) – export to LLVM
- **WebAssembly** (`--features wasm`) – emit WASM

### Optional (feature-gated)
- `ffi` – C API for embedding
- `interop` – Java/Rust polyglot
- `async` – tokio async I/O
- `simd` – SIMD array ops
- `truffle` – GraalVM-style language API

---

## Building

```bash
cargo build
cargo build --features llvm    # LLVM IR export
cargo build --features wasm    # WebAssembly backend
```

## Testing

```bash
cargo test
```

## Documentation

- [API Documentation](https://docs.rs/jvmrs)
- `ARCHITECTURE.md` – Design and components
- `spec.md` – Technical specification
- `TODO.md` – Roadmap

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
