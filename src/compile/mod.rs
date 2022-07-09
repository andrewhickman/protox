use std::{
    collections::HashMap,
    fmt::{self, Write},
    ops::{Index, IndexMut},
    path::{Path, PathBuf},
    sync::Arc,
};

use prost_types::{FileDescriptorProto, FileDescriptorSet};

use crate::{
    ast,
    check::{check_with_names, NameMap},
    error::{DynSourceCode, Error, ErrorKind},
    file::{check_shadow, path_to_file_name, ChainFileResolver, FileResolver, IncludeFileResolver},
    parse, MAX_FILE_LEN,
};

#[cfg(test)]
mod tests;

/// Options for compiling protobuf files.
pub struct Compiler {
    resolver: Box<dyn FileResolver>,
    file_map: ParsedFileMap,
    include_imports: bool,
    include_source_info: bool,
}

#[derive(Debug)]
pub(crate) struct ParsedFile {
    pub descriptor: FileDescriptorProto,
    pub name_map: NameMap,
    pub path: Option<PathBuf>,
    pub name: String,
    pub is_root: bool,
}

#[derive(Debug, Default)]
pub(crate) struct ParsedFileMap {
    files: Vec<ParsedFile>,
    file_names: HashMap<String, usize>,
}

impl Compiler {
    /// Create a new [`Compiler`] with default options and the given non-empty set of include paths.
    pub fn new(includes: impl IntoIterator<Item = impl AsRef<Path>>) -> Result<Self, Error> {
        let mut resolver = ChainFileResolver::new();

        let mut any_includes = false;
        for include in includes {
            resolver.add(IncludeFileResolver::new(include.as_ref().to_owned()));
            any_includes = true;
        }

        if !any_includes {
            return Err(Error::from_kind(ErrorKind::NoIncludePaths));
        }

        Ok(Compiler::with_import_resolver(resolver))
    }

    /// Create a new [`Compiler`] with a custom [`FileResolver`] for looking up imported files.
    pub fn with_import_resolver<R>(resolver: R) -> Self
    where
        R: FileResolver + 'static,
    {
        Compiler {
            resolver: Box::new(resolver),
            file_map: Default::default(),
            include_imports: false,
            include_source_info: false,
        }
    }

    /// Set whether the output `FileDescriptorSet` should have source info such as source locations and comments included.
    pub fn include_source_info(&mut self, yes: bool) -> &mut Self {
        self.include_source_info = yes;
        self
    }

    /// Set whether the output `FileDescriptorSet` should include dependency files.
    pub fn include_imports(&mut self, yes: bool) -> &mut Self {
        self.include_imports = yes;
        self
    }

    /// Compile the file at the given path, and add it to this `Compiler` instance.
    ///
    /// If the path is absolute, or relative to the current directory, it must reside under one of the
    /// include paths. Otherwise, it is looked up relative to the given include paths in the same way as
    /// `import` statements.
    pub fn add_file(&mut self, relative_path: impl AsRef<Path>) -> Result<&mut Self, Error> {
        let relative_path = relative_path.as_ref();
        let name = match self
            .resolver
            .resolve_path(relative_path)
            .or_else(|| path_to_file_name(relative_path))
        {
            Some(name) => name,
            None => {
                return Err(Error::from_kind(ErrorKind::FileNotIncluded {
                    path: relative_path.to_owned(),
                }))
            }
        };

        if let Some(parsed_file) = self.file_map.get_mut(&name) {
            check_shadow(&parsed_file.path, relative_path)?;
            parsed_file.is_root = true;
            return Ok(self);
        }

        let file = self.resolver.open(&name).map_err(|err| {
            if err.is_file_not_found() {
                Error::from_kind(ErrorKind::FileNotIncluded {
                    path: relative_path.to_owned(),
                })
            } else {
                err
            }
        })?;
        check_shadow(&file.path, relative_path)?;

        if file.content.len() > (MAX_FILE_LEN as usize) {
            return Err(Error::from_kind(ErrorKind::FileTooLarge {
                src: DynSourceCode::default(),
                span: None,
            }));
        }

        let source: Arc<str> = file.content.into();
        let ast = match parse::parse(&source) {
            Ok(ast) => ast,
            Err(errors) => {
                return Err(Error::parse_errors(
                    errors,
                    DynSourceCode::from((file.path, source.clone())),
                ));
            }
        };

        let mut import_stack = vec![name.clone()];
        for import in &ast.imports {
            self.add_import(
                import,
                &mut import_stack,
                DynSourceCode::from((file.path.clone(), source.clone())),
            )?;
        }

        let (descriptor, name_map) = self.check_file(&name, &ast, source, &file.path)?;

        self.file_map.add(ParsedFile {
            descriptor,
            name_map,
            name,
            path: file.path,
            is_root: true,
        });
        Ok(self)
    }

    /// Convert all added files into an instance of [`FileDescriptorSet`].
    ///
    /// Files are sorted topologically, with dependency files ordered before the files that import them.
    pub fn file_descriptor_set(&self) -> FileDescriptorSet {
        let file = if self.include_imports {
            self.file_map
                .files
                .iter()
                .map(|f| f.descriptor.clone())
                .collect()
        } else {
            self.file_map
                .files
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
        import_src: DynSourceCode,
    ) -> Result<(), Error> {
        if import_stack.contains(&import.value.value) {
            let mut cycle = String::new();
            for import in import_stack {
                write!(&mut cycle, "{} -> ", import).unwrap();
            }
            write!(&mut cycle, "{}", import.value.value).unwrap();

            return Err(Error::from_kind(ErrorKind::CircularImport { cycle }));
        }

        if self.file_map.file_names.contains_key(&import.value.value) {
            return Ok(());
        }

        let file = match self.resolver.open(&import.value.value) {
            Ok(file) if file.content.len() > (MAX_FILE_LEN as usize) => {
                return Err(Error::from_kind(ErrorKind::FileTooLarge {
                    src: import_src,
                    span: Some(import.value.span.clone().into()),
                }));
            }
            Ok(file) => file,
            Err(err) => return Err(err.add_import_context(import_src, import.span.clone())),
        };

        let source: Arc<str> = file.content.into();
        let ast = match parse::parse(&source) {
            Ok(ast) => ast,
            Err(errors) => {
                return Err(Error::parse_errors(
                    errors,
                    DynSourceCode::from((file.path, source)),
                ));
            }
        };

        import_stack.push(import.value.value.clone());
        for import in &ast.imports {
            self.add_import(
                import,
                import_stack,
                DynSourceCode::from((file.path.clone(), source.clone())),
            )?;
        }
        import_stack.pop();

        let (descriptor, name_map) =
            self.check_file(&import.value.value, &ast, source, &file.path)?;

        self.file_map.add(ParsedFile {
            descriptor,
            name_map,
            name: import.value.value.clone(),
            path: file.path,
            is_root: false,
        });
        Ok(())
    }

    fn check_file(
        &self,
        name: &str,
        ast: &ast::File,
        source: Arc<str>,
        path: &Option<PathBuf>,
    ) -> Result<(FileDescriptorProto, NameMap), Error> {
        let source_info = if self.include_source_info {
            Some(source.as_ref())
        } else {
            None
        };

        check_with_names(ast, Some(name), source_info, &self.file_map)
            .map_err(|errors| Error::check_errors(errors, (path.clone(), source)))
    }
}

impl fmt::Debug for Compiler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Compiler")
            .field("file_map", &self.file_map)
            .field("include_imports", &self.include_imports)
            .field("include_source_info", &self.include_source_info)
            .finish_non_exhaustive()
    }
}

impl ParsedFileMap {
    fn add(&mut self, file: ParsedFile) {
        self.file_names.insert(file.name.clone(), self.files.len());
        self.files.push(file);
    }

    fn get_mut(&mut self, name: &str) -> Option<&mut ParsedFile> {
        match self.file_names.get(name).copied() {
            Some(i) => Some(&mut self.files[i]),
            None => None,
        }
    }
}

impl Index<usize> for ParsedFileMap {
    type Output = ParsedFile;

    fn index(&self, index: usize) -> &Self::Output {
        &self.files[index]
    }
}

impl<'a> Index<&'a str> for ParsedFileMap {
    type Output = ParsedFile;

    fn index(&self, index: &'a str) -> &Self::Output {
        &self.files[self.file_names[index]]
    }
}

impl<'a> IndexMut<&'a str> for ParsedFileMap {
    fn index_mut(&mut self, index: &'a str) -> &mut Self::Output {
        &mut self.files[self.file_names[index]]
    }
}
