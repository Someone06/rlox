use std::io::Read;

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

fn read_file(path: &str) -> Result<String, Error> {
    let path = std::path::Path::new(path);
    let mut file = std::fs::File::open(path).map_err(|_| Error::IO)?;
    let mut code = String::new();
    file.read_to_string(&mut code).map_err(|_| Error::IO)?;
    Ok(code)
}

#[derive(Debug)]
pub enum Error {
    IO,
    Compile,
    Run,
}

pub fn run_program(path: &str) -> Result<(), Error> {
    let file = read_file(path)?;
    let chars = file.chars().collect::<Vec<char>>();
    let scanner = Scanner::new(chars.as_slice());
    let compiler = Parser::new(scanner.parse());
    let (function, symbol_table) = compiler.compile().map_err(|_| Error::Compile)?;
    let mut vm = VM::new(function, symbol_table);
    vm.interpret().map_err(|_| Error::Run)?;
    Ok(())
}
