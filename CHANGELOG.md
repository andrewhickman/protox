# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Fixed name resolution in nested messages
- Fixed source info for oneofs not including comments
- Enums now respect the allow_alias option

## [0.1.0] - 2022-07-25

### Added

- Initial release, implementing most of the functionality of protoc in rust. The main unimplemented features are:
  - Setting extension options in .proto source files is not supported
  - Some validation checks are missing

[Unreleased]: https://github.com/andrewhickman/protox/compare/0.1.0...HEAD
[0.1.0]: https://github.com/andrewhickman/protox/compare/0.0.0...0.1.0