use proc_macro::TokenStream;
use std::ops::Add;

use syn::__private::ToTokens;
use syn::ItemFn;

static mut TESTS: Vec<String> = Vec::new();
static mut CURR_MOD: String = String::new();

#[cfg(debug_assertions)]
#[proc_macro_attribute]
pub fn kernel_test(args: TokenStream, input: TokenStream) -> TokenStream {
    // Parse the input function and wrap it with the kernel test logic
    let input = syn::parse_macro_input!(input as ItemFn);
    //let args = args.to_string();
    let test_fn = input.sig.ident;
    let function_full_name = unsafe { format!("{CURR_MOD}::{test_fn}") };

    let test_body = input.block;
    let mut body_str = String::new();
    for stt in test_body.stmts {
        body_str = body_str.add(&stt.to_token_stream().to_string());
        body_str.push('\n');
    }

    let code = format!(
        r#"
        #[cfg(debug_assertions)]
        pub fn {test_fn}() {{
            {body_str}
        }}
    "#
    );

    unsafe {
        TESTS.push(function_full_name.to_string());
    }

    code.parse().expect("Generated invalid tokens")
}

#[proc_macro]
pub fn all_tests(_item: TokenStream) -> TokenStream {
    let mut code = "[".to_owned();

    unsafe {
        for test in &TESTS {
            code = code.add(&format!("({test} as fn(), \"{test}\"),"))
        }
    }

    code.push(']');

    code.parse().expect("Generated invalid tokens")
}

#[proc_macro]
pub fn kernel_test_mod(item: TokenStream) -> TokenStream {
    unsafe {
        CURR_MOD = item.to_string();
    }

    String::new().parse().expect("Generated invalid tokens")
}
