use proc_macro::{TokenStream};
use quote::quote;
use syn;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(Serial)]
pub fn serial_derive(input: TokenStream) -> TokenStream {
    let _input = parse_macro_input!(input as DeriveInput);
    let name = _input.ident;

    let data = if let syn::Data::Struct(data) = _input.data {
        data
    } else {
        unimplemented!();
    };

    let fields1 = data.fields.iter().map(|f| {
        let field_name = &f.ident;
        quote! {
            self.#field_name.serialize(vec);
        }
    });

    let fields2 = data.fields.iter().map(|f| {
        let field_name = &f.ident;
        let ty = &f.ty;
        quote! {
            #field_name: #ty::deserialize(vec, idx),
        }
    });

    let expanded = quote! (
        impl std::Serial for #name {
            fn serialize(&mut self, vec: &mut Vec<u8>) {
                #(#fields1)*
            }

            fn deserialize(vec: &Vec<u8>, idx: &mut usize) -> Self {
                Self {
                    #(#fields2)*
                }
            }
        }
    );

    expanded.into()
}