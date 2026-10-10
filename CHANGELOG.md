# Changelog

Notable changes to this project should be documented in this file.
Make sure it is up to date before performing a release.

This project follows the [Keep a Changelog](https://keepachangelog.com/en/2.0.0/) where possible.

The "title" of each release should be its first line.
A title is required for publishing a github release, so all versions should have one.

## Unreleased

## 0.1.0 - 2026-10-09
Improve `Display` argument handling.

Improved handling of display arguments and many other fixes,
largely caught and fixed by LLMs.

In particular, the implementation of`#[derive(nabla::Display)]`
has been brought closer to the implementation of `#[derive(thiserror::Error)]`

Also support `#[derive(From)]` for enums.

### Added

- Support `#[derive(nabla::From)]` for enum newtype variants selected with a field-level `#[from]`, or every variant with enum-level `#[from(all)]`.
- Accept the equivalent `#[nabla(from)]` and `#[nabla(from(all))]` aliases, including alongside `#[derive(nabla::Display)]`.

### Changed

- Change the internal implementation of `Display` fields to use locals named after them, as thiserror does (utpvlmrl)
  - Allows format strings to capture fields in width and precision parameters and explicit arguments can use fields directly or through the `.field`/`.0` shorthand
- Reject numeric placeholders combined with explicit positional arguments in tuple structs and variants as ambiguous
  - Numeric width and precision parameters such as `{0:1$}` no longer refer to tuple fields (utpvlmrl).
- Pin `nabla-macros` version to match the `nabla` crate (oylvvrkx)

### Fixed

- Prevent `Display` from panicking on raw field identifiers such as `r#type` (mmxtssrp).
- Accept escaped closing braces after `Display` placeholders, including `"{{{value}}}"` (kkoqwmoy).
- Honor explicit named and positional `Display` format arguments, including Rust expressions (nxwsuvsu).
- Resolve `Display` width and precision references to struct and enum fields (yzmwpuyu).
- Resolve derived `From` implementations to `core::convert::From` when `From` is shadowed or the prelude is disabled (rkmmztxp).
- Support the documented `#[nabla(display(...))]` alias on structs and enum variants (pvsymkqt).
- Resolve derived `Display` implementations to `core` when `core` or `write!` is shadowed or the prelude is disabled (qonoyzun).
- Report an error instead of panicking when a `Display` placeholder contains a nested `{`, such as `"{x:{}"` (mvmyomxp).
- Report the correct brace and byte index for unpaired braces in `Display` format strings (svqkmroy).
- Reject `#[display(...)]` and unknown `#[nabla(...)]` options on fields in `Display` derives instead of silently ignoring them (ppyrqzun).

## 0.1.0-beta.1
Support `#[derive(From)]` for newtype structs.

### Added
- Support `#[derive(From)]` for newtype structs (lvmntwwt)

## 0.1.0-beta.0
Initial release with `#[derive(Display)]`

