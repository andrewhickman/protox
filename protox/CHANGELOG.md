# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.1](https://github.com/andrewhickman/protox/compare/protox-v0.9.0...protox-v0.9.1) - 2025-12-01

### Other

- Avoid relying on DecodeError::new
- Remove a couple of references to the package version which release-plz doesn't update automatically
- Remove old reference to the BSD license

## [0.9.0](https://github.com/andrewhickman/protox/compare/protox-v0.8.0...protox-v0.9.0) - 2025-06-13

### Added

- [**breaking**] Update to protox 0.14.0

### Other

- Fix clippy lint
- Include test files in the crate ([#95](https://github.com/andrewhickman/protox/pull/95))
- Update protobuf version ([#94](https://github.com/andrewhickman/protox/pull/94))
- Clarify the license due to bundled protobuf sources ([#92](https://github.com/andrewhickman/protox/pull/92))
- Update MSRV badge in readme
