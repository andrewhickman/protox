use std::{fmt, io, path::PathBuf};

use logos::Span;
use miette::{Diagnostic, NamedSource, SourceCode};
use thiserror::Error;

use crate::{check::CheckError, parse::ParseError};

/// An error that can occur when compiling protobuf files.
#[derive(Debug, Diagnostic, Error)]
#[error(transparent)]
#[diagnostic(transparent)]
pub struct Error {
    kind: ErrorKind,
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
    },
    #[error("error opening imported file '{path}'")]
    OpenImport {
        path: PathBuf,
        #[source]
        err: io::Error,
        #[source_code]
        src: NamedSource,
        #[label("imported here")]
        span: Span,
    },
    #[error("import '{name}' not found")]
    ImportNotFound {
        name: String,
        #[source_code]
        src: NamedSource,
        #[label("imported here")]
        span: Span,
    },
    #[error("import cycle detected: {cycle}")]
    CircularImport { cycle: String },
    #[error("path '{path}' is not in any include path")]
    FileNotIncluded { path: PathBuf },
    #[error("path '{path}' is shadowed by '{shadow}' in the include paths")]
    #[help("Either pass '{shadow}' as the input file, or re-order the include paths so that '{path}' comes first")]
    FileShadowed { path: PathBuf, shadow: PathBuf },
}

pub(crate) struct DynSourceCode(Box<dyn SourceCode>);

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
    ) -> Result<Box<dyn miette::SpanContents<'a> + 'a>, miette::MietteError> {
        self.0
            .read_span(span, context_lines_before, context_lines_after)
    }
}

impl From<String> for DynSourceCode {
    fn from(source: String) -> Self {
        DynSourceCode(Box::new(source))
    }
}

impl From<NamedSource> for DynSourceCode {
    fn from(source: NamedSource) -> Self {
        DynSourceCode(Box::new(source))
    }
}

impl Error {
    pub(crate) fn new(kind: ErrorKind) -> Self {
        Error { kind }
    }

    #[cfg(test)]
    pub(crate) fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub(crate) fn parse_errors(mut errors: Vec<ParseError>, src: impl Into<DynSourceCode>) -> Self {
        let err = errors.remove(0);
        Error::new(ErrorKind::ParseErrors {
            err,
            src: src.into(),
            errors,
        })
    }

    pub(crate) fn check_errors(mut errors: Vec<CheckError>, src: impl Into<DynSourceCode>) -> Self {
        let err = errors.remove(0);
        Error::new(ErrorKind::CheckErrors {
            err,
            src: src.into(),
            errors,
        })
    }
}
