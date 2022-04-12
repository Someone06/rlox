# RLox - Implementing the Lox Programming Language in Rust

This repository contains a toy implementation of an interpreter for the Lox programming language.
Lox is an imperative, dynamically typed language design by Robert Nystrom for his book
[Crafting Interpreters](https://craftinginterpreters.com/).

## Implementation Status

The implementation currently supports expression, printing, global and local variables, control flow
and functions. Yet to implement are closures, classes and inheritance.

## Usage

    rlox <path-to-code-file>

## Implementation Notes

This implementation is essentially a port of the original
[C implementation of Lox](https://github.com/munificent/craftinginterpreters) to Rust. The purpose
of this implementation is for the author to check that he understood the material presented in the
book as well as to practice writing Rust code. Thus, this toy implementation values correctness and
safety over speed. This means unsafe code is avoid at the cost of speed and there are additional (
run-time) correctness checks that the C implementation does not include. An additional change that
this implementation uses reference counting (as opposed to garbage collection) for memory
management.