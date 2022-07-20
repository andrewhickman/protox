use std::path::{self, Path, PathBuf};

use crate::{error::ErrorKind, Error};

use super::{File, FileResolver};

/// An implementation of [`FileResolver`] which searches an include path on the file system.
#[derive(Debug)]
pub struct IncludeFileResolver {
    include: PathBuf,
}

impl IncludeFileResolver {
    /// Constructs a `IncludeFileResolver` that searches the given include path.
    pub fn new(include: PathBuf) -> Self {
        IncludeFileResolver { include }
    }
}

impl FileResolver for IncludeFileResolver {
    /// Converts a file system path to a unique file name.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::path::{Path, PathBuf};
    /// # use protox::file::{IncludeFileResolver, FileResolver};
    /// let resolver = IncludeFileResolver::new(PathBuf::from("/path/to/include"));
    /// assert_eq!(resolver.resolve_path(Path::new("/path/to/include/dir/foo.proto")), Some("dir/foo.proto".to_owned()));
    /// assert_eq!(resolver.resolve_path(Path::new("notincluded.proto")), None);
    /// ```
    fn resolve_path(&self, path: &Path) -> Option<String> {
        if let Some(relative_path) = strip_prefix(path, &self.include) {
            if let Some(name) = path_to_file_name(relative_path) {
                return Some(name);
            }
        }

        None
    }

    /// Opens a file by its unique name.
    ///
    /// Each include path is searched for a file with the given name and the first match is returned.
    ///
    /// # Errors
    ///
    /// If no matching file is found, an error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::{fs, path::PathBuf};
    /// # use protox::file::{IncludeFileResolver, FileResolver};
    /// # let tempdir = assert_fs::TempDir::new().unwrap();
    /// # std::env::set_current_dir(&tempdir).unwrap();
    /// fs::write("./foo.proto", "content").unwrap();
    ///
    /// let resolver = IncludeFileResolver::new(PathBuf::from("."));
    /// let file = resolver.open_file("foo.proto").unwrap();
    /// assert_eq!(file.path, Some(PathBuf::from("./foo.proto")));
    /// assert_eq!(file.content, "content");
    /// ```
    fn open_file(&self, name: &str) -> Result<File, Error> {
        let path = self.include.join(name);
        File::read(&path).map_err(|err| {
            if err.is_file_not_found() {
                Error::file_not_found(name)
            } else {
                err
            }
        })
    }
}

pub(crate) fn path_to_file_name(path: &Path) -> Option<String> {
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

pub(crate) fn check_shadow(actual_path: Option<&Path>, expected_path: &Path) -> Result<(), Error> {
    // actual_path is expected to be an include path concatenated with `expected_path`
    if let Some(actual_path) = actual_path {
        if !ends_with(actual_path, expected_path) {
            return Err(Error::from_kind(ErrorKind::FileShadowed {
                path: expected_path.to_owned(),
                shadow: actual_path.to_owned(),
            }));
        }
    }

    Ok(())
}

fn strip_prefix<'a>(path: &'a Path, prefix: &Path) -> Option<&'a Path> {
    Some(iter_after(path.components(), prefix.components())?.as_path())
}

fn ends_with(path: &Path, suffix: &Path) -> bool {
    iter_after(path.components().rev(), suffix.components().rev()).is_some()
}

/// Comparison of paths which ignores '.' components and is case-insensitive on windows.
fn iter_after<'a, 'b, I, J>(mut iter: I, mut prefix: J) -> Option<I>
where
    I: Iterator<Item = path::Component<'a>> + Clone,
    J: Iterator<Item = path::Component<'b>> + Clone,
{
    loop {
        let mut path_next = iter.clone();
        let mut prefix_next = prefix.clone();

        match (path_next.next(), prefix_next.next()) {
            (Some(path::Component::CurDir), _) => {
                iter = path_next;
            }
            (_, Some(path::Component::CurDir)) => {
                prefix = prefix_next;
            }
            (Some(ref l), Some(ref r)) if path_component_eq(l, r) => {
                iter = path_next;
                prefix = prefix_next;
            }
            (Some(_), Some(_)) => return None,
            (Some(_), None) => return Some(iter),
            (None, None) => return Some(iter),
            (None, Some(_)) => return None,
        }
    }
}

#[cfg(windows)]
fn path_component_eq(l: &path::Component, r: &path::Component) -> bool {
    l.as_os_str().eq_ignore_ascii_case(r.as_os_str())
}

#[cfg(not(windows))]
fn path_component_eq(l: &path::Component, r: &path::Component) -> bool {
    l == r
}
