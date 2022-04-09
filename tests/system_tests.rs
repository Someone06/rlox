use rlox::{run_program, Error};
use std::io::Read;

fn read_file(path: &str) -> Result<String, Error> {
    let path = std::path::Path::new(path);
    let mut file = std::fs::File::open(path).map_err(|_| Error::IO)?;
    let mut code = String::new();
    file.read_to_string(&mut code).map_err(|_| Error::IO)?;
    Ok(code)
}

fn capture_program(file: &str) -> Result<String, Error> {
    let result = run_program(file, Vec::new())?;
    String::from_utf8(result).map_err(|_| Error::IO)
}

fn expected_result(path: &str) -> Result<String, Error> {
    Ok(read_file(path)?
        .lines()
        .take_while(|l| l.starts_with("//"))
        .map(String::from)
        .map(|mut l| {
            l.replace_range(0..2, "");
            l
        })
        .collect::<Vec<String>>()
        .join("\n"))
}

fn test_program(file: &str) -> Result<(), Error> {
    let path = "tests/files/".to_string() + file + ".lox";
    let output = dbg!(capture_program(path.as_str())?);
    let expected = dbg!(expected_result(path.as_str())?);
    assert_eq!(output, expected);
    Ok(())
}

macro_rules! test {
    ($path:ident) => {
        #[test]
        fn $path() -> Result<(), Error> {
            test_program(stringify!($path))
        }
    };
}

test! {fib}
