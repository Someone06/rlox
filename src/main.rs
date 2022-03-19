use std::io::Read;

use crate::scanner::Scanner;

mod chunk;
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
                let _scanner = Scanner::new(chars.as_slice());
                println!("Success!");
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
