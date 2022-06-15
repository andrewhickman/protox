use std::path::{Path, PathBuf};

use prost_types::FileDescriptorSet;

use crate::Error;

pub struct Compiler {
    includes: Vec<PathBuf>,
}

impl Compiler {
    pub fn new(includes: impl IntoIterator<Item = impl AsRef<Path>>) -> Result<Self, Error> {
        Ok(Compiler {
            includes: includes
                .into_iter()
                .map(|path| path.as_ref().to_owned())
                .collect(),
        })
    }

    pub fn add_file(&mut self, file: impl AsRef<Path>) -> Result<(), Error> {
        let source = std::fs::read_to_string(file).unwrap();
        let _file = crate::parse::parse(&source).unwrap();
        todo!()
    }

    pub fn build_file_descriptor_set(self) -> FileDescriptorSet {
        todo!()
    }
}
