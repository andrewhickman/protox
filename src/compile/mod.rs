use std::{fmt::Write, path::Path, sync::Arc};

use miette::NamedSource;
use prost_types::{FileDescriptorProto, FileDescriptorSet};

use crate::{
    ast,
    check::NameMap,
    files::{File, FileMap, ImportResult},
    parse, Error, ErrorKind,
};

#[cfg(test)]
mod tests;

/// Options for compiling protobuf files.
#[derive(Debug)]
pub struct Compiler {
    file_map: FileMap,
    include_imports: bool,
    include_source_info: bool,
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
            file_map: FileMap::new(includes),
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
        let (resolved_include, name) = self
            .file_map
            .resolve_import_name(relative_path)
            .ok_or_else(|| {
                Error::new(ErrorKind::FileNotIncluded {
                    path: relative_path.to_owned(),
                })
            })?;

        let (source, include, path) = match self.file_map.resolve_import(&name) {
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
                    self.file_map[name.as_str()].is_root = true;
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
                return Err(Error::parse_errors(
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

        let (descriptor, name_map) = self.check_file(&name, &ast, source, &path)?;

        self.file_map.add(File {
            descriptor,
            name_map,
            name,
            include,
            path,
            is_root: true,
        });
        Ok(self)
    }

    /// Convert all added files into an instance of [`FileDescriptorSet`].
    ///
    /// Files are sorted topologically, with dependency files ordered before the files that import them.
    pub fn build_file_descriptor_set(&mut self) -> FileDescriptorSet {
        let file = if self.include_imports {
            self.file_map.iter().map(|f| f.descriptor.clone()).collect()
        } else {
            self.file_map
                .iter()
                .filter(|f| f.is_root)
                .map(|f| f.descriptor.clone())
                .collect()
        };

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

        let (source, include, path) = match self.file_map.resolve_import(&import.value.value) {
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
                return Err(Error::parse_errors(
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

        let (descriptor, name_map) = self.check_file(&import.value.value, &ast, source, &path)?;

        self.file_map.add(File {
            descriptor,
            name_map,
            name: import.value.value.clone(),
            include,
            path,
            is_root: false,
        });
        Ok(())
    }

    fn check_file(
        &self,
        name: &str,
        ast: &ast::File,
        source: Arc<str>,
        path: &Path,
    ) -> Result<(FileDescriptorProto, NameMap), Error> {
        let source_info = if self.include_source_info {
            Some(source.as_ref())
        } else {
            None
        };

        ast.to_file_descriptor(Some(name), source_info, Some(&self.file_map))
            .map_err(|errors| {
                Error::check_errors(errors, NamedSource::new(path.display().to_string(), source))
            })
    }
}
