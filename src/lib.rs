//! A rust implementation of the protobuf compiler.
#![warn(missing_debug_implementations, missing_docs)]
#![deny(unsafe_code)]
#![doc(html_root_url = "https://docs.rs/protox/0.1.0/")]

mod ast;
mod case;
mod check;
mod compile;
mod error;
mod files;
mod lines;
mod parse;

use std::sync::Arc;
use std::{convert::TryInto, path::Path};

use logos::Span;
use prost_types::{FileDescriptorProto, FileDescriptorSet};

pub use self::compile::Compiler;
pub use self::error::Error;
pub use self::files::{File, FileImportResolver, ImportResolver};

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
pub fn compile(
    files: impl IntoIterator<Item = impl AsRef<Path>>,
    includes: impl IntoIterator<Item = impl AsRef<Path>>,
) -> Result<FileDescriptorSet, Error> {
    let mut compiler = compile::Compiler::new(includes)?;

    compiler.include_source_info(true);
    compiler.include_imports(true);

    for file in files {
        compiler.add_file(file)?;
    }

    Ok(compiler.file_descriptor_set())
}

/// Parses a single protobuf source file into a [`FileDescriptorProto`].
///
/// This function only looks at the syntax of the file, without resolving type names or reading
/// imported files.
///
/// # Examples
///
/// ```
/// # use protox::parse;
/// # use prost_types::{DescriptorProto, FieldDescriptorProto, FileDescriptorProto};
/// # use prost_types::field_descriptor_proto::Label;
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
///     source_code_info: Some(Default::default()),
///     ..Default::default()
/// })
/// ```
pub fn parse(source: &str) -> Result<FileDescriptorProto, Error> {
    let ast =
        parse::parse(source).map_err(|errors| Error::parse_errors(errors, Arc::from(source)))?;
    match ast.to_file_descriptor(None, Some(source), None) {
        Ok((file_descriptor, _)) => Ok(file_descriptor),
        Err(errors) => Err(Error::check_errors(errors, Arc::from(source))),
    }
}

const MAX_MESSAGE_FIELD_NUMBER: i32 = 536870911;
const MAX_FILE_LEN: u64 = i32::MAX as u64;

fn index_to_i32(index: usize) -> i32 {
    // We enforce that all files parsed are at most i32::MAX bytes long. Therefore the indices of any
    // definitions in a single file must fit into an i32.
    index.try_into().unwrap()
}

fn s(s: impl ToString) -> Option<String> {
    Some(s.to_string())
}

fn join_span(start: Span, end: Span) -> Span {
    start.start..end.end
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
