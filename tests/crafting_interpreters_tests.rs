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
