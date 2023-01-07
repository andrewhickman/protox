use bytes::{Buf, Bytes};
use prost::{
    encoding::{check_wire_type, decode_key, decode_varint, skip_field, DecodeContext, WireType},
    DecodeError, Message,
};
use prost_types::FileDescriptorProto;

use crate::{
    file::{File, FileResolver},
    tag, Error,
};

/// An implementation of [`FileResolver`] which resolves files from a compiled [`FileDescriptorSet`](prost_types::FileDescriptorSet).
#[derive(Debug)]
pub struct DescriptorSetFileResolver {
    set: Vec<FileDescriptor>,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct FileDescriptor {
    file: FileDescriptorProto,
    encoded: Option<Bytes>,
}

impl DescriptorSetFileResolver {
    /// Creates an instance of [`DescriptorSetFileResolver`] from the file descriptor set.
    pub fn new(set: prost_types::FileDescriptorSet) -> Self {
        DescriptorSetFileResolver {
            set: set
                .file
                .into_iter()
                .map(|file| FileDescriptor {
                    encoded: None,
                    file,
                })
                .collect(),
        }
    }

    /// Creates an instance of [`DescriptorSetFileResolver`] by deserializing a [`FileDescriptorSet`](prost_types::FileDescriptorSet)
    /// from the given bytes.
    ///
    /// Unlike when going through [`new()`](DescriptorSetFileResolver::new), extension options are preserved.
    pub fn decode<B>(mut buf: B) -> Result<Self, DecodeError>
    where
        B: Buf,
    {
        let mut set = Vec::new();
        while buf.has_remaining() {
            let (key, wire_type) = decode_key(&mut buf)?;
            if key == tag::file_descriptor_set::FILE as u32 {
                check_wire_type(WireType::LengthDelimited, wire_type)?;
                let len = decode_varint(&mut buf)? as usize;
                if len > buf.remaining() {
                    return Err(DecodeError::new("buffer underflow"));
                }
                set.push(FileDescriptor::decode((&mut buf).take(len))?);
            } else {
                skip_field(wire_type, key, &mut buf, DecodeContext::default())?;
            }
        }
        Ok(DescriptorSetFileResolver { set })
    }
}

impl FileResolver for DescriptorSetFileResolver {
    fn open_file(&self, name: &str) -> Result<File, Error> {
        for file in &self.set {
            if file.file.name() == name {
                return Ok(File {
                    path: None,
                    source: None,
                    descriptor: file.file.clone(),
                    encoded: file.encoded.clone(),
                });
            }
        }

        Err(Error::file_not_found(name))
    }
}

impl FileDescriptor {
    fn decode(mut buf: impl Buf) -> Result<Self, DecodeError> {
        let encoded = buf.copy_to_bytes(buf.remaining());
        let file = FileDescriptorProto::decode(&mut encoded.as_ref())?;

        Ok(FileDescriptor {
            file,
            encoded: Some(encoded),
        })
    }
}
