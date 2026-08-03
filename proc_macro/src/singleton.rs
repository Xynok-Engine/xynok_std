use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, ItemStruct};

pub fn singleton_gen(_attr: TokenStream, item: TokenStream) -> TokenStream
{
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let static_name = format_ident!("{}_INSTANCE", name.to_string().to_uppercase());

    let output = quote! {
        // Re-emit the original struct unchanged
        #input

        // Add singleton machinery
        static #static_name: std::sync::OnceLock<std::sync::Mutex<#name>>
            = std::sync::OnceLock::new();

        impl #name {
            pub fn instance() -> std::sync::MutexGuard<'static, #name> {
                #static_name
                    .get_or_init(|| std::sync::Mutex::new(#name::default()))
                    .lock()
                    .unwrap()
            }
        }
    };
    TokenStream::from(output)
}

pub fn singleton_readonly_gen(_attr: TokenStream, item: TokenStream) -> TokenStream
{
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let static_name = format_ident!("{}_INSTANCE", name.to_string().to_uppercase());

    let output = quote! {
        #input

        static #static_name: std::sync::OnceLock<#name> = std::sync::OnceLock::new();

        impl #name {
            pub fn instance() -> &'static #name {
                #static_name.get_or_init(#name::default)
            }
        }
    };
    TokenStream::from(output)
}
