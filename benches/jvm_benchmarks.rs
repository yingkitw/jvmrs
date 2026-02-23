use criterion::{black_box, criterion_group, criterion_main, Criterion};
use jvmrs::memory::{Heap, HeapArray, StackFrame, Value};
use jvmrs::{Interpreter, Memory};

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

criterion_group!(
    benches,
    bench_class_loading,
    bench_memory_allocation,
    bench_stack_operations
);
criterion_main!(benches);
