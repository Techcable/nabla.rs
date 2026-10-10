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
/// As in thiserror, fields are bound to local references named after them
/// (`_0`, `_1`, ... for tuple fields), so placeholders, width and precision
/// parameters (`{value:width$.precision$}`), and explicit format arguments can all use them.
/// Explicit arguments also accept the `.field` and `.0` shorthand, such as
/// `#[display("{}", .value + 1)]`.
///
/// Named arguments override fields with the same name.
/// In a tuple struct or variant, numeric placeholders such as `{0}` refer to fields,
/// so they cannot be combined with explicit positional arguments;
/// use a named argument instead.
///
/// The `#[nabla(display(...))]` attribute means the same thing as `#[display(...)]`.
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
/// - A `#[display(transparent)]` attribute similar to `#[error(transparent)]` in thisserror.
///   This can be easily emulated by `#[display("{0}")]`
///
/// [`thiserror::Error`]: https://docs.rs/thiserror/2/thiserror/derive.Error.html
pub use nabla_macros::Display;
/// Derive [`core::convert::From`] for newtype structs and enum variants.
///
/// On enums, put `#[from]` on the single tuple field of each variant that should
/// have a conversion. Unmarked variants are ignored. Alternatively, put
/// `#[from(all)]` on the enum to generate conversions for every variant; this
/// requires every variant to have exactly one unnamed field.
/// The namespaced aliases `#[nabla(from)]` on a field and `#[nabla(from(all))]`
/// on an enum behave identically to the direct attributes.
///
/// Selected variants must have distinct source types, including after resolving
/// type aliases, so the generated implementations do not overlap. Generic types
/// are not currently supported.
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
///
/// Selecting individual enum variants:
/// ```
/// #[derive(nabla::From, Debug, PartialEq)]
/// enum Value {
///     Number(#[from] u32),
///     Text(#[from] String),
///     Empty,
/// }
/// assert_eq!(Value::from(42_u32), Value::Number(42));
/// assert_eq!(Value::from(String::from("hello")), Value::Text(String::from("hello")));
/// ```
///
/// Selecting every variant:
/// ```
/// #[derive(nabla::From, Debug, PartialEq)]
/// #[from(all)]
/// enum Value {
///     Number(u32),
///     Flag(bool),
/// }
/// assert_eq!(Value::from(true), Value::Flag(true));
/// ```
///
/// `#[from(all)]` rejects non-newtype variants:
/// ```compile_fail
/// #[derive(nabla::From)]
/// #[from(all)]
/// enum Value {
///     Number(u32),
///     Empty,
/// }
/// ```
///
/// Conversions from the same source type cannot target multiple variants:
/// ```compile_fail
/// #[derive(nabla::From)]
/// enum Value {
///     First(#[from] u32),
///     Second(#[from] u32),
/// }
/// ```
pub use nabla_macros::From;
