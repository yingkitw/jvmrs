mod class_file;
mod memory;
mod interpreter;

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

    // Try to load the class
    let class_path = if class_name.ends_with(".class") {
        class_name.clone()
    } else {
        format!("{}.class", class_name)
    };

    if let Err(e) = interpreter.load_class(&class_path) {
        eprintln!("Error loading class '{}': {}", class_path, e);
        process::exit(1);
    }

    // Get the actual class name from the class file
    let class_name_without_ext = class_name.trim_end_matches(".class");

    // Run the main method
    if let Err(e) = interpreter.run_main(class_name_without_ext) {
        eprintln!("Error running main method: {}", e);
        process::exit(1);
    }
}
