// Core
mod class_file;
mod class_cache;
mod class_loader;
mod memory;
mod allocator;
mod gc;
mod error;

// Execution
mod interpreter;
mod native;
mod reflection;
mod jni;
mod annotations;
mod serialization;
mod visualization;

// Compilation
mod jit;
mod cranelift_jit;
mod aot_compiler;

// Tools
mod debug;
mod profiler;
mod trace;
mod deterministic;

// Optional
mod security;
mod aop;
mod cloud;
mod hot_reload;
mod extensions;

#[cfg(feature = "ffi")]
mod ffi;
#[cfg(feature = "interop")]
mod interop;
#[cfg(feature = "async")]
mod async_io;
#[cfg(feature = "simd")]
mod simd;
#[cfg(feature = "truffle")]
mod truffle;
#[cfg(feature = "wasm")]
mod wasm_backend;

#[cfg(test)]
mod tests;

use debug::init_logging;
use interpreter::Interpreter;
use jit::AotCompiler;
use log::info;
use std::sync::Arc;

use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process;

/// Interactive REPL for JVM exploration
fn run_repl(
    disable_jit: bool,
    jit_threshold: Option<u64>,
    enable_deterministic: bool,
    enable_sanitizer: bool,
    verbose: bool,
) {
    let log_level = if env::var("JVMRS_DEBUG").is_ok() {
        log::LevelFilter::Debug
    } else if verbose {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Warn
    };
    init_logging(log_level);

    let mut interpreter = Interpreter::new();
    if disable_jit {
        interpreter.set_jit_enabled(false);
    } else if let Some(threshold) = jit_threshold {
        use jit::TieredCompilationConfig;
        interpreter = Interpreter::with_jit(TieredCompilationConfig {
            baseline_threshold: threshold,
            ..Default::default()
        });
    }
    if enable_deterministic {
        interpreter.set_deterministic(Some(deterministic::DeterministicConfig::default()));
    }
    if enable_sanitizer {
        interpreter.set_sanitizer(Some(Arc::new(security::Sanitizer::new(
            security::SecurityConfig::default(),
        ))));
    }

    println!("JVMRS REPL - Interactive JVM exploration");
    println!("Commands: run <ClassName> | load <ClassName> | classes | quit | help");
    println!();

    loop {
        print!("jvmrs> ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_err() || line.is_empty() {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts.as_slice() {
            ["quit" | "exit" | "q"] => {
                println!("Goodbye.");
                break;
            }
            ["help" | "?"] => {
                println!("  run <ClassName>   - Load and run main() of the class");
                println!("  load <ClassName>  - Load class (no execution)");
                println!("  classes           - List loaded classes");
                println!("  quit              - Exit REPL");
            }
            ["run", name] => {
                if let Err(e) = interpreter.load_class_by_name(name) {
                    eprintln!("Load error: {}", e);
                } else if let Err(e) = interpreter.run_main(name) {
                    eprintln!("Run error: {}", e);
                }
            }
            ["load", name] => {
                if let Err(e) = interpreter.load_class_by_name(name) {
                    eprintln!("Load error: {}", e);
                } else {
                    println!("Loaded {}", name);
                }
            }
            ["classes"] => {
                // List loaded classes - use reflection if available
                println!("(Loaded classes listing - use 'run <ClassName>' to execute)");
            }
            _ => {
                // Try as class name (run)
                if let Err(e) = interpreter.load_class_by_name(line) {
                    eprintln!("Unknown command or load error: {}", e);
                } else if let Err(e) = interpreter.run_main(line) {
                    eprintln!("Run error: {}", e);
                }
            }
        }
    }
}

/// Print usage information
fn print_usage(program_name: &str) {
    eprintln!("jvmrs - JVM Implementation in Rust");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("  {} [OPTIONS] <classfile>", program_name);
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("  --aot <output>        AOT compile to object file");
    eprintln!("  --no-jit              Disable JIT compilation");
    eprintln!("  --jit-threshold <n>   JIT threshold (default: 100)");
    eprintln!("  --profile             Enable profiler (flame graph data)");
    eprintln!("  --profile-output <f>  Write flame graph to file (use with --profile)");
    eprintln!("  --trace               Enable execution trace (time-travel debugging)");
    eprintln!("  --trace-output <f>   Write trace to file (use with --trace)");
    eprintln!("  --class-cache-dir <d> Use binary cache for fast class loading (.jvmc)");
    eprintln!("  --deterministic       Enable deterministic execution (fixed RNG, timestamps)");
    eprintln!("  --sanitizer           Enable security instrumentation (bounds/null checks)");
    eprintln!("  --llvm                Emit LLVM IR to stdout");
    eprintln!("  --wasm <output>       Emit WebAssembly (requires --features wasm)");
    eprintln!("  --repl                Interactive REPL for JVM exploration");
    eprintln!("  --dump                 Dump memory/stack visualization after execution");
    eprintln!("  --verbose, -v         Enable verbose logging (instruction/method trace)");
    eprintln!("  --help, -h            Print this help message");
    eprintln!();
    eprintln!("ENVIRONMENT VARIABLES:");
    eprintln!("  JVMRS_DEBUG           Enable debug logging");
    eprintln!("  JVMRS_TRACE           Enable trace logging");
    eprintln!("  JVMRS_TRACE_INSTRUCTIONS  Log each bytecode instruction");
    eprintln!("  JVMRS_TRACE_MEMORY    Log memory allocations and accesses");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("  {} HelloWorld", program_name);
    eprintln!("  {} --aot output HelloWorld", program_name);
    eprintln!("  {} --no-jit Calculator", program_name);
    eprintln!("  {} --llvm MyClass > output.ll", program_name);
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        process::exit(1);
    }

    // Parse command line arguments
    let mut aot_output: Option<String> = None;
    let mut disable_jit = false;
    let mut jit_threshold: Option<u64> = None;
    let mut generate_llvm = false;
    let mut wasm_output: Option<String> = None;
    let mut enable_profile = false;
    let mut profile_output: Option<String> = None;
    let mut enable_trace = false;
    let mut trace_output: Option<String> = None;
    let mut class_cache_dir: Option<String> = None;
    let mut enable_deterministic = false;
    let mut enable_sanitizer = false;
    let mut verbose = false;
    let mut enable_repl = false;
    let mut enable_dump = false;
    let mut class_name: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--aot" => {
                if i + 1 < args.len() {
                    aot_output = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --aot requires an output path");
                    process::exit(1);
                }
            }
            "--no-jit" => {
                disable_jit = true;
                i += 1;
            }
            "--jit-threshold" => {
                if i + 1 < args.len() {
                    jit_threshold = Some(args[i + 1].parse().expect("Invalid JIT threshold"));
                    i += 2;
                } else {
                    eprintln!("Error: --jit-threshold requires a number");
                    process::exit(1);
                }
            }
            "--llvm" => {
                generate_llvm = true;
                i += 1;
            }
            "--wasm" => {
                if i + 1 < args.len() {
                    wasm_output = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --wasm requires an output path");
                    process::exit(1);
                }
            }
            "--profile" => {
                enable_profile = true;
                i += 1;
            }
            "--profile-output" => {
                if i + 1 < args.len() {
                    profile_output = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --profile-output requires a path");
                    process::exit(1);
                }
            }
            "--trace" => {
                enable_trace = true;
                i += 1;
            }
            "--trace-output" => {
                if i + 1 < args.len() {
                    trace_output = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --trace-output requires a path");
                    process::exit(1);
                }
            }
            "--class-cache-dir" => {
                if i + 1 < args.len() {
                    class_cache_dir = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --class-cache-dir requires a path");
                    process::exit(1);
                }
            }
            "--deterministic" => {
                enable_deterministic = true;
                i += 1;
            }
            "--sanitizer" => {
                enable_sanitizer = true;
                i += 1;
            }
            "--repl" => {
                enable_repl = true;
                i += 1;
            }
            "--dump" => {
                enable_dump = true;
                i += 1;
            }
            "--verbose" | "-v" => {
                verbose = true;
                i += 1;
            }
            "--help" | "-h" => {
                print_usage(&args[0]);
                process::exit(0);
            }
            arg => {
                if arg.starts_with('-') {
                    eprintln!("Error: Unknown option: {}", arg);
                    print_usage(&args[0]);
                    process::exit(1);
                }
                class_name = Some(arg.to_string());
                i += 1;
            }
        }
    }

    // Initialize logging (after parsing --verbose)
    let log_level = if env::var("JVMRS_DEBUG").is_ok() {
        log::LevelFilter::Debug
    } else if env::var("JVMRS_TRACE").is_ok() || verbose {
        log::LevelFilter::Trace
    } else {
        log::LevelFilter::Info
    };
    init_logging(log_level);
    if verbose {
        unsafe {
            std::env::set_var("JVMRS_TRACE_INSTRUCTIONS", "1");
            std::env::set_var("JVMRS_TRACE_METHODS", "1");
        }
    }

    let class_name = class_name;

    if enable_repl {
        run_repl(disable_jit, jit_threshold, enable_deterministic, enable_sanitizer, verbose);
        return;
    }

    let class_name = match class_name {
        Some(name) => name,
        None => {
            eprintln!("Error: No class file specified");
            print_usage(&args[0]);
            process::exit(1);
        }
    };

    // Get the class name without .class extension if present
    let class_name_without_ext = class_name.trim_end_matches(".class");

    // AOT compilation mode
    if let Some(output_path) = aot_output {
        info!("AOT compilation mode: compiling '{}' to '{}'", class_name_without_ext, output_path);

        // Load the class
        let class_file = match class_file::ClassFile::from_file(class_name_without_ext) {
            Ok(class) => class,
            Err(e) => {
                // Try loading from classpath
                match class_loader::ClassLoader::new_default().load_class(class_name_without_ext) {
                    Ok(_) => {
                        // Get the loaded class
                        let loader = class_loader::ClassLoader::new_default();
                        match loader.get_class(class_name_without_ext) {
                            Some(class) => class.clone(),
                            None => {
                                eprintln!("Error loading class '{}': {}", class_name_without_ext, e);
                                process::exit(1);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error loading class '{}': {}", class_name_without_ext, e);
                        process::exit(1);
                    }
                }
            }
        };

        // Create AOT compiler
        let mut aot_compiler = match AotCompiler::new() {
            Ok(compiler) => compiler,
            Err(e) => {
                eprintln!("Error creating AOT compiler: {}", e);
                process::exit(1);
            }
        };

        // Compile the class
        let output_path = Path::new(&output_path);
        match aot_compiler.compile_class(&class_file, output_path) {
            Ok(_) => {
                println!("Successfully compiled {} to {}", class_name_without_ext, output_path.display());
            }
            Err(e) => {
                eprintln!("Error compiling class: {}", e);
                process::exit(1);
            }
        }

        return;
    }

    // LLVM IR generation mode
    if generate_llvm {
        #[cfg(feature = "llvm")]
        {
            info!("LLVM IR generation mode: generating IR for '{}'", class_name_without_ext);

            // Load the class
            let loader = class_loader::ClassLoader::new_default();
            if let Err(e) = loader.load_class(class_name_without_ext) {
                eprintln!("Error loading class '{}': {}", class_name_without_ext, e);
                process::exit(1);
            }

            let class_file = match loader.get_class(class_name_without_ext) {
                Some(class) => class,
                None => {
                    eprintln!("Error: class not found");
                    process::exit(1);
                }
            };

            // Generate LLVM IR for each method
            let mut llvm_gen = jit::llvm_backend::LlvmIrGenerator::new("jvmrs_module");

            for method in &class_file.methods {
                let method_name = class_file
                    .get_string(method.name_index)
                    .unwrap_or_else(|| "unknown".to_string());

                match llvm_gen.method_to_llvm_ir(class_file, method) {
                    Ok(ir) => println!("{}", ir),
                    Err(e) => {
                        eprintln!("Error generating LLVM IR for method '{}': {}", method_name, e);
                    }
                }
            }

            return;
        }

        #[cfg(not(feature = "llvm"))]
        {
            eprintln!("Error: LLVM feature is not enabled");
            eprintln!("Please rebuild with: cargo build --features llvm");
            process::exit(1);
        }
    }

    // WebAssembly emission mode
    if let Some(ref output_path) = wasm_output {
        #[cfg(feature = "wasm")]
        {
            use wasm_backend::WasmGenerator;
            info!("WASM emission: compiling '{}' to '{}'", class_name_without_ext, output_path);

            let mut loader = class_loader::ClassLoader::new_default();
            if let Err(e) = loader.load_class(class_name_without_ext) {
                eprintln!("Error loading class '{}': {}", class_name_without_ext, e);
                process::exit(1);
            }

            let class_file = match loader.get_class(class_name_without_ext) {
                Some(class) => class,
                None => {
                    eprintln!("Error: class not found");
                    process::exit(1);
                }
            };

            let mut wasm_gen = WasmGenerator::new();
            for method in &class_file.methods {
                let _ = wasm_gen.method_to_wasm(class_file, method);
            }
            match wasm_gen.write_to_file(Path::new(output_path)) {
                Ok(_) => println!("WASM written to {}", output_path),
                Err(e) => {
                    eprintln!("Error writing WASM: {}", e);
                    process::exit(1);
                }
            }
            return;
        }

        #[cfg(not(feature = "wasm"))]
        {
            eprintln!("Error: wasm feature is not enabled");
            eprintln!("Please rebuild with: cargo build --features wasm");
            process::exit(1);
        }
    }

    // Normal execution mode
    let mut interpreter = Interpreter::new();

    if enable_profile {
        interpreter.set_profiler(Some(Arc::new(profiler::Profiler::new())));
        info!("Profiling enabled");
    }
    if enable_trace {
        interpreter.set_trace_recorder(Some(trace::TraceRecorder::new()));
        info!("Trace recording enabled");
    }
    if let Some(ref dir) = class_cache_dir {
        interpreter.set_class_cache_dir(Some(std::path::PathBuf::from(dir)));
        info!("Class cache directory: {}", dir);
    }
    if enable_deterministic {
        interpreter.set_deterministic(Some(deterministic::DeterministicConfig::default()));
        info!("Deterministic mode enabled");
    }
    if enable_sanitizer {
        interpreter.set_sanitizer(Some(Arc::new(security::Sanitizer::new(
            security::SecurityConfig::default(),
        ))));
        info!("Security sanitizer enabled");
    }

    // Configure JIT
    if disable_jit {
        interpreter.set_jit_enabled(false);
        info!("JIT compilation disabled");
    } else if let Some(threshold) = jit_threshold {
        use jit::TieredCompilationConfig;
        let config = TieredCompilationConfig {
            baseline_threshold: threshold,
            ..Default::default()
        };
        interpreter = Interpreter::with_jit(config);
        info!("JIT threshold set to {}", threshold);
    } else {
        info!("JIT compilation enabled (threshold: 100)");
    }

    // Try to load the class using classpath resolution
    if let Err(e) = interpreter.load_class_by_name(class_name_without_ext) {
        eprintln!("Error loading class '{}': {}", class_name_without_ext, e);
        process::exit(1);
    }

    // Run the main method
    let run_result = interpreter.run_main(class_name_without_ext);

    // Memory/stack dump if enabled (also on error for debugging)
    if enable_dump {
        println!("\n{}", visualization::memory_dump_ascii(interpreter.memory()));
    }

    if let Err(e) = run_result {
        eprintln!("Error running main method: {}", e);
        process::exit(1);
    }

    // Print profiler results if enabled
    if enable_profile {
        if let Some(p) = interpreter.profiler() {
            println!("\n--- Profiler Summary ---\n{}", p.summary());
            if let Some(ref path) = profile_output {
                if let Err(e) = p.write_flame_graph(Path::new(path)) {
                    eprintln!("Warning: could not write flame graph: {}", e);
                } else {
                    println!("\nFlame graph data written to {}", path);
                }
            }
        }
    }

    // Write trace if enabled
    if enable_trace {
        if let Some(tr) = interpreter.trace_recorder() {
            if let Some(ref path) = trace_output {
                if let Err(e) = tr.write_to_file(Path::new(path)) {
                    eprintln!("Warning: could not write trace: {}", e);
                } else {
                    println!("\nTrace written to {} ({} steps)", path, tr.step_count());
                }
            } else {
                println!("\n--- Trace ({} steps) ---\n{}", tr.step_count(), tr.export_text());
            }
        }
    }
}
