# RLox - Implementing the Lox Programming Language in Rust
This repository contains a toy implementation of an interpreter for the Lox
programming language. Lox is an imperative, dynamically typed language design by
Robert Nystrom for his book
[Crafting Interpreters](https://craftinginterpreters.com/).

## Implementation Status

The implementation currently supports all feature of Lox that are implemented in
the book. None of the optional features posed as challenges in the book (such as
`break` and `continue`) have been implemented.

## Build

    cargo build --release

## Usage

    rlox <path-to-code-file>

## Implementation Notes

This implementation is essentially a port of the original
[C implementation of Lox](https://github.com/munificent/craftinginterpreters) to
Rust. The purpose of this implementation is for the author to check that he
understood the material presented in the book as well as to practice writing
Rust code. Thus, this toy implementation values correctness and safety over
speed. This means unsafe code is avoid at the cost of speed and there are
additional (run-time) correctness checks that the C implementation does not
include. An additional change that this implementation uses reference counting
(as opposed to garbage collection) for memory management. This implies that if a
Lox script creates a cyclic dependency, then all values that are referenced by
the cyclic data will not be reclaimed by the interpreter.

# RLox Development

To aid the development of RLox, testing, benchmarking and profiling support has
been set up.

## Testing

RLox has three kinds of test:

1) Unit tests which are included in the same source file as the feature they
   test,
2) system tests, which test if a given Lox program compiles and produces the
   expected output, and
3) the original testsuite from the
   [Crafting Interpreter's GitHub repository](https://github.com/munificent/craftinginterpreters)
   which has been ported over to Rust.

All the test use Rust's default test framework.
The test methods of the system tests are generated using a `macro_rules!` macro,
the Crafting Interpreter tests are generated using a proc macro located in
`libs/test_generator`.

To run the tests use `cargo test`.

## Benchmarking and Profiling

The benchmarking and profiling support currently is just a skeleton which runs a
single Lox program, that computes Fibonacci numbers.

Benchmarking uses [`criterion`](https://docs.rs/criterion/latest/criterion/).
To run the benchmark run `cargo bench`.

Profiling is done by combining `criterion` with
[`pprof`](https://docs.rs/pprof/latest/pprof/) and generates a Flame-graph.
To run the profiling use `cargo bench --bench fib_bench -- --profile-time=5`.
The generated flame-graph is found in
`target/criterion/<name-of-benchmark>/flamegraph.svg`.

## Todo: Code Coverage

Rust's source-based code coverage relies on LLVM's coverage support.
The [Rustc book](https://doc.rust-lang.org/rustc/instrument-coverage.html)
describes how to use it.
However, at the time of writing, there seems to be no convenient method to
integrate code coverage reports with running the tests, so this is missing for
now.

## Todo: Profile-guided optimization (PGO)

Rust's code generation can make use of profile data to optimize the generated
code.
The
[Rustc Book](https://doc.rust-lang.org/rustc/profile-guided-optimization.html)
describes how to do that.
However, PGO is no magic bullet.
Before using PGO, benchmarking and profiling should be extended and used to find
performance bottlenecks.
Also, at the time of writing, there seems to be no convenient way of integrating
PGO with the usual building process.

