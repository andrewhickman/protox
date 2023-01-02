use bytes::{Buf, BufMut, Bytes};
use prost::{
    decode_length_delimiter,
    encoding::{
        check_wire_type, encode_key, encode_varint, message, skip_field, DecodeContext, WireType,
    },
    DecodeError, Message,
};
use prost_types::FileDescriptorProto;

use crate::{
    file::{File, FileDescriptorKind, FileResolver},
    Error,
};

/// An implementation of [`FileResolver`] which resolves files from a compiled [`FileDescriptorSet`](prost_types::FileDescriptorSet).
#[derive(Debug)]
pub struct DescriptorSetFileResolver {
    set: Vec<FileDescriptor>,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct FileDescriptor {
    encoded: Vec<u8>,
    file: FileDescriptorProto,
}

impl DescriptorSetFileResolver {
    /// Create an instance of [`DescriptorSetFileResolver`] from the file descriptor set.
    pub fn new(set: prost_types::FileDescriptorSet) -> Self {
        DescriptorSetFileResolver {
            set: set
                .file
                .into_iter()
                .map(|file| FileDescriptor {
                    encoded: file.encode_to_vec(),
                    file,
                })
                .collect(),
        }
    }

    /// Create an instance of [`DescriptorSetFileResolver`] by deserializing a [`FileDescriptorSet`](prost_types::FileDescriptorSet)
    /// from the given bytes.
    ///
    /// Unlike when going through [`new()`](DescriptorSetFileResolver::new), extension options are preserved.
    pub fn decode<B>(mut buf: B) -> Result<Self, DecodeError>
    where
        B: Buf,
    {
        let mut set = Vec::new();
        while buf.has_remaining() {
            message::merge_repeated(
                WireType::LengthDelimited,
                &mut set,
                &mut buf,
                DecodeContext::default(),
            )?;
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
                    kind: FileDescriptorKind::Bytes(file.encoded.clone()),
                });
            }
        }

        Err(Error::file_not_found(name))
    }
}

impl Message for FileDescriptor {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        buf.put(self.encoded.as_slice());
    }

    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        B: Buf,
        Self: Sized,
    {
        if tag == crate::tag::file_descriptor_set::FILE as u32 {
            check_wire_type(WireType::LengthDelimited, wire_type)?;
            let len = decode_length_delimiter(buf)?;

            encode_key(tag, wire_type, &mut self.encoded);
            let start = self.encoded.len();
            encode_varint(len as u64, &mut self.encoded);
            self.encoded.put(buf.take(len));

            self.file
                .merge_field(tag, wire_type, &mut &self.encoded[start..], ctx)?;
        } else {
            skip_field(wire_type, tag, buf, ctx)?;
        }

        Ok(())
    }

    fn encoded_len(&self) -> usize {
        self.encoded.len()
    }

    fn clear(&mut self) {
        self.encoded.clear();
        self.file.clear();
    }
}
