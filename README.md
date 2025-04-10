[![crates.io](https://img.shields.io/crates/v/protox.svg)](https://crates.io/crates/protox/)
[![docs.rs](https://docs.rs/protox/badge.svg)](https://docs.rs/protox/)
[![deps.rs](https://deps.rs/crate/protox/0.8.0/status.svg)](https://deps.rs/crate/protox)
![MSRV](https://img.shields.io/badge/rustc-1.70+-blue.svg)
[![Continuous integration](https://github.com/andrewhickman/protox/actions/workflows/ci.yml/badge.svg)](https://github.com/andrewhickman/protox/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/andrewhickman/protox/branch/main/graph/badge.svg?token=9YKHGUUPUX)](https://codecov.io/gh/andrewhickman/protox)
![Apache 2.0 OR MIT licensed](https://img.shields.io/badge/license-Apache2.0%2FMIT-blue.svg)

# protox

An implementation of the protobuf compiler in rust, intended for use as a library with crates such as [`prost-build`](https://crates.io/crates/prost-build) to avoid needing to build `protoc`.

## Examples

Compiling a single source file:

```rust
assert_eq!(protox::compile(["root.proto"], ["."]).unwrap(), FileDescriptorSet {
    file: vec![
        FileDescriptorProto {
            name: Some("root.proto".to_owned()),
            /* ... */
        }
    ],
});
```

Usage with [`prost-build`](https://crates.io/crates/prost-build):

```rust
let file_descriptors = protox::compile(["root.proto"], ["."]).unwrap();
prost_build::compile_fds(file_descriptors).unwrap();
```

Usage with [`tonic-build`](https://crates.io/crates/tonic-build):

```rust
let file_descriptors = protox::compile(["root.proto"], ["."]).unwrap();

tonic_build::configure()
    .build_server(true)
    .compile_fds(file_descriptors)
    .unwrap();
```

### Error messages

This crate uses [`miette`](https://crates.io/crates/miette) to add additional details to errors. For nice error messages, add `miette` as a dependency with the `fancy` feature enabled and return a [`miette::Result`](https://docs.rs/miette/latest/miette/type.Result.html) from your build script.

```rust
fn main() -> miette::Result<()> {
  let _ = protox::compile(["root.proto"], ["."])?;

  Ok(())
}
```

Example error message:

```
Error:
  × name 'Bar' is not defined
   ╭─[root.proto:3:1]
 3 │ message Foo {
 4 │     Bar bar = 1;
   ·     ─┬─
   ·      ╰── found here
 5 │ }
   ╰────
```

## Minimum Supported Rust Version

Rust **1.74** or higher.

The minimum supported Rust version may be changed in the future, but it will be
done with a minor version bump.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

This project includes code imported from the Protocol Buffers project, which is
included under its original ([BSD][2]) license.

[2]: https://github.com/protocolbuffers/protobuf/blob/master/LICENSE

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

## Related projects

  * [prost](https://crates.io/crates/prost) - a protocol buffers implementation for the Rust Language
  * [protoxy](https://github.com/tardyp/protoxy) - python bindings for protox