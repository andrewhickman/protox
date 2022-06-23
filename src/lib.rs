#[allow(dead_code)]
mod ast;
#[allow(dead_code)]
mod compile;
mod parse;

use std::{fmt, path::Path};

use parse::ParseError;
use prost_types::FileDescriptorSet;
use thiserror::Error;
use miette::{Diagnostic, NamedSource};

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
#[error("oops!")]
#[diagnostic()]
enum ErrorKind {
    ParseErrors {
        #[source_code]
        src: NamedSource,
        #[related]
        related: Vec<ParseError>,
    },
}
