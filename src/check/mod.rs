use std::ops::Range;

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

mod generate;
mod names;
mod resolve;
#[cfg(test)]
mod tests;

const MAX_MESSAGE_FIELD_NUMBER: i32 = 536_870_911;
const RESERVED_MESSAGE_FIELD_NUMBERS: Range<i32> = 19_000..20_000;

pub(crate) use self::{generate::generate, names::NameMap, resolve::resolve};
use self::{names::DuplicateNameError, resolve::DuplicateNumberError};

#[derive(Error, Clone, Debug, Diagnostic, PartialEq)]
pub(crate) enum CheckError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    DuplicateName(#[from] DuplicateNameError),
    #[error("camel-case name of field '{first_name}' conflicts with field '{second_name}'")]
    DuplicateCamelCaseFieldName {
        first_name: String,
        #[label("field defined here…")]
        first: Option<SourceSpan>,
        second_name: String,
        #[label("…conflicts with field here")]
        second: Option<SourceSpan>,
    },
    #[error(transparent)]
    #[diagnostic(transparent)]
    DuplicateNumber(#[from] DuplicateNumberError),
    #[error("unknown syntax '{syntax}'")]
    #[diagnostic(help("possible values are 'proto2' and 'proto3'"))]
    UnknownSyntax {
        syntax: String,
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("the type name '{name}' was not found")]
    TypeNameNotFound {
        name: String,
        #[label("used here")]
        span: Option<SourceSpan>,
    },
    #[error("message field type '{name}' is not a message or enum")]
    InvalidMessageFieldTypeName {
        name: String,
        #[label("used here")]
        span: Option<SourceSpan>,
    },
    #[error("a map field key type must be an integer type, boolean or string")]
    InvalidMapFieldKeyType {
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("extendee type '{name}' is not a message")]
    InvalidExtendeeTypeName {
        name: String,
        #[label("used here")]
        span: Option<SourceSpan>,
    },
    #[error("message type '{message_name}' does not declare '{number}' as an extension number")]
    InvalidExtensionNumber {
        number: i32,
        message_name: String,
        #[help]
        help: Option<String>,
        #[label("used here")]
        span: Option<SourceSpan>,
    },
    #[error("method {kind} type '{name}' is not a message")]
    InvalidMethodTypeName {
        name: String,
        kind: &'static str,
        #[label("used here")]
        span: Option<SourceSpan>,
    },
    #[error("message numbers must be between 1 and {}", MAX_MESSAGE_FIELD_NUMBER)]
    InvalidMessageNumber {
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("message numbers between {} and {} are reserved", RESERVED_MESSAGE_FIELD_NUMBERS.start, RESERVED_MESSAGE_FIELD_NUMBERS.end)]
    ReservedMessageNumber {
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("range end number must be greater than start number")]
    InvalidRange {
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("enum numbers must be between {} and {}", i32::MIN, i32::MAX)]
    InvalidEnumNumber {
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("{kind} fields may not have default values")]
    InvalidDefault {
        kind: &'static str,
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("default values are not allowed in proto3")]
    Proto3DefaultValue {
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("{kind} fields are not allowed in extensions")]
    InvalidExtendFieldKind {
        kind: &'static str,
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("extension fields may not be required")]
    RequiredExtendField {
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("map fields cannot have labels")]
    MapFieldWithLabel {
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("fields must have a label with proto2 syntax (expected one of 'optional', 'repeated' or 'required')")]
    Proto2FieldMissingLabel {
        #[label("field defined here")]
        span: Option<SourceSpan>,
    },
    #[error("groups are not allowed in proto3 syntax")]
    Proto3GroupField {
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("required fields are not allowed in proto3 syntax")]
    Proto3RequiredField {
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("{kind} fields are not allowed in a oneof")]
    InvalidOneofFieldKind {
        kind: &'static str,
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("oneof fields cannot have labels")]
    OneofFieldWithLabel {
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("a oneof must have at least one field")]
    EmptyOneof {
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("unknown field '{name}' for '{namespace}'")]
    OptionUnknownField {
        name: String,
        namespace: String,
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("extension '{extension_name}' not found for message '{expected_extendee}'")]
    #[diagnostic(help("the extension exists, but it extends '{actual_extendee}'"))]
    OptionExtensionInvalidExtendee {
        extension_name: String,
        expected_extendee: String,
        actual_extendee: String,
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("cannot set field for scalar type")]
    OptionScalarFieldAccess {
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("failed to resolve type name '{name}' for option")]
    OptionInvalidTypeName {
        name: String,
        #[label("used here")]
        span: Option<SourceSpan>,
    },
    #[error("option '{name}' is already set")]
    OptionAlreadySet {
        name: String,
        #[label("first set here…")]
        first: Option<SourceSpan>,
        #[label("…and set again here")]
        second: Option<SourceSpan>,
    },
    #[error("expected value to be {expected}, but found '{actual}'")]
    ValueInvalidType {
        expected: String,
        actual: String,
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("expected value to be {expected}, but the value is out of range")]
    #[diagnostic(help("the value must be between {min} and {max} inclusive"))]
    IntegerValueOutOfRange {
        expected: String,
        actual: String,
        min: String,
        max: String,
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("expected a string, but the value is not valid utf-8")]
    StringValueInvalidUtf8 {
        #[label("defined here")]
        span: Option<SourceSpan>,
    },
    #[error("'{value_name}' is not a valid value for enum '{enum_name}'")]
    InvalidEnumValue {
        value_name: String,
        enum_name: String,
        #[label("defined here")]
        span: Option<SourceSpan>,
        #[help]
        help: Option<String>,
    },
    #[error("identifiers may not be negative")]
    NegativeIdentOutsideDefault {
        #[label("found here")]
        span: Option<SourceSpan>,
    },
}
