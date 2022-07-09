use std::{fmt, path::Path};

use super::{File, FileResolver};
use crate::Error;

/// An implementation of [`FileResolver`] which chains together several other resolvers.
///
/// When opening files, each resolver is searched in turn until the file is found.
#[derive(Default)]
pub struct ChainFileResolver {
    resolvers: Vec<Box<dyn FileResolver>>,
}

impl ChainFileResolver {
    /// Create a new, empty [`ChainFileResolver`].
    pub fn new() -> Self {
        Default::default()
    }

    /// Add a new resolver.
    ///
    /// The new resolver will be searched after all previously-added resolvers.
    pub fn add<F>(&mut self, resolver: F)
    where
        F: FileResolver + 'static,
    {
        self.resolvers.push(Box::new(resolver))
    }
}

impl FileResolver for ChainFileResolver {
    fn resolve_path(&self, path: &Path) -> Option<String> {
        for resolver in &self.resolvers {
            if let Some(name) = resolver.resolve_path(path) {
                return Some(name);
            }
        }

        None
    }

    fn open(&self, name: &str) -> Result<File, Error> {
        for resolver in &self.resolvers {
            match resolver.open(name) {
                Ok(file) => return Ok(file),
                Err(err) if err.is_file_not_found() => continue,
                Err(err) => return Err(err),
            }
        }

        Err(Error::file_not_found(name))
    }
}

impl fmt::Debug for ChainFileResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChainFileResolver").finish_non_exhaustive()
    }
}
