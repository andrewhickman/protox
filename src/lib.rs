use std::{fmt, path::Path};

use prost_types::FileDescriptorSet;

pub fn compile(
    _files: impl IntoIterator<Item = impl AsRef<Path>>,
    _includes: impl IntoIterator<Item = impl AsRef<Path>>,
) -> Result<FileDescriptorSet, Error> {
    todo!()
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
