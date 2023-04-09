use std::fmt;

use logos::Span;
use miette::{Diagnostic, NamedSource, SourceCode};
use thiserror::Error;

use crate::MAX_MESSAGE_FIELD_NUMBER;

/// An error that may occur while parsing a protobuf source file.
#[derive(Error, Diagnostic)]
#[error("{}", kind)]
#[diagnostic(forward(kind))]
pub struct ParseError {
    kind: Box<ParseErrorKind>,
    #[related]
    related: Vec<ParseErrorKind>,
    #[source_code]
    source_code: NamedSource,
}

#[derive(Error, Debug, Diagnostic, PartialEq)]
pub(crate) enum ParseErrorKind {
    #[error("invalid token")]
    InvalidToken {
        #[label("found here")]
        span: Span,
    },
    #[error("integer is too large")]
    IntegerOutOfRange {
        #[label("integer defined here")]
        span: Span,
    },
    #[error("invalid string character")]
    InvalidStringCharacters {
        #[label("invalid characters")]
        span: Span,
    },
    #[error("unterminated string")]
    UnterminatedString {
        #[label("string starts here")]
        span: Span,
    },
    #[error("invalid string escape")]
    InvalidStringEscape {
        #[label("defined here")]
        span: Span,
    },
    #[error("string is not valid utf-8")]
    InvalidUtf8String {
        #[label("defined here")]
        span: Span,
    },
    #[error("nested block comments are not supported")]
    NestedBlockComment {
        #[label("defined here")]
        span: Span,
    },
    #[error("unknown syntax '{syntax}'")]
    #[diagnostic(help("possible values are 'proto2' and 'proto3'"))]
    UnknownSyntax {
        syntax: String,
        #[label("defined here")]
        span: Span,
    },
    #[error("invalid identifier")]
    #[diagnostic(help("identifiers must consist of letters, numbers and underscores, and may not start with a number"))]
    InvalidIdentifier {
        #[label("defined here")]
        span: Span,
    },
    #[error("invalid group name")]
    #[diagnostic(help(
        "group names must consist of a capital letter followed by letters, numbers and underscores"
    ))]
    InvalidGroupName {
        #[label("defined here")]
        span: Span,
    },
    #[error("invalid group name")]
    #[diagnostic(help(
        "imports may not contain backslashes, repeated forward slashes, '.' or '..' components"
    ))]
    InvalidImport {
        #[label("defined here")]
        span: Span,
    },
    #[error("multiple package names specified")]
    DuplicatePackage {
        #[label("defined here…")]
        first: Span,
        #[label("…and again here")]
        second: Span,
    },
    #[error("whitespace is required between an integer literal and an identifier")]
    NoSpaceBetweenIntAndIdent {
        #[label("found here")]
        span: Span,
    },
    #[error("'#' comments are not allowed here")]
    HashCommentOutsideTextFormat {
        #[label("found here")]
        span: Span,
    },
    #[error("'f' suffix for float literals is not allowed")]
    FloatSuffixOutsideTextFormat {
        #[label("found here")]
        span: Span,
    },
    #[error("expected {expected}, but found '{found}'")]
    UnexpectedToken {
        expected: String,
        found: String,
        #[label("found here")]
        span: Span,
    },
    #[error("expected {expected}, but reached end of file")]
    UnexpectedEof { expected: String },
    #[error("identifiers may not be negative")]
    NegativeIdentOutsideDefault {
        #[label("found here")]
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
    #[error("default values are not allowed in proto3")]
    Proto3DefaultValue {
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
    #[error("oneof fields cannot have labels")]
    OneofFieldWithLabel {
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
    #[error("a map field key type must be an integer, boolean or string")]
    InvalidMapFieldKeyType {
        #[label("defined here")]
        span: Span,
    },
    #[error("expected value to be {expected}, but found '{actual}'")]
    ValueInvalidType {
        expected: String,
        actual: String,
        #[label("defined here")]
        span: Span,
    },
    #[error("expected value to be {expected}, but the value {actual} is out of range")]
    #[diagnostic(help("the value must be between {min} and {max} inclusive"))]
    IntegerValueOutOfRange {
        expected: String,
        actual: String,
        min: String,
        max: String,
        #[label("defined here")]
        span: Span,
    },
    #[error("a oneof must have at least one field")]
    EmptyOneof {
        #[label("defined here")]
        span: Span,
    },
    #[error("file is too large")]
    #[diagnostic(help("the maximum file length is 2,147,483,647 bytes"))]
    FileTooLarge,
}

impl ParseError {
    pub(crate) fn new(
        mut related: Vec<ParseErrorKind>,
        name: &str,
        source: impl SourceCode + Send + Sync + 'static,
    ) -> Self {
        debug_assert!(!related.is_empty());
        let kind = related.remove(0);
        ParseError {
            kind: Box::new(kind),
            related,
            source_code: NamedSource::new(name, source),
        }
    }

    #[cfg(test)]
    pub(crate) fn into_inner(mut self) -> Vec<ParseErrorKind> {
        self.related.insert(0, *self.kind);
        self.related
    }

    /// Gets the name of the file in which this error occurred.
    pub fn file(&self) -> &str {
        // TODO https://github.com/zkat/miette/pull/252
        ""
    }

    /// Gets the primary source code span associated with this error, if any.
    pub fn span(&self) -> Option<Span> {
        match &*self.kind {
            ParseErrorKind::InvalidToken { span } => Some(span.clone()),
            ParseErrorKind::IntegerOutOfRange { span } => Some(span.clone()),
            ParseErrorKind::InvalidStringCharacters { span } => Some(span.clone()),
            ParseErrorKind::UnterminatedString { span } => Some(span.clone()),
            ParseErrorKind::InvalidStringEscape { span } => Some(span.clone()),
            ParseErrorKind::InvalidUtf8String { span } => Some(span.clone()),
            ParseErrorKind::NestedBlockComment { span } => Some(span.clone()),
            ParseErrorKind::UnknownSyntax { span, .. } => Some(span.clone()),
            ParseErrorKind::InvalidIdentifier { span } => Some(span.clone()),
            ParseErrorKind::InvalidGroupName { span } => Some(span.clone()),
            ParseErrorKind::InvalidImport { span } => Some(span.clone()),
            ParseErrorKind::DuplicatePackage { .. } => None,
            ParseErrorKind::NoSpaceBetweenIntAndIdent { span } => Some(span.clone()),
            ParseErrorKind::HashCommentOutsideTextFormat { span } => Some(span.clone()),
            ParseErrorKind::FloatSuffixOutsideTextFormat { span } => Some(span.clone()),
            ParseErrorKind::UnexpectedToken { span, .. } => Some(span.clone()),
            ParseErrorKind::UnexpectedEof { .. } => None,
            ParseErrorKind::NegativeIdentOutsideDefault { span } => Some(span.clone()),
            ParseErrorKind::InvalidMessageNumber { span } => Some(span.clone()),
            ParseErrorKind::InvalidEnumNumber { span } => Some(span.clone()),
            ParseErrorKind::InvalidDefault { span, .. } => Some(span.clone()),
            ParseErrorKind::Proto3DefaultValue { span } => Some(span.clone()),
            ParseErrorKind::InvalidExtendFieldKind { span, .. } => Some(span.clone()),
            ParseErrorKind::RequiredExtendField { span } => Some(span.clone()),
            ParseErrorKind::MapFieldWithLabel { span } => Some(span.clone()),
            ParseErrorKind::OneofFieldWithLabel { span } => Some(span.clone()),
            ParseErrorKind::Proto2FieldMissingLabel { span } => Some(span.clone()),
            ParseErrorKind::Proto3GroupField { span } => Some(span.clone()),
            ParseErrorKind::Proto3RequiredField { span } => Some(span.clone()),
            ParseErrorKind::InvalidOneofFieldKind { span, .. } => Some(span.clone()),
            ParseErrorKind::InvalidMapFieldKeyType { span } => Some(span.clone()),
            ParseErrorKind::ValueInvalidType { span, .. } => Some(span.clone()),
            ParseErrorKind::IntegerValueOutOfRange { span, .. } => Some(span.clone()),
            ParseErrorKind::EmptyOneof { span } => Some(span.clone()),
            ParseErrorKind::FileTooLarge => None,
        }
    }
}

impl fmt::Debug for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(span) = self.span() {
            if let Ok(span_contents) = self.source_code.read_span(&span.into(), 0, 0) {
                if let Some(file_name) = span_contents.name() {
                    write!(f, "{}:", file_name)?;
                }

                write!(
                    f,
                    "{}:{}: ",
                    span_contents.line() + 1,
                    span_contents.column() + 1
                )?;
            }
        }

        write!(f, "{}", self)
    }
}
