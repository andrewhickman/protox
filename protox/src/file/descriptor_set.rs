use bytes::{Buf, BufMut, Bytes, BytesMut};
use prost::{
    decode_length_delimiter,
    encoding::{
        check_wire_type, encode_key, encode_varint, message, skip_field, DecodeContext, WireType,
    },
    DecodeError, Message,
};
use prost_types::FileDescriptorProto;

use crate::{
    file::{File, FileResolver},
    Error,
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
                    descriptor: file.file.clone(),
                    encoded: file.encoded.clone(),
                });
            }
        }

        Err(Error::file_not_found(name))
    }
}

impl Message for FileDescriptor {
    fn encode_raw<B>(&self, _: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        unimplemented!()
    }

    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: WireType,
        mut buf: &mut B,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        B: Buf,
        Self: Sized,
    {
        if tag == crate::tag::file_descriptor_set::FILE as u32 {
            let mut encoded = BytesMut::new();
            if let Some(bytes) = &self.encoded {
                encoded.extend_from_slice(bytes);
            }

            check_wire_type(WireType::LengthDelimited, wire_type)?;
            let len = decode_length_delimiter(&mut buf)?;

            encode_key(tag, wire_type, &mut encoded);
            let start = encoded.len();
            encode_varint(len as u64, &mut encoded);
            encoded.put(buf.take(len));

            let encoded = self.encoded.insert(encoded.freeze());
            self.file
                .merge_field(tag, wire_type, &mut &encoded[start..], ctx)?;
        } else {
            skip_field(wire_type, tag, buf, ctx)?;
        }

        Ok(())
    }

    fn encoded_len(&self) -> usize {
        unimplemented!()
    }

    fn clear(&mut self) {
        unimplemented!()
    }
}
