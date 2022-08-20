use std::io::Read;

use rlox::{run_program, Error};

fn read_file(path: &str) -> Result<String, Error> {
    let path = std::path::Path::new(path);
    let mut file = std::fs::File::open(path).map_err(|_| Error::IO)?;
    let mut code = String::new();
    file.read_to_string(&mut code).map_err(|_| Error::IO)?;
    Ok(code)
}

fn capture_program(file: &str) -> Result<String, Error> {
    match run_program(file, std::io::sink(), Vec::new(), std::io::sink()) {
        (Ok(_), out) => String::from_utf8(out.decompose().1).map_err(|_| Error::IO),
        (Err(error), _) => Err(error),
    }
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
        .join("\n")
        + "\n")
}

fn test_program(file: &str) -> Result<(), Error> {
    let path = "tests/files/system_test_files/".to_string() + file + ".lox";
    let output = capture_program(path.as_str())?;
    let expected = expected_result(path.as_str())?;
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

macro_rules! tests {
    ($($path:ident $(,)?)+) => {
        $(
            test!{$path}
        )+
    };
}

tests! {
    strings,
    shadowing,
    fib,
    logic,
    loops,
    make_counter,
    opclosure_capture_off_heap_value,
    closure_capture_variable,
    closure_test_opcloseupvalue,
    this,
    method_reference,
    op_invoke,
    super_method_call,
    super_get_closure,
    bin_search_tree
}
