use proc_macro::TokenStream;

// create a std_derive macro for a function that adds code to the front. It is intended for the main function
#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = item.to_string();
    assert!(item.contains("fn main"), "main function not found");
    let item = item.replace("fn main", "pub extern \"C\" fn main");
    let code = r#"
        use std::*;
        core::arch::global_asm!(".section .init\n _start: j rust_entry");
        #[no_mangle]
        extern "C" fn rust_entry() -> ! {
            std::_init();
        }
        #[panic_handler]
        fn panic(info: &core::panic::PanicInfo) -> ! {
            std::_on_panic(info);
        }
        #[no_mangle]
    "#;

    let item = format!("{}{}", code, item);
    item.parse().unwrap()
}
