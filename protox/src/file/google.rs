use prost_reflect::DescriptorPool;

use super::{File, FileResolver};
use crate::Error;

/// An implementation of [`FileResolver`] which resolves well-known imports such as `google/protobuf/descriptor.proto`.
#[derive(Debug, Default)]
pub struct GoogleFileResolver {
    pool: DescriptorPool,
}

impl GoogleFileResolver {
    /// Creates a new instance of [`GoogleFileResolver`].
    pub fn new() -> Self {
        GoogleFileResolver {
            pool: DescriptorPool::global(),
        }
    }
}

impl FileResolver for GoogleFileResolver {
    fn open_file(&self, name: &str) -> Result<File, Error> {
        match name {
            "google/protobuf/any.proto"
            | "google/protobuf/api.proto"
            | "google/protobuf/descriptor.proto"
            | "google/protobuf/duration.proto"
            | "google/protobuf/empty.proto"
            | "google/protobuf/field_mask.proto"
            | "google/protobuf/source_context.proto"
            | "google/protobuf/struct.proto"
            | "google/protobuf/timestamp.proto"
            | "google/protobuf/type.proto"
            | "google/protobuf/wrappers.proto"
            | "google/protobuf/compiler/plugin.proto" => {
                let file = self
                    .pool
                    .get_file_by_name(name)
                    .expect("well-known file not found");
                Ok(File::from_file_descriptor_proto(
                    file.file_descriptor_proto().clone(),
                ))
            }
            _ => Err(Error::file_not_found(name)),
        }
    }
}
