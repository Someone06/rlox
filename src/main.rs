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

fn main() {
    let mut args = std::env::args();
    if let Some(input) = args.next() {
        let path = std::path::Path::new(&input);
        if let Ok(mut file) = std::fs::File::open(path) {
            let mut code = String::new();
            if file.read_to_string(&mut code).is_ok() {
                let chars = code.chars().collect::<Vec<char>>();
                let scanner = Scanner::new(chars.as_slice());
                let compiler = Parser::new(scanner.parse());
                if let Ok((function, symbol_table)) = compiler.compile() {
                    let vm = VM::new(function, symbol_table);
                    if vm.interpret().is_ok() {
                        println!("Success.");
                    } else {
                        println!("Vm error failed.");
                    }
                } else {
                    println!("Compilation failed.");
                }
            } else {
                println!("Cannot read file.");
            }
        } else {
            println!("Cannot open file.");
        }
    } else {
        println!("Usage: rlox <path-to-lox-file>")
    }
}
