#[macro_use]
mod macros;
pub mod destructure;
pub mod fmt_args;
pub mod shorthand;

/// Collect direct and namespaced options for one derive, allowing other nabla derives to coexist.
pub fn derive_attrs(attrs: &[syn::Attribute], name: &str) -> darling::Result<Vec<syn::Meta>> {
    use quote::ToTokens;
    use syn::punctuated::Punctuated;
    use syn::{Meta, Token};

    let mut result = Vec::new();
    for attr in attrs {
        if attr.path().is_ident(name) {
            result.push(attr.meta.clone());
        } else if attr.path().is_ident("nabla") {
            for meta in attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)? {
                if meta.path().is_ident(name) {
                    result.push(meta);
                } else if !meta.path().is_ident("display") && !meta.path().is_ident("from") {
                    return Err(
                        darling::Error::unknown_field(&meta.path().to_token_stream().to_string())
                            .with_span(meta.path()),
                    );
                }
            }
        }
    }
    Ok(result)
}
