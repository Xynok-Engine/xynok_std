use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

/// auto implement Debug, Display, Copy, Clone, Hash for the enum units
pub(crate) fn enum_extensions(_attr: TokenStream, item: TokenStream) -> TokenStream
{
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clauses) = generics.split_for_impl();

    let variants = match &input.data
    {
        Data::Enum(data_enum) => &data_enum.variants,
        _ => panic!("enum_extensions could be applied to enums only."),
    };

    for variant in variants
    {
        match &variant.fields
        {
            Fields::Unit =>
            {}
            _ => panic!("enum_extensions only supports unit variants"),
        }
    }

    let variant_idents: Vec<_> = variants.iter().map(|v| &v.ident).collect();
    let variant_count = variant_idents.len();

    let variant_names: Vec<String> = variant_idents.iter().map(|i| i.to_string()).collect();

    let uint_const_idents: Vec<syn::Ident> = variant_idents.iter().map(|ident| syn::Ident::new(&format!("{}_UINT", ident.to_string().to_uppercase()), ident.span())).collect();

    let str_const_idents: Vec<syn::Ident> = variant_idents.iter().map(|ident| syn::Ident::new(&format!("{}_STR", ident.to_string().to_uppercase()), ident.span())).collect();

    let expanded = quote! {

        // Attribute macro can reemit the whole enum with modifications
        #[repr(usize)]
        #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #input

        impl #impl_generics From<#name #ty_generics> for usize #where_clauses
        {
            fn from(value: #name #ty_generics) -> Self
            {
                match value
                {
                    #(#name::#variant_idents => #name::#variant_idents as usize,)*
                }
            }
        }

        impl #impl_generics From<#name #ty_generics> for u32 #where_clauses
        {
            fn from(value: #name #ty_generics) -> Self
            {
                match value
                {
                    #(#name::#variant_idents => #name::#variant_idents as u32,)*
                }
            }
        }


        impl #impl_generics From<#name #ty_generics> for String #where_clauses
        {
            fn from(value: #name #ty_generics) -> Self
            {
                match value
                {
                    #(#name::#variant_idents => String::from(#variant_names), )*
                }
            }
        }

        #[allow(dead_code)]
        impl #impl_generics #name #ty_generics #where_clauses
        {
            pub const TOTAL: usize = #variant_count;
            #(pub const #uint_const_idents: usize = #name::#variant_idents as usize;)*
            #(pub const #str_const_idents: &'static str = #variant_names;)*

            pub const UINT_ARR: [usize; #variant_count] = [
                #(#name::#variant_idents as usize),*
            ];
            pub const STR_ARR: [&'static str; #variant_count] = [#(#variant_names),*];

            pub fn to_uint(&self) -> usize {
                *self as usize
            }

            pub fn to_str(&self) -> &'static str {
                match self {
                    #(#name::#variant_idents => #variant_names,)*
                }
            }

            pub fn from_uint(value: usize) -> Option<Self> {
                match value {
                    #(x if x == #name::#variant_idents as usize => Some(#name::#variant_idents),)*
                    _ => None,
                }
            }

            pub fn from_str(value: &str) -> Option<Self> {
                match value {
                    #(#variant_names => Some(#name::#variant_idents),)*
                    _ => None,
                }
            }
        }
    };

    TokenStream::from(expanded)
}
