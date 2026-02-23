use criterion::{black_box, criterion_group, criterion_main, Criterion};
use jvmrs::memory::{StackFrame, Value};
use jvmrs::{Interpreter, Memory};

fn bench_arithmetic_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("arithmetic_operations");

    // Benchmark integer addition
    group.bench_function("iadd", |b| {
        let mut frame = StackFrame::new(10, 10, "test".to_string());
        b.iter(|| {
            // Setup stack with two integers
            frame.push(Value::Int(black_box(10))).unwrap();
            frame.push(Value::Int(black_box(20))).unwrap();

            // Perform addition (simplified)
            let b = frame.pop().unwrap();
            let a = frame.pop().unwrap();
            let result = Value::Int(match (a, b) {
                (Value::Int(x), Value::Int(y)) => x + y,
                _ => 0,
            });
            black_box(frame.push(result));
        })
    });

    // Benchmark integer multiplication
    group.bench_function("imul", |b| {
        let mut frame = StackFrame::new(10, 10, "test".to_string());
        b.iter(|| {
            frame.push(Value::Int(black_box(10))).unwrap();
            frame.push(Value::Int(black_box(20))).unwrap();

            let b = frame.pop().unwrap();
            let a = frame.pop().unwrap();
            let result = Value::Int(match (a, b) {
                (Value::Int(x), Value::Int(y)) => x * y,
                _ => 0,
            });
            black_box(frame.push(result));
        })
    });

    group.finish();
}

fn bench_memory_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_access");

    // Benchmark field access
    group.bench_function("field_get", |b| {
        let mut memory = Memory::new();
        let addr = memory.heap.allocate("TestClass".to_string());

        // Set a field value
        if let Some(obj) = memory.heap.get_object_mut(addr) {
            obj.fields.insert("testField".to_string(), Value::Int(42));
        }

        b.iter(|| {
            if let Some(obj) = memory.heap.get_object(black_box(addr)) {
                black_box(obj.fields.get("testField"));
            }
        })
    });

    // Benchmark field set
    group.bench_function("field_set", |b| {
        let mut memory = Memory::new();
        let addr = memory.heap.allocate("TestClass".to_string());

        b.iter(|| {
            if let Some(obj) = memory.heap.get_object_mut(black_box(addr)) {
                obj.fields
                    .insert("testField".to_string(), Value::Int(black_box(42)));
            }
        })
    });

    group.finish();
}

fn bench_static_field_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("static_field_access");

    group.bench_function("static_field_get", |b| {
        let mut memory = Memory::new();
        memory.set_static(
            "TestClass".to_string(),
            "staticField".to_string(),
            Value::Int(100),
        );

        b.iter(|| {
            black_box(memory.get_static("TestClass", "staticField"));
        })
    });

    group.bench_function("static_field_set", |b| {
        let mut memory = Memory::new();

        b.iter(|| {
            memory.set_static(
                "TestClass".to_string(),
                "staticField".to_string(),
                Value::Int(black_box(42)),
            );
        })
    });

    group.finish();
}

fn bench_instruction_dispatch(c: &mut Criterion) {
    let mut group = c.benchmark_group("instruction_dispatch");

    // Benchmark simple instruction matching
    group.bench_function("opcode_match", |b| {
        b.iter(|| {
            let opcode = black_box(0x60); // iadd
            match opcode {
                0x60 => black_box("iadd"),
                0x61 => black_box("ladd"),
                0x62 => black_box("fadd"),
                0x63 => black_box("dadd"),
                0x64 => black_box("isub"),
                0x65 => black_box("lsub"),
                0x66 => black_box("fsub"),
                0x67 => black_box("dsub"),
                0x68 => black_box("imul"),
                0x69 => black_box("lmul"),
                _ => black_box("unknown"),
            }
        })
    });

    group.finish();
}

fn bench_method_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("method_execution");

    // Benchmark method call overhead
    group.bench_function("method_call_overhead", |b| {
        let mut interpreter = Interpreter::new();

        // Simple benchmark to measure method call overhead
        b.iter(|| {
            // This would normally invoke a method, but for benchmarking we just
            // simulate the overhead of setting up and tearing down a stack frame
            let mut frame = StackFrame::new(10, 10, "test".to_string());
            frame.locals[0] = Value::Int(42);
            black_box(frame.locals[0].clone());
        })
    });

    group.finish();
}

criterion_group!(
    instruction_benches,
    bench_arithmetic_operations,
    bench_memory_access,
    bench_static_field_access,
    bench_instruction_dispatch,
    bench_method_execution
);
criterion_main!(instruction_benches);
