//! Expands `.field` and `.0` shorthand in explicit format arguments.
//!
//! Ported from `thiserror-impl` (`parse_token_expr` in `attr.rs`), which is
//! also licensed under MIT OR Apache-2.0.

use proc_macro2::{Delimiter, Group, Literal, Punct, Spacing, TokenStream, TokenTree};
use quote::format_ident;
use syn::parse::ParseStream;
use syn::parse::discouraged::Speculative;
use syn::{Ident, Index, LitFloat, LitInt, Token, braced, bracketed, parenthesized, token};

/// The local variable bound to an unnamed field.
pub fn tuple_local(index: usize) -> String {
    format!("_{index}")
}

/// Rewrite `.field` to `field` and `.0` to `_0` wherever an expression begins.
///
/// Fields are bound to locals with these names, so the rewritten tokens refer to them.
/// A `.` that follows an expression (as in `self.0` or `a..0`) is left alone.
pub fn expand(input: ParseStream<'_>, mut begin_expr: bool) -> syn::Result<TokenStream> {
    let mut tokens = Vec::new();
    let mut turbofish = 0usize;
    while !input.is_empty() {
        if input.peek(token::Group) {
            let group: TokenTree = input.parse()?;
            tokens.push(group);
            begin_expr = false;
            continue;
        }

        if begin_expr && input.peek(Token![.]) {
            if input.peek2(Ident) {
                input.parse::<Token![.]>()?;
                begin_expr = false;
                continue;
            } else if input.peek2(LitInt) {
                input.parse::<Token![.]>()?;
                let int: Index = input.parse()?;
                tokens.push(TokenTree::Ident(format_ident!(
                    "{}",
                    tuple_local(int.index as usize),
                    span = int.span
                )));
                begin_expr = false;
                continue;
            } else if input.peek2(LitFloat) {
                // `.0.1` lexes as `.` followed by the float `0.1`.
                let ahead = input.fork();
                ahead.parse::<Token![.]>()?;
                let float: LitFloat = ahead.parse()?;
                let repr = float.to_string();
                let mut indices = repr.split('.').map(syn::parse_str::<Index>);
                if let (Some(Ok(first)), Some(Ok(second)), None) = (indices.next(), indices.next(), indices.next()) {
                    input.advance_to(&ahead);
                    tokens.push(TokenTree::Ident(format_ident!(
                        "{}",
                        tuple_local(first.index as usize),
                        span = float.span()
                    )));
                    let mut punct = Punct::new('.', Spacing::Alone);
                    punct.set_span(float.span());
                    tokens.push(TokenTree::Punct(punct));
                    let mut literal = Literal::u32_unsuffixed(second.index);
                    literal.set_span(float.span());
                    tokens.push(TokenTree::Literal(literal));
                    begin_expr = false;
                    continue;
                }
            }
        }

        let closes_turbofish = if input.peek(Token![<]) {
            if turbofish > 0 || follows_path_sep(&tokens) {
                turbofish += 1;
            }
            false
        } else if turbofish > 0 && input.peek(Token![>]) && !follows_hyphen(&tokens) {
            turbofish -= 1;
            true
        } else {
            false
        };

        begin_expr = input.peek(Token![break])
            || input.peek(Token![continue])
            || input.peek(Token![if])
            || input.peek(Token![in])
            || input.peek(Token![match])
            || input.peek(Token![mut])
            || input.peek(Token![return])
            || input.peek(Token![while])
            || input.peek(Token![+])
            || input.peek(Token![&])
            || input.peek(Token![!])
            || input.peek(Token![^])
            || input.peek(Token![,])
            || input.peek(Token![/])
            || input.peek(Token![=])
            || input.peek(Token![>]) && !closes_turbofish
            || input.peek(Token![<])
            || input.peek(Token![|])
            || input.peek(Token![%])
            || input.peek(Token![;])
            || input.peek(Token![*])
            || input.peek(Token![-]);

        let token = if input.peek(token::Paren) {
            let content;
            let delimiter = parenthesized!(content in input);
            nested_group(Delimiter::Parenthesis, expand(&content, true)?, delimiter.span.join())
        } else if input.peek(token::Brace) {
            let content;
            let delimiter = braced!(content in input);
            nested_group(Delimiter::Brace, expand(&content, true)?, delimiter.span.join())
        } else if input.peek(token::Bracket) {
            let content;
            let delimiter = bracketed!(content in input);
            nested_group(Delimiter::Bracket, expand(&content, true)?, delimiter.span.join())
        } else {
            input.parse()?
        };
        tokens.push(token);
    }
    Ok(TokenStream::from_iter(tokens))
}

fn nested_group(delimiter: Delimiter, stream: TokenStream, span: proc_macro2::Span) -> TokenTree {
    let mut group = Group::new(delimiter, stream);
    group.set_span(span);
    TokenTree::Group(group)
}

fn follows_path_sep(tokens: &[TokenTree]) -> bool {
    matches!(
        tokens.last_chunk(),
        Some([TokenTree::Punct(p), TokenTree::Punct(q)])
            if p.spacing() == Spacing::Joint && p.as_char() == ':' && q.as_char() == ':',
    )
}

fn follows_hyphen(tokens: &[TokenTree]) -> bool {
    matches!(
        tokens.last(),
        Some(TokenTree::Punct(p)) if p.spacing() == Spacing::Joint && p.as_char() == '-',
    )
}

#[cfg(test)]
mod test {
    use syn::parse::Parser;

    fn expand(input: &str) -> String {
        (|input: syn::parse::ParseStream<'_>| super::expand(input, false))
            .parse_str(input)
            .unwrap()
            .to_string()
    }

    #[test]
    fn rewrites_shorthand_at_expression_start() {
        for (input, expected) in [
            (", .field", ", field"),
            (", .0 + .1", ", _0 + _1"),
            (", .0.1", ", _0 . 1"),
            (", f(.0, [.value])", ", f (_0 , [value])"),
            (", x = *.r#type", ", x = *r#type"),
        ] {
            assert_eq!(expand(input), expected, "{input}");
        }
    }

    #[test]
    fn ignores_dots_after_expressions() {
        for input in [
            ", self.0",
            ", self.value",
            ", a..0",
            ", f().0",
            ", Vec::<u8>::new().len()",
        ] {
            assert_eq!(
                expand(input),
                syn::parse_str::<proc_macro2::TokenStream>(input).unwrap().to_string()
            );
        }
    }
}
