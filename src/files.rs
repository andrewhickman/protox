use std::{
    collections::HashMap,
    ffi::OsStr,
    fs,
    io::{self, Read},
    ops::{Index, IndexMut},
    path::{self, Path, PathBuf},
    sync::Arc,
};

use prost_types::FileDescriptorProto;

use crate::{check::NameMap, MAX_FILE_LEN};

#[derive(Debug)]
pub(crate) struct FileMap {
    includes: Vec<PathBuf>,
    files: Vec<File>,
    file_names: HashMap<String, usize>,
}

#[derive(Debug)]
pub(crate) struct File {
    pub descriptor: FileDescriptorProto,
    pub name_map: NameMap,
    pub include: PathBuf,
    pub path: PathBuf,
    pub name: String,
    pub is_root: bool,
}

#[derive(Debug)]
pub(crate) enum ImportResult {
    Found {
        include: PathBuf,
        path: PathBuf,
        source: Arc<str>,
    },
    AlreadyImported {
        include: PathBuf,
        path: PathBuf,
    },
    NotFound,
    OpenError {
        include: PathBuf,
        path: PathBuf,
        err: io::Error,
    },
}

impl FileMap {
    pub fn new(includes: Vec<PathBuf>) -> Self {
        FileMap {
            includes,
            files: Vec::new(),
            file_names: HashMap::new(),
        }
    }

    pub fn add(&mut self, file: File) {
        self.file_names.insert(file.name.clone(), self.files.len());
        self.files.push(file);
    }

    pub fn iter(&self) -> impl Iterator<Item = &File> {
        self.files.iter()
    }

    pub fn resolve_import(&self, name: &str) -> ImportResult {
        if let Some(file) = self.files.iter().find(|f| f.name == name) {
            return ImportResult::AlreadyImported {
                include: file.include.clone(),
                path: file.path.clone(),
            };
        }

        for include in &self.includes {
            let candidate_path = include.join(name);
            match read_file(&candidate_path) {
                Ok(source) => {
                    return ImportResult::Found {
                        include: include.to_owned(),
                        path: candidate_path,
                        source,
                    }
                }
                Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
                Err(err) => {
                    return ImportResult::OpenError {
                        include: include.to_owned(),
                        path: candidate_path,
                        err,
                    };
                }
            }
        }

        ImportResult::NotFound
    }

    pub fn resolve_import_name(&self, path: &Path) -> Option<(Option<&Path>, String)> {
        for include in &self.includes {
            if let Some(relative_path) = strip_prefix(path, include) {
                if let Some(import_name) = get_import_name(relative_path) {
                    return Some((Some(include), import_name));
                } else {
                    continue;
                }
            }
        }

        get_import_name(path).map(|import_name| (None, import_name))
    }
}

impl Index<usize> for FileMap {
    type Output = File;

    fn index(&self, index: usize) -> &Self::Output {
        &self.files[index]
    }
}

impl<'a> Index<&'a str> for FileMap {
    type Output = File;

    fn index(&self, index: &'a str) -> &Self::Output {
        &self.files[self.file_names[index]]
    }
}

impl<'a> IndexMut<&'a str> for FileMap {
    fn index_mut(&mut self, index: &'a str) -> &mut Self::Output {
        &mut self.files[self.file_names[index]]
    }
}

fn read_file(path: &Path) -> io::Result<Arc<str>> {
    let file = fs::File::open(path)?;
    let metadata = file.metadata()?;

    if metadata.len() > MAX_FILE_LEN {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "file too large"));
    }

    let mut buf = String::with_capacity(metadata.len() as usize);
    file.take(MAX_FILE_LEN).read_to_string(&mut buf)?;
    Ok(buf.into())
}

fn get_import_name(path: &Path) -> Option<String> {
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

/// Modification of std::path::Path::strip_prefix which ignores '.' components
fn strip_prefix<'a>(path: &'a Path, prefix: &Path) -> Option<&'a Path> {
    let mut path = path.components();
    let mut prefix = prefix.components();

    loop {
        let mut path_next = path.clone();
        let mut prefix_next = prefix.clone();

        match (path_next.next(), prefix_next.next()) {
            (Some(path::Component::CurDir), _) => {
                path = path_next;
            }
            (_, Some(path::Component::CurDir)) => {
                prefix = prefix_next;
            }
            (Some(ref x), Some(ref y)) if path_component_eq(x.as_os_str(), y.as_os_str()) => {
                path = path_next;
                prefix = prefix_next;
            }
            (Some(_), Some(_)) => return None,
            (Some(_), None) => return Some(path.as_path()),
            (None, None) => return Some(path.as_path()),
            (None, Some(_)) => return None,
        }
    }
}

#[cfg(windows)]
fn path_component_eq(l: &OsStr, r: &OsStr) -> bool {
    l.eq_ignore_ascii_case(r)
}

#[cfg(not(windows))]
fn path_component_eq(l: &OsStr, r: &OsStr) -> bool {
    l == r
}
