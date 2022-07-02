use std::{
    fs,
    io::Read,
    path::{self, Path, PathBuf},
};

use crate::{
    error::{DynSourceCode, ErrorKind},
    Error, MAX_FILE_LEN,
};

/// A strategy for opening imported files. The default implementation is [`FileImportResolver`] which uses the file system.
pub trait ImportResolver {
    /// Converts a file system path to a unique file name.
    fn resolve_path(&self, path: &Path) -> Option<String> {
        to_import_name(path)
    }

    /// Opens a file by its unique name.
    fn open(&self, name: &str) -> Result<File, Error>;
}

/// An opened protobuf source file, returned by [`ImportResolver::open`].
#[derive(Debug, Clone)]
pub struct File {
    /// If this is a physical file on the filesystem, the path to the file.
    pub path: Option<PathBuf>,
    /// The full content of the file as a UTF-8 string.
    pub content: String,
}

/// An implementation of [`ImportResolver`] which uses the filesystem, matching the behaviour of protoc.
#[derive(Debug)]
pub struct FileImportResolver {
    includes: Vec<PathBuf>,
}

impl FileImportResolver {
    /// Constructs a `FileImportResolver` from the set of include paths.
    ///
    /// # Errors
    ///
    /// Returns an error if the set of include paths is empty.
    pub fn new<I>(includes: I) -> Result<Self, Error>
    where
        I: IntoIterator,
        I::Item: AsRef<Path>,
    {
        let includes: Vec<_> = includes
            .into_iter()
            .map(|p| p.as_ref().to_owned())
            .collect();
        if includes.is_empty() {
            return Err(Error::from_kind(ErrorKind::NoIncludePaths));
        }

        Ok(FileImportResolver { includes })
    }
}

impl ImportResolver for FileImportResolver {
    /// Converts a file system path to a unique file name.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::path::Path;
    /// # use protox::{FileImportResolver, ImportResolver};
    /// let resolver = FileImportResolver::new(&["/path/to/include"]).unwrap();
    /// assert_eq!(resolver.resolve_path(Path::new("/path/to/include/dir/foo.proto")), Some("dir/foo.proto".to_owned()));
    /// assert_eq!(resolver.resolve_path(Path::new("dir/foo.proto")), Some("dir/foo.proto".to_owned()));
    /// assert_eq!(resolver.resolve_path(Path::new("../foo.proto")), None);
    /// ```
    fn resolve_path(&self, path: &Path) -> Option<String> {
        for include in &self.includes {
            if let Some(relative_path) = strip_prefix(path, include) {
                if let Some(name) = to_import_name(relative_path) {
                    return Some(name);
                } else {
                    continue;
                }
            }
        }

        to_import_name(path)
    }

    fn open(&self, name: &str) -> Result<File, Error> {
        for include in &self.includes {
            let candidate_path = include.join(name);
            match read_file(&candidate_path) {
                Ok(content) => {
                    return Ok(File {
                        path: Some(candidate_path),
                        content,
                    })
                }
                Err(err) if err.is_file_not_found() => continue,
                Err(err) => return Err(err),
            }
        }

        Err(Error::from_kind(ErrorKind::ImportNotFound {
            name: name.to_owned(),
            src: DynSourceCode::default(),
            span: None,
        }))
    }
}

fn read_file(path: &Path) -> Result<String, Error> {
    let map_err = |err| {
        Error::from_kind(ErrorKind::OpenFile {
            path: path.to_owned(),
            err,
            src: DynSourceCode::default(),
            span: None,
        })
    };

    let file = fs::File::open(path).map_err(map_err)?;
    let metadata = file.metadata().map_err(map_err)?;

    if metadata.len() > MAX_FILE_LEN {
        return Err(Error::from_kind(ErrorKind::FileTooLarge {
            src: DynSourceCode::default(),
            span: None,
        }));
    }

    let mut buf = String::with_capacity(metadata.len() as usize);
    file.take(MAX_FILE_LEN)
        .read_to_string(&mut buf)
        .map_err(map_err)?;
    Ok(buf)
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

/// Modification of std::path::Path::strip_prefix which ignores '.' components and is case-insensitive on windows.
pub(crate) fn strip_prefix<'a>(path: &'a Path, prefix: &Path) -> Option<&'a Path> {
    Some(iter_after(path.components(), prefix.components())?.as_path())
}

pub(crate) fn ends_with(path: &Path, suffix: &Path) -> bool {
    iter_after(path.components().rev(), suffix.components().rev()).is_some()
}

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
