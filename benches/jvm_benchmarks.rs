use criterion::{black_box, criterion_group, criterion_main, Criterion};
use jvmrs::class_file::ClassFile;
use jvmrs::memory::{Heap, HeapArray, StackFrame, Value};
use jvmrs::reflection::class_to_reflection;
use jvmrs::{Interpreter, Memory};

/// Minimal valid class file bytes for parsing benchmark
fn minimal_class_bytes() -> Vec<u8> {
    vec![
        0xCA, 0xFE, 0xBA, 0xBE, // magic
        0x00, 0x00, // minor version
        0x00, 0x34, // major version (52 = Java 8)
        0x00, 0x03, // constant pool count
        0x01, 0x00, 0x05, 0x48, 0x65, 0x6C, 0x6C, 0x6F, // CP[1]: Utf8 "Hello"
        0x07, 0x00, 0x01, // CP[2]: Class name_index=1
        0x00, 0x21, // access_flags
        0x00, 0x02, // this_class
        0x00, 0x00, // super_class
        0x00, 0x00, // interfaces_count
        0x00, 0x00, // fields_count
        0x00, 0x00, // methods_count
        0x00, 0x00, // attributes_count
    ]
}

fn bench_class_file_parsing(c: &mut Criterion) {
    let minimal_class = minimal_class_bytes();
    c.bench_function("parse_class_file", |b| {
        b.iter(|| black_box(ClassFile::parse(&minimal_class)))
    });
}

fn bench_class_loading(c: &mut Criterion) {
    let mut interpreter = Interpreter::new();

    c.bench_function("load_hello_world_class", |b| {
        b.iter(|| {
            // Load HelloWorld class (assuming it exists in test data)
            let result = interpreter.load_class_by_name(black_box("HelloWorld"));
            black_box(result)
        })
    });
}

fn bench_memory_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");

    // Benchmark object allocation
    group.bench_function("allocate_object", |b| {
        let mut heap = Heap::new();
        b.iter(|| black_box(heap.allocate("java/lang/Object".to_string())))
    });

    // Benchmark string allocation
    group.bench_function("allocate_string", |b| {
        let mut heap = Heap::new();
        b.iter(|| black_box(heap.allocate_string("Hello, World!".to_string())))
    });

    // Benchmark array allocation
    group.bench_function("allocate_array", |b| {
        use jvmrs::memory::HeapArray;
        let mut heap = Heap::new();
        b.iter(|| {
            let array = HeapArray::IntArray(vec![0; 100]);
            black_box(heap.allocate_array(array))
        })
    });

    group.finish();
}

fn bench_stack_operations(c: &mut Criterion) {
    use jvmrs::memory::{StackFrame, Value};

    let mut group = c.benchmark_group("stack_operations");

    group.bench_function("push_int", |b| {
        let mut frame = StackFrame::new(100, 100, "test".to_string());
        b.iter(|| {
            black_box(frame.push(Value::Int(42)).unwrap());
            // Pop to prevent stack overflow
            black_box(frame.pop());
        })
    });

    group.bench_function("push_pop_int", |b| {
        let mut frame = StackFrame::new(100, 100, "test".to_string());
        b.iter(|| {
            black_box(frame.push(Value::Int(42)).unwrap());
            black_box(frame.pop());
        })
    });

    group.finish();
}

fn bench_reflection_api(c: &mut Criterion) {
    let mut group = c.benchmark_group("reflection");

    group.bench_function("class_to_reflection", |b| {
        let minimal_class = minimal_class_bytes();
        let class = ClassFile::parse(&minimal_class).unwrap();
        b.iter(|| black_box(class_to_reflection(&class)))
    });

    group.bench_function("get_class_reflection", |b| {
        let mut interpreter = Interpreter::new();
        interpreter.set_jit_enabled(false);
        // Load from examples/ if HelloWorld.class exists
        let _ = interpreter.load_class_by_name("HelloWorld");
        b.iter(|| black_box(interpreter.get_class_reflection("HelloWorld")))
    });

    group.finish();
}

fn bench_interpreter_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("interpreter");

    group.bench_function("create_interpreter", |b| {
        b.iter(|| black_box(Interpreter::new()))
    });

    group.bench_function("create_interpreter_no_jit", |b| {
        b.iter(|| {
            let mut i = Interpreter::new();
            i.set_jit_enabled(false);
            black_box(i)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_class_file_parsing,
    bench_class_loading,
    bench_memory_allocation,
    bench_stack_operations,
    bench_reflection_api,
    bench_interpreter_creation
);
criterion_main!(benches);
