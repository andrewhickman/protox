use bytes::Buf;
use prost::{DecodeError, Message};

use crate::{
    file::{File, FileResolver},
    transcode_file,
    types::FileDescriptorSet,
    Error,
};

/// An implementation of [`FileResolver`] which resolves files from a compiled [`FileDescriptorSet`](prost_types::FileDescriptorSet).
#[derive(Debug)]
pub struct DescriptorSetFileResolver {
    set: FileDescriptorSet,
}

impl DescriptorSetFileResolver {
    /// Create an instance of [`DescriptorSetFileResolver`] from the file descriptor set.
    pub fn new(set: prost_types::FileDescriptorSet) -> Self {
        DescriptorSetFileResolver {
            set: transcode_file(&set, &mut Vec::new()),
        }
    }

    /// Create an instance of [`DescriptorSetFileResolver`] by deserializing a [`FileDescriptorSet`](prost_types::FileDescriptorSet)
    /// from the given bytes.
    ///
    /// Unlike when going through [`new()`](DescriptorSetFileResolver::new), extension options are preserved.
    pub fn decode<B>(buf: B) -> Result<Self, DecodeError>
    where
        B: Buf,
    {
        Ok(DescriptorSetFileResolver {
            set: FileDescriptorSet::decode(buf)?,
        })
    }
}

impl FileResolver for DescriptorSetFileResolver {
    fn open_file(&self, name: &str) -> Result<File, Error> {
        for file in &self.set.file {
            if file.name() == name {
                return Ok(File {
                    path: None,
                    source: None,
                    descriptor: file.clone(),
                });
            }
        }

        Err(Error::file_not_found(name))
    }
}
