# WASI and Next-Generation Deployment

## WebAssembly System Interface (WASI)

JVMRS can target **WASI** (WebAssembly System Interface) for running Java bytecode in WASI runtimes (Wasmer, Wasmtime, Fastly Compute@Edge, etc.).

### Build Target

```bash
# Add wasm32-wasi target
rustup target add wasm32-wasi

# Build for WASI (with wasm feature)
cargo build --target wasm32-wasi --features wasm
```

### Use Cases

| Target | Benefit |
|--------|---------|
| **Fastly Compute@Edge** | Run Java logic at the edge with low latency |
| **Cloudflare Workers** | Same as above |
| **Wasmer / Wasmtime** | Portable, sandboxed execution without a full JVM |
| **Serverless** | Small binary, fast cold start, pay-per-invocation |

### Limitations

- Current WASM backend supports a subset of JVM bytecode (bipush, iload, iadd, isub, imul, idiv, ireturn)
- No full runtime (GC, class loading) in WASM yet—methods are compiled ahead-of-time
- WASI syscalls (fd, env) require integration with wasi-libc when used in WASI runtimes

---

## Browser-Based Java Execution

Java bytecode can be compiled to WebAssembly and run directly in the browser.

### Quick Start

1. Build with wasm feature:
   ```bash
   cargo build --features wasm
   ```

2. Compile a class to WASM:
   ```bash
   ./target/debug/jvmrs --wasm app.wasm HelloWorld
   ```

3. Serve the WASM file with the browser loader:
   ```bash
   cp app.wasm examples/wasm-browser/
   cd examples/wasm-browser && python -m http.server 8080
   ```

4. Open http://localhost:8080

### Benefits

- **No JVM download**: Logic runs entirely in the browser
- **Sandboxed**: WebAssembly provides strong isolation
- **Portable**: Same bytecode targets native and browser

---

## Edge Computing Optimizations

For edge deployment:

- **Small binary**: Strip symbols, use `opt-level = "z"` for size
- **Deterministic execution**: Use `--deterministic` for reproducible edge behavior
- **Cold start**: Pre-compile hot methods to WASM at build time

---

## Serverless Platform Optimizations

- **Minimal runtime**: JVMRS can be built with `no_std` for minimal footprint
- **Fast bootstrap**: Class cache (`.jvmc`) reduces load time
- **AOT**: Use `--aot` to pre-compile to native object files for Lambda / Cloud Functions

---

See also: `docs/competitive-differentiation.md`, `ARCHITECTURE.md`
