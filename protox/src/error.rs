use std::{fmt, io, path::PathBuf};

use miette::Diagnostic;
use prost_reflect::DescriptorError;
use protox_parse::ParseError;
use thiserror::Error;

/// An error that can occur when compiling protobuf files.
#[derive(Diagnostic, Error)]
#[error(transparent)]
#[diagnostic(transparent)]
pub struct Error {
    kind: Box<ErrorKind>,
}

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum ErrorKind {
    #[error("{}", err)]
    #[diagnostic(forward(err))]
    Parse { err: ParseError },
    #[error("{}", err)]
    #[diagnostic(forward(err))]
    Check { err: DescriptorError },
    #[error("import '{name}' not found")]
    ImportNotFound { name: String },
    #[error("import cycle detected: {cycle}")]
    CircularImport { name: String, cycle: String },
    #[error("file '{path}' is not in any include path")]
    FileNotIncluded { path: PathBuf },
    #[error("path '{path}' is shadowed by '{shadow}' in the include paths")]
    #[diagnostic(help("either pass '{}' as the input file, or re-order the include paths so that '{}' comes first", shadow.display(), path.display()))]
    FileShadowed {
        name: String,
        path: PathBuf,
        shadow: PathBuf,
    },
    #[error(transparent)]
    Custom(Box<dyn std::error::Error + Send + Sync>),
}

impl Error {
    /// Creates an instance of [`struct@Error`] with an arbitrary payload.
    pub fn new<E>(error: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Error::from_kind(ErrorKind::Custom(error.into()))
    }

    /// Creates an instance of [`struct@Error`] indicating that an imported file could not be found.
    ///
    /// This error should be returned by [`FileResolver`](crate::file::FileResolver) instances if a file is not found.
    pub fn file_not_found(name: &str) -> Self {
        Error::from_kind(ErrorKind::ImportNotFound {
            name: name.to_owned(),
        })
    }

    /// The file in which this error occurred, if available.
    pub fn file(&self) -> Option<&str> {
        match &*self.kind {
            ErrorKind::Parse { err } => Some(err.file()),
            ErrorKind::Check { err } => err.file(),
            ErrorKind::ImportNotFound { name }
            | ErrorKind::CircularImport { name, .. }
            | ErrorKind::FileShadowed { name, .. } => Some(name),
            ErrorKind::FileNotIncluded { .. } => None,
            ErrorKind::Custom(_) => None,
        }
    }

    pub(crate) fn from_kind(kind: ErrorKind) -> Self {
        Error {
            kind: Box::new(kind),
        }
    }

    #[cfg(test)]
    pub(crate) fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// Returns true if this is an instance of [`Error::file_not_found()`]
    pub fn is_file_not_found(&self) -> bool {
        matches!(
            &*self.kind,
            ErrorKind::ImportNotFound { .. } | ErrorKind::FileNotIncluded { .. }
        )
    }

    /// Returns true if this error is caused by an invalid protobuf source file.
    pub fn is_parse(&self) -> bool {
        matches!(&*self.kind, ErrorKind::Parse { .. })
    }

    /// Returns true if this error is caused by an IO error while opening a file.
    pub fn is_io(&self) -> bool {
        matches!(&*self.kind, ErrorKind::Custom(err) if err.downcast_ref::<io::Error>().is_some())
    }
}

impl From<DescriptorError> for Error {
    fn from(err: DescriptorError) -> Self {
        Error::from_kind(ErrorKind::Check { err })
    }
}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Self {
        Error::from_kind(ErrorKind::Parse { err })
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::new(err)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self.kind {
            ErrorKind::Parse { err } => err.fmt(f),
            ErrorKind::Check { err } => err.fmt(f),
            ErrorKind::ImportNotFound { .. }
            | ErrorKind::CircularImport { .. }
            | ErrorKind::FileNotIncluded { .. }
            | ErrorKind::FileShadowed { .. } => write!(f, "{}", self),
            ErrorKind::Custom(err) => err.fmt(f),
        }
    }
}

#[test]
fn fmt_debug_parse() {
    let err = Error::from(protox_parse::parse("file.proto", "invalid").unwrap_err());

    assert!(err.is_parse());
    assert_eq!(err.file(), Some("file.proto"));
    assert_eq!(
        format!("{:?}", err),
        "file.proto:1:1: expected 'enum', 'extend', 'import', 'message', 'option', 'service', 'package' or ';', but found 'invalid'"
    );
}
