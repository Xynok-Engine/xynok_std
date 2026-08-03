use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Fields, Ident, ItemStruct, Meta};

pub fn getter(_attr: TokenStream, item: TokenStream) -> TokenStream
{
    let mut input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let generics = input.generics.clone();
    let (impl_generics, ty_generics, where_clauses) = generics.split_for_impl();

    let mut methods = Vec::new();

    if let Fields::Named(ref mut fields) = input.fields
    {
        for field in fields.named.iter_mut()
        {
            let field_ident = field.ident.as_ref().expect("named field has ident").clone();
            let field_ty = field.ty.clone();

            let mut kept = Vec::with_capacity(field.attrs.len());
            for attr in field.attrs.drain(..)
            {
                let path = attr.path();
                let is_get = path.is_ident("get");
                let is_get_mut = path.is_ident("get_mut");

                if !is_get && !is_get_mut
                {
                    kept.push(attr);
                    continue;
                }

                let method_name: Ident = match &attr.meta
                {
                    Meta::Path(_) => field_ident.clone(),
                    Meta::List(list) => syn::parse2::<Ident>(list.tokens.clone()).expect("expected a single identifier inside #[get(...)] / #[get_mut(...)]"),
                    Meta::NameValue(_) => panic!("#[get]/#[get_mut] does not accept name=value form"),
                };

                if is_get
                {
                    methods.push(quote! {
                        pub fn #method_name(&self) -> &#field_ty {
                            &self.#field_ident
                        }
                    });
                }
                else
                {
                    methods.push(quote! {
                        pub fn #method_name(&mut self) -> &mut #field_ty {
                            &mut self.#field_ident
                        }
                    });
                }
            }
            field.attrs = kept;
        }
    }
    else
    {
        panic!("#[getter] only supports structs with named fields");
    }

    let expanded = quote! {
        #input

        #[allow(dead_code)]
        impl #impl_generics #name #ty_generics #where_clauses
        {
            #(#methods)*
        }
    };

    TokenStream::from(expanded)
}
