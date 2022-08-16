use std::io::sink;

use criterion::{criterion_group, criterion_main, Criterion};

use rlox::{run_program as run, Error};

fn run_program(file: &str) -> Result<(), Error> {
    run(file, sink()).map(|_| ())
}

fn run_fib() {
    let result = run_program("benches/files/fib.lox");
    if let Err(error) = result {
        eprintln!("{:?}", error);
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("fib", |b| b.iter(run_fib));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
