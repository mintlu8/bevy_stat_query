use proc_macro::{Span, TokenStream as TokenStream1};
use proc_macro_error::{abort, proc_macro_error};
use quote::{ToTokens, format_ident, quote};
use syn::{
    Data, DeriveInput, Fields, Ident, LitInt, LitStr, Meta, Type, parse_macro_input,
    punctuated::Punctuated, spanned::Spanned, token::Comma,
};

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
#[proc_macro_derive(Stat, attributes(stat))]
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

/// Implement `StatDispatch` and `StatDispatchTo` on a enum container of multiple stats.
///
/// # Attributes
///
/// * `#[stat_dispatch(serde)]`
///
///   Generates a `name` based serialization for this struct and implement `SerializeEntry` to enable value serialization.
///
/// * `#[stat_dispatch(serde_value)]`
///
///   Implement `SerializeEntry` to enable value serialization without creating a serde implementation for this struct.
///
/// * `#[on_value(serde)]`
///
///   Copy to the generated value type.
#[proc_macro_error]
#[proc_macro_derive(StatDispatch, attributes(stat_dispatch, on_value))]
pub fn stat_dispatch(tokens: TokenStream1) -> TokenStream1 {
    let input = parse_macro_input!(tokens as DeriveInput);
    let data_enum = match input.data {
        Data::Struct(_) | Data::Union(_) => {
            abort!(input.ident.span(), "Expected enum.")
        }
        syn::Data::Enum(data_enum) => data_enum,
    };
    let vis = input.vis;
    let name = input.ident;
    let value_name = format_ident!("{}Value", name);
    let mut get_uid_branches = Vec::new();
    let mut from_stat_branches = Vec::new();
    let mut serialize_stat_branches = Vec::new();
    let mut deserialize_stat_branches = Vec::new();
    let mut deserialize_branches = Vec::new();
    let mut variants = Vec::new();
    let mut variant_tys = Vec::new();
    let mut value_attrs = Vec::new();
    let mut generate_serde_block = false;
    let mut generate_serde_value_block = false;

    for attr in input.attrs {
        if let Meta::List(meta_list) = attr.meta {
            if meta_list.path.is_ident("stat_dispatch") {
                if let Ok(idents) =
                    meta_list.parse_args_with(Punctuated::<Ident, Comma>::parse_terminated)
                {
                    for ident in idents {
                        if ident == "serde" {
                            generate_serde_block = true;
                            generate_serde_value_block = true;
                        } else if ident == "serde_value" {
                            generate_serde_value_block = true;
                        } else {
                            abort!(meta_list.span(), "Expected 'serde' or 'serde_value'.")
                        }
                    }
                } else {
                    abort!(meta_list.span(), "Expected \"serde\".")
                }
            } else if meta_list.path.is_ident("on_value") {
                value_attrs.push(meta_list.tokens);
            }
        }
    }
    for variant in &data_enum.variants {
        let variant_ident = &variant.ident;
        variants.push(variant_ident);
        match &variant.fields {
            Fields::Named(_) => {
                abort!(
                    variant.ident.span(),
                    "Expected either a single unnamed field or a unit variant matching a unit struct's name."
                )
            }
            Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    abort!(
                        variant.ident,
                        "Expected either a single unnamed field or a unit variant matching a unit struct's name."
                    )
                }
                let variant_ty = &fields.unnamed[0].ty;
                variant_tys.push(variant_ty.to_token_stream());
                get_uid_branches.push(quote! {
                    #name::#variant_ident(stat) => ::bevy_stat_query::Stat::as_uid(stat)
                });
                deserialize_branches.push(quote! {
                    #name::#variant_ident(stat) => Ok(#value_name::#variant_ident(
                        <<#variant_ty as ::bevy_stat_query::Stat>::Value as ::bevy_stat_query::Deserialize>::deserialize(deserializer)?
                    ))
                });
                from_stat_branches.push(quote! {
                    #name::#variant_ident(stat)
                });
                serialize_stat_branches.push(quote! {
                    #name::#variant_ident(stat) => stat.name()
                });
                deserialize_stat_branches.push(quote! {
                    for entry in #variant_ident::values() {
                        if __name == entry.name() {
                            return Ok(#name::#variant_ident(entry));
                        }
                    }
                });
            }
            Fields::Unit => {
                let variant_ty = &variant_ident;
                variant_tys.push(variant_ty.to_token_stream());
                get_uid_branches.push(quote! {
                    #name::#variant_ident => ::bevy_stat_query::Stat::as_uid(&#variant_ident)
                });
                deserialize_branches.push(quote! {
                    #name::#variant_ident => Ok(#value_name::#variant_ident(
                        <<#variant_ty as ::bevy_stat_query::Stat>::Value as ::bevy_stat_query::Deserialize>::deserialize(deserializer)?
                    ))
                });
                from_stat_branches.push(quote! {
                    #name::#variant_ident
                });
                serialize_stat_branches.push(quote! {
                    #name::#variant_ident => #variant_ident.name()
                });
                deserialize_stat_branches.push(quote! {
                    for entry in #variant_ident::values() {
                        if __name == entry.name() {
                            return Ok(#name::#variant_ident);
                        }
                    }
                });
            }
        }
    }

    let serialize_block = if generate_serde_block {
        quote! {
            impl ::bevy_stat_query::Serialize for #name {
                fn serialize<S: ::bevy_stat_query::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    let name = match self {
                        #(#serialize_stat_branches),*
                    };
                    ::bevy_stat_query::Serialize::serialize(name, serializer)
                }
            }

            impl<'de> ::bevy_stat_query::Deserialize<'de> for #name {
                fn deserialize<D: ::bevy_stat_query::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    let __name = <String as ::bevy_stat_query::Deserialize>::deserialize(deserializer)?;
                    #(#deserialize_stat_branches)*
                    Err(::bevy_stat_query::DError::custom(format!("Unknown stat {__name}.")))
                }
            }
        }
    } else {
        quote! {}
    };

    let serialize_value_block = if generate_serde_value_block {
        quote! {
            impl ::bevy_stat_query::SerializeEntry for #name {
                fn serialize_item<S: ::bevy_stat_query::Serializer>(
                    &self,
                    value: &Self::Value,
                    serializer: S,
                ) -> Result<S::Ok, S::Error> {
                    match value {
                        #(#value_name::#variants(value) => {
                            ::bevy_stat_query::Serialize::serialize(value, serializer)
                        })*
                    }
                }
            }

            impl ::bevy_stat_query::DeserializeEntry for #name {
                fn deserialize_item<'de, D: ::bevy_stat_query::Deserializer<'de>>(
                    &self,
                    deserializer: D,
                ) -> Result<Self::Value, D::Error> {
                    match self {
                        #(#deserialize_branches),*
                    }
                }
            }
        }
    } else {
        quote! {}
    };

    quote! {
        #[derive(Debug, Clone)]
        #(#[#value_attrs])*
        #vis enum #value_name {
            #(#variants(<#variant_tys as ::bevy_stat_query::Stat>::Value)),*
        }

        #serialize_block
        #serialize_value_block

        impl ::bevy_stat_query::StatDispatch for #name {
            type Value = #value_name;

            fn get_uid(&self) -> ::bevy_stat_query::StatUid {
                match self {
                    #(#get_uid_branches),*
                }
            }

            fn join_to(&self, value: &Self::Value, into: &mut dyn ::bevy_stat_query::ShareableAny) {
                match value {
                    #(#value_name::#variants(value) => {
                        if let Some(into) = into.downcast_mut::<<#variant_tys as ::bevy_stat_query::Stat>::Value>() {
                            ::bevy_stat_query::StatValue::join(into, value);
                        }
                    })*
                }
            }
        }

        #(
            impl ::bevy_stat_query::StatDispatchTo<#variant_tys> for #name {
                fn from_stat(stat: #variant_tys) -> Self {
                    #from_stat_branches
                }

                fn from_value(value: <#variant_tys as ::bevy_stat_query::Stat>::Value) -> Self::Value {
                    #value_name::#variants(value)
                }

                fn get_value(value: &Self::Value) -> Option<&<#variant_tys as ::bevy_stat_query::Stat>::Value> {
                    if let #value_name::#variants(item) = value {
                        Some(item)
                    } else {
                        None
                    }
                }

                fn get_value_mut(value: &mut Self::Value) -> Option<&mut <#variant_tys as ::bevy_stat_query::Stat>::Value>{
                    if let #value_name::#variants(item) = value {
                        Some(item)
                    } else {
                        None
                    }
                }

                fn get_value_owned(value: Self::Value) -> Option<<#variant_tys as ::bevy_stat_query::Stat>::Value> {
                    if let #value_name::#variants(item) = value {
                        Some(item)
                    } else {
                        None
                    }
                }

                fn try_join_to(&self, value: &Self::Value, into: &mut <#variant_tys as ::bevy_stat_query::Stat>::Value) {
                    if let #value_name::#variants(item) = value {
                        ::bevy_stat_query::StatValue::join(into, item)
                    }
                }
            }
        )*
    }.into()
}
