use logos::Span;
use miette::Diagnostic;
use prost_types::{source_code_info, FileDescriptorProto, SourceCodeInfo};
use thiserror::Error;

use crate::{ast, compile::ParsedFileMap, MAX_MESSAGE_FIELD_NUMBER};

mod ir;
mod names;
mod span;
#[cfg(test)]
mod tests;

pub(crate) use self::names::NameMap;

pub(crate) fn check(
    ast: &ast::File,
    name: Option<&str>,
    source: Option<&str>,
) -> Result<FileDescriptorProto, Vec<CheckError>> {
    let ir = ir::File::build(ast);
    let source_code_info = source.map(|src| ir.get_source_code_info(src));
    let file_descriptor = ir.check(None)?;

    Ok(FileDescriptorProto {
        name: name.map(ToOwned::to_owned),
        source_code_info,
        ..file_descriptor
    })
}

pub(crate) fn check_with_names(
    ast: &ast::File,
    name: Option<&str>,
    source: Option<&str>,
    file_map: &ParsedFileMap,
) -> Result<(FileDescriptorProto, NameMap), Vec<CheckError>> {
    let ir = ir::File::build(ast);
    let name_map = ir.get_names(file_map)?;
    let source_code_info = source.map(|src| ir.get_source_code_info(src));
    let file_descriptor = ir.check(Some(&name_map))?;

    Ok((
        FileDescriptorProto {
            name: name.map(ToOwned::to_owned),
            source_code_info,
            ..file_descriptor
        },
        name_map,
    ))
}

#[derive(Error, Clone, Debug, Diagnostic, PartialEq)]
pub(crate) enum CheckError {
    #[error("name '{name}' is defined twice")]
    DuplicateNameInFile {
        name: String,
        #[label("first defined here…")]
        first: Span,
        #[label]
        #[label("…and again here")]
        second: Span,
    },
    #[error("name '{name}' is already defined in imported file '{first_file}'")]
    DuplicateNameInFileAndImport {
        name: String,
        first_file: String,
        #[label("defined here")]
        second: Span,
    },
    #[error("name '{name}' is defined twice in imported files '{first_file}' and '{second_file}'")]
    DuplicateNameInImports {
        name: String,
        first_file: String,
        second_file: String,
    },
    #[error("camel-case name of field '{first_name}' conflicts with field '{second_name}'")]
    DuplicateCamelCaseFieldName {
        first_name: String,
        #[label("field defined here…")]
        first: Span,
        second_name: String,
        #[label("…conflicts with field here")]
        second: Span,
    },
    #[error("the type name '{name}' was not found")]
    TypeNameNotFound {
        name: String,
        #[label("used here")]
        span: Span,
    },
    #[error("message field type '{name}' is not a message or enum")]
    InvalidMessageFieldTypeName {
        name: String,
        #[label("used here")]
        span: Span,
    },
    #[error("extendee type '{name}' is not a message")]
    InvalidExtendeeTypeName {
        name: String,
        #[label("used here")]
        span: Span,
    },
    #[error("method {kind} type '{name}' is not a message")]
    InvalidMethodTypeName {
        name: String,
        kind: &'static str,
        #[label("used here")]
        span: Span,
    },
    #[error("message numbers must be between 1 and {}", MAX_MESSAGE_FIELD_NUMBER)]
    InvalidMessageNumber {
        #[label("defined here")]
        span: Span,
    },
    #[error("enum numbers must be between {} and {}", i32::MIN, i32::MAX)]
    InvalidEnumNumber {
        #[label("defined here")]
        span: Span,
    },
    #[error("{kind} fields may not have default values")]
    InvalidDefault {
        kind: &'static str,
        #[label("defined here")]
        span: Span,
    },
    #[error("{kind} fields are not allowed in extensions")]
    InvalidExtendFieldKind {
        kind: &'static str,
        #[label("defined here")]
        span: Span,
    },
    #[error("extension fields may not be required")]
    RequiredExtendField {
        #[label("defined here")]
        span: Span,
    },
    #[error("map fields cannot have labels")]
    MapFieldWithLabel {
        #[label("defined here")]
        span: Span,
    },
    #[error("fields must have a label with proto2 syntax (expected one of 'optional', 'repeated' or 'required')")]
    Proto2FieldMissingLabel {
        #[label("field defined here")]
        span: Span,
    },
    #[error("groups are not allowed in proto3 syntax")]
    Proto3GroupField {
        #[label("defined here")]
        span: Span,
    },
    #[error("required fields are not allowed in proto3 syntax")]
    Proto3RequiredField {
        #[label("defined here")]
        span: Span,
    },
    #[error("{kind} fields are not allowed in a oneof")]
    InvalidOneofFieldKind {
        kind: &'static str,
        #[label("defined here")]
        span: Span,
    },
    #[error("oneof fields cannot have labels")]
    OneofFieldWithLabel {
        #[label("defined here")]
        span: Span,
    },
}

impl<'a> ir::File<'a> {
    fn get_source_code_info(&self, source: &str) -> SourceCodeInfo {
        todo!()
    }

    fn check(&self, names: Option<&NameMap>) -> Result<FileDescriptorProto, Vec<CheckError>> {
        todo!()
    }
}
