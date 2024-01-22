use super::{File, FileResolver, ProtoxFileIO};
use crate::Error;
use std::path::Path;
use std::sync::Arc;

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

impl FileResolver for GoogleFileResolver {
    fn open_file(&self, name: &str, file_io: Arc<dyn ProtoxFileIO>) -> Result<File, Error> {
        let source = file_io.read_proto(Path::new(name)).map_err(Error::new)?;
        File::from_source(name, &source)
    }
}
