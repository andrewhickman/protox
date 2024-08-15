use std::{fmt, io, path::PathBuf};

use miette::{Diagnostic, NamedSource, SourceCode, SourceOffset, SourceSpan};
use prost_reflect::DescriptorError;
use protox_parse::ParseError;
use thiserror::Error;

use crate::file::File;

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
    #[error("error opening file '{path}'")]
    OpenFile {
        name: String,
        path: PathBuf,
        #[source]
        err: io::Error,
    },
    #[error("file '{name}' is too large")]
    #[diagnostic(help("the maximum file length is 2,147,483,647 bytes"))]
    FileTooLarge { name: String },
    #[error("file '{name}' is not valid utf-8")]
    FileInvalidUtf8 { name: String },
    #[error("file '{name}' not found")]
    FileNotFound { name: String },
    #[error("import '{name}' not found")]
    ImportNotFound {
        #[label("imported here")]
        span: Option<SourceSpan>,
        #[source_code]
        source_code: NamedSource<String>,
        name: String,
    },
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
        Error::from_kind(ErrorKind::FileNotFound {
            name: name.to_owned(),
        })
    }

    /// The file in which this error occurred, if available.
    pub fn file(&self) -> Option<&str> {
        match &*self.kind {
            ErrorKind::Parse { err } => Some(err.file()),
            ErrorKind::Check { err } => err.file(),
            ErrorKind::OpenFile { name, .. }
            | ErrorKind::FileTooLarge { name }
            | ErrorKind::FileInvalidUtf8 { name }
            | ErrorKind::FileNotFound { name }
            | ErrorKind::CircularImport { name, .. }
            | ErrorKind::FileShadowed { name, .. } => Some(name),
            ErrorKind::FileNotIncluded { .. } => None,
            ErrorKind::Custom(_) => None,
            ErrorKind::ImportNotFound { source_code, .. } => Some(source_code.name()),
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
            ErrorKind::FileNotFound { .. }
                | ErrorKind::ImportNotFound { .. }
                | ErrorKind::FileNotIncluded { .. }
        )
    }

    /// Returns true if this error is caused by an invalid protobuf source file.
    pub fn is_parse(&self) -> bool {
        matches!(
            &*self.kind,
            ErrorKind::Parse { .. }
                | ErrorKind::FileTooLarge { .. }
                | ErrorKind::FileInvalidUtf8 { .. }
        )
    }

    /// Returns true if this error is caused by an IO error while opening a file.
    pub fn is_io(&self) -> bool {
        match &*self.kind {
            ErrorKind::OpenFile { .. } => true,
            ErrorKind::Custom(err) if err.downcast_ref::<io::Error>().is_some() => true,
            _ => false,
        }
    }

    pub(crate) fn into_import_error(self, file: &File, import_idx: usize) -> Self {
        fn find_span(file: &File, import_idx: usize) -> Option<SourceSpan> {
            if let Some(sci) = &file.descriptor.source_code_info {
                if let Some(source) = file.source() {
                    for location in &sci.location {
                        if location.path == vec![3, import_idx as i32] {
                            if location.span.len() != 3 {
                                continue;
                            }
                            let start_line = *location.span.get(0)? as usize + 1;
                            let start_col = *location.span.get(1)? as usize + 1;
                            let end_col = *location.span.get(2)? as usize + 1;
                            return Some(SourceSpan::new(
                                SourceOffset::from_location(source, start_line, start_col),
                                end_col - start_col,
                            ));
                        }
                    }
                }
            }
            None
        }
        match *self.kind {
            ErrorKind::FileNotFound { name } => {
                let source_code: NamedSource<String> =
                    NamedSource::new(file.name(), file.source().or(Some("")).unwrap().to_string());
                let span = find_span(file, import_idx);
                Error::from_kind(ErrorKind::ImportNotFound {
                    span,
                    source_code,
                    name,
                })
            }
            _ => self,
        }
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
            ErrorKind::OpenFile { err, .. } => write!(f, "{}: {}", self, err),
            ErrorKind::FileTooLarge { .. }
            | ErrorKind::FileInvalidUtf8 { .. }
            | ErrorKind::FileNotFound { .. }
            | ErrorKind::CircularImport { .. }
            | ErrorKind::FileNotIncluded { .. }
            | ErrorKind::FileShadowed { .. } => write!(f, "{}", self),
            ErrorKind::Custom(err) => err.fmt(f),
            ErrorKind::ImportNotFound {
                span, source_code, ..
            } => {
                write!(f, "{}:", source_code.name())?;
                if let Some(span) = span {
                    if let Ok(span_contents) = source_code.read_span(span.into(), 0, 0) {
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
    }
}

#[test]
fn fmt_debug_io() {
    let err = Error::from_kind(ErrorKind::OpenFile {
        name: "file.proto".into(),
        path: "path/to/file.proto".into(),
        err: io::Error::new(io::ErrorKind::Other, "io error"),
    });

    assert!(err.is_io());
    assert_eq!(err.file(), Some("file.proto"));
    assert_eq!(
        format!("{:?}", err),
        "error opening file 'path/to/file.proto': io error"
    );
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
