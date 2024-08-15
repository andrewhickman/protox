# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## Fixed

- Improved error spans for import errors ([#80](https://github.com/andrewhickman/protox/pull/80))

## [0.7.0] - 2024-07-08

### Changed
- Updated to prost [**0.13.0**](https://github.com/tokio-rs/prost/releases/tag/v0.13.0)

## [0.6.0] - 2024-02-07

### Changed

- The minimum supported rust version is now **1.70.0**.
- Updated the miette dependency to version [7.0.0](https://crates.io/crates/miette/7.0.0).

## [0.5.1] - 2023-11-02

### Added

- The [`prost`](https://crates.io/crates/prost) and [`prost-reflect`](https://crates.io/crates/prost-reflect) dependencies are now re-exported at the crate root, for easier use in build scripts.

## [0.5.0] - 2023-09-01

### Changed

- The minimum supported rust version is now **1.64.0**.
- Updated to prost [**0.12.0**](https://github.com/tokio-rs/prost/releases/tag/v0.12.0)
- Protox now validates that all referenced types are included in imported files (#6)

## [0.4.1] - 2023-06-25

## Fixed

- Fixed a panic while parsing an invalid file like `{ $`.

## [0.4.0] - 2023-06-18

## Changed

- **Breaking**: `Compiler::files` is now returns an iterator of `FileMetadata` instances, to avoid some clones while compiling a file.

## [0.3.5] - 2023-06-05

### Changed

- The error when reading invalid UTF-8 is now classified as a parse error, not an IO error.

## [0.3.4] - 2023-06-05

### Added

- Added new methods to get error details: `Error::is_parse`, `Error::is_io` and `Error::file`.

## [0.3.3] - 2023-04-27

### Changed

- Updated logos dependency to [0.13.0](https://github.com/maciejhirsz/logos/releases/tag/v0.13).
- Reduce minimum supported rust version to 1.60.0 to match [prost-reflect](https://crates.io/crates/prost-reflect).

## [0.3.1] - 2023-04-10

### Added

- Added the [`Compiler::files`](https://docs.rs/protox/latest/protox/struct.Compiler.html#method.files) method to get all imported files. This may be used to emit [`rerun-if-changed`](https://doc.rust-lang.org/cargo/reference/build-scripts.html#rerun-if-changed) directives in a build script.

### Changed

- The `Debug` representation of `Error` is now more concise and readable, to support usage with `unwrap` in build scripts.

## [0.3.0] - 2023-04-04

### Added

- Added the [`Compiler::descriptor_pool`](https://docs.rs/protox/latest/protox/struct.Compiler.html#method.descriptor_pool) method to get the descriptor pool containing all referenced files.

### Changed

- Updated `prost-reflect` dependency to [0.11.0](https://crates.io/crates/prost-reflect/0.11.0).
- Renamed `File::to_file_descriptor_proto` to `File::file_descriptor_proto` and changed it to return a reference instead of cloning.

## [0.2.2] - 2023-02-19

### Changed

- `protox_parse::parse` will no longer automatically populate the `json_name` field of fields. This behaviour has moved to `prost-reflect` (see (#27)[https://github.com/andrewhickman/prost-reflect/pull/27]), so the behaviour of `protox::compile` is unchanged.

## [0.2.1] - 2023-01-07

### Fixed

- Fixed decoding of `DescriptorSetFileResolver`.

## [0.2.0] - 2023-01-04

### Changed

- **Breaking**: The `parse()` function now takes an additional argument for the file name.
- **Breaking**: `Compiler::add_file` is renamed to `Compiler::open_file`.

### Fixed

- Fixed name resolution in nested messages
- Fixed source info for oneofs not including comments
- Enums now respect the allow_alias option
- Extension options are now supported
- More validation checks have been added (some still remain, see [#5](https://github.com/andrewhickman/prost-reflect/issues/5))

## [0.1.0] - 2022-07-25

### Added

- Initial release, implementing most of the functionality of protoc in rust. The main unimplemented features are:
  - Setting extension options in .proto source files is not supported
  - Some validation checks are missing

[Unreleased]: https://github.com/andrewhickman/protox/compare/0.7.0...HEAD
[0.7.0]: https://github.com/andrewhickman/protox/compare/0.6.1...0.7.0
[0.6.1]: https://github.com/andrewhickman/protox/compare/0.6.0...0.6.1
[0.6.0]: https://github.com/andrewhickman/protox/compare/0.5.1...0.6.0
[0.5.1]: https://github.com/andrewhickman/protox/compare/0.5.0...0.5.1
[0.5.0]: https://github.com/andrewhickman/protox/compare/0.4.1...0.5.0
[0.4.1]: https://github.com/andrewhickman/protox/compare/0.4.0...0.4.1
[0.4.0]: https://github.com/andrewhickman/protox/compare/0.3.5...0.4.0
[0.3.5]: https://github.com/andrewhickman/protox/compare/0.3.4...0.3.5
[0.3.4]: https://github.com/andrewhickman/protox/compare/0.3.3...0.3.4
[0.3.3]: https://github.com/andrewhickman/protox/compare/0.3.1...0.3.3
[0.3.1]: https://github.com/andrewhickman/protox/compare/0.3.0...0.3.1
[0.3.0]: https://github.com/andrewhickman/protox/compare/0.2.2...0.3.0
[0.2.2]: https://github.com/andrewhickman/protox/compare/0.2.1...0.2.2
[0.2.1]: https://github.com/andrewhickman/protox/compare/0.2.0...0.2.1
[0.2.0]: https://github.com/andrewhickman/protox/compare/0.1.0...0.2.0
[0.1.0]: https://github.com/andrewhickman/protox/compare/0.0.0...0.1.0