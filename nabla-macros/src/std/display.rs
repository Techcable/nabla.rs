use core::fmt::{Display, Formatter};
use std::collections::{HashMap, HashSet};

use darling::{FromDeriveInput, FromMeta, FromVariant};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, quote};
use syn::ext::IdentExt;
use syn::parse::Parser;
use syn::spanned::Spanned;
use syn::{Attribute, Member};

use crate::utils::destructure::{DestructuredType, RefStyle};
use crate::utils::fmt_args::{FormatArgs, FormatString, FormatStringPart};

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
fn prefix_field(x: impl Display) -> String {
    let name = x.to_string();
    format!("{FIELD_PREFIX}{}", name.strip_prefix("r#").unwrap_or(&name))
}

fn rewrite_fmt_str(
    args: &FormatArgs,
    field_args: &HashMap<FieldSpec, Ident>,
    used_fields: &mut HashSet<FieldSpec>,
    count_fields: &mut HashSet<FieldSpec>,
) -> darling::Result<FormatString> {
    let fmt = &args.format_string;
    let mut new_parts = Vec::new();
    for part in fmt.parts() {
        new_parts.push(match part {
            FormatStringPart::Literal(_) | FormatStringPart::EscapedOpenBrace | FormatStringPart::EscapedCloseBrace => {
                part.clone()
            }
            FormatStringPart::ArgRef(orig_ref) => {
                let mut new_arg = orig_ref.clone();
                new_arg.argument = rewrite_arg(&orig_ref.argument, args, field_args, used_fields)?;
                if let Some(spec) = &orig_ref.fmt_spec {
                    new_arg.fmt_spec = Some(rewrite_fmt_spec(spec, |name| {
                        rewrite_arg(name, args, field_args, count_fields)
                    })?);
                }
                FormatStringPart::ArgRef(new_arg)
            }
        });
    }
    Ok(FormatString::from_parts(&new_parts, fmt.span()))
}

fn rewrite_arg(
    name: &str,
    args: &FormatArgs,
    field_args: &HashMap<FieldSpec, Ident>,
    used_fields: &mut HashSet<FieldSpec>,
) -> darling::Result<String> {
    if name.is_empty()
        || args.keyword_args.keys().any(|key| key == name)
        || (!args.positional_args.is_empty() && name.parse::<usize>().is_ok())
    {
        return Ok(name.into());
    }
    let member = Ident::parse_any
        .parse_str(name)
        .map(Member::Named)
        .or_else(|_| syn::parse_str::<Member>(name));
    let field = FieldSpec::from(&member.map_err(|cause| {
        darling::Error::custom(format!("Failed to parse fmt argument: {cause}")).with_span(args.format_string.lit())
    })?);
    let rewritten = field_args
        .get(&field)
        .ok_or_else(|| darling::Error::custom(format!("Unknown field `{field}`")).with_span(args.format_string.lit()))?
        .to_string();
    used_fields.insert(field);
    Ok(rewritten)
}

/// Rewrite width and precision parameters, preserving fill, flags, and format type.
fn rewrite_fmt_spec(spec: &str, mut rewrite: impl FnMut(&str) -> darling::Result<String>) -> darling::Result<String> {
    fn count(
        rest: &mut &str,
        output: &mut String,
        rewrite: &mut impl FnMut(&str) -> darling::Result<String>,
    ) -> darling::Result<()> {
        if let Some((name, suffix)) = rest.split_once('$')
            && (Ident::parse_any.parse_str(name).is_ok() || name.parse::<usize>().is_ok())
        {
            output.push_str(&rewrite(name)?);
            output.push('$');
            *rest = suffix;
        } else {
            let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
            output.push_str(&rest[..digits]);
            *rest = &rest[digits..];
        }
        Ok(())
    }

    let mut rest = spec;
    let mut chars = rest.char_indices();
    let first = chars.next();
    let second = chars.next();
    if let Some((index, ch @ ('<' | '>' | '^'))) = second {
        rest = &rest[index + ch.len_utf8()..];
    } else if let Some((_, ch @ ('<' | '>' | '^'))) = first {
        rest = &rest[ch.len_utf8()..];
    }
    for flag in ["+", "-", "#"] {
        rest = rest.strip_prefix(flag).unwrap_or(rest);
    }
    // `0$` is argument zero, whereas `0width$` enables zero padding.
    if !rest.starts_with("0$") {
        rest = rest.strip_prefix('0').unwrap_or(rest);
    }
    let mut output = spec[..spec.len() - rest.len()].to_owned();
    count(&mut rest, &mut output, &mut rewrite)?;
    if let Some(precision) = rest.strip_prefix('.') {
        output.push('.');
        rest = precision;
        count(&mut rest, &mut output, &mut rewrite)?;
    }
    output.push_str(rest);
    Ok(output)
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
impl<'a> From<&'a syn::Member> for FieldSpec {
    fn from(value: &'a Member) -> Self {
        match value {
            Member::Named(name) => FieldSpec::Named(name.unraw().to_string()),
            Member::Unnamed(index) => FieldSpec::Unnamed(index.index as usize),
        }
    }
}

struct WriteExpandInfo<'a> {
    fields: &'a darling::ast::Fields<syn::Field>,
    attr: DisplayAttr,
}
fn expand_write(mut variant: WriteExpandInfo) -> darling::Result<TokenStream> {
    let field_specs = variant
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| match field.ident {
            Some(ref name) => FieldSpec::Named(name.unraw().to_string()),
            None => FieldSpec::Unnamed(index),
        })
        .collect::<Vec<_>>();
    let mut used_fields = HashSet::new();
    let mut count_fields = HashSet::new();
    let field_args = field_specs
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let mut name = format!("__nabla_arg_{index}");
            while variant.attr.fmt.keyword_args.keys().any(|key| key == &name) {
                name.push('_');
            }
            (field.clone(), Ident::new(&name, Span::call_site()))
        })
        .collect::<HashMap<_, _>>();
    variant.attr.fmt.format_string =
        rewrite_fmt_str(&variant.attr.fmt, &field_args, &mut used_fields, &mut count_fields)?;
    for field in &field_specs {
        if !used_fields.contains(field) && !count_fields.contains(field) {
            continue;
        }
        let prefixed_name = Ident::new(&prefix_field(field), Span::call_site());
        // Formatting counts require usize values, while destructured fields are borrowed.
        let value = if count_fields.contains(field) {
            quote!(*#prefixed_name)
        } else {
            prefixed_name.to_token_stream()
        };
        let existing = variant.attr.fmt.keyword_args.insert(field_args[field].clone(), value);
        assert!(existing.is_none());
    }
    let fmt = &variant.attr.fmt;
    Ok(quote!(::core::write!(__nabla_formatter, #fmt)))
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

fn destructure(fields: &darling::ast::Fields<syn::Field>) -> TokenStream {
    DestructuredType::new(fields, prefix_field)
        .ref_style(Some(RefStyle::Immutable))
        .field_renamer(|name| prefix_field(name))
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
                let attr = variant.attrs.clone().ok_or_else(|| {
                    darling::Error::custom("Variant is missing a #[display(...)] attr").with_span(&variant.ident)
                })?;
                let write = expand_write(WriteExpandInfo {
                    fields: &variant.fields,
                    attr,
                })?;

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
            let attr = derive.attrs.clone().ok_or_else(|| {
                darling::Error::custom("Type is missing a #[display(...)] attr").with_span(&derive.ident)
            })?;
            let write = expand_write(WriteExpandInfo { fields, attr })?;
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

#[cfg(false)]
mod rewrite {
    use darling::error::Accumulator;
    use proc_macro2::{Group, Literal, Spacing, TokenStream, TokenTree};
    use syn::Lit;

    fn rewrite_field_ref_shorthand(stream: TokenStream, acc: &mut Accumulator) -> TokenStream {
        fn parse_lit(lit: &Literal) -> Option<Lit> {
            syn::parse2::<Lit>(lit.to_token_stream()).ok()
        }
        let rewrite_shorthand = |orig_stream: TokenStream| -> TokenStream {
            let mut iter = orig_stream.clone().into_iter();
            let first = iter.next();
            let second = iter.next();
            if let Some(TokenTree::Punct(ref punct)) = first
                && punct.as_char() == '.'
                && punct.spacing() == Spacing::Alone
            {
                match second {
                    Some(TokenTree::Literal(lit)) if let Ok(index) = lit.to_string().parse::<u32>() => {
                        fields.check_valid_field()
                    }
                    Some(TokenTree::Ident(ident)) => {
                        todo!()
                    }
                }
            } else {
                orig_stream.clone()
            }
        };
        rewrite_shorthand(stream)
            .into_iter()
            .map(|tree| match tree {
                TokenTree::Group(orig_group) => {
                    let mut new_group = Group::new(
                        orig_group.delimiter(),
                        rewrite_field_ref_shorthand(orig_group.stream(), fields, &mut *acc),
                    );
                    new_group.set_span(orig_group.span());
                    TokenTree::Group(new_group)
                }
                TokenTree::Ident(_) | TokenTree::Punct(_) | TokenTree::Literal(_) => tree,
            })
            .collect()
    }
}
