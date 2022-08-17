use criterion::{criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};

fn run_program(file: &str) -> Result<(), rlox::Error> {
    rlox::run_program(file, std::io::sink(), std::io::sink(), std::io::sink()).0
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

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = criterion_benchmark
}
criterion_main!(benches);
