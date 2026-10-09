# Changelog

Notable changes to this project should be documented in this file.
Make sure it is up to date before performing a release.

This project follows the [Keep a Changelog](https://keepachangelog.com/en/2.0.0/) where possible.

The "title" of each release should be its first line.
A title is required for publishing a github release, so all versions should have one.

## Unreleased

### Fixed

- Prevent `Display` from panicking on raw field identifiers such as `r#type` (mmxtssrp).
- Accept escaped closing braces after `Display` placeholders, including `"{{{value}}}"` (kkoqwmoy).

## 0.1.0-beta.1
Support `#[derive(From)]` for newtype structs.

### Added
- Support `#[derive(From)]` for newtype structs (lvmntwwt)

## 0.1.0-beta.0
Initial release with `#[derive(Display)]`

