//! A rust implementation of the protobuf compiler.
//!
//! For convenient compilation of protobuf source files in a single function, see
//! [`compile()`]. For more options see [`Compiler`].
//!
//! # Examples
//!
//! Usage with prost-build:
//!
//! ```
//! # use std::{env, fs, path::PathBuf};
//! # use prost::Message;
//! # use protox::compile;
//! # let tempdir = assert_fs::TempDir::new().unwrap();
//! # env::set_current_dir(&tempdir).unwrap();
//! # env::set_var("OUT_DIR", tempdir.path());
//! # fs::write("root.proto", "").unwrap();
//! let file_descriptors = compile(&["root.proto"], &["."]).unwrap();
//! let file_descriptor_path = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"))
//!     .join("file_descriptor_set.bin");
//! fs::write(&file_descriptor_path, file_descriptors.encode_to_vec()).unwrap();
//!
//! prost_build::Config::new()
//!     .file_descriptor_set_path(&file_descriptor_path)
//!     .skip_protoc_run()
//!     .compile_protos(&["root.proto"], &["."])
//!     .unwrap();
//! ```
#![warn(missing_debug_implementations, missing_docs)]
#![deny(unsafe_code)]
#![doc(html_root_url = "https://docs.rs/protox/0.1.0/")]

pub mod file;

mod compile;
mod error;
mod tag;

use std::path::Path;

pub use self::compile::Compiler;
pub use self::error::Error;

/// Convenience function for compiling a set of protobuf files.
///
/// For more control over how files are compiled, see [`Compiler`]. This function is equivalent to:
/// ```rust
/// # use protox::Compiler;
/// # fn main() -> Result<(), protox::Error> {
/// # let files: Vec<std::path::PathBuf> = vec![];
/// # let includes: Vec<std::path::PathBuf> = vec![".".into()];
/// let mut compiler = Compiler::new(includes)?;
/// compiler.include_source_info(true);
/// compiler.include_imports(true);
/// for file in files {
///     compiler.add_file(file)?;
/// }
/// compiler.file_descriptor_set();
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
/// # let tempdir = assert_fs::TempDir::new().unwrap();
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
/// assert_eq!(compile(&["root.proto"], &["."]).unwrap(), FileDescriptorSet {
///     file: vec![
///         FileDescriptorProto {
///             name: Some("bar.proto".to_owned()),
///             message_type: vec![DescriptorProto {
///                 name: Some("Bar".to_owned()),
///                 ..Default::default()
///             }],
///             source_code_info: Some(SourceCodeInfo {
///                 /* ... */
/// #               location: vec![
/// #                   Location { path: vec![], span: vec![1, 4, 19], ..Default::default() },
/// #                   Location { path: vec![4, 0], span: vec![1, 4, 19], ..Default::default() },
/// #                   Location { path: vec![4, 0, 1], span: vec![1, 12, 15], ..Default::default() },
/// #               ],
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
///                 /* ... */
/// #               location: vec![
/// #                   Location { path: vec![], span: vec![1, 4, 5, 5], ..Default::default() },
/// #                   Location { path: vec![3, 0], span: vec![1, 4, 23], ..Default::default() },
/// #                   Location { path: vec![4, 0], span: vec![3, 4, 5, 5], ..Default::default() },
/// #                   Location { path: vec![4, 0, 1], span: vec![3, 12, 15], ..Default::default() },
/// #                   Location { path: vec![4, 0, 2, 0], span: vec![4, 8, 29], ..Default::default() },
/// #                   Location { path: vec![4, 0, 2, 0, 1], span: vec![4, 21, 24], ..Default::default() },
/// #                   Location { path: vec![4, 0, 2, 0, 3], span: vec![4, 27, 28], ..Default::default() },
/// #                   Location { path: vec![4, 0, 2, 0, 4], span: vec![4, 8, 16], ..Default::default() },
/// #                   Location { path: vec![4, 0, 2, 0, 6], span: vec![4, 17, 20], ..Default::default() },
/// #               ],
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
    let mut compiler = Compiler::new(includes)?;

    compiler.include_source_info(true);
    compiler.include_imports(true);

    for file in files {
        compiler.add_file(file)?;
    }

    Ok(compiler.file_descriptor_set())
}

const MAX_FILE_LEN: u64 = i32::MAX as u64;

#[cfg(test)]
fn with_current_dir(path: impl AsRef<Path>, f: impl FnOnce()) {
    use std::{
        env::{current_dir, set_current_dir},
        sync::Mutex,
    };

    use once_cell::sync::Lazy;
    use scopeguard::defer;

    static CURRENT_DIR_LOCK: Lazy<Mutex<()>> = Lazy::new(Default::default);

    let _lock = CURRENT_DIR_LOCK
        .lock()
        .unwrap_or_else(|err| err.into_inner());

    let prev_dir = current_dir().unwrap();
    defer!({
        let _ = set_current_dir(prev_dir);
    });

    set_current_dir(path).unwrap();
    f();
}
