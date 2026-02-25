//! Build script: optionally generate Rust bindings from .class files at compile time.
//!
//! Set JVMRS_BINDGEN_CLASSES to a comma-separated list of class paths to generate bindings.
//! Example: JVMRS_BINDGEN_CLASSES=com.example.Foo,com.example.Bar

use std::env;
use std::path::Path;

fn main() {
    let classes = env::var("JVMRS_BINDGEN_CLASSES").ok();
    let out_dir = env::var("OUT_DIR").unwrap();

    if let Some(list) = classes {
        for class_name in list.split(',').map(|s| s.trim()) {
            if class_name.is_empty() {
                continue;
            }
            let path = format!("{}.class", class_name.replace('.', "/"));
            if Path::new(&path).exists() {
                println!("cargo:rerun-if-changed={}", path);
                // TODO: parse class, emit Rust bindings to out_dir
                // For now, this is a placeholder
                let _ = out_dir;
            }
        }
    }

    println!("cargo:rerun-if-env-changed=JVMRS_BINDGEN_CLASSES");
}
