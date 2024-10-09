use proc_macro::TokenStream;
use std::ops::Add;

use syn::{ItemFn, ItemStruct};

static mut TESTS: Vec<String> = Vec::new();
static mut PERF_TESTS: Vec<String> = Vec::new();
static mut CURR_MOD: String = String::new();

#[proc_macro_attribute]
pub fn kernel_test(_args: TokenStream, input: TokenStream) -> TokenStream {
    let function = input.to_string();

    let input_fn = syn::parse_macro_input!(input as ItemFn);
    let test_fn = input_fn.sig.ident;
    let function_full_name =  unsafe { format!("{CURR_MOD}::{test_fn}") };

    let code = format!(
        r#"
        pub {function}
    "#
    );

    unsafe {
        TESTS.push(function_full_name.to_string());
    }

    code.parse().expect("Generated invalid tokens")
}

#[proc_macro_attribute]
pub fn kernel_perf(_args: TokenStream, input: TokenStream) -> TokenStream {
    let struct_ = input.to_string();

    let input_struct = syn::parse_macro_input!(input as ItemStruct);
    let test_struct = input_struct.ident;
    let struct_full_name = unsafe { format!("{CURR_MOD}::{test_struct}") };

    let code = format!(
        r#"
        pub {struct_}
    "#
    );

    unsafe {
        PERF_TESTS.push(struct_full_name.to_string());
    }

    code.parse().expect("Generated invalid tokens")
}

#[proc_macro]
pub fn all_tests(_item: TokenStream) -> TokenStream {
    let mut code = "[".to_owned();

    unsafe {
        for test in TESTS.iter() {
            let function_name = test.split(':').last().unwrap();
            code = code.add(&format!("({test} as fn(), \"{function_name}\"),"));
        }
    }

    code.push(']');

    code.parse().expect("Generated invalid tokens")
}

#[proc_macro]
pub fn all_perf_tests(_item: TokenStream) -> TokenStream {
    let mut code = format!("println!(\"Running {} performance tests.\");", unsafe { PERF_TESTS.len() });

    unsafe {
        for test in PERF_TESTS.iter() {
            let struct_name = test.split(':').last().unwrap();
            code = code.add(&format!("run_perf_test::<{test}>(\"{struct_name}\");"));
        }
    }

    code.parse().expect("Generated invalid tokens")
}

#[proc_macro]
pub fn kernel_test_mod(item: TokenStream) -> TokenStream {
    unsafe {
        CURR_MOD = item.to_string();
    }

    String::new().parse().expect("Generated invalid tokens")
}
