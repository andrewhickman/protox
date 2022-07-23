//! A rust implementation of the protobuf compiler.
#![warn(missing_debug_implementations, missing_docs)]
#![deny(unsafe_code)]
#![doc(html_root_url = "https://docs.rs/protox/0.1.0/")]

pub mod file;

mod ast;
mod case;
mod check;
mod compile;
mod error;
mod lines;
mod options;
mod parse;
mod tag;
#[cfg(test)]
mod tests;
mod types;

use std::fmt;
use std::sync::Arc;
use std::{convert::TryInto, path::Path};

use lines::LineResolver;
use logos::Span;
use prost::Message;

use crate::types::{source_code_info, FileDescriptorProto};

pub use self::compile::Compiler;
pub use self::error::Error;

/// Convenience function for compiling a set of protobuf files.
///
/// This function is equivalent to:
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
    let mut compiler = compile::Compiler::new(includes)?;

    compiler.include_source_info(true);
    compiler.include_imports(true);

    for file in files {
        compiler.add_file(file)?;
    }

    Ok(compiler.file_descriptor_set())
}

/// Parses a single protobuf source file into a [`FileDescriptorProto`](prost_types::FileDescriptorProto).
///
/// This function only looks at the syntax of the file, without resolving type names or reading
/// imported files.
///
/// # Examples
///
/// ```
/// # use protox::parse;
/// # use prost_types::{DescriptorProto, FieldDescriptorProto, FileDescriptorProto, SourceCodeInfo, source_code_info::Location, field_descriptor_proto::Label};
/// #
/// let source = r#"
///     syntax = "proto3";
///     import "dep.proto";
///
///     message Foo {
///         Bar bar = 1;
///     }
/// "#;
/// let file_descriptor = parse(source).unwrap();
/// assert_eq!(file_descriptor, FileDescriptorProto {
///     syntax: Some("proto3".to_owned()),
///     dependency: vec!["dep.proto".to_owned()],
///     message_type: vec![DescriptorProto {
///         name: Some("Foo".to_owned()),
///         field: vec![FieldDescriptorProto {
///             label: Some(Label::Optional as _),
///             name: Some("bar".to_owned()),
///             number: Some(1),
///             type_name: Some("Bar".to_owned()),
///             json_name: Some("bar".to_owned()),
///             ..Default::default()
///         }],
///         ..Default::default()
///     }],
///     source_code_info: Some(SourceCodeInfo {
///         /* ... */
/// #       location: vec![
/// #            Location { path: vec![], span: vec![1, 4, 6, 5], ..Default::default() },
/// #            Location { path: vec![3, 0], span: vec![2, 4, 23], ..Default::default() },
/// #            Location { path: vec![4, 0], span: vec![4, 4, 6, 5], ..Default::default() },
/// #            Location { path: vec![4, 0, 1], span: vec![4, 12, 15], ..Default::default() },
/// #            Location { path: vec![4, 0, 2, 0], span: vec![5, 8, 20], ..Default::default() },
/// #            Location { path: vec![4, 0, 2, 0, 1], span: vec![5, 12, 15], ..Default::default() },
/// #            Location { path: vec![4, 0, 2, 0, 3], span: vec![5, 18, 19], ..Default::default() },
/// #            Location { path: vec![4, 0, 2, 0, 6], span: vec![5, 8, 11], ..Default::default() },
/// #            Location { path: vec![12], span: vec![1, 4, 22], ..Default::default() },
/// #       ],
///     }),
///     ..Default::default()
/// })
/// ```
pub fn parse(source: &str) -> Result<prost_types::FileDescriptorProto, Error> {
    parse_internal(source, &LineResolver::new(source))
        .map(|file| transcode_file(&file, &mut Vec::new()))
}

fn parse_internal(source: &str, lines: &LineResolver) -> Result<FileDescriptorProto, Error> {
    let ast =
        parse::parse(source).map_err(|errors| Error::parse_errors(errors, Arc::from(source)))?;
    check::generate(ast, lines).map_err(|errors| Error::check_errors(errors, Arc::from(source)))
}

const MAX_FILE_LEN: u64 = i32::MAX as u64;

fn index_to_i32(index: usize) -> i32 {
    // We enforce that all files parsed are at most i32::MAX bytes long. Therefore the indices of any
    // definitions in a single file must fit into an i32.
    index.try_into().unwrap()
}

fn join_span(start: Span, end: Span) -> Span {
    start.start..end.end
}

fn make_name(namespace: &str, name: impl fmt::Display) -> String {
    if namespace.is_empty() {
        name.to_string()
    } else {
        format!("{}.{}", namespace, name)
    }
}

fn make_absolute_name(namespace: &str, name: impl fmt::Display) -> String {
    if namespace.is_empty() {
        format!(".{}", name)
    } else {
        format!(".{}.{}", namespace, name)
    }
}

fn strip_leading_dot(name: &str) -> &str {
    name.strip_prefix('.').unwrap_or(name)
}

fn parse_namespace(name: &str) -> &str {
    match name.rsplit_once('.') {
        Some((namespace, _)) => namespace,
        None => "",
    }
}

fn resolve_span(
    lines: Option<&LineResolver>,
    locations: &[source_code_info::Location],
    path: &[i32],
) -> Option<Span> {
    let lines = lines?;
    let index = locations
        .binary_search_by(|location| location.path.as_slice().cmp(path))
        .ok()?;
    lines.resolve_proto_span(&locations[index].span)
}

fn transcode_file<T, U>(file: &T, buf: &mut Vec<u8>) -> U
where
    T: Message,
    U: Message + Default,
{
    buf.clear();
    buf.reserve(file.encoded_len());
    file.encode(buf)
        .expect("vec should have sufficient capacity");
    U::decode(buf.as_slice()).expect("incompatible message types")
}

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
