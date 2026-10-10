//! Parses format arguments.

use core::error::Error;
use core::fmt::{Debug, Display, Formatter, Write};
use core::str::FromStr;

use indexmap::IndexMap;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, TokenStreamExt};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Lit, LitStr, Token};

/// A format specification that can be passed to [`format_args!`] and friends.
#[derive(Debug, Clone)]
pub struct FormatArgs {
    pub format_string: FormatString,
    pub positional_args: Vec<TokenStream>,
    pub keyword_args: IndexMap<Ident, TokenStream>,
}
impl Parse for FormatArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let literal: LitStr = input.parse()?;
        let format_string = FormatString::from_str_spanned(&literal.value(), literal.span())
            .map_err(|error| syn::Error::new(literal.span(), error))?;
        let mut result = FormatArgs {
            format_string,
            positional_args: Vec::new(),
            keyword_args: IndexMap::new(),
        };
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }
            if input.peek(Ident::peek_any) && input.peek2(Token![=]) {
                let key = input.call(Ident::parse_any)?.unraw();
                input.parse::<Token![=]>()?;
                let value: Expr = input.parse()?;
                if result
                    .keyword_args
                    .insert(key.clone(), value.to_token_stream())
                    .is_some()
                {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("Key `{key}` is specified more than once"),
                    ));
                }
            } else {
                if !result.keyword_args.is_empty() {
                    return Err(input.error("Cannot have positional arguments after keyword arguments"));
                }
                result.positional_args.push(input.parse::<Expr>()?.to_token_stream());
            }
        }
        Ok(result)
    }
}
impl ToTokens for FormatArgs {
    fn to_tokens(&self, dest: &mut TokenStream) {
        fn insert_comma(dest: &mut TokenStream) {
            <syn::Token![,] as Default>::default().to_tokens(dest);
        }
        self.format_string.lit().to_tokens(dest);
        for arg in &self.positional_args {
            insert_comma(dest);
            dest.append_all(arg.clone());
        }
        for (name, value) in &self.keyword_args {
            insert_comma(dest);
            dest.append(name.clone());
            <syn::Token![=] as Default>::default().to_tokens(dest);
            dest.append_all(value.clone());
        }
    }
}

/// The format specification string.
#[derive(Debug, Clone)]
pub struct FormatString {
    lit: LitStr,
    text: String,
    parts: Vec<FormatStringPart>,
}
impl FormatString {
    pub fn lit(&self) -> &LitStr {
        &self.lit
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn parts(&self) -> &[FormatStringPart] {
        &self.parts
    }

    #[track_caller]
    pub fn from_parts(parts: &[FormatStringPart], span: Span) -> Self {
        let mut text = String::new();
        for part in parts {
            part.validate();
            write!(&mut text, "{part}").expect("write! to String is infallible");
        }
        FormatString {
            lit: LitStr::new(&text, span),
            text,
            parts: parts.to_vec(),
        }
    }

    pub fn from_str_spanned(original: &str, span: Span) -> Result<Self, FormatStringParseError> {
        let mut remaining = original;
        let mut parts = Vec::new();
        // Takes `remaining` explicitly, since capturing it would copy the initial value.
        let pos = |remaining: &str| original.len() - remaining.len();
        let unpaired_bracket = |c: char, remaining: &str| {
            FormatStringParseError(format!("Unpaired and unescaped `{c}` at byte index {}", pos(remaining)))
        };
        'parse: loop {
            match remaining.chars().next() {
                Some('{') => {
                    if let Some(newly_remaining) = remaining.strip_prefix("{{") {
                        remaining = newly_remaining;
                        parts.push(FormatStringPart::EscapedOpenBrace);
                        continue 'parse;
                    } else if let Some(closing_index) = remaining.find('}') {
                        let format_text = &remaining[1..closing_index];
                        if let Some(nested_index) = format_text.find('{') {
                            return Err(FormatStringParseError(format!(
                                "Unexpected `{{` inside placeholder at byte index {}",
                                pos(remaining) + 1 + nested_index
                            )));
                        }
                        remaining = &remaining[closing_index + 1..];
                        let arg = if let Some((arg, spec)) = format_text.split_once(':') {
                            FormatArgRef {
                                argument: arg.into(),
                                fmt_spec: Some(spec.into()),
                            }
                        } else {
                            FormatArgRef {
                                argument: format_text.into(),
                                fmt_spec: None,
                            }
                        };
                        parts.push(FormatStringPart::ArgRef(arg));
                    } else {
                        return Err(unpaired_bracket('{', remaining));
                    }
                }
                Some('}') => {
                    if let Some(newly_remaining) = remaining.strip_prefix("}}") {
                        remaining = newly_remaining;
                        parts.push(FormatStringPart::EscapedCloseBrace);
                    } else {
                        return Err(unpaired_bracket('}', remaining));
                    }
                }
                Some(other) => {
                    remaining = &remaining[other.len_utf8()..];
                    if let Some(FormatStringPart::Literal(part)) = parts.last_mut() {
                        part.push(other);
                    } else {
                        parts.push(FormatStringPart::Literal(String::from(other)));
                    }
                }
                None => {
                    #[cfg(test)]
                    assert_eq!(
                        Self::from_parts(&parts, Span::call_site()).text,
                        original,
                        "Text {original:?} does not roundtrip"
                    );
                    return Ok(FormatString {
                        lit: LitStr::new(original, span),
                        text: original.into(),
                        parts,
                    });
                }
            }
        }
    }
}
impl darling::FromMeta for FormatString {
    fn from_value(value: &Lit) -> darling::Result<Self> {
        if let Lit::Str(format_string) = value {
            FormatString::from_str_spanned(&format_string.value(), format_string.span())
                .map_err(|cause| darling::Error::custom(cause).with_span(&format_string))
        } else {
            Err(darling::Error::unexpected_lit_type(value))
        }
    }
}
#[derive(Debug)]
pub struct FormatStringParseError(String);
impl Error for FormatStringParseError {}
impl Display for FormatStringParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "Failed to parse format string: {}", self.0)
    }
}
impl Display for FormatString {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.text())
    }
}
impl FromStr for FormatString {
    type Err = FormatStringParseError;

    fn from_str(original: &str) -> Result<Self, Self::Err> {
        Self::from_str_spanned(original, Span::call_site())
    }
}
impl ToTokens for FormatString {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.lit.to_tokens(tokens);
    }
}
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FormatStringPart {
    Literal(String),
    EscapedOpenBrace,
    EscapedCloseBrace,
    ArgRef(FormatArgRef),
}
impl FormatStringPart {
    /// Check this part is valid, raising an error otherwise.
    ///
    /// The result of parsing a [`FormatString`] is always valid.
    #[track_caller]
    pub fn validate(&self) {
        match self {
            FormatStringPart::Literal(text) => {
                validate::forbid_chars(self, text, validate::BRACE_CHARS);
            }
            FormatStringPart::EscapedOpenBrace | FormatStringPart::EscapedCloseBrace => { /* always valid */ }
            FormatStringPart::ArgRef(arg_ref) => {
                arg_ref.validate();
            }
        }
    }
}
impl Display for FormatStringPart {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            FormatStringPart::Literal(text) => f.write_str(text),
            FormatStringPart::EscapedOpenBrace => f.write_str("{{"),
            FormatStringPart::EscapedCloseBrace => f.write_str("}}"),
            FormatStringPart::ArgRef(fmt_arg) => {
                // NOTE: <FormatArg as Display> already wraps in braces
                write!(f, "{fmt_arg}")
            }
        }
    }
}
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FormatArgRef {
    pub argument: String,
    pub fmt_spec: Option<String>,
}
impl FormatArgRef {
    #[track_caller]
    fn validate(&self) {
        validate::forbid_chars(&format_args!("argument in {self:?}"), &self.argument, &['{', '}', ':']);
        if let Some(ref fmt_spec) = self.fmt_spec {
            validate::forbid_chars(&format_args!("fmt spec in {self:?}"), fmt_spec, validate::BRACE_CHARS);
        }
    }
}
impl Display for FormatArgRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{{arg}", arg = self.argument)?;
        if let Some(ref spec) = self.fmt_spec {
            write!(f, ":{spec}")?;
        }
        f.write_char('}')
    }
}

mod validate {
    use core::fmt::Debug;

    pub const BRACE_CHARS: &[char] = &['{', '}'];
    #[track_caller]
    pub fn forbid_chars(this: &dyn Debug, s: &str, forbidden: &[char]) {
        for c in s.chars() {
            assert!(!forbidden.contains(&c), "{this:?} contains invalid character `{c}`");
        }
    }
}

#[cfg(test)]
mod test {
    use super::{FormatArgs, FormatString};

    #[test]
    fn invalid_explicit_arguments() {
        for (input, expected) in [
            (r#""{x}", x = 1, x = 2"#, "specified more than once"),
            (r#""{x}", x = 1, 2"#, "positional arguments after keyword"),
        ] {
            let error = syn::parse_str::<FormatArgs>(input).unwrap_err();
            assert!(error.to_string().contains(expected), "{error}");
        }
    }

    #[test]
    fn braces_after_arguments() {
        for valid in ["{{{value}}}", "{value}}}", "{{{{{value}}}}}"] {
            assert_eq!(valid.parse::<FormatString>().unwrap().text(), valid);
        }
        for invalid in ["{value}}", "{value}}}}", "{value"] {
            assert!(invalid.parse::<FormatString>().is_err(), "{invalid}");
        }
    }

    #[test]
    fn unpaired_brace_errors() {
        for (input, expected) in [
            ("{value", "`{` at byte index 0"),
            ("ab {value", "`{` at byte index 3"),
            ("ab }", "`}` at byte index 3"),
            ("{a} {b}}", "`}` at byte index 7"),
        ] {
            let error = input.parse::<FormatString>().unwrap_err().to_string();
            assert!(error.contains(expected), "{input}: {error}");
        }
    }

    #[test]
    fn nested_open_brace_in_placeholder() {
        for (input, index) in [("{x:{}", 3), ("{a{b}", 2), ("ab{x:>{width}$}", 6)] {
            let error = input.parse::<FormatString>().unwrap_err().to_string();
            assert!(
                error.contains(&format!("Unexpected `{{` inside placeholder at byte index {index}")),
                "{input}: {error}"
            );
        }
    }
}
