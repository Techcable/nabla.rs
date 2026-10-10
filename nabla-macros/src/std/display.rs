use core::fmt::{Display, Formatter};

use darling::ast::Style;
use darling::{FromDeriveInput, FromMeta, FromVariant};
use indexmap::IndexMap;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, quote};
use syn::Attribute;
use syn::ext::IdentExt;
use syn::parse::Parser;

use crate::utils::destructure::{DestructuredType, RefStyle};
use crate::utils::fmt_args::{FormatArgs, FormatString, FormatStringPart};
use crate::utils::shorthand;

#[derive(FromDeriveInput, Debug)]
#[darling(forward_attrs(nabla, display))]
struct DeriveDisplay {
    ident: Ident,
    data: darling::ast::Data<DisplayVariant, syn::Field>,
    #[darling(with = parse_display_attrs)]
    attrs: Option<DisplayAttr>,
}

#[derive(FromVariant, Debug)]
#[darling(forward_attrs(nabla, display))]
struct DisplayVariant {
    ident: Ident,
    #[darling(with = parse_display_attrs)]
    attrs: Option<DisplayAttr>,
    fields: darling::ast::Fields<syn::Field>,
}

#[expect(clippy::needless_pass_by_value, reason = "signature required by darling")]
fn parse_display_attrs(attrs: Vec<Attribute>) -> darling::Result<Option<DisplayAttr>> {
    let mut res = None;
    for meta in &crate::utils::derive_attrs(&attrs, "display")? {
        if res.is_some() {
            return Err(
                darling::Error::custom("The #[display(..)] attribute should not be duplicated").with_span(meta.path()),
            );
        }
        let attr = <DisplayAttr as FromMeta>::from_meta(meta)?;
        res = Some(attr);
    }
    Ok(res)
}

#[derive(Debug, Clone)]
struct DisplayAttr {
    fmt: FormatArgs,
}
impl FromMeta for DisplayAttr {
    fn from_meta(meta: &syn::Meta) -> darling::Result<Self> {
        let list = meta.require_list()?;
        Ok(Self {
            fmt: syn::parse2(list.tokens.clone())?,
        })
    }
}

const FIELD_PREFIX: &str = "__nabla_field_";

/// Rewrite placeholders that refer to fields, binding each referenced field to a format argument.
///
/// Fields are also in scope as locals (see [`destructure`]), so other placeholders and all
/// width/precision parameters are left for rustc to resolve by capturing variables.
fn rewrite_fmt_str(
    args: &FormatArgs,
    fields: &darling::ast::Fields<syn::Field>,
    bindings: &mut IndexMap<Ident, Ident>,
) -> darling::Result<FormatString> {
    let fmt = &args.format_string;
    let mut new_parts = Vec::new();
    for part in fmt.parts() {
        let FormatStringPart::ArgRef(orig_ref) = part else {
            new_parts.push(part.clone());
            continue;
        };
        let mut new_ref = orig_ref.clone();
        if let Some(spec) = referenced_field(&orig_ref.argument, args, fields.style == Style::Tuple)?
            && let Some(index) = fields
                .iter()
                .enumerate()
                .position(|(index, field)| field_spec(index, field) == spec)
        {
            let mut var = format!("{FIELD_PREFIX}{spec}");
            while args.named_args.contains(&var) {
                var.insert(0, '_');
            }
            let var = Ident::new(&var, Span::call_site());
            new_ref.argument = var.to_string();
            bindings
                .entry(var)
                .or_insert_with(|| field_local(index, &fields.fields[index]));
        }
        new_parts.push(FormatStringPart::ArgRef(new_ref));
    }
    Ok(FormatString::from_parts(&new_parts, fmt.lit().span()))
}

/// Determine which field a placeholder argument could refer to.
///
/// Returns `None` for implicit and explicit format arguments, which rustc resolves itself.
fn referenced_field(argument: &str, args: &FormatArgs, has_unnamed_fields: bool) -> darling::Result<Option<FieldSpec>> {
    if let Ok(index) = argument.parse::<usize>() {
        if !has_unnamed_fields {
            return Ok(None);
        }
        if let Some(ref positional) = args.first_positional {
            return Err(darling::Error::custom(
                "Ambiguous reference to positional arguments by number in a tuple struct or variant; \
                 change this to a named argument",
            )
            .with_span(positional));
        }
        Ok(Some(FieldSpec::Unnamed(index)))
    } else if argument.starts_with("r#") || args.named_args.contains(argument) {
        Ok(None)
    } else {
        Ok(Ident::parse_any
            .parse_str(argument)
            .ok()
            .map(|_| FieldSpec::Named(argument.into())))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum FieldSpec {
    Named(String),
    Unnamed(usize),
}
impl Display for FieldSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            FieldSpec::Named(name) => f.write_str(name),
            FieldSpec::Unnamed(index) => write!(f, "{index}"),
        }
    }
}
fn field_spec(index: usize, field: &syn::Field) -> FieldSpec {
    match field.ident {
        Some(ref name) => FieldSpec::Named(name.unraw().to_string()),
        None => FieldSpec::Unnamed(index),
    }
}
/// The local a field is bound to by [`destructure`].
fn field_local(index: usize, field: &syn::Field) -> Ident {
    match field.ident {
        Some(ref name) => name.clone(),
        None => Ident::new(&shorthand::tuple_local(index), Span::call_site()),
    }
}

fn expand_write(fields: &darling::ast::Fields<syn::Field>, attr: &DisplayAttr) -> darling::Result<TokenStream> {
    let mut bindings = IndexMap::new();
    let mut fmt = attr.fmt.clone();
    fmt.format_string = rewrite_fmt_str(&attr.fmt, fields, &mut bindings)?;
    let write = quote!(::core::write!(__nabla_formatter, #fmt));
    Ok(if bindings.is_empty() {
        write
    } else {
        let (vars, values): (Vec<_>, Vec<_>) = bindings.into_iter().unzip();
        quote! {
            match (#(#values,)*) {
                (#(#vars,)*) => #write
            }
        }
    })
}

/// Reject display options on fields, and unknown namespaced options.
fn check_field_attrs(fields: &darling::ast::Fields<syn::Field>) -> darling::Result<()> {
    for field in fields.iter() {
        if let Some(meta) = crate::utils::derive_attrs(&field.attrs, "display")?.first() {
            return Err(darling::Error::custom("#[display(...)] is not supported on fields").with_span(meta.path()));
        }
    }
    Ok(())
}

/// Bind fields to locals named after them, and `_0`, `_1`, ... for tuple fields.
fn destructure(fields: &darling::ast::Fields<syn::Field>) -> TokenStream {
    DestructuredType::new(fields, shorthand::tuple_local)
        .ref_style(Some(RefStyle::Immutable))
        .to_token_stream()
}

pub fn derive_display(input: &syn::DeriveInput) -> darling::Result<TokenStream> {
    if !input.generics.params.is_empty() {
        return Err(darling::Error::custom("Generic params are not currently supported").with_span(&input.generics));
    }
    let derive = DeriveDisplay::from_derive_input(input)?;
    let target_name = &input.ident;
    let body = match derive.data {
        darling::ast::Data::Enum(ref variants) => {
            let mut match_arms = Vec::new();
            for variant in variants {
                check_field_attrs(&variant.fields)?;
                let ident = &variant.ident;
                let destructure = destructure(&variant.fields);
                let attr = variant.attrs.as_ref().ok_or_else(|| {
                    darling::Error::custom("Variant is missing a #[display(...)] attr").with_span(&variant.ident)
                })?;
                let write = expand_write(&variant.fields, attr)?;

                match_arms.push(quote! {
                    Self::#ident #destructure => {
                        #write
                    }
                });
            }
            quote! {
                match *self {
                    #(#match_arms)*
                }
            }
        }
        darling::ast::Data::Struct(ref fields) => {
            check_field_attrs(fields)?;
            let ident = &derive.ident;
            let attr = derive.attrs.as_ref().ok_or_else(|| {
                darling::Error::custom("Type is missing a #[display(...)] attr").with_span(&derive.ident)
            })?;
            let write = expand_write(fields, attr)?;
            let destructure = destructure(fields);
            quote! {
                let #ident #destructure = *self;
                #write
            }
        }
    };
    Ok(quote! {
        #[automatically_derived]
        impl ::core::fmt::Display for #target_name {
            // Every field is bound, whether or not the format string uses it.
            #[allow(unused_variables)]
            fn fmt(&self, __nabla_formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                #body
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::derive_display;

    #[test]
    fn duplicate_display_attributes() {
        for attrs in [
            quote::quote!(#[display("a")] #[nabla(display("b"))]),
            quote::quote!(#[nabla(display("a"))] #[display("b")]),
            quote::quote!(#[nabla(display("a"), display("b"))]),
            quote::quote!(#[nabla(display("a"))] #[nabla(display("b"))]),
        ] {
            let input = syn::parse_quote!(#attrs struct Example;);
            let error = derive_display(&input).unwrap_err();
            assert!(error.to_string().contains("should not be duplicated"), "{error}");
        }
    }

    #[test]
    fn invalid_namespaced_attributes() {
        for attrs in [
            quote::quote!(#[nabla(unknown("a"))]),
            quote::quote!(#[nabla(display)]),
            quote::quote!(#[nabla(display(42))]),
            quote::quote!(#[nabla(display("{x}", x = 1, x = 2))]),
        ] {
            let input = syn::parse_quote!(#attrs struct Example;);
            assert!(derive_display(&input).is_err());
        }
    }

    #[test]
    fn ambiguous_positional_arguments() {
        for source in [
            "#[display(\"{0}\", 1)] struct S(u32);",
            "enum E { #[display(\"{1}\", .0)] V(u32, u32) }",
        ] {
            let input = syn::parse_str(source).unwrap();
            let error = derive_display(&input).unwrap_err();
            assert!(error.to_string().contains("Ambiguous reference"), "{source}: {error}");
        }
        for source in [
            "#[display(\"{0}\", 1)] struct S { value: u32 }",
            "#[display(\"{0}\", 1)] struct S;",
            "#[display(\"{0} {x}\", x = 1)] struct S(u32);",
        ] {
            let input = syn::parse_str(source).unwrap();
            assert!(derive_display(&input).is_ok(), "{source}");
        }
    }

    #[test]
    fn invalid_field_attributes() {
        for (source, message) in [
            (
                "#[display(\"{0}\")] struct S(#[display(\"x\")] u32);",
                "not supported on fields",
            ),
            (
                "#[display(\"{0}\")] struct S(#[nabla(display(\"x\"))] u32);",
                "not supported on fields",
            ),
            ("#[display(\"{0}\")] struct S(#[nabla(bogus)] u32);", "Unknown field"),
            (
                "enum E { #[display(\"{v}\")] V { #[nabla(bogus)] v: u32 } }",
                "Unknown field",
            ),
        ] {
            let input = syn::parse_str(source).unwrap();
            let error = derive_display(&input).unwrap_err();
            assert!(error.to_string().contains(message), "{source}: {error}");
        }
    }

    #[test]
    fn allows_from_field_attributes() {
        for source in [
            "#[display(\"{0}\")] struct S(#[nabla(from)] u32);",
            "enum E { #[display(\"{0}\")] V(#[nabla(from)] u32) }",
        ] {
            let input = syn::parse_str(source).unwrap();
            assert!(derive_display(&input).is_ok(), "{source}");
        }
    }
}
