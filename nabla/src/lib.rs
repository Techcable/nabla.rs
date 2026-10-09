//! Derive macros for rust that are both powerful and flexible.
//!
//! Named after the mathematical symbol for the gradient (multivariable derivative).
//!
//! Available derive macros:
//! - [`Display`](core::fmt::Display) works similar to the [`thiserror::Error`] macro,
//!   but only implements the `Display` trait and not `std::error::Error`.
//!
//! [`thiserror::Error`]: https://docs.rs/thiserror/2/thiserror/derive.Error.html
#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

/// Derive [`core::fmt::Display`] in a manner similar to the [`thiserror::Error`] derive macro.
///
/// Each struct or enum variant can have a `#[display("fmtstr")]` attribute
/// where `"fmtstr"` has access to the fields of the struct/variant.
/// Explicit format arguments accept Rust expressions, including `self.field`.
/// Named arguments override fields with the same name. When positional arguments
/// are supplied, numeric placeholders refer to those arguments; otherwise they
/// refer to tuple fields.
///
/// The `#[nabla(display(...))` attribute means the same thing as `#[display(...)]`.
/// This may be useful to avoid conflicts or for clarity.
///
/// # Examples
/// Using a traditional struct:
/// ```
/// # extern crate nabla_macros as nabla;
/// #[derive(nabla::Display, Debug)]
/// #[display("Hello World from {a} and {b}")]
/// struct Hello {
///     a: u32,
///     b: &'static str,
///     extra: Option<bool>,
/// }
/// assert_eq!(
///     Hello { a: 4, b: "bob", extra: None }.to_string(),
///     "Hello World from 4 and bob"
/// );
/// ```
/// Using an enum:
/// ```
/// #[derive(nabla::Display, Debug)]
/// enum SimpleError {
///     #[display("Expected {expected} items, but got {actual}")]
///     WrongCount {
///         expected: usize,
///         actual: usize
///     },
///     #[display("{0}")]
///     Io(std::io::Error),
///     #[display("Request is too large")]
///     RequestTooLarge,
/// }
/// assert_eq!(
///     SimpleError::WrongCount { expected: 3, actual: 4 }.to_string(),
///     "Expected 3 items, but got 4"
/// );
/// assert_eq!(
///     SimpleError::RequestTooLarge.to_string(),
///     "Request is too large"
/// );
/// let err = std::io::Error::other("IO error message");
/// assert_eq!(
///     SimpleError::Io(err).to_string(),
///     "IO error message",
/// )
/// ```
///
///
/// # Not Yet Implemented
/// - Use of `.0` or `.field` shorthand to reference fields in format args
/// - A `#[display(transparent)]` attribute similar to `#[error(transparent)]` in thisserror.
///   This can be easily emulated by `#[display("{0}")]`
///
/// [`thiserror::Error`]: https://docs.rs/thiserror/2/thiserror/derive.Error.html
pub use nabla_macros::Display;
/// Derive [`core::convert::From`] for newtype structs.
///
/// # Examples
/// ```
/// extern crate nabla_macros as nabla;
///
/// #[derive(nabla::From, Eq, PartialEq, Debug)]
/// struct Wrapper(u32);
/// assert_eq!(
///     <Wrapper as From<u32>>::from(3),
///     Wrapper(3)
/// );
/// ```
pub use nabla_macros::From;
