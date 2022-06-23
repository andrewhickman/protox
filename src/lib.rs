#[allow(dead_code)]
mod ast;
#[allow(dead_code)]
mod compile;
mod parse;

use std::{
    io,
    path::{Path, PathBuf},
};

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
    #[error("error opening file '{path}'")]
    OpenFile {
        path: PathBuf,
        #[source]
        err: io::Error,
    },
}
