//! The underlying proc-macro crate for `nabla`.
//!
//! Do not use this crate directly.

#[macro_use]
mod utils;
pub(crate) mod std;

extern crate proc_macro;

wrap_derive_macros! {
    #[proc_macro_derive(Display, attributes(nabla, display))]
    fn derive_display() => std::display::derive_display;
}

wrap_derive_macros! {
    #[proc_macro_derive(From)]
    fn derive_from() => std::from::derive_from;
}
