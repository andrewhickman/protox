//! Handling of protobuf source files
mod include;

pub use include::IncludeFileResolver;

pub(crate) use include::check_shadow;

use std::path::{self, Path, PathBuf};

use crate::Error;

/// A strategy for locating protobuf source files. The default implementation is [`IncludeFileResolver`] which uses the file system.
pub trait FileResolver {
    /// Converts a file system path to a unique file name.
    fn resolve_path(&self, path: &Path) -> Option<String> {
        to_import_name(path)
    }

    /// Opens a file by its unique name.
    fn open(&self, name: &str) -> Result<File, Error>;
}

/// An opened protobuf source file, returned by [`FileResolver::open`].
#[derive(Debug, Clone)]
pub struct File {
    /// If this is a physical file on the filesystem, the path to the file.
    pub path: Option<PathBuf>,
    /// The full content of the file as a UTF-8 string.
    pub content: String,
}

fn to_import_name(path: &Path) -> Option<String> {
    let mut name = String::new();
    for component in path.components() {
        match component {
            path::Component::Normal(component) => {
                if let Some(component) = component.to_str() {
                    if !name.is_empty() {
                        name.push('/');
                    }
                    name.push_str(component);
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }

    Some(name)
}
