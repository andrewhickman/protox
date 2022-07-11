//! Handling of protobuf source files

mod chain;
mod google;
mod include;

pub use chain::ChainFileResolver;
pub use google::GoogleFileResolver;
pub use include::IncludeFileResolver;

pub(crate) use include::{check_shadow, path_to_file_name};

use std::path::{Path, PathBuf};

use crate::Error;

/// A strategy for locating protobuf source files. The default implementation is [`IncludeFileResolver`] which uses the file system.
pub trait FileResolver {
    /// Converts a file system path to a unique file name.
    fn resolve_path(&self, _path: &Path) -> Option<String> {
        None
    }

    /// Opens a file by its unique name.
    ///
    /// # Errors
    ///
    /// If the file is not found, the implementation should return [`Error::file_not_found`].
    fn open_file(&self, name: &str) -> Result<File, Error>;
}

/// An opened protobuf source file, returned by [`FileResolver::open`].
#[derive(Debug, Clone)]
pub struct File {
    /// If this is a physical file on the filesystem, the path to the file.
    pub path: Option<PathBuf>,
    /// The full content of the file as a UTF-8 string.
    pub content: String,
}

impl<T> FileResolver for Box<T>
where
    T: FileResolver + ?Sized,
{
    fn resolve_path(&self, path: &Path) -> Option<String> {
        (**self).resolve_path(path)
    }

    fn open_file(&self, name: &str) -> Result<File, Error> {
        (**self).open_file(name)
    }
}
