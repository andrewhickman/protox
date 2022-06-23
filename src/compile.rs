use std::path::{Path, PathBuf};

use miette::NamedSource;
use prost_types::FileDescriptorSet;

use crate::{parse, Error, ErrorKind};

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
        // for include in &self.includes {}

        let source = std::fs::read_to_string(file.as_ref()).map_err(|err| Error {
            kind: ErrorKind::OpenFile {
                err,
                path: file.as_ref().to_owned(),
            },
        })?;
        match parse::parse(&source) {
            Ok(_) => Ok(()),
            Err(errors) => Err(Error {
                kind: ErrorKind::ParseErrors {
                    src: NamedSource::new(file.as_ref().display().to_string(), source),
                    errors,
                },
            }),
        }
    }

    pub fn build_file_descriptor_set(self) -> FileDescriptorSet {
        FileDescriptorSet::default()
        // todo!()
    }
}

fn naive_path_eq(lhs: &Path, rhs: &Path) -> bool {
    lhs.components().eq(rhs.components())
}
