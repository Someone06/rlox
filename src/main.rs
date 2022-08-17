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

fn main() {
    let mut args = std::env::args();
    args.next();
    if let Some(input) = args.next() {
        if let Ok(code) = std::fs::read_to_string(input.clone()) {
            let chars = code.chars().collect::<Vec<char>>();
            let scanner = Scanner::new(chars.as_slice());
            let compiler = Parser::new(scanner.parse(), std::io::stderr());
            if let Ok((function, symbol_table, _)) = compiler.compile() {
                let vm = VM::new(function, symbol_table);
                let _ = vm.interpret();
            } else {
                println!("Compilation failed");
            }
        } else {
            println!("Cannot read file '{}'", input);
        }
    } else {
        println!("Usage: rlox <path-to-lox-file>")
    }
}
