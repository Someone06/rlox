use std::process::ExitCode;

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

fn main() -> ExitCode {
    let mut args = std::env::args();
    args.next();
    if let Some(path) = args.next() {
        match run(&path) {
            Ok(_) => ExitCode::SUCCESS,
            Err(error) => ExitCode::from(error.get_error_code()),
        }
    } else {
        println!("Usage: rlox <path-to-lox-file>");
        ExitCode::from(64)
    }
}

fn run(path: &str) -> Result<(), rlox::Error> {
    rlox::run_program(
        path,
        std::io::stderr(),
        std::io::stdout(),
        std::io::stderr(),
    )
    .0
}
