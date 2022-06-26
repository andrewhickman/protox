use std::{
    fmt::Write,
    fs, io,
    path::{self, Component, Path, PathBuf},
    sync::Arc,
};

use miette::NamedSource;
use prost_types::FileDescriptorSet;

use crate::{ast, parse, Error, ErrorKind};

#[cfg(test)]
mod tests;

/// Options for compiling protobuf files.
#[derive(Debug)]
pub struct Compiler {
    includes: Vec<PathBuf>,
    files: Vec<File>,
    include_imports: bool,
    include_source_info: bool,
}

#[derive(Debug)]
struct File {
    ast: ast::File,
    source: Arc<str>,
    include: PathBuf,
    path: PathBuf,
    name: String,
}

#[derive(Debug)]
enum ImportResult {
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

impl Compiler {
    /// Create a new compiler with default options and the given non-empty set of include paths.
    pub fn new(includes: impl IntoIterator<Item = impl AsRef<Path>>) -> Result<Self, Error> {
        let includes: Vec<_> = includes
            .into_iter()
            .map(|path| path.as_ref().to_owned())
            .collect();
        if includes.is_empty() {
            return Err(Error::new(ErrorKind::NoIncludePaths));
        }
        Ok(Compiler {
            includes,
            files: Vec::new(),
            include_imports: false,
            include_source_info: false,
        })
    }

    /// Set whether the output `FileDescriptorSet` should have source info such as source locations and comments included.
    pub fn include_source_info(&mut self, yes: bool) -> &mut Self {
        self.include_source_info = yes;
        self
    }

    /// Set whether the output `FileDescriptorSet` should include dependency files.
    pub fn include_imports(&mut self, yes: bool) -> &mut Self {
        self.include_imports = yes;
        // TODO: implement it
        self
    }

    /// Compile the file at the given path, and add it to this `Compiler` instance.
    ///
    /// If the path is absolute, or relative to the current directory, it must reside under one of the
    /// include paths. Otherwise, it is looked up relative to the given include paths in the same way as
    /// `import` statements.
    pub fn add_file(&mut self, relative_path: impl AsRef<Path>) -> Result<&mut Self, Error> {
        let relative_path = relative_path.as_ref();
        let (resolved_include, name) =
            self.resolve_import_name(relative_path).ok_or_else(|| {
                Error::new(ErrorKind::FileNotIncluded {
                    path: relative_path.to_owned(),
                })
            })?;

        let (source, include, path) = match self.resolve_import(&name) {
            ImportResult::Found {
                include,
                path,
                source,
            } => {
                if resolved_include.is_some() && resolved_include != Some(&include) {
                    return Err(Error::new(ErrorKind::FileShadowed {
                        path: relative_path.to_owned(),
                        shadow: path,
                    }));
                } else {
                    (source, include, path)
                }
            }
            ImportResult::AlreadyImported { include, path } => {
                if resolved_include.is_some() && resolved_include != Some(&include) {
                    return Err(Error::new(ErrorKind::FileShadowed {
                        path: relative_path.to_owned(),
                        shadow: path,
                    }));
                } else {
                    return Ok(self);
                }
            }
            ImportResult::NotFound => {
                return Err(Error::new(ErrorKind::FileNotIncluded {
                    path: relative_path.to_owned(),
                }))
            }
            ImportResult::OpenError { include, path, err } => {
                if resolved_include.is_some() && resolved_include != Some(&include) {
                    return Err(Error::new(ErrorKind::FileShadowed {
                        path: relative_path.to_owned(),
                        shadow: path,
                    }));
                } else {
                    return Err(Error::new(ErrorKind::OpenFile { path, err }));
                }
            }
        };

        let ast = match parse::parse(&source) {
            Ok(ast) => ast,
            Err(errors) => {
                return Err(Error::parse_error(
                    errors,
                    NamedSource::new(path.display().to_string(), source.clone()),
                ));
            }
        };

        let mut import_stack = vec![name.clone()];
        for import in &ast.imports {
            self.add_import(
                import,
                &mut import_stack,
                NamedSource::new(path.display().to_string(), source.clone()),
            )?;
        }

        self.files.push(File {
            ast,
            source,
            name,
            include,
            path,
        });
        Ok(self)
    }

    /// Convert all added files into an instance of [`FileDescriptorSet`].
    ///
    /// Files are sorted topologically, with dependency files ordered before the files that import them.
    pub fn build_file_descriptor_set(&mut self) -> FileDescriptorSet {
        let file = self
            .files
            .iter()
            .map(|f| {
                let src = if self.include_source_info {
                    Some(f.source.as_ref())
                } else {
                    None
                };
                f.ast.to_file_descriptor(src)
            })
            .collect();

        // TODO: CHECK / RESOLVE

        FileDescriptorSet { file }
    }

    fn add_import(
        &mut self,
        import: &ast::Import,
        import_stack: &mut Vec<String>,
        src: NamedSource,
    ) -> Result<(), Error> {
        if import_stack.contains(&import.value.value) {
            let mut cycle = String::new();
            for import in import_stack {
                write!(&mut cycle, "{} -> ", import).unwrap();
            }
            write!(&mut cycle, "{}", import.value.value).unwrap();

            return Err(Error::new(ErrorKind::CircularImport { cycle }));
        }

        let (source, include, path) = match self.resolve_import(&import.value.value) {
            ImportResult::Found {
                include,
                path,
                source,
            } => (source, include, path),
            ImportResult::AlreadyImported { .. } => {
                return Ok(());
            }
            ImportResult::NotFound => {
                return Err(Error::new(ErrorKind::ImportNotFound {
                    name: import.value.value.clone(),
                    span: import.span.clone(),
                    src,
                }))
            }
            ImportResult::OpenError { path, err, .. } => {
                return Err(Error::new(ErrorKind::OpenImport {
                    path,
                    err,
                    src,
                    span: import.span.clone(),
                }));
            }
        };

        let ast = match parse::parse(&source) {
            Ok(ast) => ast,
            Err(errors) => {
                return Err(Error::parse_error(
                    errors,
                    NamedSource::new(path.display().to_string(), source.clone()),
                ));
            }
        };

        import_stack.push(import.value.value.clone());
        for import in &ast.imports {
            self.add_import(
                import,
                import_stack,
                NamedSource::new(path.display().to_string(), source.clone()),
            )?;
        }
        import_stack.pop();

        self.files.push(File {
            ast,
            source,
            name: import.value.value.clone(),
            include,
            path,
        });
        Ok(())
    }

    fn resolve_import_name(&self, path: &Path) -> Option<(Option<&Path>, String)> {
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

    fn resolve_import(&self, name: &str) -> ImportResult {
        if let Some(file) = self.files.iter().find(|f| f.name == name) {
            return ImportResult::AlreadyImported {
                include: file.include.clone(),
                path: file.path.clone(),
            };
        }

        for include in &self.includes {
            let candidate_path = include.join(name);
            match fs::read_to_string(&candidate_path) {
                Ok(source) => {
                    return ImportResult::Found {
                        include: include.to_owned(),
                        path: candidate_path,
                        source: source.into(),
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
            (Some(Component::CurDir), _) => {
                path = path_next;
            }
            (_, Some(Component::CurDir)) => {
                prefix = prefix_next;
            }
            (Some(ref x), Some(ref y)) if x == y => {
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
