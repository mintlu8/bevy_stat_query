use proc_macro::{Span, TokenStream as TokenStream1};
use proc_macro_error::{abort, proc_macro_error};
use quote::{quote, ToTokens};
use syn::{parse_macro_input, spanned::Spanned, DeriveInput, Fields, LitInt, LitStr, Type};

/// Derive macro for `Stat`.
///
/// # Syntax
///
/// The macro works for unit structs and fieldless enums
/// with unsigned `repr`.
///
/// ```
/// #[derive(Debug, Clone, Copy, Stat)]
/// #[stat(value = "StatIntPercentAdditive<i32>")]
/// pub struct MyStat;
/// ```
///
/// or
///
/// ```
/// #[derive(Debug, Clone, Copy, Stat)]
/// #[stat(value = "StatIntPercentAdditive<i32>")]
/// pub enum MyStat {
///     #[default] A,
///     B,
///     C,
/// }
/// ```
///
/// * `#[default]`
///
/// If specified, guarantees no panic even if a bad id
/// is encountered, this likely will not happen in normal usage,
/// as id is not used in serialization.
#[proc_macro_error]
#[proc_macro_derive(Stat, attributes(stat, default))]
pub fn stat(tokens: TokenStream1) -> TokenStream1 {
    let input = parse_macro_input!(tokens as DeriveInput);
    let crate0 = quote! {::bevy_stat_query};
    let name = input.ident;

    let mut value = None;

    for attr in input.attrs {
        if !attr.path().is_ident("stat") {
            continue;
        }
        let _ = attr.parse_nested_meta(|parse| {
            if !parse.path.is_ident("value") {
                return Ok(());
            }
            let Ok(s) = parse.value()?.parse::<LitStr>() else {
                abort!(parse.path.span(), "Expected #[stat(value = \"StatValue\")]")
            };
            value = match s.parse::<Type>() {
                Ok(v) => Some(v),
                Err(e) => abort!(s.span(), "{}", e),
            };
            Ok(())
        });
    }

    let Some(value) = value else {
        abort!(Span::call_site(), "Expected #[stat(value = \"StatValue\")]")
    };

    match input.data {
        syn::Data::Struct(s) => {
            let Fields::Unit = s.fields else {
                abort!(s.struct_token.span, "Only supports unit structs and enums.");
            };
            quote! {
                impl #crate0::Stat for #name {
                    type Value = #value;

                    fn name(&self) -> &'static str {
                        stringify!(#name)
                    }

                    fn vtable() -> &'static #crate0::StatVTable<Self> {
                        #crate0::vtable!(#name)
                    }

                    fn as_index(&self) -> u64 {
                        0
                    }

                    fn from_index(_: u64) -> Self {
                        #name
                    }

                    fn values() -> impl IntoIterator<Item = Self> {
                        [#name]
                    }
                }
            }
            .into()
        }
        syn::Data::Enum(e) => {
            let mut default = quote! {
                panic!("Invalid value for {}: {}.", stringify!(#name), value)
            };
            for v in &e.variants {
                let variant = &v.ident;
                if !matches!(v.fields, Fields::Unit) {
                    abort!(v.span(), "Only fieldless enums are supported.")
                }
                for attr in &v.attrs {
                    if attr.path().is_ident("default") {
                        default = quote! {#name::#variant}
                    }
                }
            }
            let names = e.variants.iter().map(|x| &x.ident);
            let names2 = e.variants.iter().map(|x| &x.ident);
            let names3 = e.variants.iter().map(|x| &x.ident);
            let names4 = e.variants.iter().map(|x| &x.ident);
            let mut last = 0u64;
            let indices: Vec<_> = e
                .variants
                .iter()
                .map(|x| match &x.discriminant {
                    None => {
                        last += 1;
                        last
                    }
                    Some((_, expr)) => {
                        let Ok(lit) = syn::parse2::<LitInt>(expr.into_token_stream()) else {
                            abort!(expr.span(), "Expected a number");
                        };
                        let Ok(num) = lit.base10_parse::<u64>() else {
                            abort!(expr.span(), "Expected unsigned number");
                        };
                        last = num;
                        num
                    }
                })
                .collect();

            quote! {
                impl #crate0::Stat for #name {
                    type Value = #value;

                    fn name(&self) -> &'static str {
                        match self {
                            #(#name::#names => stringify!(#names),)*
                        }
                    }

                    fn vtable() -> &'static #crate0::StatVTable<Self> {
                        #crate0::vtable!(#name)
                    }

                    fn as_index(&self) -> u64 {
                        match self {
                            #(#name::#names2 => #indices,)*
                        }
                    }

                    fn from_index(value: u64) -> Self {
                        match value {
                            #(#indices => #name::#names3,)*
                            _ => #default
                        }
                    }

                    fn values() -> impl IntoIterator<Item = Self> {
                        [#(#name::#names4),*]
                    }
                }
            }
            .into()
        }
        syn::Data::Union(u) => {
            abort!(u.union_token.span, "Only supports unit structs and enums.");
        }
    }
}

/// Allow the type to convert to `Attribute`.
///
/// # Supported types
/// * Unit struct
/// * Fieldless enum with `#[repr(u64)]`
/// * Newtype of u64
///
/// This is usable with `bitflags!` in impl mode.
#[proc_macro_error]
#[proc_macro_derive(Attribute)]
pub fn attribute(tokens: TokenStream1) -> TokenStream1 {
    let input = parse_macro_input!(tokens as DeriveInput);
    let crate0 = quote! {::bevy_stat_query};
    let name = input.ident;
    let uniq = quote! {
        {
            #[used]
            static THING: ::std::sync::atomic::AtomicU8 = ::std::sync::atomic::AtomicU8::new(0);
            &THING as *const ::std::sync::atomic::AtomicU8 as usize
        }
    };
    match input.data {
        syn::Data::Struct(s) => match s.fields {
            Fields::Named(_) => abort!(
                s.struct_token.span,
                "Only supports unit structs, bitflags and enums."
            ),
            Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    abort!(
                        s.struct_token.span,
                        "Only supports unit structs, bitflags and enums."
                    );
                }
                quote! {
                    impl From<#name> for #crate0::Attribute<'static> {
                        fn from(value: #name) -> #crate0::Attribute<'static> {
                            #crate0::Attribute::Enum{
                                tag: #uniq,
                                index: value.0 as u64,
                            }
                        }
                    }

                    impl From<&#name> for #crate0::Attribute<'static> {
                        fn from(value: &#name) -> #crate0::Attribute<'static> {
                            #crate0::Attribute::Enum{
                                tag: #uniq,
                                index: value.0 as u64,
                            }
                        }
                    }
                }
                .into()
            }
            Fields::Unit => quote! {
                impl From<#name> for #crate0::Attribute<'static> {
                    fn from(_: #name) -> #crate0::Attribute<'static> {
                        #crate0::Attribute::Enum{
                            tag: #uniq,
                            index: 0,
                        }
                    }
                }

                impl From<&#name> for #crate0::Attribute<'static> {
                    fn from(_: &#name) -> #crate0::Attribute<'static> {
                        #crate0::Attribute::Enum{
                            tag: #uniq,
                            index: 0,
                        }
                    }
                }
            }
            .into(),
        },
        syn::Data::Enum(fields) => {
            let f1 = fields.variants.iter().map(|x| &x.ident);
            let f2 = fields.variants.iter().map(|x| &x.ident);
            quote! {
                impl From<#name> for #crate0::Attribute<'static> {
                    fn from(value: #name) -> #crate0::Attribute<'static> {
                        #crate0::Attribute::Enum{
                            tag: #uniq,
                            index: value as u64,
                        }
                    }
                }

                impl From<&#name> for #crate0::Attribute<'static> {
                    fn from(value: &#name) -> #crate0::Attribute<'static> {
                        let variant = match value {
                            #(#name::#f1 => #name::#f2),*
                        };
                        #crate0::Attribute::Enum{
                            tag: #uniq,
                            index: variant as u64,
                        }
                    }
                }
            }
        }
        .into(),
        syn::Data::Union(u) => {
            abort!(
                u.union_token.span,
                "Only supports unit structs, bitflags and enums."
            );
        }
    }
}
