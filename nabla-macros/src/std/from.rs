use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DataStruct, Fields};

pub fn derive_from(input: &syn::DeriveInput) -> darling::Result<TokenStream> {
    if !input.generics.params.is_empty() {
        return Err(darling::Error::custom("Generic params are not currently supported").with_span(&input.generics));
    }
    let fields = match input.data {
        Data::Struct(DataStruct {
            fields: Fields::Unit, ..
        }) => return Err(darling::Error::custom("Unit structs are not supported").with_span(&input.ident)),
        Data::Struct(DataStruct {
            fields: Fields::Named(ref named),
            ..
        }) => {
            return Err(
                darling::Error::custom("Structs with named fields are not currently supported")
                    .with_span(&named.brace_token.span),
            );
        }
        Data::Struct(DataStruct {
            fields: Fields::Unnamed(ref unnamed),
            ..
        }) => unnamed,
        Data::Enum(ref data) => {
            return Err(darling::Error::custom("enums are not currently supported").with_span(&data.enum_token));
        }
        Data::Union(ref data) => {
            return Err(darling::Error::custom("unions are not supported").with_span(&data.union_token));
        }
    };
    match fields.unnamed.len() {
        0 => {
            Err(darling::Error::custom("Cannot derive From on struct with no fields")
                .with_span(&fields.paren_token.span))
        }
        1 => {
            let single = &fields.unnamed[0];
            let single_ty = &single.ty;
            let ident = &input.ident;
            Ok(quote! {
                #[automatically_derived]
                impl ::core::convert::From<#single_ty> for #ident {
                    #[inline]
                    fn from(x: #single_ty) -> Self {
                        #ident(x)
                    }
                }
            })
        }
        2.. => Err(
            darling::Error::custom("Structs with multiple fields are not currently supported")
                .with_span(&fields.unnamed[1]),
        ),
    }
}
