use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DataEnum, DataStruct, Fields, Ident, Meta};

pub fn derive_from(input: &syn::DeriveInput) -> darling::Result<TokenStream> {
    if !input.generics.params.is_empty() {
        return Err(darling::Error::custom("Generic params are not currently supported").with_span(&input.generics));
    }
    if !matches!(input.data, Data::Enum(_))
        && let Some(attr) = crate::utils::derive_attrs(&input.attrs, "from")?.first()
    {
        return Err(darling::Error::custom("#[from(all)] is only supported on enums").with_span(attr));
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
            return derive_enum(&input.ident, data, &input.attrs);
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

fn derive_enum(ident: &Ident, data: &DataEnum, attrs: &[Attribute]) -> darling::Result<TokenStream> {
    let mut all = false;
    for attr in &crate::utils::derive_attrs(attrs, "from")? {
        if all {
            return Err(darling::Error::custom("Duplicate #[from(all)] attribute").with_span(attr));
        }
        let option = attr.require_list()?.parse_args::<Ident>()?;
        if option != "all" {
            return Err(darling::Error::custom("Expected #[from(all)] on the enum").with_span(attr));
        }
        all = true;
    }

    let mut implementations = TokenStream::new();
    for variant in &data.variants {
        if let Some(attr) = crate::utils::derive_attrs(&variant.attrs, "from")?.first() {
            return Err(darling::Error::custom("Place #[from] on the variant's single tuple field").with_span(attr));
        }
        let mut selected = all;
        for field in &variant.fields {
            let mut marked = false;
            for attr in &crate::utils::derive_attrs(&field.attrs, "from")? {
                if !matches!(attr, Meta::Path(_)) {
                    return Err(
                        darling::Error::custom("Expected #[from] without arguments on a variant field").with_span(attr),
                    );
                }
                if marked {
                    return Err(darling::Error::custom("Duplicate #[from] attribute").with_span(attr));
                }
                marked = true;
                selected = true;
            }
        }
        if !selected {
            continue;
        }
        let Fields::Unnamed(fields) = &variant.fields else {
            return Err(darling::Error::custom("From requires a single-field tuple variant").with_span(variant));
        };
        if fields.unnamed.len() != 1 {
            return Err(darling::Error::custom("From requires a single-field tuple variant").with_span(variant));
        }
        let ty = &fields.unnamed[0].ty;
        let variant_ident = &variant.ident;
        implementations.extend(quote! {
            #[automatically_derived]
            impl ::core::convert::From<#ty> for #ident {
                #[inline]
                fn from(value: #ty) -> Self {
                    Self::#variant_ident(value)
                }
            }
        });
    }
    Ok(implementations)
}

#[cfg(test)]
mod tests {
    use super::derive_from;

    #[test]
    fn rejects_non_newtype_variants_in_all_mode() {
        for variant in [
            quote::quote!(Unit),
            quote::quote!(Empty()),
            quote::quote!(Pair(u32, u64)),
            quote::quote!(Named { value: u32 }),
        ] {
            for attr in [quote::quote!(#[from(all)]), quote::quote!(#[nabla(from(all))])] {
                let input = syn::parse_quote!(#attr enum Example { Valid(u8), #variant });
                let error = derive_from(&input).unwrap_err();
                assert!(error.to_string().contains("single-field tuple variant"), "{error}");
            }
        }
    }

    #[test]
    fn rejects_marked_non_newtype_fields() {
        for variant in [
            quote::quote!(Pair(
                #[from]
                u32,
                u64
            )),
            quote::quote!(Named {
                #[from]
                value: u32
            }),
        ] {
            let input = syn::parse_quote!(enum Example { #variant });
            let error = derive_from(&input).unwrap_err();
            assert!(error.to_string().contains("single-field tuple variant"), "{error}");
        }
    }

    #[test]
    fn rejects_invalid_from_attributes() {
        for input in [
            syn::parse_quote!(
                #[from]
                enum Example {
                    Value(u32),
                }
            ),
            syn::parse_quote!(
                #[from(unknown)]
                enum Example {
                    Value(u32),
                }
            ),
            syn::parse_quote!(
                #[from(all)]
                #[from(all)]
                enum Example {
                    Value(u32),
                }
            ),
            syn::parse_quote!(
                enum Example {
                    #[from]
                    Value(u32),
                }
            ),
            syn::parse_quote!(
                enum Example {
                    Value(#[from(all)] u32),
                }
            ),
            syn::parse_quote!(
                enum Example {
                    Value(
                        #[from]
                        #[from]
                        u32,
                    ),
                }
            ),
            syn::parse_quote!(
                #[from(all)]
                struct Example(u32);
            ),
        ] {
            assert!(derive_from(&input).is_err());
        }
    }

    #[test]
    fn unmarked_enum_generates_no_conversions() {
        let input = syn::parse_quote!(
            enum Example {
                Unit,
                Value(u32),
                Pair(u32, u64),
            }
        );
        assert!(derive_from(&input).unwrap().is_empty());
    }

    #[test]
    fn rejects_invalid_namespaced_from_attributes() {
        for (source, message) in [
            ("#[from(all)] #[nabla(from(all))] enum E { V(u32) }", "Duplicate"),
            ("#[nabla(from(all))] #[from(all)] enum E { V(u32) }", "Duplicate"),
            ("#[nabla(from(all), from(all))] enum E { V(u32) }", "Duplicate"),
            ("enum E { V(#[from] #[nabla(from)] u32) }", "Duplicate"),
            ("enum E { V(#[nabla(from)] #[from] u32) }", "Duplicate"),
            ("enum E { V(#[nabla(from, from)] u32) }", "Duplicate"),
            ("enum E { #[nabla(from)] V(u32) }", "single tuple field"),
            ("enum E { V(#[nabla(from(all))] u32) }", "without arguments"),
            ("enum E { V(#[nabla(from)] u32, u64) }", "single-field tuple"),
            ("enum E { V { #[nabla(from)] value: u32 } }", "single-field tuple"),
            ("#[nabla(from(unknown))] enum E { V(u32) }", "Expected #[from(all)]"),
            ("#[nabla(from(all))] struct S(u32);", "only supported on enums"),
            ("#[nabla(unknown)] enum E { V(u32) }", "Unknown field"),
        ] {
            let input = syn::parse_str(source).unwrap();
            let error = derive_from(&input).unwrap_err();
            assert!(error.to_string().contains(message), "{source}: {error}");
        }
    }
}
