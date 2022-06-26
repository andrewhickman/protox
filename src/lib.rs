#[allow(dead_code)]
mod ast;
#[allow(dead_code)]
mod compile;
mod parse;

use std::{
    io,
    path::{Path, PathBuf},
};

use logos::Span;
use miette::{Diagnostic, NamedSource};
use parse::ParseError;
use prost_types::FileDescriptorSet;
use thiserror::Error;

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

#[derive(Debug, Diagnostic, Error)]
#[error(transparent)]
#[diagnostic(transparent)]
pub struct Error {
    kind: ErrorKind,
}

#[derive(Debug, Diagnostic, Error)]
enum ErrorKind {
    #[error("error parsing file")]
    ParseErrors {
        #[source_code]
        src: NamedSource,
        #[diagnostic(transparent)]
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

impl Error {
    fn new(kind: ErrorKind) -> Self {
        Error { kind }
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
