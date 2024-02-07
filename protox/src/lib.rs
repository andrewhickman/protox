//! A rust implementation of the protobuf compiler.
//!
//! For convenient compilation of protobuf source files in a single function, see
//! [`compile()`]. For more options see [`Compiler`].
//!
//! # Examples
//!
//! Usage with [`prost-build`](https://crates.io/crates/prost-build):
//!
//! ```
//! # use std::{env, fs, path::PathBuf};
//! # use prost::Message;
//! # let tempdir = tempfile::TempDir::new().unwrap();
//! # env::set_current_dir(&tempdir).unwrap();
//! # env::set_var("OUT_DIR", tempdir.path());
//! # fs::write("root.proto", "").unwrap();
//! let file_descriptors = protox::compile(["root.proto"], ["."]).unwrap();
//! prost_build::compile_fds(file_descriptors).unwrap();
//! ```
//!
//! Usage with [`tonic-build`](https://crates.io/crates/tonic-build):
//!
//! ```rust
//! # use std::{env, fs, path::PathBuf};
//! # let tempdir = tempfile::TempDir::new().unwrap();
//! # env::set_current_dir(&tempdir).unwrap();
//! # env::set_var("OUT_DIR", tempdir.path());
//! # fs::write("root.proto", "").unwrap();
//! use protox::prost::Message;
//!
//! let file_descriptors = protox::compile(["root.proto"], ["."]).unwrap();
//!
//! let file_descriptor_path = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"))
//!     .join("file_descriptor_set.bin");
//! fs::write(&file_descriptor_path, file_descriptors.encode_to_vec()).unwrap();
//!
//! tonic_build::configure()
//!     .build_server(true)
//!     .file_descriptor_set_path(&file_descriptor_path)
//!     .skip_protoc_run()
//!     .compile(&["root.proto"], &["."])
//!     .unwrap();
//! ```
//!
//! ### Error messages
//!
//! This crate uses [`miette`](https://crates.io/crates/miette) to add additional details to errors. For nice error messages, add `miette` as a dependency with the `fancy` feature enabled and return a [`miette::Result`](https://docs.rs/miette/latest/miette/type.Result.html) from your build script.
//!
//! ```rust
//! # use std::{env, fs, path::PathBuf};
//! # let tempdir = tempfile::TempDir::new().unwrap();
//! # env::set_current_dir(&tempdir).unwrap();
//! # env::set_var("OUT_DIR", tempdir.path());
//! # fs::write("root.proto", "").unwrap();
//! fn main() -> miette::Result<()> {
//!   let _ = protox::compile(["root.proto"], ["."])?;
//!
//!   Ok(())
//! }
//! ```
//!
//! Example error message:
//!
//! ```text
//! Error:
//!   × name 'Bar' is not defined
//!    ╭─[root.proto:3:1]
//!  3 │ message Foo {
//!  4 │     Bar bar = 1;
//!    ·     ─┬─
//!    ·      ╰── found here
//!  5 │ }
//!    ╰────
//! ```
#![warn(missing_debug_implementations, missing_docs)]
#![deny(unsafe_code)]
#![doc(html_root_url = "https://docs.rs/protox/0.6.0/")]

pub mod file;

mod compile;
mod error;

use std::path::Path;

pub use {prost, prost_reflect};

pub use self::compile::Compiler;
pub use self::error::Error;

/// Compiles a set of protobuf files using the given include paths.
///
/// For more control over how files are compiled, see [`Compiler`]. This function is equivalent to:
///
/// ```rust
/// # use protox::Compiler;
/// # fn main() -> Result<(), protox::Error> {
/// # let files: Vec<std::path::PathBuf> = vec![];
/// # let includes: Vec<std::path::PathBuf> = vec![".".into()];
/// let file_descriptor_set = Compiler::new(includes)?
///     .include_source_info(true)
///     .include_imports(true)
///     .open_files(files)?
///     .file_descriptor_set();
/// # Ok(())
/// # }
/// ```
///
/// # Examples
///
/// ```
/// # use std::fs;
/// # use prost_types::{
/// #    DescriptorProto, FieldDescriptorProto, field_descriptor_proto::{Label, Type}, FileDescriptorSet, FileDescriptorProto,
/// #    SourceCodeInfo, source_code_info::Location
/// # };
/// # use protox::compile;
/// # let tempdir = tempfile::TempDir::new().unwrap();
/// # std::env::set_current_dir(&tempdir).unwrap();
/// #
/// fs::write("bar.proto", "
///     message Bar { }
/// ").unwrap();
/// fs::write("root.proto", "
///     import 'bar.proto';
///
///     message Foo {
///         optional Bar bar = 1;
///     }
/// ").unwrap();
///
/// assert_eq!(compile(["root.proto"], ["."]).unwrap(), FileDescriptorSet {
///     file: vec![
///         FileDescriptorProto {
///             name: Some("bar.proto".to_owned()),
///             message_type: vec![DescriptorProto {
///                 name: Some("Bar".to_owned()),
///                 ..Default::default()
///             }],
///             source_code_info: Some(SourceCodeInfo {
///                 location: vec![
///                     Location { path: vec![], span: vec![1, 4, 19], ..Default::default() },
///                     Location { path: vec![4, 0], span: vec![1, 4, 19], ..Default::default() },
///                     Location { path: vec![4, 0, 1], span: vec![1, 12, 15], ..Default::default() },
///                ],
///             }),
///             ..Default::default()
///         },
///         FileDescriptorProto {
///             name: Some("root.proto".to_owned()),
///             dependency: vec!["bar.proto".to_owned()],
///             message_type: vec![DescriptorProto {
///                 name: Some("Foo".to_owned()),
///                 field: vec![FieldDescriptorProto {
///                     name: Some("bar".to_owned()),
///                     number: Some(1),
///                     label: Some(Label::Optional as _),
///                     r#type: Some(Type::Message as _),
///                     type_name: Some(".Bar".to_owned()),
///                     json_name: Some("bar".to_owned()),
///                     ..Default::default()
///                 }],
///                 ..Default::default()
///             }],
///             source_code_info: Some(SourceCodeInfo {
///                 location: vec![
///                     Location { path: vec![], span: vec![1, 4, 5, 5], ..Default::default() },
///                     Location { path: vec![3, 0], span: vec![1, 4, 23], ..Default::default() },
///                     Location { path: vec![4, 0], span: vec![3, 4, 5, 5], ..Default::default() },
///                     Location { path: vec![4, 0, 1], span: vec![3, 12, 15], ..Default::default() },
///                     Location { path: vec![4, 0, 2, 0], span: vec![4, 8, 29], ..Default::default() },
///                     Location { path: vec![4, 0, 2, 0, 1], span: vec![4, 21, 24], ..Default::default() },
///                     Location { path: vec![4, 0, 2, 0, 3], span: vec![4, 27, 28], ..Default::default() },
///                     Location { path: vec![4, 0, 2, 0, 4], span: vec![4, 8, 16], ..Default::default() },
///                     Location { path: vec![4, 0, 2, 0, 6], span: vec![4, 17, 20], ..Default::default() },
///                 ],
///             }),
///             ..Default::default()
///         },
///     ],
///     ..Default::default()
/// });
/// ```
pub fn compile(
    files: impl IntoIterator<Item = impl AsRef<Path>>,
    includes: impl IntoIterator<Item = impl AsRef<Path>>,
) -> Result<prost_types::FileDescriptorSet, Error> {
    Ok(Compiler::new(includes)?
        .include_source_info(true)
        .include_imports(true)
        .open_files(files)?
        .file_descriptor_set())
}
