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
    for attr in &attrs {
        if attr.path().is_ident("display") {
            if res.is_some() {
                return Err(
                    darling::Error::custom("The #[display(..)] attribute should not be duplicated")
                        .with_span(attr.path()),
                );
            }
            let attr = <DisplayAttr as FromMeta>::from_meta(&attr.meta)?;
            res = Some(attr);
        } else if attr.path().is_ident("nabla") {
            return Err(
                darling::Error::custom("The #[nabla(...)] attribute is not currently implemented")
                    .with_span(attr.path()),
            );
        }
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
                let orig_arg = &*orig_ref.argument;
                if orig_arg.is_empty()
                    || args.keyword_args.keys().any(|key| key == orig_arg)
                    || (!args.positional_args.is_empty() && orig_arg.parse::<usize>().is_ok())
                {
                    new_parts.push(part.clone());
                    continue;
                }
                let member = Ident::parse_any
                    .parse_str(orig_arg)
                    .map(Member::Named)
                    .or_else(|_| syn::parse_str::<Member>(orig_arg));
                let orig_arg = FieldSpec::from(&member.map_err(|cause| {
                    darling::Error::custom(format!("Failed to parse fmt argument: {cause}")).with_span(fmt.lit())
                })?);
                new_arg.argument = field_args
                    .get(&orig_arg)
                    .ok_or_else(|| darling::Error::custom(format!("Unknown field `{orig_arg}`")).with_span(fmt.lit()))?
                    .to_string();
                used_fields.insert(orig_arg);
                FormatStringPart::ArgRef(new_arg)
            }
        });
    }
    Ok(FormatString::from_parts(&new_parts, fmt.span()))
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
    variant.attr.fmt.format_string = rewrite_fmt_str(&variant.attr.fmt, &field_args, &mut used_fields)?;
    for field in &field_specs {
        if !used_fields.contains(field) {
            continue;
        }
        let prefixed_name = Ident::new(&prefix_field(field), Span::call_site());
        let existing = variant
            .attr
            .fmt
            .keyword_args
            .insert(field_args[field].clone(), prefixed_name.to_token_stream());
        assert!(existing.is_none());
    }
    let fmt = &variant.attr.fmt;
    Ok(quote!(write!(__nabla_formatter, #fmt)))
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
        impl core::fmt::Display for #target_name {
            fn fmt(&self, __nabla_formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                #body
            }
        }
    })
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
