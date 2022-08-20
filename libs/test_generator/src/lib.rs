use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse, LitStr};

#[proc_macro]
pub fn make_tests(input: TokenStream) -> TokenStream {
    let input = parse::<LitStr>(input).expect("Could not parse make_test! input.");
    let path = input.value();
    let files = glob::glob(format!("{}{}", path, "/*/*.lox").as_str()).unwrap();

    let mut vec: Vec<(String, String)> = Vec::new();
    for path in files {
        let path = path.unwrap();
        let file_name = path.file_stem().unwrap().to_str().unwrap();
        let dir_name = path
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();
        let path = path.to_str().unwrap().to_string();
        let function_name = format!("{}_{}", dir_name, file_name);
        vec.push((function_name, path));
    }

    let name = vec.iter().map(|s| format_ident!("{}", s.0.as_str()));
    let path = vec.iter().map(|s| s.1.as_str());

    let res = quote! {
        #(
            #[test]
            fn #name() {
               test_program(#path);
            }
        )*
    };

    proc_macro::TokenStream::from(res)
}
