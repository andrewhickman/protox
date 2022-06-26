//! A rust implementation of the protobuf compiler.
#![warn(missing_debug_implementations, missing_docs)]
#![deny(unsafe_code)]
#![doc(html_root_url = "https://docs.rs/protox/0.1.0/")]

mod ast;
mod compile;
mod parse;

use core::fmt;
use std::{
    io,
    path::{Path, PathBuf},
};

use logos::Span;
use miette::{Diagnostic, NamedSource, SourceCode};
use parse::ParseError;
use prost_types::{FileDescriptorProto, FileDescriptorSet};
use thiserror::Error;

pub use self::compile::Compiler;

const MAX_MESSAGE_FIELD_NUMBER: i32 = 536870911;

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
/// compiler.build_file_descriptor_set();
/// # Ok(())
/// # }
/// ```
pub fn compile(
    files: impl IntoIterator<Item = impl AsRef<Path>>,
    includes: impl IntoIterator<Item = impl AsRef<Path>>,
) -> Result<FileDescriptorSet, Error> {
    let mut compiler = compile::Compiler::new(includes)?;

    for file in files {
        compiler.add_file(file)?;
    }

    Ok(compiler.build_file_descriptor_set())
}

/// Parse a single protobuf source file into a [`FileDescriptorProto`].
///
/// This function only looks at the syntax of the file, without resolving type names or reading
/// imported files.
pub fn parse(source: &str) -> Result<FileDescriptorProto, Error> {
    let ast =
        parse::parse(source).map_err(|errors| Error::parse_error(errors, source.to_owned()))?;
    let _file = ast.to_file_descriptor(Some(source));
    // TODO check
    todo!()
}

/// An error that can occur when compiling protobuf files.
#[derive(Debug, Diagnostic, Error)]
#[error(transparent)]
#[diagnostic(transparent)]
pub struct Error {
    kind: ErrorKind,
}

#[derive(Debug, Diagnostic, Error)]
enum ErrorKind {
    #[error("{}", err)]
    #[diagnostic(forward(err))]
    ParseErrors {
        err: ParseError,
        #[source_code]
        src: DynSourceCode,
        #[related]
        errors: Vec<ParseError>,
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

struct DynSourceCode(Box<dyn SourceCode>);

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
    fn new(kind: ErrorKind) -> Self {
        Error { kind }
    }

    fn parse_error(mut errors: Vec<ParseError>, src: impl Into<DynSourceCode>) -> Self {
        let err = errors.remove(0);
        Error::new(ErrorKind::ParseErrors {
            err,
            src: src.into(),
            errors,
        })
    }
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
