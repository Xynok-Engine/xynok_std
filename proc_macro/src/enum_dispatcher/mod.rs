mod cache;

use cache::{UniqueItemId, cache_enum, cache_trait, defer_link, enums_waiting_for_trait};

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use std::collections::HashMap;
use syn::{
    FnArg, GenericArgument, GenericParam, Ident, ItemEnum, ItemTrait, Path, PathArguments, TraitItem, Type, WherePredicate,
    parse::{Parse, ParseStream},
    parse2,
    visit_mut::{self, VisitMut},
};

// ── Args parser ───────────────────────────────────────────────────────────────
// #[enum_dispatcher]                   → trait_path: None
// #[enum_dispatcher(Foo)]              → trait_path: Some("Foo")
// #[enum_dispatcher(Foo<T, U>)]        → trait_path: Some("Foo<T, U>")
// #[enum_dispatcher(Foo<f32, String>)] → trait_path: Some("Foo<f32, String>")
struct MacroArgs
{
    trait_path: Option<Path>,
}

impl Parse for MacroArgs
{
    fn parse(input: ParseStream) -> syn::Result<Self> { if input.is_empty() { Ok(MacroArgs { trait_path: None }) } else { Ok(MacroArgs { trait_path: Some(input.parse()?) }) } }
}

// ── Entry point ───────────────────────────────────────────────────────────────
pub(crate) fn enum_dispatcher(args: TokenStream, input: TokenStream) -> TokenStream
{
    let args2: TokenStream2 = args.into();
    let input2: TokenStream2 = input.into();

    let MacroArgs { trait_path } = match parse2::<MacroArgs>(args2)
    {
        Ok(a) => a,
        Err(e) => return e.to_compile_error().into(),
    };

    match trait_path
    {
        None => on_trait(input2),
        Some(path) => on_enum(path, input2),
    }
}

// ── #[enum_dispatcher] placed on the trait ───────────────────────────────────
fn on_trait(input: TokenStream2) -> TokenStream
{
    let trait_def = match parse2::<ItemTrait>(input)
    {
        Ok(t) => t,
        Err(e) => return e.to_compile_error().into(),
    };

    if let Err(e) = validate_trait_methods(&trait_def)
    {
        return e.to_compile_error().into();
    }

    cache_trait(&trait_def);

    // Any enums that arrived before the trait can now be resolved
    let waiting_enums = enums_waiting_for_trait(&trait_def.ident.to_string(), trait_def.generics.params.len());

    let deferred_impls = waiting_enums.iter().map(|(enum_def, trait_path)| generate_dispatch_impl(trait_path, &trait_def, enum_def));

    quote! {
        #trait_def
        #(#deferred_impls)*
    }
    .into()
}

// ── #[enum_dispatcher(Foo<T, U>)] placed on the enum ─────────────────────────
fn on_enum(trait_path: Path, input: TokenStream2) -> TokenStream
{
    let enum_def = match parse2::<ItemEnum>(input)
    {
        Ok(e) => e,
        Err(e) => return e.to_compile_error().into(),
    };

    // Extract base name + generic count from the path argument
    // "Foo<T, U>"  →  base = "Foo", num_generics = 2
    // "Foo<f32>"   →  base = "Foo", num_generics = 1
    // "Foo"        →  base = "Foo", num_generics = 0
    let last_seg = trait_path.segments.last().unwrap();
    let trait_base_name = last_seg.ident.to_string();
    let trait_num_generics = match &last_seg.arguments
    {
        PathArguments::AngleBracketed(a) => a.args.len(),
        _ => 0,
    };

    let enum_num_generics = enum_def.generics.params.len();

    // Validate that every type-parameter-looking argument in the trait path
    // (i.e. a single bare ident such as `T` or `U`) is actually declared as a
    // generic parameter on the enum.  Concrete types (f32, String, …) are fine
    // regardless.  This correctly accepts `Processor<T, T>` on `AudioEffect<T>`
    // while still rejecting `Processor<T, U>` on `AudioEffect<T>` (U undeclared).
    let enum_param_names: std::collections::HashSet<String> = enum_def.generics.params.iter().filter_map(|p| if let GenericParam::Type(tp) = p { Some(tp.ident.to_string()) } else { None }).collect();

    if let PathArguments::AngleBracketed(angle) = &trait_path.segments.last().unwrap().arguments
    {
        for arg in &angle.args
        {
            if let GenericArgument::Type(Type::Path(tp)) = arg
                && tp.qself.is_none()
                && tp.path.segments.len() == 1
            {
                let name = tp.path.segments[0].ident.to_string();
                // If it looks like a type parameter (single uppercase-ish ident)
                // and is NOT declared on the enum, that is an error.
                if !enum_param_names.contains(&name) && enum_num_generics > 0
                {
                    return syn::Error::new_spanned(
                        &trait_path,
                        format!(
                            "trait path argument `{}` is not a generic parameter of enum `{}` \
                                 — either declare it on the enum or use a concrete type",
                            name, enum_def.ident,
                        ),
                    )
                    .to_compile_error()
                    .into();
                }
            }
        }
    }

    cache_enum(&enum_def);

    // Trait already cached → generate impl immediately
    if let Some(trait_def) = cache::get_trait(&trait_base_name, trait_num_generics)
    {
        let impl_block = generate_dispatch_impl(&trait_path, &trait_def, &enum_def);
        return quote! {
            #enum_def
            #impl_block
        }
        .into();
    }

    // Trait not cached yet → register a deferred link, preserving the original
    // trait path string so it can be reconstructed exactly during resolution.
    let trait_path_str = quote::quote!(#trait_path).to_string();
    defer_link(UniqueItemId::new(trait_base_name, trait_num_generics), UniqueItemId::new(enum_def.ident.to_string(), enum_num_generics), trait_path_str);

    quote!(#enum_def).into()
}

// ── Validation ────────────────────────────────────────────────────────────────
fn validate_trait_methods(trait_def: &ItemTrait) -> Result<(), syn::Error>
{
    for item in &trait_def.items
    {
        let TraitItem::Fn(method) = item
        else
        {
            continue;
        };

        // 1. Must have a self receiver
        let has_self = method.sig.inputs.iter().any(|a| matches!(a, FnArg::Receiver(_)));
        if !has_self
        {
            return Err(syn::Error::new_spanned(
                &method.sig,
                format!(
                    "method `{}` must have a self receiver (`self`, `&self`, or `&mut self`) — \
                     static methods cannot be dispatched",
                    method.sig.ident
                ),
            ));
        }

        // 2. Must NOT have method-level generics (breaks object safety)
        if !method.sig.generics.params.is_empty()
        {
            return Err(syn::Error::new_spanned(
                &method.sig.generics,
                format!(
                    "method `{}` has generic parameters — method-level generics break object \
                     safety. Move generics to the trait level: `trait Foo<T>` not `fn bar<T>()`",
                    method.sig.ident
                ),
            ));
        }

        // 3. Must NOT have method-level where clauses on generic params
        if let Some(wc) = &method.sig.generics.where_clause
            && !wc.predicates.is_empty()
        {
            return Err(syn::Error::new_spanned(
                wc,
                format!(
                    "method `{}` has a where clause — method-level where clauses break \
                         object safety",
                    method.sig.ident
                ),
            ));
        }

        // 4. Must NOT return bare `Self`
        if let syn::ReturnType::Type(_, ty) = &method.sig.output
            && is_bare_self(ty)
        {
            return Err(syn::Error::new_spanned(ty, format!("method `{}` returns `Self` — returning `Self` breaks object safety", method.sig.ident)));
        }
    }

    Ok(())
}

fn is_bare_self(ty: &Type) -> bool { if let Type::Path(tp) = ty { tp.qself.is_none() && tp.path.segments.len() == 1 && tp.path.segments[0].ident == "Self" } else { false } }

// ── Generic substituter ───────────────────────────────────────────────────────
// Used ONLY when the enum is non-generic and the trait path has concrete args.
// e.g. trait Foo<T>  +  #[enum_dispatcher(Foo<f32>)]  +  enum Bar (no generics)
//      → substitutes T → f32 in method signatures
struct GenericSubstituter
{
    map: HashMap<String, Type>,
}

impl VisitMut for GenericSubstituter
{
    fn visit_type_mut(&mut self, ty: &mut Type)
    {
        if let Type::Path(type_path) = ty
            && type_path.qself.is_none()
            && type_path.path.segments.len() == 1
        {
            let ident = type_path.path.segments[0].ident.to_string();
            if let Some(concrete) = self.map.get(&ident)
            {
                *ty = concrete.clone();
                return;
            }
        }
        visit_mut::visit_type_mut(self, ty);
    }
}

// ── Build T → concrete type substitution map ─────────────────────────────────
// Only relevant when enum is non-generic (concrete dispatch).
// trait Foo<T, U>  +  path Foo<f32, String>  →  { "T": f32, "U": String }
fn build_substitution_map(trait_def: &ItemTrait, trait_path: &Path) -> HashMap<String, Type>
{
    let mut map = HashMap::new();

    let trait_params: Vec<&Ident> = trait_def.generics.params.iter().filter_map(|p| if let GenericParam::Type(tp) = p { Some(&tp.ident) } else { None }).collect();

    let concrete_args: Vec<Type> = match &trait_path.segments.last().unwrap().arguments
    {
        PathArguments::AngleBracketed(args) => args.args.iter().filter_map(|a| if let GenericArgument::Type(ty) = a { Some(ty.clone()) } else { None }).collect(),
        _ => vec![],
    };

    for (param, concrete) in trait_params.iter().zip(concrete_args.iter())
    {
        map.insert(param.to_string(), concrete.clone());
    }

    map
}

// ── Core codegen ──────────────────────────────────────────────────────────────
fn generate_dispatch_impl(
    trait_path: &Path, // e.g. Foo<T, U>  or  Foo<f32, String>
    trait_def: &ItemTrait,
    enum_def: &ItemEnum,
) -> TokenStream2
{
    let enum_name = &enum_def.ident;

    // Collect enum variant idents (only tuple variants supported)
    let variants: Vec<&Ident> = enum_def.variants.iter().map(|v| &v.ident).collect();

    // Collect the inner type of each tuple variant for where bounds.
    // e.g. Scale(ScaleProcessor<T>) → ScaleProcessor<T>
    let variant_inner_types: Vec<&Type> = enum_def.variants.iter().filter_map(|v| if let syn::Fields::Unnamed(fields) = &v.fields { fields.unnamed.first().map(|f| &f.ty) } else { None }).collect();

    // ── Build substitution map ─────────────────────────────────────────────
    // Always build trait-param → path-arg map, e.g.:
    //   Processor<T, U>  +  path Processor<T, T>    →  { T→T, U→T }  (rewrites U)
    //   Processor<T, U>  +  path Processor<T, U>    →  { T→T, U→U }  (no-op)
    //   Processor<T, U>  +  path Processor<f32,f32> →  { T→f32, U→f32 }
    // This handles all three cases uniformly: repeated params, identity, concrete.
    let path_has_args = matches!(
        &trait_path.segments.last().unwrap().arguments,
        PathArguments::AngleBracketed(a) if !a.args.is_empty()
    );
    let subst_map = if path_has_args { build_substitution_map(trait_def, trait_path) } else { HashMap::new() };
    let needs_substitution = !subst_map.is_empty();
    let mut substituter = GenericSubstituter { map: subst_map };

    // ── Build impl methods ────────────────────────────────────────────────
    let impl_methods = trait_def.items.iter().filter_map(|item| {
        let TraitItem::Fn(method) = item
        else
        {
            return None;
        };

        let mut sig = method.sig.clone();
        if needs_substitution
        {
            substituter.visit_signature_mut(&mut sig);
        }

        let method_name = &sig.ident;

        let call_args: Vec<_> = sig.inputs.iter().filter_map(|arg| if let FnArg::Typed(pt) = arg { Some(&pt.pat) } else { None }).collect();

        let arms = variants.iter().map(|variant| {
            quote! {
                #enum_name::#variant(inner) => inner.#method_name(#(#call_args),*),
            }
        });

        Some(quote! {
            #sig {
                match self {
                    #(#arms)*
                }
            }
        })
    });

    // ── Build impl generics ───────────────────────────────────────────────
    // Use the enum's own generics for the impl block:
    //   impl<T, U> Foo<T, U> for Bar<T, U>
    //   impl<T: Clone, U: Hash> Foo<T, U> for Bar<T, U>
    let (impl_generics, ty_generics, existing_where) = enum_def.generics.split_for_impl();

    // For each variant inner type emit:  InnerType: TraitPath
    // e.g. ScaleProcessor<T>: Processor<T, T>
    // This is required when the enum is generic — the compiler needs proof that
    // the inner types actually implement the trait for the generic T.
    let existing_predicates: Vec<&WherePredicate> = existing_where.map(|wc| wc.predicates.iter().collect()).unwrap_or_default();

    let where_clause = quote! {
        where
            #(#variant_inner_types: #trait_path,)*
            #(#existing_predicates,)*
    };

    quote! {
        impl #impl_generics #trait_path for #enum_name #ty_generics #where_clause {
            #(#impl_methods)*
        }
    }
}
