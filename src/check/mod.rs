use std::ops::Range;

use logos::Span;
use miette::Diagnostic;
use thiserror::Error;

use crate::{ast, compile::ParsedFileMap, types::FileDescriptorProto};

#[allow(clippy::module_inception)]
mod check;
mod ir;
mod names;
mod span;
#[cfg(test)]
mod tests;

const MAX_MESSAGE_FIELD_NUMBER: i32 = 536_870_911;
const RESERVED_MESSAGE_FIELD_NUMBERS: Range<i32> = 19_000..20_000;

use self::names::DuplicateNameError;
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
    let name_map = NameMap::from_ir(&ir, file_map)?;
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
    #[error(transparent)]
    #[diagnostic(transparent)]
    DuplicateName(#[from] DuplicateNameError),
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
    #[error("a map field key type must be a numeric type or string")]
    InvalidMapFieldKeyType {
        #[label("defined here")]
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
    #[error("message numbers between {} and {} are reserved", RESERVED_MESSAGE_FIELD_NUMBERS.start, RESERVED_MESSAGE_FIELD_NUMBERS.end)]
    ReservedMessageNumber {
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
    #[error("unknown field '{name}' for '{namespace}'")]
    OptionUnknownField {
        name: String,
        namespace: String,
        #[label("defined here")]
        span: Span,
    },
    #[error("cannot set field for scalar type")]
    OptionScalarFieldAccess {
        #[label("defined here")]
        span: Span,
    },
    #[error("failed to resolve type name '{name}' for option")]
    OptionInvalidTypeName {
        name: String,
        #[label("defined here")]
        span: Span,
    },
    #[error("expected option value to be {expected}, but found '{actual}'")]
    OptionValueInvalidType {
        expected: String,
        actual: String,
        #[label("defined here")]
        span: Span,
    },
    #[error("expected option value to be {expected}, but the value is out of range")]
    #[help("the value must be between {min} and {max}, inclusive")]
    OptionValueOutOfRange {
        expected: String,
        actual: String,
        min: String,
        max: String,
        #[label("defined here")]
        span: Span,
    },
    #[error("option value is not valid utf-8")]
    OptionValueInvalidUtf8 {
        #[label("defined here")]
        span: Span,
    },
}
