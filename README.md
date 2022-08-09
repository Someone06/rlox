# RLox - Implementing the Lox Programming Language in Rust
This repository contains a toy implementation of an interpreter for the Lox
programming language. Lox is an imperative, dynamically typed language design by
Robert Nystrom for his book
[Crafting Interpreters](https://craftinginterpreters.com/).

## Implementation Status

The implementation currently supports all feature of Lox that are implemented in
the book. None of the optional features posed as challenges in the book (such as
`break` and `continue`) have been implemented.

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