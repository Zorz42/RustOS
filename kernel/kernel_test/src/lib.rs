use proc_macro::TokenStream;
use std::ops::Add;
use std::sync::Mutex;
use syn::{ItemFn, ItemStruct};

static TESTS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static PERF_TESTS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static CURR_MOD: Mutex<String> = Mutex::new(String::new());

#[proc_macro_attribute]
pub fn kernel_test(_args: TokenStream, input: TokenStream) -> TokenStream {
    let function = input.to_string();

    let input_fn = syn::parse_macro_input!(input as ItemFn);
    let test_fn = input_fn.sig.ident;
    let function_full_name =  format!("{}::{}", CURR_MOD.lock().unwrap(), test_fn);

    let code = format!(
        r#"
        pub {function}
    "#
    );

    TESTS.lock().unwrap().push(function_full_name.to_string());

    code.parse().expect("Generated invalid tokens")
}

#[proc_macro_attribute]
pub fn kernel_perf(_args: TokenStream, input: TokenStream) -> TokenStream {
    let struct_ = input.to_string();

    let input_struct = syn::parse_macro_input!(input as ItemStruct);
    let test_struct = input_struct.ident;
    let struct_full_name = format!("{}::{}", CURR_MOD.lock().unwrap(), test_struct);

    let code = format!(
        r#"
        pub {struct_}
    "#
    );

    PERF_TESTS.lock().unwrap().push(struct_full_name.to_string());

    code.parse().expect("Generated invalid tokens")
}

#[proc_macro]
pub fn all_tests(_item: TokenStream) -> TokenStream {
    let mut code = "[".to_owned();

    for test in TESTS.lock().unwrap().iter() {
        let function_name = test.split(':').last().unwrap();
        code = code.add(&format!("({test} as fn(), \"{function_name}\"),"));
    }

    code.push(']');

    code.parse().expect("Generated invalid tokens")
}

#[proc_macro]
pub fn all_perf_tests(_item: TokenStream) -> TokenStream {
    let mut code = format!("println!(\"Running {} performance tests.\");", PERF_TESTS.lock().unwrap().len());

    for test in PERF_TESTS.lock().unwrap().iter() {
        let struct_name = test.split(':').last().unwrap();
        code = code.add(&format!("run_perf_test::<{test}>(\"{struct_name}\");"));
    }

    code.parse().expect("Generated invalid tokens")
}

#[proc_macro]
pub fn kernel_test_mod(item: TokenStream) -> TokenStream {
    *CURR_MOD.lock().unwrap() = item.to_string();

    String::new().parse().expect("Generated invalid tokens")
}
