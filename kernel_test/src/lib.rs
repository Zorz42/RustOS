#![feature(proc_macro_quote)]

use proc_macro::{quote, TokenStream};

use syn::ItemFn;

#[cfg(debug_assertions)]
#[proc_macro_attribute]
pub fn kernel_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input function and wrap it with the kernel test logic
    let input = syn::parse_macro_input!(item as ItemFn);
    let test_fn = input.sig.ident;
    let test_body = input.block;

    let expanded = quote! {
        #[test]
        fn #test_fn() {
            // Setup code for kernel tests (e.g., initialize hardware, set up memory)
            // Call the actual test function
            #test_body
            // Tear down code for kernel tests (e.g., cleanup resources, reset hardware)
        }
    };
    expanded.into()
}
