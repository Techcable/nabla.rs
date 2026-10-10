# Changelog

Notable changes to this project should be documented in this file.
Make sure it is up to date before performing a release.

This project follows the [Keep a Changelog](https://keepachangelog.com/en/2.0.0/) where possible.

The "title" of each release should be its first line.
A title is required for publishing a github release, so all versions should have one.

## Unreleased

### Added

- Support `#[derive(nabla::From)]` for enum newtype variants selected with a field-level `#[from]`, or every variant with enum-level `#[from(all)]`.
- Accept the equivalent `#[nabla(from)]` and `#[nabla(from(all))]` aliases, including alongside `#[derive(nabla::Display)]`.

### Fixed

- Prevent `Display` from panicking on raw field identifiers such as `r#type` (mmxtssrp).
- Accept escaped closing braces after `Display` placeholders, including `"{{{value}}}"` (kkoqwmoy).
- Honor explicit named and positional `Display` format arguments, including Rust expressions (nxwsuvsu).
- Resolve `Display` width and precision references to struct and enum fields (yzmwpuyu).
- Resolve derived `From` implementations to `core::convert::From` when `From` is shadowed or the prelude is disabled (rkmmztxp).
- Support the documented `#[nabla(display(...))]` alias on structs and enum variants (pvsymkqt).
- Resolve derived `Display` implementations to `core` when `core` or `write!` is shadowed or the prelude is disabled (qonoyzun).
- Report an error instead of panicking when a `Display` placeholder contains a nested `{`, such as `"{x:{}"` (mvmyomxp).

## 0.1.0-beta.1
Support `#[derive(From)]` for newtype structs.

### Added
- Support `#[derive(From)]` for newtype structs (lvmntwwt)

## 0.1.0-beta.0
Initial release with `#[derive(Display)]`

