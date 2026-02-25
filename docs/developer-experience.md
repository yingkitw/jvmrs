# Developer Experience

## Performance Profiling with Rust Tooling

JVMRS includes an integrated profiler that exports data compatible with **cargo-flamegraph** and Brendan Gregg's **flamegraph.pl**.

### Usage

```bash
# Run with profiler, output collapsed stack format
./target/debug/jvmrs --profile --profile-output profile.txt HelloWorld

# Generate flame graph (requires flamegraph.pl or inferno)
cat profile.txt | flamegraph.pl > flamegraph.svg

# Or use cargo-flamegraph for Rust + JVM combined profiling
cargo flamegraph --bin jvmrs -- HelloWorld
```

### Output Format

The profiler writes collapsed stack format:
```
Main.main;Util.helper 1500
Main.main 800
```

Compatible with: flamegraph.pl, inferno, cargo-flamegraph.

---

## Hot Reloading for Java/Rust Hybrid Development

The `hot_reload` module provides an API for detecting when Java class files change and triggering reloads.

```rust
use jvmrs::hot_reload::HotReloadManager;

let mut mgr = HotReloadManager::new();
mgr.set_enabled(true);
mgr.register_reloadable("MyClass", "myMethod");

if mgr.class_file_changed(Path::new("MyClass.class")) {
    // Reload class and replace method
}
```

Full hotswap requires JIT support for function redefinition (planned).

---

## Rust Documentation for Java APIs

Use `cargo doc` to generate documentation. The `reflection` module exposes Java class metadata:

```rust
let reflection = interpreter.get_reflection_api();
let class_info = interpreter.get_class_reflection("java/lang/String");
// class_info contains method signatures, fields, etc.
```

For Java API docs, consider generating from `ClassReflection` to Markdown or HTML.

---

## IDE Support

Future work:
- VS Code / Cursor extension for cross-language debugging
- LSP for Java source when used with JVMRS
