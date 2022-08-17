use std::path::{Path, PathBuf};

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref EXPECTED_OUTPUT_PATTERN: Regex =
        Regex::new(r"// expect: ?(?P<expected>.*)").unwrap();
    static ref EXPECTED_ERROR_PATTER: Regex = Regex::new(r"// (?P<error>Error.*)").unwrap();
    static ref EXPECTED_ERROR_PATTERN: Regex = Regex::new(r"// (?P<error>Error.*)").unwrap();
    static ref ERROR_LINE_PATTERN: Regex =
        Regex::new(r"// \[(c )?line (?P<line>\d+)\] (?P<error>Error.*)").unwrap();
    static ref EXPECTED_RUNTIME_ERROR_PATTERN: Regex =
        Regex::new(r"// expect runtime error: (?P<error>.+)").unwrap();
    static ref SYNTAX_ERROR_PATTERN: Regex =
        Regex::new(r"\[.*line (?P<line>\d+)\] (?P<error>Error.+)").unwrap();
    static ref STACK_TRACE_PATTERN: Regex = Regex::new(r"\[line (?P<line>\d+)\]").unwrap();
}

pub struct ExpectedOutput {
    line: usize,
    output: String,
}

impl ExpectedOutput {
    pub fn new(line: usize, output: String) -> Self {
        Self { line, output }
    }

    pub fn line_number(&self) -> usize {
        self.line
    }

    pub fn line(&self) -> &str {
        &self.output
    }
}

pub struct Test {
    path: PathBuf,
    expected_output: Vec<ExpectedOutput>,
    expected_errors: Vec<String>,
    expected_runtime_error: Option<ExpectedOutput>,
    expected_exit_code: u32,
    failures: Vec<String>,
}

#[derive(Debug)]
pub enum TestParseError {
    CannotReadFile,
    HasCompileAndRuntimeError,
}

impl Test {
    pub fn parse(path: PathBuf) -> Result<Self, TestParseError> {
        let mut test = Test {
            path,
            expected_output: vec![],
            expected_errors: vec![],
            expected_runtime_error: None,
            expected_exit_code: 0,
            failures: vec![],
        };

        let code =
            std::fs::read_to_string(&test.path).map_err(|_| TestParseError::CannotReadFile)?;

        for (line_number, line) in code.split('\n').enumerate() {
            if let Some(capture) = EXPECTED_OUTPUT_PATTERN.captures_iter(line).next() {
                test.expected_output.push(ExpectedOutput::new(
                    line_number,
                    capture["expected"].to_string(),
                ));
                continue;
            }

            if let Some(capture) = EXPECTED_ERROR_PATTERN.captures_iter(line).next() {
                test.expected_errors
                    .push(format!("[{}] {}", line_number, &capture["error"]));
                test.expected_exit_code = 65;
                continue;
            }

            if let Some(capture) = ERROR_LINE_PATTERN.captures_iter(line).next() {
                test.expected_errors
                    .push(format!("[{}] {}", &capture["line"], &capture["error"]));
                test.expected_exit_code = 65;
                continue;
            }

            if let Some(capture) = EXPECTED_RUNTIME_ERROR_PATTERN.captures_iter(line).next() {
                test.expected_runtime_error = Some(ExpectedOutput::new(
                    line_number,
                    capture["error"].to_string(),
                ));
                test.expected_exit_code = 70;
                continue;
            }

            if !test.expected_errors.is_empty() && test.expected_runtime_error.is_some() {
                eprintln!("Test cannot have expected compile error and expected runtime error!");
                return Err(TestParseError::HasCompileAndRuntimeError);
            }
        }

        Ok(test)
    }
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }
    pub fn expected_output(&self) -> &[ExpectedOutput] {
        &self.expected_output
    }
    pub fn expected_errors(&self) -> &[String] {
        &self.expected_errors
    }
    pub fn expected_runtime_error(&self) -> Option<&ExpectedOutput> {
        self.expected_runtime_error.as_ref()
    }
    pub fn expected_exit_code(&self) -> u32 {
        self.expected_exit_code
    }
    pub fn failures(&self) -> &[String] {
        &self.failures
    }
}

fn run_and_validate_test(test: &Test) {
    let (_, output) = rlox::run_program(
        test.path().to_str().unwrap(),
        Vec::<u8>::new(),
        Vec::<u8>::new(),
        Vec::<u8>::new(),
    );

    let (compiler_out, vm_out, vm_err) = output.decompose();
    let compiler_out = String::from_utf8(compiler_out)
        .unwrap()
        .lines()
        .map(String::from)
        .collect::<Vec<String>>();
    let vm_out = String::from_utf8(vm_out)
        .unwrap()
        .lines()
        .map(String::from)
        .collect::<Vec<String>>();
    let vm_err = String::from_utf8(vm_err)
        .unwrap()
        .lines()
        .map(String::from)
        .collect::<Vec<String>>();

    validate_compiler_errors(test, &compiler_out);
    validate_runtime_errors(test, &vm_err);
    validate_output(test, &vm_out);

    // TODO: Obtain exit code from VM.
    // validate_exit_code(test, _);
}

fn validate_runtime_errors(test: &Test, actual_runtime_error: &[String]) {
    if let Some(expected_runtime_error) = test.expected_runtime_error() {
        assert!(
            actual_runtime_error.len() >= 2,
            "Expected runtime error '{}' but got none.",
            expected_runtime_error.line()
        );
        assert_eq!(
            actual_runtime_error[0],
            expected_runtime_error.line(),
            "Expected runtime error '{}' but got '{}'",
            expected_runtime_error.line(),
            actual_runtime_error[0]
        );

        match actual_runtime_error[1..].iter().find_map(|line| {
            STACK_TRACE_PATTERN
                .captures_iter(line)
                .next()
                .map(|capture| capture["line"].parse::<usize>().unwrap())
        }) {
            Some(stack_trace_line) => assert_eq!(
                stack_trace_line,
                expected_runtime_error.line_number(),
                "Expected runtime error on line {} but was on line {}",
                expected_runtime_error.line_number(),
                stack_trace_line
            ),
            None => panic!(
                "Expected stack trace but got '{}'",
                actual_runtime_error[1..].concat()
            ),
        };
    }
}

fn validate_compiler_errors(test: &Test, actual_compiler_errors: &[String]) {
    if test.expected_runtime_error().is_some() {
        return;
    }

    let mut found_errors: Vec<String> = Vec::with_capacity(test.expected_errors().len());
    for line in actual_compiler_errors
        .iter()
        .filter(|line| !line.is_empty())
    {
        match SYNTAX_ERROR_PATTERN
            .captures_iter(line)
            .next()
            .map(|capture| {
                ExpectedOutput::new(
                    capture["line"].parse::<usize>().unwrap(),
                    capture["error"].to_string(),
                )
            }) {
            Some(actual_output) => {
                let found_error =
                    format!("[{}] {}", actual_output.line_number(), actual_output.line());
                assert!(
                    test.expected_errors().contains(&found_error),
                    "Unexpected error: '{}'",
                    line
                );
                found_errors.push(found_error);
            }
            None => panic!("Unexpected output on stderr: '{}'", line),
        }
    }

    for expected_error in test.expected_errors() {
        assert!(
            found_errors.contains(expected_error),
            "Missing expected error: '{}'",
            expected_error
        );
    }
}

fn validate_exit_code(test: &Test, actual_exit_code: u32) {
    assert_eq!(
        actual_exit_code,
        test.expected_exit_code(),
        "Expected return code '{}’ but got '{}'",
        test.expected_exit_code(),
        actual_exit_code
    );
}

fn validate_output(test: &Test, actual_output_lines: &[String]) {
    let actual = match actual_output_lines
        .last()
        .map_or(false, |line| line.is_empty())
    {
        true => &actual_output_lines[1..],
        false => actual_output_lines,
    };

    let expected = test.expected_output();
    assert_eq!(
        actual.len(),
        expected.len(),
        "Expected '{}' output lines but got '{}'.",
        expected.len(),
        actual.len()
    );
    for i in 0..expected.len() {
        let expected = &expected[i];
        let actual = &actual[i];
        assert_eq!(
            actual,
            expected.line(),
            "Expected output '{}' on line {} but got '{}'.",
            expected.line(),
            expected.line_number(),
            actual
        );
    }
}

pub fn test_program(path: &str) {
    let test = Test::parse(PathBuf::from(path)).unwrap();
    run_and_validate_test(&test);
}
