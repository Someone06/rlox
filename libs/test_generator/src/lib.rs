fn input_to_glob_pattern(input: proc_macro::TokenStream) -> String {
    let mut input = syn::parse::<syn::LitStr>(input)
        .expect("Could not parse make_test! input.")
        .value();

    if !input.ends_with('/') {
        input.push('/');
    }
    input.push_str("*/*.lox");
    input
}

fn glob_to_function_name_and_path(path: &glob::GlobResult) -> (String, String) {
    let path = path
        .as_ref()
        .expect("Globbing the target directory should not fail.")
        .as_path();
    let file_name = path
        .file_stem()
        .expect("We only glob *.lox files, so there has to be a file step.")
        .to_str()
        .expect("Turning the OsString into a utf-8 string should not fail.");
    let dir_name = path
        .parent()
        .expect("We only glob sub-directoires, so there should be a parent directory.")
        .file_name()
        .expect("We globed the sub-directory of a sub-directory so, the path does not terminate in '..'.")
        .to_str()
        .expect("Turning the OsString into a utf-8 string should not fail.");
    let path = path
        .to_str()
        .expect("Turning the OsString into a utf-8 string should not fail.")
        .to_string();
    let function_name = format!("{}_{}", dir_name, file_name);
    (function_name, path)
}

fn glob_pattern_to_function_name_and_path(pattern: &str) -> Vec<(String, String)> {
    let mut functions = glob::glob(pattern)
        .expect("Glob pattern should be correct if the given input is a path.")
        .map(|glob_res| glob_to_function_name_and_path(&glob_res))
        .collect::<Vec<(String, String)>>();
    functions.sort();
    functions
}

#[proc_macro]
pub fn make_tests(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let pattern = input_to_glob_pattern(input);
    let functions = glob_pattern_to_function_name_and_path(pattern.as_str());

    let name = functions
        .iter()
        .map(|s| quote::format_ident!("{}", s.0.as_str()));
    let path = functions.iter().map(|s| s.1.as_str());

    let res = quote::quote! {
        #(
            #[test]
            fn #name() {
               test_program(#path);
            }
        )*
    };

    proc_macro::TokenStream::from(res)
}
