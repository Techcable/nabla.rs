//! Parses format arguments.

use core::cell::Cell;
use core::error::Error;
use core::fmt::{Debug, Display, Formatter, Write};
use core::str::FromStr;

use darling::ast::NestedMeta;
use indexmap::IndexMap;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, TokenStreamExt};
use syn::{Lit, LitStr, Meta, Path};

/// A format specification that can be passed to [`format_args!`] and friends.
#[derive(Debug, Clone)]
pub struct FormatArgs {
    pub format_string: FormatString,
    pub positional_args: Vec<TokenStream>,
    pub keyword_args: IndexMap<Ident, TokenStream>,
}
impl darling::FromMeta for FormatArgs {
    fn from_list(items: &[NestedMeta]) -> darling::Result<Self> {
        let mut items = items.iter();
        let first = items.next().ok_or_else(|| darling::Error::too_few_items(1))?;
        let format_string = FormatString::from_nested_meta(first)?;
        let mut result = FormatArgs {
            format_string,
            positional_args: Vec::new(),
            keyword_args: IndexMap::new(),
        };
        let seen_keyword_args = Cell::new(false);
        let mut errors = darling::Error::accumulator();
        let handle_kv = |result: &mut FormatArgs, key: &Path, value: &TokenStream| {
            seen_keyword_args.set(true);
            let Some(key) = key.get_ident() else {
                return Err(darling::Error::custom("Keyword arg name must be an identifier").with_span(&key));
            };
            if result.keyword_args.contains_key(key) {
                Err(darling::Error::custom(format!("Key `{key}` is specified more than once")).with_span(&key.span()))
            } else {
                result.keyword_args.insert(key.clone(), value.clone());
                Ok(())
            }
        };
        for item in items {
            match item {
                NestedMeta::Meta(Meta::NameValue(kv)) => {
                    errors.handle(handle_kv(&mut result, &kv.path, &kv.value.to_token_stream()));
                }
                meta @ (NestedMeta::Meta(Meta::Path(_) | Meta::List(_)) | NestedMeta::Lit(_)) => {
                    let positional_arg = meta.to_token_stream();
                    if seen_keyword_args.get() {
                        errors.push(
                            darling::Error::custom("Cannot have positional arguments after keyword arguments")
                                .with_span(&positional_arg),
                        );
                    } else {
                        result.positional_args.push(positional_arg);
                    }
                }
                NestedMeta::NameValueInvalidExpr(value) => {
                    errors.handle(handle_kv(&mut result, &value.path, &value.value));
                }
            }
        }
        errors.finish_with(result)
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
        let current_pos = || original.len() - remaining.len();
        let unpaired_bracket =
            |c: char| FormatStringParseError(format!("Unpaired and unescaped `{c}` at byte index {}", current_pos()));
        'parse: loop {
            match remaining.chars().next() {
                Some('{') => {
                    if let Some(newly_remaining) = remaining.strip_prefix("{{") {
                        remaining = newly_remaining;
                        parts.push(FormatStringPart::EscapedOpenBrace);
                        continue 'parse;
                    } else if let Some(closing_index) = remaining.find('}') {
                        let format_text = &remaining[1..closing_index];
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
                        return Err(unpaired_bracket('}'));
                    }
                }
                Some('}') => {
                    if let Some(newly_remaining) = remaining.strip_prefix("}}") {
                        remaining = newly_remaining;
                        parts.push(FormatStringPart::EscapedCloseBrace);
                    } else {
                        return Err(unpaired_bracket('}'));
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
    use super::FormatString;

    #[test]
    fn braces_after_arguments() {
        for valid in ["{{{value}}}", "{value}}}", "{{{{{value}}}}}"] {
            assert_eq!(valid.parse::<FormatString>().unwrap().text(), valid);
        }
        for invalid in ["{value}}", "{value}}}}", "{value"] {
            assert!(invalid.parse::<FormatString>().is_err(), "{invalid}");
        }
    }
}
