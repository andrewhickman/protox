#[allow(dead_code)]
mod ast;
#[allow(dead_code)]
mod compile;
mod parse;

use std::{fmt, path::Path};

use prost_types::FileDescriptorSet;

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

pub struct Error {
    msg: String,
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.msg.fmt(f)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.msg.fmt(f)
    }
}

impl std::error::Error for Error {}
