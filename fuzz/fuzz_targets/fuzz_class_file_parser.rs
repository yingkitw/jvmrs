//! Fuzz target for the JVMRS class file parser.
//! Run with: cargo fuzz run fuzz_class_file_parser

#![no_main]

use libfuzzer_sys::fuzz_target;
use jvmrs::class_file::ClassFile;

fuzz_target!(|data: &[u8]| {
    // Feed arbitrary bytes to the class file parser.
    // We expect it to never panic or UB - it should return Err for invalid input.
    let _ = ClassFile::parse(data);
});
