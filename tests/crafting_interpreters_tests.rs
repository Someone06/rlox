use test_generator::make_tests;

use crate::ci_test_utilities::test_program;

mod ci_test_utilities;

make_tests!("tests/files/crafting_interpreters_test_files");
