use std::{fmt, io, path::PathBuf, sync::Arc};

use miette::{Diagnostic, MietteError, NamedSource, SourceCode, SourceSpan};
use thiserror::Error;

use crate::{check::CheckError, parse::ParseError};

/// An error that can occur when compiling protobuf files.
#[derive(Debug, Diagnostic, Error)]
#[error(transparent)]
#[diagnostic(transparent)]
pub struct Error {
    kind: Box<ErrorKind>,
}

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum ErrorKind {
    #[error("{}", err)]
    #[diagnostic(forward(err))]
    ParseErrors {
        err: ParseError,
        #[source_code]
        src: DynSourceCode,
        #[related]
        errors: Vec<ParseError>,
    },
    #[error("{}", err)]
    #[diagnostic(forward(err))]
    CheckErrors {
        err: CheckError,
        #[source_code]
        src: DynSourceCode,
        #[related]
        errors: Vec<CheckError>,
    },
    #[error("at least once include path must be provided")]
    NoIncludePaths,
    #[error("error opening file '{path}'")]
    OpenFile {
        path: PathBuf,
        #[source]
        err: io::Error,
        #[source_code]
        src: DynSourceCode,
        #[label("imported here")]
        span: Option<SourceSpan>,
    },
    #[error("file is too large")]
    #[help("the maximum file length is 2,147,483,647 bytes")]
    FileTooLarge {
        #[source_code]
        src: DynSourceCode,
        #[label("imported here")]
        span: Option<SourceSpan>,
    },
    #[error("import '{name}' not found")]
    ImportNotFound {
        name: String,
        #[source_code]
        src: DynSourceCode,
        #[label("imported here")]
        span: Option<SourceSpan>,
    },
    #[error("import cycle detected: {cycle}")]
    CircularImport { cycle: String },
    #[error("path '{path}' is not in any include path")]
    FileNotIncluded { path: PathBuf },
    #[error("path '{path}' is shadowed by '{shadow}' in the include paths")]
    #[help("Either pass '{shadow}' as the input file, or re-order the include paths so that '{path}' comes first")]
    FileShadowed { path: PathBuf, shadow: PathBuf },
    #[error(transparent)]
    Custom(Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Default)]
pub(crate) struct DynSourceCode(Option<Box<dyn SourceCode>>);

impl fmt::Debug for DynSourceCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("DynSourceCode").finish_non_exhaustive()
    }
}

impl SourceCode for DynSourceCode {
    fn read_span<'a>(
        &'a self,
        span: &miette::SourceSpan,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> Result<Box<dyn miette::SpanContents<'a> + 'a>, MietteError> {
        if let Some(src) = &self.0 {
            src.read_span(span, context_lines_before, context_lines_after)
        } else {
            Err(MietteError::OutOfBounds)
        }
    }
}

impl From<(Option<PathBuf>, Arc<str>)> for DynSourceCode {
    fn from((path, source): (Option<PathBuf>, Arc<str>)) -> Self {
        if let Some(path) = path {
            DynSourceCode::from(NamedSource::new(path.display().to_string(), source))
        } else {
            DynSourceCode::from(source)
        }
    }
}

impl From<Arc<str>> for DynSourceCode {
    fn from(source: Arc<str>) -> Self {
        DynSourceCode(Some(Box::new(source)))
    }
}

impl From<NamedSource> for DynSourceCode {
    fn from(source: NamedSource) -> Self {
        DynSourceCode(Some(Box::new(source)))
    }
}

impl Error {
    /// Create an instance of [`struct@Error`] with an arbitrary payload.
    pub fn new<E>(error: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Error::from_kind(ErrorKind::Custom(error.into()))
    }

    pub(crate) fn from_kind(kind: ErrorKind) -> Self {
        Error {
            kind: Box::new(kind),
        }
    }

    pub(crate) fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub(crate) fn is_file_not_found(&self) -> bool {
        match &*self.kind {
            ErrorKind::OpenFile { err, .. } => err.kind() == io::ErrorKind::NotFound,
            _ => false,
        }
    }

    pub(crate) fn parse_errors(mut errors: Vec<ParseError>, src: impl Into<DynSourceCode>) -> Self {
        let err = errors.remove(0);
        Error::from_kind(ErrorKind::ParseErrors {
            err,
            src: src.into(),
            errors,
        })
    }

    pub(crate) fn check_errors(mut errors: Vec<CheckError>, src: impl Into<DynSourceCode>) -> Self {
        let err = errors.remove(0);
        Error::from_kind(ErrorKind::CheckErrors {
            err,
            src: src.into(),
            errors,
        })
    }

    pub(crate) fn add_import_context(
        mut self,
        import_src: impl Into<DynSourceCode>,
        import_span: impl Into<SourceSpan>,
    ) -> Self {
        match &mut *self.kind {
            ErrorKind::OpenFile { src, span, .. } => {
                *src = import_src.into();
                *span = Some(import_span.into());
            }
            ErrorKind::ImportNotFound { src, span, .. } => {
                *src = import_src.into();
                *span = Some(import_span.into());
            }
            _ => (),
        };

        self
    }
}
