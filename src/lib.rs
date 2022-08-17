use std::io::Write;

use crate::compile::Parser;
use crate::scanner::Scanner;
use crate::vm::VM;

mod chunk;
mod classes;
mod compile;
mod function;
mod intern_string;
mod opcodes;
mod scanner;
mod tokens;
mod value;
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

pub struct Output<C: Write, VO: Write, VE: Write> {
    compiler_output: C,
    vm_out: VO,
    vm_err: VE,
}

impl<C: Write, VO: Write, VE: Write> Output<C, VO, VE> {
    pub fn new(compiler_output: C, vm_out: VO, vm_err: VE) -> Self {
        Self {
            compiler_output,
            vm_out,
            vm_err,
        }
    }

    pub fn decompose(self) -> (C, VO, VE) {
        (self.compiler_output, self.vm_out, self.vm_err)
    }
}

pub fn run_program<C: Write, VO: Write, VE: Write>(
    path: &str,
    compiler_output: C,
    vm_output: VO,
    vm_err: VE,
) -> (Result<(), Error>, Output<C, VO, VE>) {
    if let Ok(file) = read_file(path) {
        let chars = file.chars().collect::<Vec<char>>();
        let scanner = Scanner::new(chars.as_slice());
        let compiler = Parser::new(scanner.parse(), compiler_output);
        let compiler_res = compiler.compile();
        match compiler_res {
            Ok((function, symbol_table, compiler_out)) => {
                let vm = VM::with_write(function, symbol_table, vm_output, vm_err);
                match vm.interpret() {
                    Ok((vm_out, vm_err)) => (Ok(()), Output::new(compiler_out, vm_out, vm_err)),
                    Err((_, vm_out, vm_err)) => {
                        (Err(Error::Run), Output::new(compiler_out, vm_out, vm_err))
                    }
                }
            }
            Err(compiler_out) => (
                Err(Error::Compile),
                Output::new(compiler_out, vm_output, vm_err),
            ),
        }
    } else {
        (
            Err(Error::IO),
            Output::new(compiler_output, vm_output, vm_err),
        )
    }
}
