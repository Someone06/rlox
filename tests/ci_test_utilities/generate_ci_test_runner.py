from glob import glob
from pathlib import Path

OUTPUT_FILE_PATH = "tests/crafting_interpreters_tests.rs"

test_template = """
#[test]
fn {}() {{
    test_program("{}");
}}
"""

tests = dict()
for file in glob("tests/files/crafting_interpreters_test_files/*/*.lox"):
    file = Path(file)
    file_name = file.stem
    dir_name = file.parent.name
    function_name = dir_name + "_" + file_name
    test = test_template.format(function_name, file)
    tests[function_name] = test

text = "mod ci_test_utilities;\n\nuse crate::ci_test_utilities::test_program;\n"
for key in sorted(tests):
   text += tests[key]

with open(OUTPUT_FILE_PATH, 'w') as file:
    file.write(text)