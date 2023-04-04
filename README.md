[![crates.io](https://img.shields.io/crates/v/protox.svg)](https://crates.io/crates/protox/)
[![docs.rs](https://docs.rs/protox/badge.svg)](https://docs.rs/protox/)
[![deps.rs](https://deps.rs/crate/protox/0.3.0/status.svg)](https://deps.rs/crate/protox)
![MSRV](https://img.shields.io/badge/rustc-1.61+-blue.svg)
[![Continuous integration](https://github.com/andrewhickman/protox/actions/workflows/ci.yml/badge.svg)](https://github.com/andrewhickman/protox/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/andrewhickman/protox/branch/main/graph/badge.svg?token=9YKHGUUPUX)](https://codecov.io/gh/andrewhickman/protox)
![Apache 2.0 OR MIT licensed](https://img.shields.io/badge/license-Apache2.0%2FMIT-blue.svg)

# protox

An implementation of the protobuf compiler in rust, intended for use as a library with crates such as [`prost-build`](https://crates.io/crates/prost-build) to avoid needing to build `protoc`.

## Usage

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

Usage with `prost-build`:

```rust
let file_descriptors = compile(["root.proto"], ["."]).unwrap();
let file_descriptor_path = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"))
    .join("file_descriptor_set.bin");
fs::write(&file_descriptor_path, file_descriptors.encode_to_vec()).unwrap();

prost_build::Config::new()
    .file_descriptor_set_path(&file_descriptor_path)
    .skip_protoc_run()
    .compile_protos(["root.proto"], ["."])
    .unwrap();
```

## Minimum Supported Rust Version

Rust **1.61** or higher.

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
