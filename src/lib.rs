use std::io::Write;

use crate::compile::Parser;
use crate::scanner::Scanner;
use crate::vm::VM;

mod chunk;
mod compile;
mod function;
mod intern_string;
mod scanner;
mod tokens;
mod vm;

#[derive(Debug)]
pub enum Error {
    IO,
    Compile,
    Run,
}

fn read_file(path: &str) -> Result<String, Error> {
    std::fs::read_to_string(path).map_err(|_| Error::IO)
}

pub fn run_program<W: Write>(path: &str, write: W) -> Result<W, Error> {
    let file = read_file(path)?;
    let chars = file.chars().collect::<Vec<char>>();
    let scanner = Scanner::new(chars.as_slice());
    let compiler = Parser::new(scanner.parse());
    let (function, symbol_table) = compiler.compile().map_err(|_| Error::Compile)?;
    let vm = VM::with_write(function, symbol_table, write);
    vm.interpret().map_err(|_| Error::Run)
}
