mod class_file;
mod memory;
mod interpreter;
mod error;
mod class_loader;

#[cfg(test)]
mod tests;

use interpreter::Interpreter;

use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <classfile>", args[0]);
        eprintln!("Example: {} HelloWorld", args[0]);
        eprintln!("         {} Calculator", args[0]);
        process::exit(1);
    }

    let class_name = &args[1];

    // Create interpreter
    let mut interpreter = Interpreter::new();

    // Get the class name without .class extension if present
    let class_name_without_ext = class_name.trim_end_matches(".class");

    // Try to load the class using classpath resolution
    if let Err(e) = interpreter.load_class_by_name(class_name_without_ext) {
        eprintln!("Error loading class '{}': {}", class_name_without_ext, e);
        process::exit(1);
    }

    // Run the main method
    if let Err(e) = interpreter.run_main(class_name_without_ext) {
        eprintln!("Error running main method: {}", e);
        process::exit(1);
    }
}
