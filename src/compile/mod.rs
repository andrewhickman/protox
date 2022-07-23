use std::{
    collections::HashMap,
    fmt::{self, Write},
    ops::{Index, IndexMut},
    path::{Path, PathBuf},
};

use logos::Span;
use miette::NamedSource;
use prost::Message;

use crate::{
    check::{self, NameMap},
    error::{DynSourceCode, Error, ErrorKind},
    file::{
        check_shadow, path_to_file_name, ChainFileResolver, File, FileResolver, GoogleFileResolver,
        IncludeFileResolver,
    },
    index_to_i32, resolve_span, tag, transcode_file,
    types::{FileDescriptorProto, FileDescriptorSet},
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
    pub is_root: bool,
}

#[derive(Debug, Default)]
pub(crate) struct ParsedFileMap {
    files: Vec<ParsedFile>,
    file_names: HashMap<String, usize>,
}

impl Compiler {
    /// Create a new [`Compiler`] with default options and the given set of include paths.
    ///
    /// In addition to the given include paths, the [`Compiler`] instance will be able to import
    /// standard files like `google/protobuf/descriptor.proto`.
    pub fn new<I, P>(includes: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let mut resolver = ChainFileResolver::new();

        for include in includes {
            resolver.add(IncludeFileResolver::new(include.as_ref().to_owned()));
        }

        resolver.add(GoogleFileResolver::new());

        Ok(Compiler::with_file_resolver(resolver))
    }

    /// Create a new [`Compiler`] with a custom [`FileResolver`] for looking up imported files.
    pub fn with_file_resolver<R>(resolver: R) -> Self
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

    /// Set whether the output `FileDescriptorSet` should include source info.
    ///
    /// If set, the file descriptors returned by [`file_descriptor_set`](Compiler::file_descriptor_set) will have
    /// the [`FileDescriptorProto::source_code_info`](prost_types::FileDescriptorProto::source_code_info) field
    /// populated with source locations and comments.
    pub fn include_source_info(&mut self, yes: bool) -> &mut Self {
        self.include_source_info = yes;
        self
    }

    /// Set whether the output `FileDescriptorSet` should include imported files.
    ///
    /// By default, only files explicitly added with [`add_file`](Compiler::add_file) are returned by [`file_descriptor_set`](Compiler::file_descriptor_set).
    /// If this option is set, imported files are included too.
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
            check_shadow(parsed_file.path.as_deref(), relative_path)?;
            parsed_file.is_root = true;
            return Ok(self);
        }

        let file = self.resolver.open_file(&name).map_err(|err| {
            if err.is_file_not_found() {
                Error::from_kind(ErrorKind::FileNotIncluded {
                    path: relative_path.to_owned(),
                })
            } else {
                err
            }
        })?;
        check_shadow(file.path(), relative_path)?;

        let mut import_stack = vec![name.clone()];
        for (index, import) in file.descriptor.dependency.iter().enumerate() {
            self.add_import(
                import,
                resolve_span(
                    file.lines.as_ref(),
                    file.descriptor
                        .source_code_info
                        .as_ref()
                        .map(|s| s.location.as_slice())
                        .unwrap_or(&[]),
                    &[tag::file::DEPENDENCY, index_to_i32(index)],
                ),
                &mut import_stack,
                make_source(&name, file.path(), file.source()),
            )?;
        }
        drop(import_stack);

        let path = file.path.clone();
        let (descriptor, name_map) = self.check_file(&name, file)?;

        self.file_map.add(ParsedFile {
            descriptor,
            name_map,
            path,
            is_root: true,
        });
        Ok(self)
    }

    /// Convert all added files into an instance of [`FileDescriptorSet`](prost_types::FileDescriptorSet).
    ///
    /// Files are sorted topologically, with dependency files ordered before the files that import them.
    pub fn file_descriptor_set(&self) -> prost_types::FileDescriptorSet {
        let mut buf = Vec::new();

        let file = self
            .file_map
            .files
            .iter()
            .filter(|f| self.include_imports || f.is_root)
            .map(|f| {
                if self.include_source_info {
                    transcode_file(&f.descriptor, &mut buf)
                } else {
                    prost_types::FileDescriptorProto {
                        source_code_info: None,
                        ..transcode_file(&f.descriptor, &mut buf)
                    }
                }
            })
            .collect();

        prost_types::FileDescriptorSet { file }
    }

    /// Convert all added files into an instance of [`FileDescriptorSet`](prost_types::FileDescriptorSet) and encodes it.
    ///
    /// This is equivalent to `file_descriptor_set()?.encode_to_vec()`, with the exception that extension
    /// options are included.
    pub fn encode_file_descriptor_set(&self) -> Vec<u8> {
        let file = self
            .file_map
            .files
            .iter()
            .filter(|f| self.include_imports || f.is_root)
            .map(|f| {
                if self.include_source_info {
                    f.descriptor.clone()
                } else {
                    FileDescriptorProto {
                        source_code_info: None,
                        ..f.descriptor.clone()
                    }
                }
            })
            .collect();

        FileDescriptorSet { file }.encode_to_vec()
    }

    pub(crate) fn into_parsed_file_map(self) -> ParsedFileMap {
        self.file_map
    }

    fn add_import(
        &mut self,
        file_name: &str,
        span: Option<Span>,
        import_stack: &mut Vec<String>,
        import_src: DynSourceCode,
    ) -> Result<(), Error> {
        if import_stack.iter().any(|name| name == file_name) {
            let mut cycle = String::new();
            for import in import_stack {
                write!(&mut cycle, "{} -> ", import).unwrap();
            }
            write!(&mut cycle, "{}", file_name).unwrap();

            return Err(Error::from_kind(ErrorKind::CircularImport { cycle }));
        }

        if self.file_map.file_names.contains_key(file_name) {
            return Ok(());
        }

        let file = match self.resolver.open_file(file_name) {
            Ok(file) => file,
            Err(err) => return Err(err.add_import_context(import_src, span)),
        };

        import_stack.push(file_name.to_owned());
        for (index, import) in file.descriptor.dependency.iter().enumerate() {
            self.add_import(
                import,
                resolve_span(
                    file.lines.as_ref(),
                    file.descriptor
                        .source_code_info
                        .as_ref()
                        .map(|s| s.location.as_slice())
                        .unwrap_or(&[]),
                    &[tag::file::DEPENDENCY, index_to_i32(index)],
                ),
                import_stack,
                make_source(file_name, file.path(), file.source()),
            )?;
        }
        import_stack.pop();

        let path = file.path.clone();
        let (descriptor, name_map) = self.check_file(file_name, file)?;

        self.file_map.add(ParsedFile {
            descriptor,
            name_map,
            path,
            is_root: false,
        });
        Ok(())
    }

    fn check_file(
        &self,
        file_name: &str,
        file: File,
    ) -> Result<(FileDescriptorProto, NameMap), Error> {
        let path = file.path.as_deref();
        let source = file.source.as_deref();
        let name_map = NameMap::from_proto(&file.descriptor, &self.file_map, file.lines.as_ref())
            .map_err(|errors| {
            Error::check_errors(errors, make_source(file_name, path, source))
        })?;

        let mut descriptor = file.descriptor;
        if descriptor.name().is_empty() {
            descriptor.name = Some(file_name.to_owned());
        }

        check::resolve(&mut descriptor, file.lines.as_ref(), &name_map)
            .map_err(|errors| Error::check_errors(errors, make_source(file_name, path, source)))?;

        Ok((descriptor, name_map))
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

impl ParsedFile {
    pub fn name(&self) -> &str {
        self.descriptor.name()
    }
}

impl ParsedFileMap {
    fn add(&mut self, file: ParsedFile) {
        self.file_names
            .insert(file.name().to_owned(), self.files.len());
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

fn make_source(name: &str, path: Option<&Path>, source: Option<&str>) -> DynSourceCode {
    if let Some(source) = source {
        let name = match path {
            Some(path) => path.display().to_string(),
            None => name.to_owned(),
        };

        NamedSource::new(name, source.to_owned()).into()
    } else {
        DynSourceCode::default()
    }
}
