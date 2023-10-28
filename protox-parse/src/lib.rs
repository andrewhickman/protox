//! Parsing of protobuf source files.
//!
//! See the documentation for [`parse()`] for details.
#![warn(missing_debug_implementations, missing_docs)]
#![deny(unsafe_code)]
#![doc(html_root_url = "https://docs.rs/protox-parse/0.5.0/")]

use logos::Span;
use prost_types::FileDescriptorProto;

pub use self::error::ParseError;

mod ast;
mod case;
mod error;
mod generate;
mod lex;
mod parse;
mod tag;
#[cfg(test)]
mod tests;

const MAX_MESSAGE_FIELD_NUMBER: i32 = 536_870_911;

/// Parses a single protobuf source file into a [`FileDescriptorProto`].
///
/// This function only looks at the syntax of the file, without resolving type names or reading
/// imported files.
///
/// # Examples
///
/// ```
/// # use protox_parse::parse;
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
/// let file_descriptor = parse("foo.proto", source).unwrap();
/// assert_eq!(file_descriptor, FileDescriptorProto {
///     name: Some("foo.proto".to_owned()),
///     syntax: Some("proto3".to_owned()),
///     dependency: vec!["dep.proto".to_owned()],
///     message_type: vec![DescriptorProto {
///         name: Some("Foo".to_owned()),
///         field: vec![FieldDescriptorProto {
///             label: Some(Label::Optional as _),
///             name: Some("bar".to_owned()),
///             number: Some(1),
///             type_name: Some("Bar".to_owned()),
///             ..Default::default()
///         }],
///         ..Default::default()
///     }],
///     source_code_info: Some(SourceCodeInfo {
///         location: vec![
///             Location { path: vec![], span: vec![1, 4, 6, 5], ..Default::default() },
///             Location { path: vec![3, 0], span: vec![2, 4, 23], ..Default::default() },
///             Location { path: vec![4, 0], span: vec![4, 4, 6, 5], ..Default::default() },
///             Location { path: vec![4, 0, 1], span: vec![4, 12, 15], ..Default::default() },
///             Location { path: vec![4, 0, 2, 0], span: vec![5, 8, 20], ..Default::default() },
///             Location { path: vec![4, 0, 2, 0, 1], span: vec![5, 12, 15], ..Default::default() },
///             Location { path: vec![4, 0, 2, 0, 3], span: vec![5, 18, 19], ..Default::default() },
///             Location { path: vec![4, 0, 2, 0, 6], span: vec![5, 8, 11], ..Default::default() },
///             Location { path: vec![12], span: vec![1, 4, 22], ..Default::default() },
///         ],
///     }),
///     ..Default::default()
/// })
/// ```
pub fn parse(name: &str, source: &str) -> Result<FileDescriptorProto, ParseError> {
    if source.len() > MAX_FILE_LEN {
        return Err(ParseError::new(
            vec![error::ParseErrorKind::FileTooLarge],
            name,
            String::default(),
        ));
    }

    let ast = parse::parse_file(source)
        .map_err(|errors| ParseError::new(errors, name, source.to_owned()))?;

    generate::generate_file(ast, name, source)
        .map_err(|errors| ParseError::new(errors, name, source.to_owned()))
}

const MAX_FILE_LEN: usize = i32::MAX as usize;

fn index_to_i32(index: usize) -> i32 {
    // We enforce that all files parsed are at most i32::MAX bytes long. Therefore the indices of any
    // definitions in a single file must fit into an i32.
    index.try_into().unwrap()
}

fn join_span(start: Span, end: Span) -> Span {
    start.start..end.end
}
