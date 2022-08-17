from glob import glob
from pathlib import Path
from sys import exit

OUTPUT_FILE_PATH = Path("tests/crafting_interpreters_test_runner.rs")
INPUT_TESTS_FILE = Path"tests/crafting_interpreters_tests.rs")

if not OUTPUT_FILE_PATH.is_file() or not INPUT_TESTS_FILE.is_file:
    exit()

text = ""
with open(INPUT_TESTS_FILE, 'r') as f:
    text = f.read()

for file in glob("tests/crafting_interpreters_tests/*/*.lox"):
    file = Path(file)
    file_name = file.stem
    dir_name = file.parent.name
    function_name = dir_name + "_" + file_name
    test = """
#[test]
fn {}() {{
    test_program("{}");
}}
    """
    test = test.format(function_name, file)
    text += test

with open(OUTPUT_FILE_PATH, 'w') as file:
    file.write(text)