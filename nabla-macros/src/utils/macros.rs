macro_rules! wrap_derive_macros {
    (
        $(
            $(#[doc = $doc:expr])*
            #[proc_macro_derive($($derive_spec:tt)*)]
            $(#[$extra:meta])*
            fn $target:ident() => $inner:path;
        )*
    ) => {
        $(
            $(#[doc = $doc])*
            #[proc_macro_derive($($derive_spec)*)]
            $(#[$extra])*
            #[cfg_attr(feature = "unstable-debug-expand", macro_expander::debug_expand_macro)]
            pub fn $target(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
                let input = syn::parse_macro_input!(input as syn::DeriveInput);
                let res: Result<proc_macro2::TokenStream, darling::Error> = $inner(&input);
                res.unwrap_or_else(darling::Error::write_errors).into()
            }
        )*
    }
}
