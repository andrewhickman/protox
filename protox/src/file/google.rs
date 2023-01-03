use super::{File, FileResolver};
use crate::Error;

/// An implementation of [`FileResolver`] which resolves well-known imports such as `google/protobuf/descriptor.proto`.
#[derive(Debug, Default)]
pub struct GoogleFileResolver {
    _priv: (),
}

impl GoogleFileResolver {
    /// Creates a new instance of [`GoogleFileResolver`].
    pub fn new() -> Self {
        Default::default()
    }
}

macro_rules! include_proto {
    ($name:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/protobuf/src/google/protobuf/",
            $name
        ))
    };
}

pub(crate) const ANY: &str = include_proto!("any.proto");
pub(crate) const API: &str = include_proto!("api.proto");
pub(crate) const DESCRIPTOR: &str = include_proto!("descriptor.proto");
pub(crate) const DURATION: &str = include_proto!("duration.proto");
pub(crate) const EMPTY: &str = include_proto!("empty.proto");
pub(crate) const FIELD_MASK: &str = include_proto!("field_mask.proto");
pub(crate) const SOURCE_CONTEXT: &str = include_proto!("source_context.proto");
pub(crate) const STRUCT: &str = include_proto!("struct.proto");
pub(crate) const TIMESTAMP: &str = include_proto!("timestamp.proto");
pub(crate) const TYPE: &str = include_proto!("type.proto");
pub(crate) const WRAPPERS: &str = include_proto!("wrappers.proto");
pub(crate) const COMPILER_PLUGIN: &str = include_proto!("compiler/plugin.proto");

impl FileResolver for GoogleFileResolver {
    fn open_file(&self, name: &str) -> Result<File, Error> {
        let source = match name {
            "google/protobuf/any.proto" => ANY,
            "google/protobuf/api.proto" => API,
            "google/protobuf/descriptor.proto" => DESCRIPTOR,
            "google/protobuf/duration.proto" => DURATION,
            "google/protobuf/empty.proto" => EMPTY,
            "google/protobuf/field_mask.proto" => FIELD_MASK,
            "google/protobuf/source_context.proto" => SOURCE_CONTEXT,
            "google/protobuf/struct.proto" => STRUCT,
            "google/protobuf/timestamp.proto" => TIMESTAMP,
            "google/protobuf/type.proto" => TYPE,
            "google/protobuf/wrappers.proto" => WRAPPERS,
            "google/protobuf/compiler/plugin.proto" => COMPILER_PLUGIN,
            _ => return Err(Error::file_not_found(name)),
        };

        File::from_source(name, source)
    }
}
